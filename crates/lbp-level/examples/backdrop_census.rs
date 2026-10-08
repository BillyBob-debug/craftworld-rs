//! D119: read-only census of how in-band levels reference their backdrop: the PWorld
//! `backdrop` Thing (its parts, planGUID and PLevelSettings `backdropMesh`) and the PWorld
//! `backdropPlan` descriptor, plus the head of every referenced plan resource.
//! Output: a NEW JSON file under ignored `local-analysis/`; nothing else is written.
//! Usage: backdrop_census SNAPSHOT GAME-DIR[;GAME-DIR] NEW-JSON
//! Counting unit: distinct level content hash (levels the D107 reader completes).
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::serial::Descriptor;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const PAGE: usize = 4096;

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
    max: usize,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > max {
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

fn guid(d: &Option<Descriptor>) -> Option<String> {
    d.as_ref().and_then(|d| d.guid).map(|g| format!("{g:08x}"))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: backdrop_census SNAPSHOT GAME-DIR NEW-JSON".into());
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
    let mut levels: BTreeMap<String, String> = BTreeMap::new();
    let mut paths: BTreeMap<String, String> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        paths.insert(r.map_row.guid.to_ascii_lowercase(), r.map_row.path.clone());
        if r.display_type() == "level" {
            levels
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_insert_with(|| r.map_row.path.clone());
        }
    }
    let mut rows = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut plans = BTreeSet::new();
    let mut meshes = BTreeSet::new();
    for (sha, path) in &levels {
        control.check()?;
        let Ok(key) = lbp_preview::resolve(&catalog, None, Some(sha)) else {
            continue;
        };
        let bytes = read_all(&source, &key, &control, lbp_level::level::MAX_INPUT)?;
        let Ok(report) = lbp_level::read(&bytes, Default::default(), &|| false) else {
            continue;
        };
        if report.stop.is_some() {
            continue;
        }
        *counts.entry("levels_complete".into()).or_default() += 1;
        let world = report.things.first().and_then(|t| t.data.world.as_ref());
        let Some(world) = world else {
            *counts.entry("no_world_part".into()).or_default() += 1;
            continue;
        };
        let backdrop = world.backdrop.map(|i| &report.things[i]);
        let thing_plan = backdrop
            .and_then(|t| t.plan_guid)
            .map(|g| format!("{g:08x}"));
        let mesh = backdrop
            .and_then(|t| t.data.level_settings.as_ref())
            .and_then(|l| guid(&l.backdrop_mesh));
        let plan = guid(&world.backdrop_plan);
        let key_of = |name: &str, v: bool| format!("{name}: {}", if v { "set" } else { "unset" });
        *counts
            .entry(key_of("backdrop_thing", backdrop.is_some()))
            .or_default() += 1;
        *counts
            .entry(key_of("backdrop_new_thing", world.backdrop_new.is_some()))
            .or_default() += 1;
        *counts
            .entry(key_of("backdrop_plan", plan.is_some()))
            .or_default() += 1;
        *counts
            .entry(key_of("backdrop_mesh", mesh.is_some()))
            .or_default() += 1;
        *counts
            .entry(format!(
                "thing_plan_guid == backdrop_plan: {}",
                thing_plan.is_some() && thing_plan == plan
            ))
            .or_default() += 1;
        if let Some(p) = &plan {
            plans.insert(p.clone());
        }
        if let Some(m) = &mesh {
            meshes.insert(m.clone());
        }
        rows.push(json!({
            "sha1": sha,
            "path": path,
            "head": format!("0x{:x}", report.revision.head),
            "backdrop_thing": backdrop.map(|t| t.index),
            "backdrop_parts": backdrop.map(|t| t.parts.clone()),
            "backdrop_thing_plan_guid": thing_plan,
            "backdrop_mesh": mesh,
            "backdrop_plan": plan,
            "backdrop_new_thing": world.backdrop_new,
            "backdrop_new_plan": guid(&world.backdrop_new_plan),
        }));
    }
    let mut plan_rows = Vec::new();
    let mut plan_heads = BTreeMap::<String, usize>::new();
    for p in &plans {
        let g = u32::from_str_radix(p, 16).map_err(|e| e.to_string())?;
        let bytes = lbp_preview::resolve(&catalog, Some(g), None)
            .and_then(|k| read_all(&source, &k, &control, lbp_level::plan::MAX_INPUT));
        let head = match &bytes {
            Ok(b) if b.len() >= 8 => format!(
                "{}/0x{:x}",
                String::from_utf8_lossy(&b[..4]),
                u32::from_be_bytes(b[4..8].try_into().unwrap())
            ),
            Ok(_) => "short".into(),
            Err(e) => format!("unresolved: {e}"),
        };
        *plan_heads.entry(head.clone()).or_default() += 1;
        let mut row = json!({"guid": p, "path": paths.get(p), "head": head});
        if let Ok(b) = &bytes {
            match lbp_level::read_plan(b, Default::default(), &|| false) {
                Ok(rep) => {
                    let mut parts = BTreeMap::<&str, usize>::new();
                    for t in &rep.things {
                        for part in &t.parts {
                            *parts.entry(part).or_default() += 1;
                        }
                    }
                    let status = match (&rep.stop, rep.trailing_thing_bytes) {
                        (None, Some(0)) => "complete".to_string(),
                        (None, n) => format!("complete_with_trailing {n:?}"),
                        (Some(s), _) => format!("stopped: {s:?}"),
                    };
                    *counts
                        .entry(format!("plan {}", status.split(':').next().unwrap()))
                        .or_default() += 1;
                    row["plan_status"] = json!(status);
                    row["things"] = json!(rep.things.len());
                    row["roots"] = json!(rep.roots.len());
                    row["parts"] = json!(parts);
                    row["render_mesh_things"] = json!(rep
                        .things
                        .iter()
                        .filter(|t| t.data.render_mesh.is_some())
                        .count());
                    row["shape_things"] =
                        json!(rep.things.iter().filter(|t| t.data.shape.is_some()).count());
                }
                Err(e) => {
                    *counts.entry("plan wrapper error".into()).or_default() += 1;
                    row["plan_status"] = json!(format!("wrapper: {e}"));
                }
            }
        }
        plan_rows.push(row);
    }
    let value = json!({
        "schema_version": 1,
        "milestone": "D119",
        "unit": "distinct level content hash (D107-complete levels)",
        "counts": counts,
        "distinct_backdrop_plans": plans.len(),
        "distinct_backdrop_meshes": meshes.len(),
        "plan_heads": plan_heads,
        "plans": plan_rows,
        "backdrop_meshes": meshes.iter().map(|m| json!({"guid": m, "path": paths.get(m)})).collect::<Vec<_>>(),
        "rows": rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("counts {counts:?}");
    println!(
        "distinct backdrop plans {}, meshes {}; plan heads {plan_heads:?}",
        plans.len(),
        meshes.len()
    );
    Ok(())
}

fn main() {
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("spawn census worker");
    if let Err(e) = worker.join().expect("census worker panicked") {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
