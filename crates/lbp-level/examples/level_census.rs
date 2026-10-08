//! D107: read-only census of the independent RLevel/Thing reader over every level
//! resource (magic-confirmed `level` MAP rows, grouped by content hash) in a snapshot.
//! Output: a NEW JSON file under ignored `local-analysis/`; nothing else is written.
//! Usage: level_census SNAPSHOT GAME-DIR[;GAME-DIR] NEW-JSON
//! Counting unit: distinct level content hash. Statuses (see docs/d107-level-reader.md):
//! wrapper_error, out_of_band, stopped_in_things, world_things_walked, complete.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::{Limits, Stop};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;
const MAX_LEVELS: usize = 10_000;
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn read_all(source: &AttachedSource, key: &LocationKey, control: &LoadControl) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > lbp_level::level::MAX_INPUT {
        return Err("archived size exceeds level input budget".into());
    }
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let page = source.read_hex_checked(key, bytes.len() as u64, (size - bytes.len()).min(PAGE), control)?;
        if page.bytes.is_empty() {
            return Err("truncated source read".into());
        }
        bytes.extend(page.bytes);
    }
    Ok(bytes)
}

pub fn status(stop: &Option<Stop>, walked: bool, things: usize) -> &'static str {
    match stop {
        None => "complete",
        Some(_) if walked => "world_things_walked",
        Some(_) if things == 0 => "out_of_band",
        Some(_) => "stopped_in_things",
    }
}

fn stop_key(stop: &Stop) -> String {
    match stop {
        Stop::UnsupportedPart { part, .. } => format!("part {part}"),
        Stop::Unsupported { what, .. } => what.clone(),
        Stop::Invalid { what, .. } => format!("invalid: {what}"),
        Stop::Budget { what, .. } => format!("budget: {what}"),
        Stop::Cancelled => "cancelled".into(),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: level_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    let allowed = Path::new("local-analysis").canonicalize().map_err(|e| e.to_string())?;
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
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "level" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    if by_hash.is_empty() || by_hash.len() > MAX_LEVELS {
        return Err("level hash budget".into());
    }
    let mut rows = Vec::new();
    let mut total_bytes = 0u64;
    let mut statuses = BTreeMap::<&str, usize>::new();
    let mut stops = BTreeMap::<String, usize>::new();
    let mut first_part_stops = BTreeMap::<String, usize>::new();
    let mut part_things = BTreeMap::<&str, usize>::new();
    let mut part_levels = BTreeMap::<&str, usize>::new();
    let mut heads = BTreeMap::<String, usize>::new();
    let (mut things_started, mut things_complete, mut with_pos, mut with_mesh, mut with_shape) = (0, 0, 0, 0, 0);
    let mut max_depth = 0;
    for (sha, guids) in &by_hash {
        control.check()?;
        let mut row = json!({"sha1": sha, "guids": guids.len()});
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
        let report = match lbp_level::read(&bytes, Limits::default(), &|| control.check().is_err()) {
            Ok(r) => r,
            Err(e) => {
                row["status"] = json!("wrapper_error");
                row["reason"] = json!(e.to_string());
                *statuses.entry("wrapper_error").or_default() += 1;
                rows.push(row);
                continue;
            }
        };
        let rev = report.revision;
        *heads.entry(format!("0x{:x}", rev.head)).or_default() += 1;
        let st = status(&report.stop, report.world_things_walked.is_some(), report.things.len());
        *statuses.entry(st).or_default() += 1;
        let mut hist = BTreeMap::<&str, usize>::new();
        for t in &report.things {
            for p in &t.parts {
                *hist.entry(p).or_default() += 1;
            }
        }
        for (p, n) in &hist {
            *part_things.entry(p).or_default() += n;
            *part_levels.entry(p).or_default() += 1;
        }
        let complete = report.things.iter().filter(|t| t.complete).count();
        let pos = report.things.iter().filter(|t| t.data.pos.is_some()).count();
        let mesh = report.things.iter().filter(|t| t.data.render_mesh.is_some()).count();
        let shape = report.things.iter().filter(|t| t.data.shape.is_some()).count();
        things_started += report.things.len();
        things_complete += complete;
        with_pos += pos;
        with_mesh += mesh;
        with_shape += shape;
        max_depth = max_depth.max(report.max_depth);
        if let Some(s) = &report.stop {
            *stops.entry(stop_key(s)).or_default() += 1;
            if let Stop::UnsupportedPart { part, parts_revision, .. } = s {
                *first_part_stops
                    .entry(format!("{part} @ head 0x{:x} partsRevision 0x{parts_revision:x}", rev.head))
                    .or_default() += 1;
            }
        }
        row["head"] = json!(format!("0x{:x}", rev.head));
        row["branch"] = json!(format!("0x{:x}", rev.branch_id));
        row["compression"] = json!(rev.compression);
        row["status"] = json!(st);
        row["stop"] = json!(report.stop);
        row["stop_label"] = json!(report.stop.as_ref().map(Stop::label));
        row["expanded_bytes"] = json!(report.expanded_bytes);
        row["consumed_bytes"] = json!(report.consumed_bytes);
        row["cross_play_hashes"] = json!(report.cross_play_hashes);
        row["world_things_walked"] = json!(report.world_things_walked);
        row["things_started"] = json!(report.things.len());
        row["things_complete"] = json!(complete);
        row["things_with_pos"] = json!(pos);
        row["things_with_render_mesh"] = json!(mesh);
        row["things_with_shape"] = json!(shape);
        row["detached_shapes"] = json!(report.detached_shapes);
        row["max_depth"] = json!(report.max_depth);
        row["tags"] = json!(report.tags);
        row["part_histogram"] = json!(hist);
        rows.push(row);
    }
    source.check_stable()?;
    let mut top: Vec<_> = stops.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let value = json!({
        "schema_version": 1, "milestone": "D107",
        "toolkit_commit": "d6a2ef80b5a09999f08f23a803b516096055e5c9",
        "scope": "every magic-confirmed level MAP row, grouped by content hash",
        "unit": "distinct level content hash",
        "levels": by_hash.len(),
        "status_counts": statuses,
        "stop_counts": stops,
        "first_unsupported_part_counts": first_part_stops,
        "head_counts": heads,
        "things_started": things_started, "things_complete": things_complete,
        "things_with_pos": with_pos, "things_with_render_mesh": with_mesh, "things_with_shape": with_shape,
        "max_depth": max_depth,
        "part_thing_counts": part_things, "part_level_counts": part_levels,
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
    println!("levels {}; status {:?}", by_hash.len(), statuses);
    println!("things started {things_started}, complete {things_complete}, pos {with_pos}, render_mesh {with_mesh}, shape {with_shape}, max depth {max_depth}");
    for (k, n) in top.iter().take(12) {
        println!("  stop {n:>4}  {k}");
    }
    Ok(())
}

fn main() {
    // Inline references nest; the reader bounds depth, the stack must fit that bound.
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("spawn census worker");
    if let Err(e) = worker.join().expect("census worker panicked") {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
