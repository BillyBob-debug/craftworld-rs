//! Read-only corpus census of diffuse route *selection* outcomes per material.
//! Selection only: texture descriptors, wrap modes, UVs and pixels are not checked,
//! so "selected" is not payload compatibility or an asset count.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path};

fn classify(graph: &lbp_resource::material::GraphMetadata) -> (String, Option<String>) {
    let default = match lbp_preview::select_solid_diffuse(graph) {
        Ok(Some(s)) if s.unwired_diffuse => return ("unwired_black".into(), None),
        Ok(Some(_)) => return ("solid_color".into(), None),
        Ok(None) | Err(_) => match lbp_preview::select_small_solid(graph) {
            Ok(Some(_)) => return ("solid_color".into(), None),
            _ => lbp_preview::select_diffuse(graph),
        },
    };
    let reason = match default {
        Ok(_) => return ("texture_route".into(), None),
        Err(e) => e,
    };
    if lbp_preview::select_diffuse_with_animation_base(graph, true).is_ok() {
        return ("animation_base_only".into(), Some(reason));
    }
    let white = lbp_preview::DiffuseTint {
        bits: [1f32.to_bits(); 4],
    };
    // D105: the loader's projection (plain substitution for the evaluator fallback).
    if let Ok((derived, Some(p))) = lbp_preview::project_instance_graph(graph, white, true) {
        if matches!(lbp_preview::select_solid_diffuse(&derived), Ok(Some(_)))
            // D89 ADD is refused under a supplied instance color (loader guard).
            || lbp_preview::select_instance_diffuse(&derived, &p, true)
                .is_ok_and(|s| s.texture_add.is_none())
        {
            return ("instance_color_only".into(), Some(reason));
        }
    }
    ("unsupported".into(), Some(reason))
}

/// D86 (docs/d86-thing-color.md): why a THING_COLOR material is not instance-color
/// only. Projection error verbatim, or the derived graph's default-route reason.
/// Also returns each ThingColor's subtype, parameter words and outgoing wires.
fn thing_color_reason(
    graph: &lbp_resource::material::GraphMetadata,
) -> Option<(String, Vec<String>)> {
    let n = graph.boxes.len();
    if n > 1024 || graph.wires.len() > 4096 {
        return None;
    }
    let ok = |v: i32| v >= 0 && (v as usize) < n;
    let output = graph.boxes.iter().position(|b| b.type_raw == 0)?;
    let mut seen = vec![false; n];
    let mut pending: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| ok(w.box_from) && w.box_to as usize == output && w.port_to_raw == 0)
        .map(|w| w.box_from as usize)
        .collect();
    while let Some(i) = pending.pop() {
        if std::mem::replace(&mut seen[i], true) {
            continue;
        }
        for w in &graph.wires {
            if ok(w.box_from) && ok(w.box_to) && w.box_to as usize == i {
                pending.push(w.box_from as usize);
            }
        }
    }
    let things: Vec<String> = (0..n)
        .filter(|&i| seen[i] && graph.boxes[i].type_raw == 2)
        .map(|i| {
            let b = &graph.boxes[i];
            let out: Vec<String> = graph
                .wires
                .iter()
                .filter(|w| ok(w.box_from) && ok(w.box_to) && w.box_from as usize == i)
                .map(|w| {
                    let sw: String = w
                        .swizzle_raw
                        .iter()
                        .filter(|&&c| c != 0)
                        .map(|&c| c as char)
                        .collect();
                    format!(
                        "{}:{}{}",
                        graph.boxes[w.box_to as usize].type_raw, w.port_to_raw, sw
                    )
                })
                .collect();
            format!(
                "sub{} params{:?} out[{}]",
                b.subtype_raw,
                b.parameters_raw,
                out.join(",")
            )
        })
        .collect();
    if things.is_empty() {
        return None;
    }
    let white = lbp_preview::DiffuseTint {
        bits: [1f32.to_bits(); 4],
    };
    let reason = match lbp_preview::project_instance_color(graph, white) {
        Err(e) => e,
        Ok((_, None)) => "projection_unused".into(),
        Ok((derived, Some(p))) => match lbp_preview::select_solid_diffuse(&derived) {
            Ok(Some(_)) => "projected_selected".into(),
            Err(e) => format!("projected_unselected: {e}"),
            Ok(None) => match lbp_preview::select_instance_diffuse(&derived, &p, true) {
                Ok(_) => "projected_selected".into(),
                Err(e) => format!("projected_unselected: {e}"),
            },
        },
    };
    // D105: why the plain ThingColor substitution is not evaluated either.
    let reason = match lbp_preview::instance_graph_refusal(graph, white) {
        Some(d105) => format!("{reason} | D105 {d105}"),
        None => reason,
    };
    Some((reason, things))
}

