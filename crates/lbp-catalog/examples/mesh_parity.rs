//! Bounded read-only mesh-prefix comparison against pinned external stream helpers.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let prefix_only = args.len() == 6 && args[5] == "--prefix-only";
    let groups_mode = args.len() == 6 && args[5] == "--groups";
    let uv_mode = args.len() == 6 && args[5] == "--uv";
    if args.len() != 5 && !prefix_only && !uv_mode && !groups_mode {
        return Err(
            "mesh_parity SNAPSHOT GAME-DIR ORACLE NEW-JSON [--prefix-only | --uv | --groups]"
                .into(),
        );
    }
    let output = Path::new(&args[4]);
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
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let oracle: Value =
        serde_json::from_reader(std::fs::File::open(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9" {
        return Err("oracle pin differs".into());
    }
    let samples = oracle["samples"].as_array().ok_or("samples absent")?;
    if samples.len() > 48 {
        return Err("sample budget".into());
    }
    let (mut total, mut mismatch, mut failures, mut compared) = (0u64, 0, 0, 0);
    let mut rows = vec![];
    for sample in samples {
        let sha = sample["sha1"].as_str().ok_or("sha absent")?;
        let location = catalog
            .snapshot
            .resource_inspections
            .iter()
            .find(|l| l.sha1.as_deref() == Some(sha))
            .ok_or("candidate absent")?;
        if location.size > lbp_mesh::MAX_INPUT as u64 {
            return Err("input budget".into());
        }
        total = total.checked_add(location.size).ok_or("total overflow")?;
        if total > 64 * 1024 * 1024 {
            return Err("total budget".into());
        }
        let key = LocationKey {
            path: location.source_path.clone(),
            offset: location.offset,
            size: location.size,
        };
        let mut data = Vec::with_capacity(key.size as usize);
        while data.len() < key.size as usize {
            let count = (key.size as usize - data.len()).min(4096);
            data.extend(
                source
                    .read_hex_checked(&key, data.len() as u64, count, &control)?
                    .bytes,
            );
        }
        let parsed = if groups_mode {
            lbp_mesh::decode_geometry(&data, || false).map_err(|e| e.to_string()).map(|g| {
                use sha2::{Digest, Sha256};
                Value::Array(g.primitive_triangles.iter().map(|r| {
                    let mut digest = Sha256::new();
                    for t in &g.triangles[r.clone()] { for i in t { digest.update(i.to_be_bytes()); } }
                    json!({"start":r.start,"end":r.end,"triangle_indices_sha256":format!("{:x}",digest.finalize())})
                }).collect())
            })
        } else if uv_mode {
            lbp_mesh::decode_uvs(&data, || false)
                .map_err(|e| e.to_string())
                .and_then(|uv| serde_json::to_value(uv.metadata).map_err(|e| e.to_string()))
        } else if prefix_only {
            lbp_mesh::inspect(&data, || false)
                .map_err(|e| e.to_string())
                .and_then(|p| serde_json::to_value(p).map_err(|e| e.to_string()))
        } else {
            lbp_mesh::decode_geometry(&data, || false)
                .map_err(|e| e.to_string())
                .and_then(|p| serde_json::to_value(p.metadata).map_err(|e| e.to_string()))
        };
        match parsed {
            Ok(value) => {
                // Gson prints f32 decimals whereas serde_json::Value widens to f64.
                // All layout/payload fields and canonical IEEE bounds bits compare exactly.
                let mut canonical = value.clone();
                let mut expected = sample[if groups_mode {
                    "groups"
                } else if uv_mode {
                    "uv"
                } else if prefix_only {
                    "prefix"
                } else {
                    "geometry"
                }]
                .clone();
                let normalized = if uv_mode || groups_mode {
                    None
                } else if prefix_only {
                    canonical.as_object_mut()
                } else {
                    canonical["prefix"].as_object_mut()
                };
                if let Some(p) = normalized {
                    p.remove("position_bounds");
                }
                let normalized = if uv_mode || groups_mode {
                    None
                } else if prefix_only {
                    expected.as_object_mut()
                } else {
                    expected["prefix"].as_object_mut()
                };
                if let Some(p) = normalized {
                    p.remove("position_bounds");
                }
                let equal = sample["status"] == "parsed_prefix" && canonical == expected;
                compared += 1;
                mismatch += usize::from(!equal);
                let mut row = json!({"sha1":sha,"status":"parsed_prefix","oracle_equal":equal});
                row[if groups_mode {
                    "groups"
                } else if uv_mode {
                    "uv"
                } else {
                    "geometry"
                }] = value;
                rows.push(row);
            }
            Err(e) => {
                failures += 1;
                rows.push(json!({"sha1":sha,"status":"not_parsed","reason":e.to_string()}));
            }
        }
    }
    source.check_stable()?;
    let report = json!({"schema_version":1,"scope":if groups_mode {"mesh_primitive_partitions"} else if uv_mode {"mesh_uv_metadata"} else if prefix_only {"mesh_prefix_only"}else{"mesh_geometry_metadata"},"compared":compared,"mismatches":mismatch,"failures":failures,"archived_bytes_read":total,"samples":rows});
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut f, &report).map_err(|e| e.to_string())?;
    f.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("Mesh parity: {compared} compared, {mismatch} mismatches, {failures} failures");
    if mismatch > 0 || failures > 0 || compared == 0 {
        Err("mesh prefix comparison failed".into())
    } else {
        Ok(())
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
