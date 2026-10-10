//! Read-only census of what feeds each output-node port of every material graph
//! (P3c: which of specular / glow / reflection ports carry a static colour, one sampler,
//! or a computed subgraph). Structure only: no textures are read or decoded.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

/// What one wire into an output port comes from.
fn feed(
    graph: &lbp_resource::material::GraphMetadata,
    w: &lbp_resource::material::WireMetadata,
) -> String {
    let Some(b) = usize::try_from(w.box_from)
        .ok()
        .and_then(|i| graph.boxes.get(i))
    else {
        return "dangling".into();
    };
    let wired = graph.wires.iter().any(|x| x.box_to == w.box_from);
    let swz = w.swizzle_raw != [0; 5] || w.port_from_raw != 0;
    let kind = match (b.type_raw, wired) {
        (3, false) => "color",
        (1, _) => "sampler",
        (t, false) => return format!("type{t}_unwired{}", if swz { "_swz" } else { "" }),
        (t, true) => return format!("type{t}_computed{}", if swz { "_swz" } else { "" }),
    };
    format!("{kind}{}", if swz { "_swz" } else { "" })
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("output_port_census SNAPSHOT GAME-DIR MATERIAL-CENSUS NEW-JSON".into());
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
    let census: Value =
        serde_json::from_reader(std::fs::File::open(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let locations = census["locations"].as_array().ok_or("locations required")?;
    if locations.is_empty() || locations.len() > 20_000 {
        return Err("location budget".into());
    }
    let mut rows = vec![];
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut ports: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut combos: BTreeMap<String, usize> = BTreeMap::new();
    let mut bytes_read = 0u64;
    for location in locations {
        let Some(sha) = location["sha1_candidates"]
            .as_array()
            .and_then(|c| c.first())
            .and_then(Value::as_str)
        else {
            *classes.entry("no_hash".into()).or_default() += 1;
            continue;
        };
        // Archive entries with no MAP row (e.g. disc copies superseded by an update) are
        // not catalog assets; count them instead of failing the census.
        if !catalog.by_hash.contains_key(sha) {
            *classes.entry("no_map_row".into()).or_default() += 1;
            continue;
        }
        let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
        if key.size > lbp_resource::material::MAX_INPUT as u64 {
            *classes.entry("over_input_budget".into()).or_default() += 1;
            continue;
        }
        bytes_read = bytes_read.checked_add(key.size).ok_or("total overflow")?;
        if bytes_read > 128 * 1024 * 1024 {
            return Err("total budget".into());
        }
        let mut data = Vec::with_capacity(key.size as usize);
        while data.len() < key.size as usize {
            let count = (key.size as usize - data.len()).min(4096);
            let page = source
                .read_hex_checked(&key, data.len() as u64, count, &control)?
                .bytes;
            if page.len() != count {
                return Err("short page".into());
            }
            data.extend(page);
        }
        match lbp_resource::material::inspect_graph(&data, || control.check().is_err()) {
            Ok(g) => {
                let graph = &g.graph;
                let Some(out) = graph.boxes.iter().position(|b| b.type_raw == 0) else {
                    *classes.entry("no_output_node".into()).or_default() += 1;
                    continue;
                };
                *classes.entry("graphs".into()).or_default() += 1;
                let mut used = vec![];
                for w in graph.wires.iter().filter(|w| w.box_to as usize == out) {
                    let f = feed(graph, w);
                    *ports
                        .entry(format!("port{}", w.port_to_raw))
                        .or_default()
                        .entry(f.clone())
                        .or_default() += 1;
                    used.push(format!("{}:{f}", w.port_to_raw));
                }
                used.sort();
                *combos.entry(used.join(" ")).or_default() += 1;
            }
            Err(_) => *classes.entry("graph_not_parsed".into()).or_default() += 1,
        }
        rows.push(sha.to_string());
    }
    let report = json!({"schema_version":1,"scope":"output_node_port_feeds_structure_only","materials":rows.len(),"archived_bytes_read":bytes_read,"class_counts":classes,"port_feed_counts":ports,"port_combination_counts":combos});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(
        serde_json::to_string_pretty(&report)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{} materials, {} archived bytes: {:?}",
        rows.len(),
        bytes_read,
        classes
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
