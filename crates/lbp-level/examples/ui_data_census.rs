//! U4: read-only census of the slot list, pins, palette and packs readers over every
//! magic-confirmed `slot_list`/`pins`/`palette`/`packs` MAP row in a snapshot, grouped by
//! content hash. Output: a NEW JSON file under ignored `local-analysis/`; nothing else is
//! written. Usage: ui_data_census SNAPSHOT GAME-DIR NEW-JSON
//! Counting unit: distinct content hash. Statuses: complete, complete_with_trailing_bytes,
//! stopped, excluded_text_form, wrapper_error, unresolved, read_failed (docs/u4-slot-pin-palette.md).
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::slots::{self, UiData};
use lbp_level::Limits;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;
const MAX_FILES: usize = 20_000;
const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const TYPES: [&str; 4] = ["slot_list", "pins", "palette", "packs"];

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > slots::MAX_INPUT {
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

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: ui_data_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
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
        return Err("new output under ignored local-analysis required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    // hash -> (type, guids, paths)
    let mut by_hash: BTreeMap<String, (String, BTreeSet<String>, BTreeSet<String>)> =
        BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if TYPES.contains(&r.display_type()) {
            let e = by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_insert_with(|| {
                    (
                        r.display_type().to_owned(),
                        Default::default(),
                        Default::default(),
                    )
                });
            e.1.insert(r.map_row.guid.to_ascii_lowercase());
            e.2.insert(r.map_row.path.clone());
        }
    }
    if by_hash.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let mut rows = Vec::new();
    let mut total_bytes = 0u64;
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut stops = BTreeMap::<String, usize>::new();
    let mut heads = BTreeMap::<String, usize>::new();
    let mut items = BTreeMap::<&str, usize>::new();
    let mut slot_types = BTreeMap::<String, usize>::new();
    let (mut slots_named, mut slots_tagged, mut slots_total) = (0, 0, 0);
    for (sha, (ty, guids, paths)) in &by_hash {
        control.check()?;
        let mut row = json!({"sha1": sha, "type": ty, "guids": guids, "paths": paths});
        let mut status = |row: &mut serde_json::Value, s: &str| {
            row["status"] = json!(s);
            *statuses.entry(format!("{ty}: {s}")).or_default() += 1;
        };
        let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Ok(k) => k,
            Err(e) => {
                status(&mut row, "unresolved");
                row["reason"] = json!(e);
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
                status(&mut row, "read_failed");
                row["reason"] = json!(e);
                rows.push(row);
                continue;
            }
        };
        let report = match slots::read(&bytes, Limits::default(), &|| control.check().is_err()) {
            Ok(r) => r,
            Err(e) => {
                // A text-serialized file (e.g. a developer scratch file) is excluded, not
                // counted as a reader failure.
                let text = e == lbp_resource::binary::Error::Unsupported(slots::NON_BINARY_TEXT);
                status(
                    &mut row,
                    if text {
                        "excluded_text_form"
                    } else {
                        "wrapper_error"
                    },
                );
                row["reason"] = json!(e.to_string());
                rows.push(row);
                continue;
            }
        };
        *heads
            .entry(format!("{ty} 0x{:x}", report.revision.head))
            .or_default() += 1;
        row["head"] = json!(format!("0x{:x}", report.revision.head));
        row["compression"] = json!(report.revision.compression);
        row["checks"] = json!(report.checks);
        match &report.result {
            Err(stop) => {
                status(&mut row, "stopped");
                row["stop"] = json!(stop);
                row["stop_label"] = json!(stop.label());
                *stops.entry(format!("{ty}: {}", stop.label())).or_default() += 1;
            }
            Ok(data) => {
                let trailing = report.checks.trailing_bytes;
                status(
                    &mut row,
                    if trailing == 0 {
                        "complete"
                    } else {
                        "complete_with_trailing_bytes"
                    },
                );
                let slot_iter: Vec<&slots::Slot> = match data {
                    UiData::SlotList(l) => {
                        *items.entry("slots").or_default() += l.slots.len();
                        l.slots.iter().collect()
                    }
                    UiData::Packs { packs } => {
                        *items.entry("packs").or_default() += packs.len();
                        packs.iter().map(|p| &p.slot).collect()
                    }
                    UiData::Pins { pins } => {
                        *items.entry("pins").or_default() += pins.len();
                        row["pin_ids"] = json!(pins.iter().map(|p| p.id).collect::<Vec<_>>());
                        Vec::new()
                    }
                    UiData::Palette(p) => {
                        *items.entry("palette_plans").or_default() += p.plans.len();
                        row["plans"] = json!(p.plans.len());
                        row["converted_plans"] = json!(p.converted_plans.as_ref().map(Vec::len));
                        Vec::new()
                    }
                };
                for s in &slot_iter {
                    slots_total += 1;
                    slots_named += usize::from(!s.name.is_empty());
                    slots_tagged +=
                        usize::from(s.translation_tag.as_ref().is_some_and(|t| !t.is_empty()));
                    *slot_types
                        .entry(s.id.type_name().unwrap_or("unknown").to_owned())
                        .or_default() += 1;
                }
                // Small sample for the report; names are user-visible strings of the game.
                row["slot_sample"] = json!(slot_iter
                    .iter()
                    .take(3)
                    .map(|s| json!({"type": s.id.type_name(), "number": s.id.number,
                        "name": s.name, "tag": s.translation_tag}))
                    .collect::<Vec<_>>());
                row["items"] = json!(slot_iter.len());
            }
        }
        rows.push(row);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "U4",
        "toolkit_commit": "d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "scope": "every magic-confirmed slot_list/pins/palette/packs MAP row, grouped by content hash",
        "unit": "distinct content hash",
        "files": by_hash.len(),
        "status_counts": statuses,
        "stop_counts": stops,
        "head_counts": heads,
        "item_counts": items,
        "slots_total": slots_total, "slots_with_name": slots_named, "slots_with_translation_tag": slots_tagged,
        "slot_type_counts": slot_types,
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
    println!("files {}; status {:?}", by_hash.len(), statuses);
    println!("items {items:?}; slots {slots_total} (named {slots_named}, tagged {slots_tagged})");
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
