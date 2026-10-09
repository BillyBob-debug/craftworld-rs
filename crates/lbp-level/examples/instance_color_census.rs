//! B176: read-only census of per-instance colors (`lbp_level::instance_color`) over every
//! magic-confirmed `level` MAP row and every magic-confirmed `plan` MAP row of a snapshot,
//! each grouped by content hash. Per mesh key: histogram of RENDER_MESH editorColor (packed
//! ARGB hex) with Thing counts and distinct file counts; per material key: PShape.color.
//! Output: a NEW JSON file under an ignored `local-analysis/` directory; nothing else is
//! written. Usage: instance_color_census SNAPSHOT GAME-DIR[;GAME-DIR] NEW-JSON
//! Counting unit: one Thing per containing level (or plan) content hash. Partially read
//! files contribute the Things read before the stop. docs/instance-color-census.md.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_level::instance_color::{Histogram, InstanceColors, SCOPE};
use lbp_level::Limits;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const MAX_FILES: usize = 100_000;
const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;

fn hist_json(h: &Histogram) -> Value {
    json!({"totals": h.totals(), "keys": h.keys()})
}

fn section(c: &InstanceColors, files: usize, statuses: &BTreeMap<&str, usize>) -> Value {
    json!({
        "files": files,
        "status_counts": statuses,
        "render_mesh_by_mesh": hist_json(&c.meshes),
        "shape_by_gfx_material": hist_json(&c.shapes_by_gfx_material),
        "shape_by_physics_material": hist_json(&c.shapes_by_physics_material),
    })
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: instance_color_census SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    let parent_ok = output
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "local-analysis");
    if output.exists() || !parent_ok {
        return Err("new output under an ignored local-analysis directory required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut levels: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut plans: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        let set = match r.display_type() {
            "level" => &mut levels,
            "plan" => &mut plans,
            _ => continue,
        };
        set.entry(r.map_row.sha1.to_ascii_lowercase())
            .or_default()
            .insert(r.map_row.path.clone());
    }
    if levels.is_empty() || levels.len() + plans.len() > MAX_FILES {
        return Err("file count budget".into());
    }
    let limits = Limits {
        max_depth: 4096,
        ..Limits::default()
    };
    let mut total_bytes = 0u64;
    let mut out = BTreeMap::new();
    for (kind, set) in [("levels", &levels), ("plans", &plans)] {
        let mut colors = InstanceColors::default();
        let mut statuses = BTreeMap::<&str, usize>::new();
        for sha in set.keys() {
            control.check()?;
            let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
                Ok(k) => k,
                Err(_) => {
                    *statuses.entry("unresolved").or_default() += 1;
                    continue;
                }
            };
            total_bytes += key.size;
            if total_bytes > MAX_TOTAL_BYTES {
                return Err("total byte budget".into());
            }
            let max = if kind == "levels" {
                lbp_level::level::MAX_INPUT
            } else {
                lbp_level::plan::MAX_INPUT
            };
            let bytes = match lbp_preview::level_scene::read_resource(&source, &key, &control, max)
            {
                Ok(b) => b,
                Err(_) => {
                    *statuses.entry("read_failed").or_default() += 1;
                    continue;
                }
            };
            let cancel = || control.check().is_err();
            let (stopped, things) = if kind == "levels" {
                match lbp_level::read(&bytes, limits, &cancel) {
                    Ok(r) => (r.stop.is_some(), r.things),
                    Err(_) => {
                        *statuses.entry("wrapper_error").or_default() += 1;
                        continue;
                    }
                }
            } else {
                match lbp_level::read_plan(&bytes, limits, &cancel) {
                    Ok(r) => (r.stop.is_some(), r.things),
                    Err(_) => {
                        *statuses.entry("wrapper_error").or_default() += 1;
                        continue;
                    }
                }
            };
            let st = match (stopped, things.is_empty()) {
                (false, _) => "complete",
                (true, true) => "stopped_without_things",
                (true, false) => "partial_things",
            };
            *statuses.entry(st).or_default() += 1;
            colors.add_things(sha, &things);
        }
        let s = section(&colors, set.len(), &statuses);
        println!(
            "{kind}: {} files; status {statuses:?}; render_mesh {}; shape_by_gfx_material {}",
            set.len(),
            s["render_mesh_by_mesh"]["totals"],
            s["shape_by_gfx_material"]["totals"]
        );
        out.insert(kind, s);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1,
        "milestone": "B176",
        "scope": SCOPE,
        "unit": "Thing with the part, per containing level (or plan) content hash; files = distinct content hashes",
        "color_format": "editorColor and packed PShape.color as 8 hex digits of the stored u32 (ARGB per Toolkit); ffffffff = default opaque white (-1); v4:r,g,b,a = pre-0x38a PShape float color",
        "evidence": "PRenderMesh.editorColor and PShape.color field layout CONFIRMED_FROM_COMMUNITY_CODE (docs/d80-thing-color.md E7); values CONFIRMED_FROM_LOCAL_DATA as stored; link to THING_COLOR not shown",
        "archived_bytes_read": total_bytes,
        "levels": out.remove("levels"),
        "plans": out.remove("plans"),
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
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
