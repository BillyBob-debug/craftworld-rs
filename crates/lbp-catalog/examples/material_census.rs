//! Bounded diagnostic coverage, no new formats, asset bytes or animation keys.
//! Existing reader provenance: docs/material-prefix.md, material-graph.md,
//! material-tail.md. Census policy/coverage/tests: docs/material-census.md.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_resource::{binary::Error, material};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};
const MAX_LOCATIONS: usize = 20_000;
// 128 MiB since D68: the 01.33 overlay has 8,159 material locations (01.00: 5,163).
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Default)]
struct Candidate {
    inspections: BTreeSet<usize>,
    map_rows: BTreeSet<usize>,
    hashes: BTreeSet<String>,
    magics: BTreeSet<String>,
    blocked: bool,
}
type Key = (String, u64, u64);
struct Plan {
    locations: BTreeMap<Key, Candidate>,
    map_rows: Vec<Value>,
}
fn material_row(r: &lbp_catalog::Resource) -> bool {
    r.confirmed_type.as_deref() == Some("graphics_material")
        || r.type_hint
            .as_ref()
            .is_some_and(|h| h.name == "graphics_material")
}
fn plan(catalog: &Catalog) -> Result<Plan, String> {
    let mut locations = BTreeMap::<Key, Candidate>::new();
    for (i, l) in catalog.snapshot.resource_inspections.iter().enumerate() {
        if l.inspection
            .confirmed_type
            .as_ref()
            .is_some_and(|k| k.name == "graphics_material")
            || l.inspection.magic_hex == "474d5462"
            || catalog.inspection_resources[i]
                .iter()
                .any(|&r| material_row(&catalog.snapshot.resources[r]))
        {
            let c = locations
                .entry((l.source_path.clone(), l.offset, l.size))
                .or_default();
            c.inspections.insert(i);
            c.map_rows
                .extend(catalog.inspection_resources[i].iter().copied());
            if let Some(hash) = &l.sha1 {
                c.hashes.insert(hash.clone());
            }
            c.magics.insert(l.inspection.magic_hex.clone());
            c.blocked |= l.inspection.status == "blocked_encrypted";
            if l.inspection.magic_hex.len() == 8 {
                if let Ok(raw) = u32::from_str_radix(&l.inspection.magic_hex, 16) {
                    c.blocked |= lbp_resource::is_protected_prefix(&raw.to_be_bytes());
                }
            }
            if locations.len() > MAX_LOCATIONS {
                return Err("Material location count budget".into());
            }
        }
    }
    let mut map_rows = Vec::new();
    for (i, r) in catalog.snapshot.resources.iter().enumerate() {
        if material_row(r) {
            let indices: Vec<_> = r.inspection_indices().collect();
            map_rows.push(json!({"catalog_row_index":i,"map_row":r.map_row,
                "type_hint":r.type_hint,"confirmed_type":r.confirmed_type,"inspection_indices":indices,
                "status":if indices.is_empty() {"unavailable_in_selected_snapshot"} else {"candidate_locations_present"},
                "archive_candidates":r.archive_candidates,"loose_candidates":r.loose_candidates}));
            if map_rows.len() > MAX_LOCATIONS {
                return Err("Material MAP row count budget".into());
            }
        }
    }
    map_rows.sort_by_key(|r| r["catalog_row_index"].as_u64().unwrap());
    Ok(Plan {
        locations,
        map_rows,
    })
}
fn outcome(error: &Error) -> Value {
    let status = match error {
        Error::Blocked => "blocked_protected",
        Error::Budget => "parser_budget",
        Error::Cancelled => "cancelled",
        Error::Unsupported(_) => "unsupported",
        Error::Invalid(_) => "invalid_for_current_reader",
    };
    json!({"status":status,"reason":error.to_string()})
}
fn parsed_stages(data: &[u8], cancel: &impl Fn() -> bool) -> Value {
    let tail_error = match material::inspect_tail(data, cancel) {
        Ok(v) => {
            return json!({"prefix":{"status":"parsed","revision":v.inspection.material.prefix.revision,
            "expanded_bytes":v.inspection.material.expanded_bytes,"wrap_s_raw":v.inspection.material.prefix.wrap_s_raw,"wrap_t_raw":v.inspection.material.prefix.wrap_t_raw},
            "graph":{"status":"parsed","box_count":v.inspection.graph.box_count,"wire_count":v.inspection.graph.wire_count},
            "tail":{"status":"parsed","sound_enum_raw":v.tail.sound_enum_raw,
            "parameter_animation_count":v.tail.parameter_animation_count,"unparsed_bytes":v.tail.unparsed_bytes,
            "component_value_counts":v.tail.parameter_animations.iter().map(|a|a.component_value_count).collect::<Vec<_>>()}})
        }
        Err(e) => outcome(&e),
    };
    let graph_error = match material::inspect_graph(data, cancel) {
        Ok(v) => {
            return json!({"prefix":{"status":"parsed","revision":v.material.prefix.revision,"expanded_bytes":v.material.expanded_bytes,"wrap_s_raw":v.material.prefix.wrap_s_raw,"wrap_t_raw":v.material.prefix.wrap_t_raw},
            "graph":{"status":"parsed","box_count":v.graph.box_count,"wire_count":v.graph.wire_count},"tail":tail_error})
        }
        Err(e) => outcome(&e),
    };
    let prefix = match material::inspect(data, cancel) {
        Ok(v) => {
            json!({"status":"parsed","revision":v.prefix.revision,"expanded_bytes":v.expanded_bytes,"wrap_s_raw":v.prefix.wrap_s_raw,"wrap_t_raw":v.prefix.wrap_t_raw})
        }
        Err(e) => outcome(&e),
    };
    json!({"prefix":prefix,"graph":graph_error,"tail":tail_error})
}
fn bump(counts: &mut BTreeMap<String, u64>, key: String) {
    *counts.entry(key).or_default() += 1;
}
fn census(
    plan: Plan,
    read: impl FnMut(&LocationKey, u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
) -> Value {
    census_with_limit(plan, read, cancel, MAX_TOTAL_BYTES)
}
fn census_with_limit(
    plan: Plan,
    mut read: impl FnMut(&LocationKey, u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
    max_total: u64,
) -> Value {
    let mut rows = Vec::new();
    let mut bytes_read = 0u64;
    let mut statuses = BTreeMap::new();
    let mut stages = BTreeMap::new();
    let mut revisions = BTreeMap::new();
    let mut graph_shapes = BTreeMap::new();
    let mut tails = BTreeMap::new();
    let mut sounds = BTreeMap::new();
    let mut sizes = BTreeMap::new();
    let mut eligible = 0;
    let mut attempted = 0;
    let mut cancelled = false;
    for ((path, offset, size), c) in plan.locations {
        let key = LocationKey { path, offset, size };
        let mut row = json!({"source_path":key.path,"offset":offset,"size":size,
            "inspection_indices":c.inspections,"map_row_indices":c.map_rows,"sha1_candidates":c.hashes,"snapshot_magic_candidates":c.magics});
        bump(
            &mut sizes,
            if size <= 4096 {
                "le_4KiB"
            } else if size <= 16384 {
                "le_16KiB"
            } else if size <= 65536 {
                "le_64KiB"
            } else {
                "gt_64KiB"
            }
            .into(),
        );
        let status = if c.blocked {
            "blocked_protected_no_read"
        } else if !c.magics.contains("474d5462") {
            "not_confirmed_clear_GMTb_no_read"
        } else {
            eligible += 1;
            if cancelled || cancel() {
                cancelled = true;
                "not_attempted_cancelled"
            } else if size > material::MAX_INPUT as u64 {
                "not_attempted_resource_budget"
            } else if bytes_read.checked_add(size).is_none_or(|n| n > max_total) {
                "not_attempted_aggregate_budget"
            } else {
                attempted += 1;
                let mut data = Vec::with_capacity(size as usize);
                let result = (|| -> Result<(), String> {
                    while data.len() < size as usize {
                        if cancel() {
                            cancelled = true;
                            return Err("cancelled".into());
                        }
                        // Read only magic first; never continue if input became protected.
                        let n = if data.is_empty() {
                            (size as usize).min(4)
                        } else {
                            (size as usize - data.len()).min(4096)
                        };
                        let page = read(&key, data.len() as u64, n)?;
                        if page.len() != n {
                            return Err("short_or_oversized_read".into());
                        }
                        bytes_read += n as u64;
                        data.extend(page);
                        if lbp_resource::is_protected_prefix(&data) {
                            return Err("protected_preflight".into());
                        }
                        if data.len() >= 4 && data[..4] != *b"GMTb" {
                            return Err("changed_or_unsupported_magic".into());
                        }
                    }
                    Ok(())
                })();
                match result {
                    Ok(()) => {
                        let metadata = parsed_stages(&data, &cancel);
                        for stage in ["prefix", "graph", "tail"] {
                            bump(
                                &mut stages,
                                format!("{stage}:{}", metadata[stage]["status"].as_str().unwrap()),
                            );
                        }
                        if metadata["prefix"]["status"] == "parsed" {
                            bump(&mut revisions, metadata["prefix"]["revision"].to_string());
                        }
                        if metadata["graph"]["status"] == "parsed" {
                            bump(
                                &mut graph_shapes,
                                format!(
                                    "{}:{}",
                                    metadata["graph"]["box_count"], metadata["graph"]["wire_count"]
                                ),
                            );
                        }
                        if metadata["tail"]["status"] == "parsed" {
                            bump(
                                &mut tails,
                                format!(
                                    "{}:{}",
                                    metadata["tail"]["parameter_animation_count"],
                                    metadata["tail"]["unparsed_bytes"]
                                ),
                            );
                            bump(&mut sounds, metadata["tail"]["sound_enum_raw"].to_string());
                        }
                        row["stages"] = metadata;
                        if row["stages"]["prefix"]["status"] == "cancelled"
                            || row["stages"]["graph"]["status"] == "cancelled"
                            || row["stages"]["tail"]["status"] == "cancelled"
                        {
                            cancelled = true;
                        }
                        "stage_inspection_completed"
                    }
                    Err(e) => {
                        row["read_error"] = json!(if matches!(
                            e.as_str(),
                            "cancelled"
                                | "short_or_oversized_read"
                                | "protected_preflight"
                                | "changed_or_unsupported_magic"
                        ) {
                            e.as_str()
                        } else {
                            "validated_source_read_failed"
                        });
                        if e == "cancelled" {
                            "read_cancelled"
                        } else if e == "protected_preflight" {
                            "blocked_protected_preflight"
                        } else {
                            "read_failed"
                        }
                    }
                }
            }
        };
        bump(&mut statuses, status.into());
        row["status"] = json!(status);
        rows.push(row);
    }
    let unavailable = plan
        .map_rows
        .iter()
        .filter(|r| r["status"] == "unavailable_in_selected_snapshot")
        .count();
    json!({"schema_version":2,"scope":"selected_snapshot_material_metadata_coverage_not_full_game",
        "limits":{"max_locations_and_map_rows":MAX_LOCATIONS,"max_resource_bytes":material::MAX_INPUT,"max_total_payload_read_bytes":max_total},
        "counts":{"unique_candidate_locations":rows.len(),"eligible_snapshot_clear_locations":eligible,"attempted_locations":attempted,
            "map_rows":plan.map_rows.len(),"unavailable_material_map_rows":unavailable,"payload_bytes_read":bytes_read,"cancelled":cancelled},
        "location_status_counts":statuses,"stage_status_counts":stages,"parsed_revision_counts":revisions,
        "graph_box_wire_counts":graph_shapes,"tail_animation_count_residual_counts":tails,"raw_sound_counts":sounds,"stored_size_bucket_counts":sizes,
        "map_rows":plan.map_rows,"locations":rows})
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("material_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("Output parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("New output under ignored local-analysis required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let plan = plan(&catalog)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let started = Instant::now();
    let mut report = census(
        plan,
        |key, offset, n| {
            source
                .read_hex_checked(key, offset, n, &control)
                .map(|p| p.bytes)
        },
        || control.check().is_err() || started.elapsed() > Duration::from_secs(120),
    );
    source.check_stable()?;
    report["source_scope"] = json!(catalog.snapshot.source_scope);
    report["build"] = catalog.snapshot.build.clone();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"counts":report["counts"],"location_status_counts":report["location_status_counts"],"stage_status_counts":report["stage_status_counts"]})
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("material_census: {e}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> Catalog {
        let mut s = lbp_catalog::demo_snapshot();
        s.resource_inspections[0]
            .inspection
            .confirmed_type
            .as_mut()
            .unwrap()
            .name = "graphics_material".into();
        s.resource_inspections[0].inspection.magic_hex = "474d5462".into();
        for i in [1, 2, 3, 4] {
            s.resources[i].type_hint = Some(lbp_catalog::Hint {
                name: "graphics_material".into(),
                basis: "extension_inference".into(),
            });
        }
        Catalog::build(s, &LoadControl::default()).unwrap()
    }
    #[test]
    fn selection_preserves_aliases_missing_hints_and_orphans_without_confirming_extensions() {
        let mut c = catalog();
        c.inspection_resources[0].clear(); // Authored orphan, selected by confirmed type.
        let copy = serde_json::from_value(
            serde_json::to_value(&c.snapshot.resource_inspections[0]).unwrap(),
        )
        .unwrap();
        c.snapshot.resource_inspections.push(copy);
        c.inspection_resources.push(vec![]); // Same key, distinct inspection observation.
        let p = plan(&c).unwrap();
        assert_eq!(p.locations.len(), 3);
        assert_eq!(p.map_rows.len(), 4);
        assert_eq!(
            p.locations
                .values()
                .find(|v| v.map_rows.is_empty())
                .unwrap()
                .inspections
                .len(),
            2
        );
        let alias = p
            .locations
            .values()
            .find(|v| v.map_rows.len() == 2)
            .unwrap();
        assert_eq!(alias.map_rows.iter().copied().collect::<Vec<_>>(), [1, 2]);
        let r = census(p, |_, _, n| Ok(vec![0; n]), || false);
        assert_eq!(r["counts"]["unavailable_material_map_rows"], 1);
        assert_eq!(r["location_status_counts"]["blocked_protected_no_read"], 1);
        assert_eq!(
            r["location_status_counts"]["not_confirmed_clear_GMTb_no_read"],
            1
        );
    }
    fn one(size: u64) -> Plan {
        let mut c = Candidate::default();
        c.magics.insert("474d5462".into());
        Plan {
            locations: BTreeMap::from([(("synthetic.farc".into(), 0, size), c)]),
            map_rows: vec![],
        }
    }
    #[test]
    fn budgets_protection_and_cancellation_prevent_payload_reads() {
        let mut c = catalog();
        c.snapshot.resource_inspections[0].inspection.magic_hex = "474d5465".into();
        let p = plan(&c).unwrap();
        let r = census(
            p,
            |_, _, _| panic!("known protected or unconfirmed must not read"),
            || false,
        );
        assert_eq!(r["location_status_counts"]["blocked_protected_no_read"], 2);
        for size in [material::MAX_INPUT as u64 + 1, MAX_TOTAL_BYTES + 1] {
            let r = census(one(size), |_, _, _| panic!("must not read"), || false);
            assert_eq!(r["counts"]["attempted_locations"], 0);
        }
        let r = census(one(80), |_, _, _| panic!("cancelled read"), || true);
        assert_eq!(r["counts"]["cancelled"], true);
        let r = census(one(80), |_, _, _| Ok(b"GMTe".to_vec()), || false);
        assert_eq!(r["counts"]["payload_bytes_read"], 4);
        assert_eq!(
            r["location_status_counts"]["blocked_protected_preflight"],
            1
        );
    }
    #[test]
    fn deterministic_order_errors_and_parser_outcomes_remain_distinct() {
        let a = census(one(4), |_, _, _| Ok(b"GMTb".to_vec()), || false);
        let b = census(one(4), |_, _, _| Ok(b"GMTb".to_vec()), || false);
        assert_eq!(a, b);
        assert_eq!(
            a["stage_status_counts"]["prefix:invalid_for_current_reader"],
            1
        );
        assert_eq!(
            outcome(&Error::Unsupported("revision"))["status"],
            "unsupported"
        );
        assert_eq!(outcome(&Error::Budget)["status"], "parser_budget");
        let r = census(
            one(80),
            |_, _, _| Err("external path-sensitive error".into()),
            || false,
        );
        assert_eq!(
            r["locations"][0]["read_error"],
            "validated_source_read_failed"
        );
    }
    #[test]
    fn aggregate_limit_mid_page_cancel_and_conflicting_identities_are_preserved() {
        let mut p = one(4);
        let mut c = Candidate::default();
        c.magics.insert("474d5462".into());
        c.hashes.extend(["a".repeat(40), "b".repeat(40)]);
        p.locations.insert(("z.farc".into(), 0, 4), c);
        let r = census_with_limit(p, |_, _, _| Ok(b"GMTb".to_vec()), || false, 4);
        assert_eq!(r["counts"]["payload_bytes_read"], 4);
        assert_eq!(
            r["location_status_counts"]["not_attempted_aggregate_budget"],
            1
        );
        assert_eq!(
            r["locations"][1]["sha1_candidates"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let flag = std::cell::Cell::new(false);
        let r = census(
            one(8),
            |_, _, _| {
                flag.set(true);
                Ok(b"GMTb".to_vec())
            },
            || flag.get(),
        );
        assert_eq!(r["location_status_counts"]["read_cancelled"], 1);
        assert_eq!(r["counts"]["payload_bytes_read"], 4);
        assert_eq!(r["counts"]["cancelled"], true);
    }
}
