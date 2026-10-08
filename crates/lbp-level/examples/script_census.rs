//! U10: read-only census of the RScript reader over every magic-confirmed `script` MAP
//! row in a snapshot, grouped by content hash. Output: a NEW JSON file under ignored
//! `local-analysis/` and, when LISTING-DIR is given, one NEW text listing per script in
//! that NEW directory under `local-analysis/` (derived research material, never commit).
//! Usage: script_census SNAPSHOT GAME-DIR NEW-JSON [NEW-LISTING-DIR]
//! Counting unit: distinct content hash. Statuses: complete, complete_with_trailing_bytes,
//! stopped, wrapper_error, unresolved, read_failed (docs/u10-script-reader.md).
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::script::{self, modifier_names, MNEMONICS};
use lbp_level::Limits;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;
const MAX_FILES: usize = 50_000;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > script::MAX_INPUT {
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

/// Class or super-class names that suggest front-end UI (docs/ui-research.md PC check 3).
const UI_WORDS: [&str; 8] = [
    "gooey",
    "poppet",
    "popit",
    "start_menu",
    "pod",
    "hud",
    "scoreboard",
    "frontend",
];

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
    if !(4..=5).contains(&args.len()) {
        return Err("usage: script_census SNAPSHOT GAME-DIR NEW-JSON [NEW-LISTING-DIR]".into());
    }
    let output = Path::new(&args[3]);
    under_local_analysis(output)?;
    let listing_dir = args.get(4).map(Path::new);
    if let Some(d) = listing_dir {
        under_local_analysis(d)?;
        std::fs::create_dir(d).map_err(|e| e.to_string())?;
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut by_hash: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "script" {
            let e = by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default();
            e.0.insert(r.map_row.guid.to_ascii_lowercase());
            e.1.insert(r.map_row.path.clone());
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
    let mut layout = BTreeMap::<&str, usize>::new();
    let mut opcodes = BTreeMap::<String, usize>::new();
    let mut checks_total = BTreeMap::<&str, usize>::new();
    let mut name_miss_kinds = BTreeMap::<&str, usize>::new();
    let mut name_miss_values = BTreeMap::<u32, usize>::new();
    let mut natives = BTreeMap::<String, BTreeSet<String>>::new();
    let mut ui_classes = BTreeSet::<String>::new();
    let (mut functions, mut native_functions, mut instructions) = (0usize, 0usize, 0usize);
    for (sha, (guids, paths)) in &by_hash {
        control.check()?;
        let mut row = json!({"sha1": sha, "guids": guids, "paths": paths});
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
        let report = match script::read(&bytes, Limits::default(), &|| control.check().is_err()) {
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
        row["compression"] = json!(report.revision.compression);
        let s = match report.result {
            Err(stop) => {
                row["status"] = json!("stopped");
                row["stop_label"] = json!(stop.label());
                row["stop"] = json!(stop);
                *statuses.entry("stopped").or_default() += 1;
                *stops.entry(stop.label()).or_default() += 1;
                rows.push(row);
                continue;
            }
            Ok(s) => s,
        };
        let st = if report.trailing_bytes == 0 {
            "complete"
        } else {
            "complete_with_trailing_bytes"
        };
        *statuses.entry(st).or_default() += 1;
        *layout
            .entry(if head & 0xffff >= 0x3d9 {
                "u16 ranges (>= 0x3d9)"
            } else {
                "i32 ranges (< 0x3d9)"
            })
            .or_default() += 1;
        let (text, checks) = s.listing();
        for (k, v) in [
            ("unknown_opcodes", checks.unknown_opcodes),
            ("name_index_out_of_range", checks.name_index_out_of_range),
            ("unnamed_type_refs", checks.unnamed_type_refs),
            (
                "function_range_out_of_range",
                checks.function_range_out_of_range,
            ),
            (
                "constant_index_out_of_range",
                checks.constant_index_out_of_range,
            ),
            (
                "member_index_out_of_range",
                checks.member_index_out_of_range,
            ),
        ] {
            *checks_total.entry(k).or_default() += v;
        }
        for (k, v) in &checks.name_out_of_range_by_kind {
            *name_miss_kinds.entry(k).or_default() += v;
        }
        for v in &checks.name_out_of_range_values {
            *name_miss_values.entry(*v).or_default() += 1;
        }
        for &bits in &s.bytecode {
            let name = MNEMONICS
                .get((bits & 0xff) as usize)
                .map_or_else(|| format!("?? 0x{:02x}", bits & 0xff), |m| (*m).to_owned());
            *opcodes.entry(name).or_default() += 1;
        }
        let native: Vec<String> = s
            .function_defs
            .iter()
            .filter(|f| f.modifiers & 2 != 0)
            .map(|f| s.name(f.name).unwrap_or("<name?>").to_owned())
            .collect();
        functions += s.function_defs.len();
        native_functions += native.len();
        instructions += s.bytecode.len();
        if !native.is_empty() {
            natives
                .entry(s.class_name.clone())
                .or_default()
                .extend(native.iter().cloned());
        }
        let lower = s.class_name.to_ascii_lowercase();
        if UI_WORDS.iter().any(|w| lower.contains(w)) {
            ui_classes.insert(s.class_name.clone());
        }
        row["status"] = json!(st);
        row["trailing_bytes"] = json!(report.trailing_bytes);
        row["class"] = json!(s.class_name);
        row["class_modifiers"] = json!(modifier_names(s.modifiers));
        row["super_class"] = json!(s.super_class);
        row["up_to_date_script_present"] = json!(s.up_to_date_script.is_some());
        row["functions"] = json!(s.function_defs.len());
        row["native_functions"] = json!(native.len());
        row["fields"] = json!(s.field_defs.len());
        row["instructions"] = json!(s.bytecode.len());
        row["strings_a"] = json!(s.strings_a.len());
        row["strings_w"] = json!(s.strings_w.len());
        row["depending_guids"] = json!(s.depending_guids.as_ref().map(Vec::len));
        row["listing_checks"] = json!(checks);
        if let Some(d) = listing_dir {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(d.join(format!("{sha}.txt")))
                .map_err(|e| e.to_string())?;
            f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        rows.push(row);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "U10",
        "toolkit_commit": "d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "scope": "every magic-confirmed script MAP row, grouped by content hash",
        "unit": "distinct content hash",
        "files": by_hash.len(),
        "status_counts": statuses,
        "stop_counts": stops,
        "head_counts": heads,
        "layout_counts": layout,
        "functions": functions, "native_functions": native_functions, "instructions": instructions,
        "listing_check_totals": checks_total,
        "name_out_of_range_by_kind": name_miss_kinds,
        "name_out_of_range_values_by_script_count": name_miss_values,
        "opcode_counts": opcodes,
        "native_functions_by_class": natives,
        "ui_named_classes": ui_classes,
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
    println!("scripts {}; status {:?}", by_hash.len(), statuses);
    println!("functions {functions} (native {native_functions}), instructions {instructions}; checks {checks_total:?}");
    println!(
        "heads {heads:?}; layouts {layout:?}; UI-named classes {}",
        ui_classes.len()
    );
    let mut values: Vec<_> = name_miss_values.iter().collect();
    values.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!(
        "name misses by kind {name_miss_kinds:?}; top values (index: scripts) {:?}",
        &values[..values.len().min(12)]
    );
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
