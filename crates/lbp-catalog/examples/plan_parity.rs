//! Bounded read-only modern PLNb prefix comparison; no Things/asset output.
//! External reader provenance: docs/plan-prefix.md; independent reader: plan.rs.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use lbp_resource::plan;
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, path::Path, sync::atomic::Ordering};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("plan_parity SNAPSHOT GAME-DIR ORACLE NEW-JSON".into());
    }
    let output = Path::new(&args[4]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.extension().and_then(|s| s.to_str()) != Some("json")
        || output.exists()
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
    if oracle["schema_version"] != 1
        || oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9"
        || oracle["scope"] != "plan_prefix_metadata_no_thing_traversal"
    {
        return Err("oracle schema/pin/scope differs".into());
    }
    let samples = oracle["samples"].as_array().ok_or("samples absent")?;
    if samples.len() > 48 {
        return Err("sample budget".into());
    }
    let (mut total, mut compared, mut mismatches, mut failures) = (0u64, 0, 0, 0);
    let mut rows = vec![];
    for sample in samples {
        let sha = sample["sha1"].as_str().ok_or("sha absent")?;
        let candidates: Vec<_> = catalog
            .snapshot
            .resource_inspections
            .iter()
            .filter(|l| l.sha1.as_deref() == Some(sha))
            .collect();
        if candidates.len() != 1 {
            return Err(
                "oracle hash has absent/ambiguous locations; refine location provenance".into(),
            );
        }
        let location = candidates[0];
        if location.size > plan::MAX_INPUT as u64 {
            return Err("input budget".into());
        }
        total = total.checked_add(location.size).ok_or("total overflow")?;
        if total > 64 * 1024 * 1024 {
            return Err("read budget".into());
        }
        let key = LocationKey {
            path: location.source_path.clone(),
            offset: location.offset,
            size: location.size,
        };
        let length = usize::try_from(key.size).map_err(|_| "size overflow")?;
        let mut data = Vec::with_capacity(length);
        while data.len() < length {
            let count = (length - data.len()).min(4096);
            let page = source
                .read_hex_checked(&key, data.len() as u64, count, &control)?
                .bytes;
            if page.len() != count {
                return Err("short source page".into());
            }
            data.extend(page);
        }
        match plan::inspect(&data, || control.cancelled.load(Ordering::Relaxed)) {
            Ok(metadata) => {
                let value = serde_json::to_value(metadata).map_err(|e| e.to_string())?;
                let expected = json!({"compression_flags":sample["compression_flags"],"expanded_bytes":sample["expanded_bytes"],"prefix":sample["prefix"]});
                let equal = sample["status"] == "parsed_prefix" && value == expected;
                compared += 1;
                mismatches += usize::from(!equal);
                rows.push(json!({"sha1":sha,"status":"parsed_prefix","oracle_equal":equal,"metadata":value}));
            }
            Err(error) => {
                failures += 1;
                rows.push(json!({"sha1":sha,"status":"failed","reason":error.to_string()}));
            }
        }
    }
    source.check_stable()?;
    let report = json!({"schema_version":1,"scope":"plan_prefix_metadata","compared":compared,
        "mismatches":mismatches,"failures":failures,"archived_bytes_read":total,"samples":rows});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "Plan prefix parity: {compared} compared, {mismatches} mismatches, {failures} failures"
    );
    if compared == 0 || mismatches != 0 || failures != 0 {
        Err("plan prefix parity failed".into())
    } else {
        Ok(())
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
