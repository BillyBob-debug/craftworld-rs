//! D81: read-only corpus-wide mesh census (viewable / all-diffuse), output stays ignored.
//! Scope: every MAP row whose confirmed type is `mesh`, grouped by content hash and
//! loaded once per hash with default preview options; outcomes apply to every GUID
//! naming that hash (asset unit: distinct catalog GUID). Viewable and all-diffuse use
//! the compatibility_census definitions (`lbp_preview::census`). Complete is not
//! certified (needs a visual check). Mesh inspections whose hash has no MAP row are
//! excluded and counted. Optional `--hashes MANIFEST` restricts to a sample manifest
//! (`samples[].sha1`) for cross-checks against compatibility_census.
//! D85: opt-in `--animation-base` and `--instance-rgba R,G,B,A` (parsed by
//! `lbp_preview::census`, same values as compatibility_census). Without them the output
//! is unchanged (schema 1); with them the report is schema 2, the scope names the
//! opt-ins and `preview_options` records them.
//! Asset finish: opt-in `--blockers` adds, per mesh, the nonempty groups without a
//! diffuse binding (material GUID/hash + refusal reason) and a top-level
//! `blocking_materials` ranking: for each material key, the catalog GUIDs of meshes it
//! blocks and of meshes it alone blocks (fixing it alone makes them all-diffuse).
//! Ranking only; not an asset count. Without the flag the output is unchanged.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::census::{
    apply_preview_flag, coverage, geometry_available, preview_scope_suffix, CorpusTally,
    MeshOutcome,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