/// Compact signature of the diffuse ancestry: nodes as type.subtype[+named animations],
/// edges as from>to:port[swizzle]. Diagnostic grouping only.
fn shape(graph: &lbp_resource::material::GraphMetadata) -> Option<String> {
    let n = graph.boxes.len();
    if n > 1024 || graph.wires.len() > 4096 {
        return None;
    }
    let ok = |v: i32| v >= 0 && (v as usize) < n;
    let output = graph.boxes.iter().position(|b| b.type_raw == 0)?;
    let root = graph
        .wires
        .iter()
        .find(|w| ok(w.box_from) && w.box_to as usize == output && w.port_to_raw == 0)?;
    let mut order = vec![];
    let mut seen = vec![false; n];
    let mut pending = vec![root.box_from as usize];
    while let Some(i) = pending.pop() {
        if seen[i] || order.len() > 64 {
            continue;
        }
        seen[i] = true;
        order.push(i);
        for w in &graph.wires {
            if ok(w.box_from) && ok(w.box_to) && w.box_to as usize == i {
                pending.push(w.box_from as usize);
            }
        }
    }
    let local = |i: usize| order.iter().position(|&o| o == i).unwrap();
    let nodes: Vec<_> = order
        .iter()
        .map(|&i| {
            let b = &graph.boxes[i];
            let named = b
                .animations
                .iter()
                .filter(|a| a.name_hex.bytes().any(|v| v != b'0'))
                .count();
            format!("{}.{}+{}", b.type_raw, b.subtype_raw, named)
        })
        .collect();
    let mut edges: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| ok(w.box_from) && ok(w.box_to))
        .filter(|w| seen[w.box_from as usize] && seen[w.box_to as usize])
        .map(|w| {
            let sw: String = w
                .swizzle_raw
                .iter()
                .filter(|&&c| c != 0)
                .map(|&c| c as char)
                .collect();
            format!(
                "{}>{}:{}{}",
                local(w.box_from as usize),
                local(w.box_to as usize),
                w.port_to_raw,
                sw
            )
        })
        .collect();
    edges.sort();
    Some(format!("{} | {}", nodes.join(","), edges.join(",")))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("diffuse_route_census SNAPSHOT GAME-DIR MATERIAL-CENSUS NEW-JSON".into());
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
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut thing_reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut thing_nodes: BTreeMap<String, usize> = BTreeMap::new();
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
        let (class, reason, signature, thing) =
            match lbp_resource::material::inspect_graph(&data, || control.check().is_err()) {
                Ok(g) => {
                    let (c, r) = classify(&g.graph);
                    let sig = (c == "unsupported").then(|| shape(&g.graph)).flatten();
                    let thing = (c == "unsupported")
                        .then(|| thing_color_reason(&g.graph))
                        .flatten();
                    (c, r, sig, thing)
                }
                Err(e) => ("graph_not_parsed".into(), Some(e.to_string()), None, None),
            };
        *classes.entry(class.clone()).or_default() += 1;
        if let Some(r) = &reason {
            *reasons.entry(format!("{class}: {r}")).or_default() += 1;
        }
        if let Some((r, nodes)) = &thing {
            *thing_reasons.entry(r.clone()).or_default() += 1;
            for node in nodes {
                *thing_nodes.entry(node.clone()).or_default() += 1;
            }
        }
        let thing_reason = thing.map(|(r, _)| r);
        rows.push(json!({"sha1":sha,"class":class,"reason":reason,"shape":signature,"thing_color_reason":thing_reason}));
    }
    rows.sort_by(|a, b| a["sha1"].as_str().cmp(&b["sha1"].as_str()));
    let report = json!({"schema_version":4,"scope":"diffuse_route_selection_only_not_payload_or_asset_count","materials":rows.len(),"archived_bytes_read":bytes_read,"class_counts":classes,"reason_counts":reasons,"thing_color_reason_counts":thing_reasons,"thing_color_node_counts":thing_nodes,"materials_detail":rows});
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
