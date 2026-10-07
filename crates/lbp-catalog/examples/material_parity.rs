//! Bounded read-only GMT prefix/graph/tail metadata parity; opaque bytes/keys never emitted.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let tail_mode = args.len() == 6 && args[5] == "--tail";
    let graph_mode = tail_mode || (args.len() == 6 && args[5] == "--graph");
    if args.len() != 5 && !graph_mode {
        return Err("material_parity SNAPSHOT GAME-DIR ORACLE NEW-JSON [--graph|--tail]".into());
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
    let scope = if tail_mode {
        "PS3_GMT_tail_metadata_no_shader_bytes"
    } else if graph_mode {
        "PS3_GMT_graph_metadata_no_shader_bytes"
    } else {
        "PS3_GMT_prefix_only_no_shader_bytes"
    };
    if oracle["schema_version"] != 1 || oracle["scope"] != scope {
        return Err("oracle scope/schema differs".into());
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
        if location.size > lbp_resource::material::MAX_INPUT as u64 {
            return Err("input budget".into());
        }
        total = total.checked_add(location.size).ok_or("total overflow")?;
        if total > 16 * 1024 * 1024 {
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
        let parsed = if tail_mode {
            lbp_resource::material::inspect_tail(&data, || {
                control.cancelled.load(std::sync::atomic::Ordering::Relaxed)
            })
            .map_err(|e| e.to_string())
            .and_then(|p| serde_json::to_value(p).map_err(|e| e.to_string()))
        } else if graph_mode {
            lbp_resource::material::inspect_graph(&data, || {
                control.cancelled.load(std::sync::atomic::Ordering::Relaxed)
            })
            .map_err(|e| e.to_string())
            .and_then(|p| serde_json::to_value(p).map_err(|e| e.to_string()))
        } else {
            lbp_resource::material::inspect(&data, || {
                control.cancelled.load(std::sync::atomic::Ordering::Relaxed)
            })
            .map_err(|e| e.to_string())
            .and_then(|p| serde_json::to_value(p).map_err(|e| e.to_string()))
        };
        match parsed {
            Ok(value) => {
                let mut expected = json!({"compression_flags":sample["compression_flags"],
                    "expanded_bytes":sample["expanded_bytes"],"prefix":sample["prefix"]});
                if graph_mode {
                    expected["graph"] = sample["graph"].clone();
                }
                if tail_mode {
                    expected["tail"] = sample["tail"].clone();
                }
                let equal = sample["status"]
                    == if graph_mode {
                        "parsed_graph"
                    } else {
                        "parsed_prefix"
                    }
                    && (!tail_mode || sample["tail_status"] == "parsed_tail")
                    && value == expected;
                compared += 1;
                mismatch += usize::from(!equal);
                rows.push(json!({"sha1":sha,"status":if tail_mode {"parsed_tail"} else if graph_mode {"parsed_graph"} else {"parsed_prefix"},"oracle_equal":equal,"metadata":value}));
            }
            Err(e) => {
                failures += 1;
                rows.push(json!({"sha1":sha,"status":"not_parsed","reason":e.to_string()}));
            }
        }
    }
    source.check_stable()?;
    let report = json!({"schema_version":1,"scope":if tail_mode { "material_tail_metadata" } else if graph_mode { "material_graph_metadata" } else { "material_prefix_metadata" },"compared":compared,"mismatches":mismatch,"failures":failures,"archived_bytes_read":total,"samples":rows});
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut f, &report).map_err(|e| e.to_string())?;
    f.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "Material metadata parity: {compared} compared, {mismatch} mismatches, {failures} failures"
    );
    if mismatch > 0 || failures > 0 || compared == 0 {
        Err("material metadata comparison failed".into())
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