const MAX_MESH_HASHES: usize = 20_000;
/// Aggregate stored size of resolved mesh keys (oversized meshes read only a header).
const MAX_AGGREGATE_MESH_BYTES: u64 = 1024 * 1024 * 1024;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        return Err(
            "corpus_mesh_census SNAPSHOT GAME-DIR NEW-JSON [--hashes MANIFEST] [--max-seconds N] [--animation-base] [--instance-rgba R,G,B,A] [--blockers]"
                .into(),
        );
    }
    let output = Path::new(&args[3]);
    let mut cohort: Option<(String, BTreeSet<String>)> = None;
    let mut max_seconds = 7200u64;
    let mut options = lbp_preview::PreviewOptions::default();
    let mut blockers = false;
    let mut flags = args[4..].iter();
    while let Some(flag) = flags.next() {
        if apply_preview_flag(&mut options, flag, || flags.next().map(String::as_str))? {
            continue;
        }
        match flag.as_str() {
            "--hashes" => {
                let path = flags.next().ok_or("manifest required")?;
                let manifest: Value =
                    serde_json::from_reader(std::fs::File::open(path).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                let samples = manifest["samples"].as_array().ok_or("samples required")?;
                if samples.is_empty() || samples.len() > MAX_MESH_HASHES {
                    return Err("manifest sample budget".into());
                }
                let mut set = BTreeSet::new();
                for s in samples {
                    let sha = s["sha1"].as_str().ok_or("sha required")?;
                    if !set.insert(sha.to_ascii_lowercase()) {
                        return Err("duplicate manifest sample".into());
                    }
                }
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                cohort = Some((name, set));
            }
            "--blockers" => blockers = true,
            "--max-seconds" => {
                max_seconds = flags
                    .next()
                    .ok_or("seconds required")?
                    .parse()
                    .map_err(|_| "invalid seconds")?;
                if max_seconds == 0 || max_seconds > 86_400 {
                    return Err("time budget must be 1..=86400 s".into());
                }
            }
            _ => return Err("unknown option".into()),
        }
    }
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new ignored output required".into());
    }
    let started = Instant::now();
    let deadline = Duration::from_secs(max_seconds);
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;

    // Mesh rows by hash, deterministic (BTreeMap) and alias-preserving.
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut type_rows: BTreeMap<String, usize> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        let kind = r.display_type();
        if kind.contains("mesh") {
            *type_rows.entry(kind.to_owned()).or_default() += 1;
        }
        if kind != "mesh" {
            continue;
        }
        by_hash
            .entry(r.map_row.sha1.to_ascii_lowercase())
            .or_default()
            .insert(r.map_row.guid.to_ascii_lowercase());
    }
    // Mesh-typed inspections whose hash has no MAP row: not catalog assets.
    let unmapped: BTreeSet<String> = catalog
        .snapshot
        .resource_inspections
        .iter()
        .filter(|l| {
            l.inspection
                .confirmed_type
                .as_ref()
                .is_some_and(|k| k.name == "mesh")
        })
        .filter_map(|l| l.sha1.as_deref().map(str::to_ascii_lowercase))
        .filter(|sha| !catalog.by_hash.contains_key(sha))
        .collect();
    let corpus_hashes = by_hash.len();
    if let Some((_, set)) = &cohort {
        if let Some(missing) = set.iter().find(|s| !by_hash.contains_key(*s)) {
            return Err(format!("cohort hash {missing} is not a mesh MAP row"));
        }
        by_hash.retain(|sha, _| set.contains(sha));
    }
    if by_hash.is_empty() || by_hash.len() > MAX_MESH_HASHES {
        return Err("mesh hash budget".into());
    }

    let mut tally = CorpusTally::default();
    let mut rows = Vec::with_capacity(by_hash.len());
    let mut mesh_bytes = 0u64;
    let mut blocking: BTreeMap<String, Blocker> = BTreeMap::new();
    for (n, (sha, guids)) in by_hash.iter().enumerate() {
        control.check()?;
        if started.elapsed() > deadline {
            return Err(format!("time budget exceeded after {n} mesh hashes"));
        }
        let mut mesh_blocked: Vec<Value> = Vec::new();
        let outcome = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Err(e) => MeshOutcome::Failed(e),
            Ok(key) => {
                mesh_bytes = mesh_bytes
                    .checked_add(key.size)
                    .ok_or("mesh byte overflow")?;
                if mesh_bytes > MAX_AGGREGATE_MESH_BYTES {
                    return Err("aggregate mesh input budget".into());
                }
                match lbp_preview::load_mesh_groups_with_options(
                    &source, &catalog, &key, &control, options,
                ) {
                    Err(e) => MeshOutcome::Failed(e),
                    Ok(preview) => {
                        let g = &preview.geometry;
                        if blockers {
                            mesh_blocked = blocked_groups(&preview);
                        }
                        let (coverage, neutral_reasons) = coverage(
                            geometry_available(&g.positions, &g.triangles),
                            g.primitive_triangles.iter().zip(&preview.bindings).map(
                                |(range, b)| {
                                    (range.len(), b.as_ref().map(|_| ()).map_err(String::as_str))
                                },
                            ),
                        );
                        MeshOutcome::Loaded {
                            coverage,
                            neutral_reasons,
                        }
                    }
                }
            }
        };
        tally.record(guids, &outcome)?;
        let mut row = match &outcome {
            MeshOutcome::Failed(e) => {
                json!({"sha1":sha,"guids":guids,"status":"failed","reason":e})
            }
            MeshOutcome::Loaded {
                coverage: c,
                neutral_reasons,
            } => {
                json!({"sha1":sha,"guids":guids,"status":if !c.geometry_available {"no_geometry"} else if c.all_diffuse() {"all_diffuse"} else {"viewable"},"nonempty_groups":c.nonempty_groups,"supported_groups":c.supported_groups,"neutral_reasons":neutral_reasons})
            }
        };
        if blockers {
            if let MeshOutcome::Loaded { coverage: c, .. } = &outcome {
                if c.geometry_available && !c.all_diffuse() {
                    record_blockers(&mut blocking, guids, &mesh_blocked);
                }
            }
            row["blocked_groups"] = Value::Array(mesh_blocked);
        }
        rows.push(row);
    }
    source.check_stable()?;
    let suffix = preview_scope_suffix(&options);
    let scope = match &cohort {
        Some((name, _)) => format!("cohort_manifest:{name}{suffix}"),
        None => format!("corpus_all_mesh_map_rows{suffix}"),
    };
    let opt_in = !suffix.is_empty();
    let mut report = json!({
        "schema_version": if opt_in { 2 } else { 1 },
        "tool": "corpus_mesh_census",
        "scope": scope,
        "scope_note": if opt_in {
            "MAP rows with confirmed type mesh (snapshot), one load per content hash, opt-in PreviewOptions recorded in preview_options (supplied values, not verified game instances)"
        } else {
            "MAP rows with confirmed type mesh (snapshot), one load per content hash, default PreviewOptions (no animation base, no supplied instance color)"
        },
        "asset_unit": "distinct_catalog_guid",
        "definitions": {"viewable":"mesh loads and has a nonzero-area triangle (geometry availability, not visual fidelity)","all_diffuse":"viewable and every nonempty primitive group has a loaded diffuse binding (binding coverage, not visual verification)","complete":"not certified; requires manual visual check"},
        "budgets": {"max_mesh_hashes": MAX_MESH_HASHES, "max_aggregate_mesh_bytes": MAX_AGGREGATE_MESH_BYTES, "per_mesh_input_bytes": lbp_mesh::MAX_INPUT, "max_seconds": max_seconds},
        "snapshot_mesh_type_rows": type_rows,
        "corpus_mesh_hashes": corpus_hashes,
        "excluded_unmapped_mesh_hashes": unmapped.len(),
        "totals": {
            "mesh_hashes": tally.mesh_hashes,
            "catalog_guids": tally.catalog_guids.len(),
            "viewable_guids": tally.viewable_guids.len(),
            "all_diffuse_guids": tally.all_diffuse_guids.len(),
            "complete_verified_guids": 0,
            "viewable_hashes": tally.viewable_hashes,
            "all_diffuse_hashes": tally.all_diffuse_hashes,
            "failed_hashes": tally.failed_by_reason.values().sum::<usize>(),
            "no_geometry_hashes": tally.no_geometry_hashes,
            "viewable_without_groups_hashes": tally.viewable_without_groups,
            "nonempty_groups": tally.nonempty_groups,
            "supported_groups": tally.supported_groups,
            "mesh_input_bytes": mesh_bytes
        },
        "failed_by_reason": tally.failed_by_reason,
        "viewable_not_all_diffuse_by_group_reason": tally.partial_by_neutral_reason,
        "viewable_guid_list": tally.viewable_guids,
        "all_diffuse_guid_list": tally.all_diffuse_guids,
        "meshes": rows
    });
    if opt_in {
        report["preview_options"] = json!({
            "animation_base_enabled": options.animation_base,
            "animation_time": null,
            "supplied_instance_color_bits": options.instance_color.map(|c| c.bits),
            "supplied_instance_rgba": options.instance_color.map(|c| c.values()),
            "verified_game_instance": false
        });
    }
    if blockers {
        report["blockers_scope"] = json!(
            "ranking of material keys by meshes they keep from all-diffuse; not an asset count"
        );
        report["blocking_materials"] = rank_blockers(&blocking);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "{scope}: {} mesh hashes / {} GUIDs: {} viewable GUIDs, {} all-diffuse GUIDs, 0 complete; {} failed hashes, {} no-geometry; {} unmapped mesh hashes excluded; {:.0} s",
        tally.mesh_hashes,
        tally.catalog_guids.len(),
        tally.viewable_guids.len(),
        tally.all_diffuse_guids.len(),
        report["totals"]["failed_hashes"],
        tally.no_geometry_hashes,
        unmapped.len(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
#[derive(Default)]
struct Blocker {
    reasons: BTreeSet<String>,
    mesh_guids: BTreeSet<String>,
    sole_mesh_guids: BTreeSet<String>,
}

/// Nonempty primitive groups without a diffuse binding, in primitive order.
fn blocked_groups(preview: &lbp_preview::GroupedMesh) -> Vec<Value> {
    let g = &preview.geometry;
    g.metadata
        .primitives
        .iter()
        .zip(&g.primitive_triangles)
        .zip(&preview.bindings)
        .enumerate()
        .filter(|(_, ((_, range), _))| !range.is_empty())
        .filter_map(|(i, ((p, _), b))| {
            b.as_ref().err().map(|reason| {
                json!({"group":i,"key":material_key(p.material_guid, p.material_hash.as_deref()),"reason":reason})
            })
        })
        .collect()
}

fn material_key(guid: Option<u32>, hash: Option<&str>) -> String {
    match (guid, hash) {
        (Some(g), _) => format!("g{g}"),
        (None, Some(h)) => format!("h{}", h.to_ascii_lowercase()),
        (None, None) => "none".into(),
    }
}

fn record_blockers(
    map: &mut BTreeMap<String, Blocker>,
    guids: &BTreeSet<String>,
    groups: &[Value],
) {
    let keys: BTreeSet<&str> = groups.iter().filter_map(|v| v["key"].as_str()).collect();
    for v in groups {
        let (Some(key), Some(reason)) = (v["key"].as_str(), v["reason"].as_str()) else {
            continue;
        };
        let entry = map.entry(key.to_owned()).or_default();
        entry.reasons.insert(reason.to_owned());
        entry.mesh_guids.extend(guids.iter().cloned());
        if keys.len() == 1 {
            entry.sole_mesh_guids.extend(guids.iter().cloned());
        }
    }
}

/// Sorted by sole-blocked GUIDs, then blocked GUIDs, then key (deterministic).
fn rank_blockers(map: &BTreeMap<String, Blocker>) -> Value {
    let mut v: Vec<_> = map.iter().collect();
    v.sort_by(|a, b| {
        (b.1.sole_mesh_guids.len(), b.1.mesh_guids.len())
            .cmp(&(a.1.sole_mesh_guids.len(), a.1.mesh_guids.len()))
            .then_with(|| a.0.cmp(b.0))
    });
    Value::Array(
        v.into_iter()
            .map(|(k, b)| json!({"key":k,"reasons":b.reasons,"blocked_mesh_guids":b.mesh_guids.len(),"sole_blocked_mesh_guids":b.sole_mesh_guids.len(),"sole_blocked_guid_list":b.sole_mesh_guids}))
            .collect(),
    )
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
