//! D122: read-only parity of the independent RBevel reader (`lbp_level::bevel`) against the
//! pinned Toolkit oracle output (scripts/BevelOracle.java, d6a2ef80b5a09999f08f23a803b516096055e5c9).
//! Usage: bevel_parity SNAPSHOT GAME-DIR[;GAME-DIR] ORACLE-JSON NEW-JSON
//! Compares per bevel: head, branch, compression flags, consumed/trailing bytes, the four
//! material descriptors, UV scales, autoSmoothCutoffAngle, every vertex (y, z, rigidity
//! as exact f32 bits, smoothWithPrevious, gmatSlot, mapping mode), includeBackface,
//! smoothWithFront, relaxStrength, subDivRadius, fixedBevelSize, spongy, softPhysics
//! descriptor and textureRepeats. Unit: unique bevel payload (SHA-1).
//! Output: a NEW JSON file under an ignored `local-analysis/` directory.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_level::bevel::{self, Bevel};
use lbp_level::serial::{hex, Descriptor};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

const PAGE: usize = 4096;

fn read_all(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<Vec<u8>, String> {
    let size = usize::try_from(key.size).map_err(|_| "size overflow")?;
    if size > bevel::MAX_INPUT {
        return Err("archived size exceeds bevel input budget".into());
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

fn desc(d: &Option<Descriptor>) -> Value {
    match d {
        None => Value::Null,
        Some(d) => json!({
            "guid": d.guid.map(|g| format!("{g:08x}")),
            "sha1": d.sha1.map(|h| hex(&h)),
        }),
    }
}

fn bits(v: f32) -> Value {
    json!(v.to_bits() as i32)
}

/// Projection in the oracle's shape (floats as raw bits).
fn project(b: &Bevel) -> Value {
    json!({
        "head": b.revision.head,
        "branch_id": b.revision.branch_id,
        "compression_flags": b.revision.compression,
        "consumed_bytes": b.consumed_bytes,
        "trailing_bytes": b.trailing_bytes,
        "materials": b.materials.iter().map(desc).collect::<Vec<_>>(),
        "uv_scales_bits": b.uv_scales.iter().map(|&v| bits(v)).collect::<Vec<_>>(),
        "auto_smooth_cutoff_angle_bits": bits(b.auto_smooth_cutoff_angle),
        "vertices": b.vertices.iter().map(|v| json!({
            "y": bits(v.y), "z": bits(v.z), "rigidity": bits(v.rigidity),
            "smooth_with_previous": v.smooth_with_previous, "gmat_slot": v.gmat_slot,
            "mapping_mode": v.mapping_mode().map(|m| m.name()),
        })).collect::<Vec<_>>(),
        "include_backface": b.include_backface,
        "smooth_with_front": b.smooth_with_front,
        "relax_strength_bits": bits(b.relax_strength),
        "sub_div_radius_bits": bits(b.sub_div_radius),
        "fixed_bevel_size_bits": bits(b.fixed_bevel_size),
        "spongy": b.spongy,
        "soft_physics_settings": desc(&b.soft_physics_settings),
        "texture_repeats_bits": bits(b.texture_repeats),
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: bevel_parity SNAPSHOT GAME-DIR ORACLE-JSON NEW-JSON".into());
    }
    let output = Path::new(&args[4]);
    let parent_ok = output
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "local-analysis");
    if output.exists() || !parent_ok {
        return Err("new output under an ignored local-analysis directory required".into());
    }
    let oracle: Value =
        serde_json::from_slice(&std::fs::read(&args[3]).map_err(|e| format!("oracle: {e}"))?)
            .map_err(|e| format!("oracle json: {e}"))?;
    if oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9"
        || oracle["scope"] != "bevb_toolkit_rbevel_fields_v1"
    {
        return Err("oracle pin/scope differs".into());
    }
    let rows = oracle["bevels"].as_array().ok_or("oracle bevels")?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut status = BTreeMap::<String, usize>::new();
    let mut field_mismatch = BTreeMap::<String, usize>::new();
    let mut out_rows = Vec::new();
    let (mut vertices, mut slot_hist, mut mode_hist) = (
        0usize,
        BTreeMap::<i8, usize>::new(),
        BTreeMap::<String, usize>::new(),
    );
    for row in rows {
        control.check()?;
        let sha = row["sha1"].as_str().unwrap_or_default().to_string();
        let mut r = json!({"sha1": sha, "guid": row["guid"], "oracle_status": row["status"]});
        let st = if row["status"] != "parsed_exact" && row["status"] != "parsed_trailing_bytes" {
            "oracle_not_parsed".to_string()
        } else {
            let key = LocationKey {
                path: row["source_path"].as_str().unwrap_or_default().to_string(),
                offset: row["offset"].as_u64().unwrap_or(0),
                size: row["stored_bytes"].as_u64().unwrap_or(0),
            };
            match read_all(&source, &key, &control).and_then(|b| {
                bevel::read(&b, &|| control.check().is_err()).map_err(|e| e.to_string())
            }) {
                Err(e) => {
                    r["reason"] = json!(e);
                    "rust_failed".into()
                }
                Ok(b) => {
                    vertices += b.vertices.len();
                    for v in &b.vertices {
                        *slot_hist.entry(v.gmat_slot).or_default() += 1;
                        *mode_hist
                            .entry(v.mapping_mode().map_or("unknown", |m| m.name()).into())
                            .or_default() += 1;
                    }
                    let mine = project(&b);
                    let mut diffs = Vec::new();
                    for (k, v) in mine.as_object().expect("object") {
                        if &row[k] != v {
                            diffs.push(k.clone());
                            *field_mismatch.entry(k.clone()).or_default() += 1;
                        }
                    }
                    if diffs.is_empty() {
                        "match".into()
                    } else {
                        r["mismatched_fields"] = json!(diffs);
                        r["rust"] = mine;
                        "mismatch".into()
                    }
                }
            }
        };
        *status.entry(st.clone()).or_default() += 1;
        r["status"] = json!(st);
        out_rows.push(r);
    }
    source.check_stable()?;
    let value = json!({
        "schema_version": 1, "milestone": "D122",
        "unit": "unique bevel payload (SHA-1)",
        "oracle_status_counts": oracle["status_counts"],
        "status_counts": status,
        "field_mismatch_counts": field_mismatch,
        "vertices": vertices,
        "gmat_slot_histogram": slot_hist.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
        "mapping_mode_histogram": mode_hist,
        "rows": out_rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "bevels {}; status {:?}; field mismatches {:?}; vertices {vertices}",
        rows.len(),
        status,
        field_mismatch
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
