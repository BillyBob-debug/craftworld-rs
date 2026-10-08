//! U5: read-only census of the InventoryItemDetails / RPlan details reader over every
//! magic-confirmed `plan` MAP row in a snapshot, grouped by content hash. Output: a NEW
//! JSON file under ignored `local-analysis/`; nothing else is written.
//! Usage: plan_details_census SNAPSHOT GAME-DIR NEW-JSON
//! Counting unit: distinct content hash. Statuses: complete, complete_with_trailing_bytes,
//! no_details, stopped, wrapper_error, unresolved, read_failed (docs/u5-item-details.md).
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::item_details::{self, type_names, Placement, Ref};
use lbp_level::serial::hex;
use lbp_level::Limits;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;
const MAX_FILES: usize = 100_000;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > item_details::MAX_INPUT {
        return Err("archived size exceeds input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(
            key,
            bytes.len() as u64,
            (size - bytes.len()).min(PAGE),
            control,
        )?;
        if page.bytes.is_empty() {
            return Err("truncated source read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

fn under_local_analysis(p: &Path) -> Result<(), String> {
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if p.exists()
        || !p
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new output under ignored local-analysis required".into());
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: plan_details_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    under_local_analysis(output)?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut known_guids = BTreeSet::new();
    let mut known_hashes = BTreeSet::new();
    for r in &catalog.snapshot.resources {
        known_guids.insert(r.map_row.guid.to_ascii_lowercase());
        known_hashes.insert(r.map_row.sha1.to_ascii_lowercase());
        if r.display_type() == "plan" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    if by_hash.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let mut rows = Vec::new();
    let mut total_bytes = 0u64;
    let mut statuses = BTreeMap::<&str, usize>::new();
    let mut stops = BTreeMap::<String, usize>::new();
    let mut heads = BTreeMap::<String, usize>::new();
    let mut types = BTreeMap::<String, usize>::new();
    let mut counts = BTreeMap::<&str, usize>::new();
    for (sha, guids) in &by_hash {
        control.check()?;
        let mut row = json!({"sha1": sha, "guids": guids});
        let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Ok(k) => k,
            Err(e) => {
                row["status"] = json!("unresolved");
                row["reason"] = json!(e);
                *statuses.entry("unresolved").or_default() += 1;
                rows.push(row);
                continue;
            }
        };
        total_bytes += key.size;
        if total_bytes > MAX_TOTAL_BYTES {
            return Err("total byte budget".into());
        }
        let bytes = match read_all(&source, &key, &control) {
            Ok(b) => b,
            Err(e) => {
                row["status"] = json!("read_failed");
                row["reason"] = json!(e);
                *statuses.entry("read_failed").or_default() += 1;
                rows.push(row);
                continue;
            }
        };
        let report = match item_details::read_plan_details(&bytes, Limits::default(), &|| {
            control.check().is_err()
        }) {
            Ok(r) => r,
            Err(e) => {
                row["status"] = json!("wrapper_error");
                row["reason"] = json!(e.to_string());
                *statuses.entry("wrapper_error").or_default() += 1;
                rows.push(row);
                continue;
            }
        };
        let head = report.revision.head;
        *heads.entry(format!("0x{head:x}")).or_default() += 1;
        row["head"] = json!(format!("0x{head:x}"));
        let p = match report.result {
            Err(stop) => {
                row["status"] = json!("stopped");
                row["stop_label"] = json!(stop.label());
                row["stop"] = json!(stop);
                *statuses.entry("stopped").or_default() += 1;
                *stops.entry(stop.label()).or_default() += 1;
                rows.push(row);
                continue;
            }
            Ok(p) => p,
        };
        let Some(d) = p.details else {
            row["status"] = json!("no_details");
            *statuses.entry("no_details").or_default() += 1;
            rows.push(row);
            continue;
        };
        let st = if report.trailing_bytes == 0 {
            "complete"
        } else {
            "complete_with_trailing_bytes"
        };
        *statuses.entry(st).or_default() += 1;
        let mut count = |k: &'static str, v: bool| {
            if v {
                *counts.entry(k).or_default() += 1;
            }
        };
        let icon_resolved = d.icon.is_some_and(|i| {
            i.guid
                .is_some_and(|g| known_guids.contains(&format!("{g:08x}")))
                || i.sha1.is_some_and(|h| known_hashes.contains(&hex(&h)))
        });
        count("with_icon", d.icon.is_some());
        count("icon_resolved_in_catalog", icon_resolved);
        count("with_title_key", d.title_key.is_some_and(|k| k != 0));
        count("with_translation_tag", d.translation_tag.is_some());
        count("with_user_created", matches!(d.user_created, Ref::Value(_)));
        count("with_photo", matches!(d.photo, Ref::Value(_)));
        count("with_eyetoy", matches!(d.eyetoy, Ref::Value(_)));
        count(
            "with_creator",
            matches!(&d.creator, Ref::Value(c) if !c.is_empty()),
        );
        count(
            "shared_references",
            [
                matches!(d.creation_history, Ref::Shared),
                matches!(d.user_created, Ref::Shared),
                matches!(d.photo, Ref::Shared),
                matches!(d.eyetoy, Ref::Shared),
                matches!(d.creator, Ref::Shared),
            ]
            .contains(&true),
        );
        count(
            "placement_keys",
            matches!(p.placement, Some(Placement::Keys { .. })),
        );
        let names = type_names(d.type_bits, head);
        if names.is_empty() {
            *types.entry("(none)".into()).or_default() += 1;
        }
        for n in &names {
            *types.entry((*n).to_owned()).or_default() += 1;
        }
        row["status"] = json!(st);
        row["trailing_bytes"] = json!(report.trailing_bytes);
        row["types"] = json!(names);
        row["type_bits"] = json!(format!("0x{:08x}", d.type_bits));
        row["title_key"] = json!(d.title_key.map(|k| format!("{k:08x}")));
        row["description_key"] = json!(d.description_key.map(|k| format!("{k:08x}")));
        row["icon"] = json!(d.icon);
        row["icon_resolved"] = json!(icon_resolved);
        row["placement"] = json!(p.placement);
        row["indices"] = json!([d.location_index, d.category_index, d.primary_index]);
        row["tool_type"] = json!(d.tool_type);
        rows.push(row);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "U5",
        "toolkit_commit": "d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "scope": "every magic-confirmed plan MAP row, grouped by content hash",
        "unit": "distinct content hash",
        "files": by_hash.len(),
        "status_counts": statuses,
        "stop_counts": stops,
        "head_counts": heads,
        "detail_counts": counts,
        "type_counts": types,
        "archived_bytes_read": total_bytes,
        "rows": rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("plans {}; status {:?}", by_hash.len(), statuses);
    println!("details {counts:?}");
    for (k, n) in stops.iter().take(12) {
        println!("  stop {n:>4}  {k}");
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
