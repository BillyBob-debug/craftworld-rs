//! D119: compares `lbp_level::read_plan` with the Toolkit PlanThingOracle output
//! (scripts/PlanThingOracle.java) per plan and per Thing in tag order: UID, planGUID,
//! present parts, POS world matrix (bit-equal f32), RENDER_MESH mesh/anim GUIDs and bone
//! count, PLevelSettings backdropMesh, Thing count and consumed thingData bytes.
//! Read-only; prints a summary, writes nothing.
//! Usage: plan_parity SNAPSHOT GAME-DIR[;GAME-DIR] ORACLE-JSON
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::Value;
use std::path::Path;

fn hex(g: Option<u32>) -> Value {
    g.map_or(Value::Null, |g| Value::String(format!("{g:08x}")))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: plan_parity SNAPSHOT GAME-DIR ORACLE-JSON".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let oracle: Value =
        serde_json::from_str(&std::fs::read_to_string(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let plans = oracle["plans"].as_array().ok_or("oracle plans")?;
    let (mut agree, mut differ, mut things_compared) = (0usize, 0usize, 0usize);
    for p in plans {
        let guid = p["guid"].as_str().ok_or("guid")?;
        let g = u32::from_str_radix(guid, 16).map_err(|e| e.to_string())?;
        let key = lbp_preview::resolve(&catalog, Some(g), None)?;
        let bytes = lbp_preview::level_scene::read_resource(
            &source,
            &key,
            &control,
            lbp_level::plan::MAX_INPUT,
        )?;
        let rep = lbp_level::read_plan(&bytes, Default::default(), &|| false)
            .map_err(|e| format!("{guid}: {e}"))?;
        let mut diffs = Vec::new();
        if rep.stop.is_some() {
            diffs.push(format!("stopped {:?}", rep.stop));
        }
        let consumed = rep
            .thing_data_bytes
            .zip(rep.trailing_thing_bytes)
            .map(|(n, t)| n - t);
        if consumed.map(|c| c as u64) != p["consumed_thing_bytes"].as_u64() {
            diffs.push(format!("consumed {consumed:?}"));
        }
        if Some(rep.roots.len() as u64) != p["roots"].as_u64() {
            diffs.push("roots".into());
        }
        let ot = p["things"].as_array().ok_or("things")?;
        if ot.len() != rep.things.len() {
            diffs.push(format!("thing count {} vs {}", rep.things.len(), ot.len()));
        }
        for (t, o) in rep.things.iter().zip(ot) {
            things_compared += 1;
            let mut d = Vec::new();
            if t.uid.map(i64::from) != o["uid"].as_i64() {
                d.push("uid");
            }
            if hex(t.plan_guid) != o["plan_guid"] {
                d.push("plan_guid");
            }
            let parts: Vec<Value> = t
                .parts
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect();
            if Value::Array(parts) != o["parts"] {
                d.push("parts");
            }
            let world = t.data.pos.as_ref().map(|p| p.world);
            let oworld = o["world"].as_array().map(|a| {
                a.iter()
                    .map(|v| v.as_f64().unwrap_or(f64::NAN) as f32)
                    .collect::<Vec<_>>()
            });
            let same = match (world, oworld) {
                (None, None) => true,
                (Some(w), Some(ow)) => {
                    ow.len() == 16 && w.iter().zip(&ow).all(|(a, b)| a.to_bits() == b.to_bits())
                }
                _ => false,
            };
            if !same {
                d.push("world");
            }
            let rm = t.data.render_mesh.as_ref();
            if hex(rm.and_then(|m| m.mesh.as_ref()).and_then(|m| m.guid)) != o["mesh"] {
                d.push("mesh");
            }
            if hex(rm.and_then(|m| m.anim.as_ref()).and_then(|m| m.guid)) != o["anim"] {
                d.push("anim");
            }
            if rm.map(|m| m.bone_things.len() as u64) != o["bones"].as_u64() {
                d.push("bones");
            }
            let bm = t
                .data
                .level_settings
                .as_ref()
                .and_then(|l| l.backdrop_mesh.as_ref())
                .and_then(|m| m.guid);
            if hex(bm) != o["backdrop_mesh"] {
                d.push("backdrop_mesh");
            }
            if !d.is_empty() {
                diffs.push(format!("thing {}: {}", t.index, d.join(",")));
            }
        }
        if diffs.is_empty() {
            agree += 1;
        } else {
            differ += 1;
            println!("{guid} 0x{:x}: {}", rep.revision.head, diffs.join("; "));
        }
    }
    println!(
        "plans {} agree {agree} differ {differ}; Things compared {things_compared}",
        plans.len()
    );
    if differ > 0 {
        return Err("parity differences".into());
    }
    Ok(())
}

fn main() {
    let worker = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(run)
        .expect("spawn parity worker");
    if let Err(e) = worker.join().expect("parity worker panicked") {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
