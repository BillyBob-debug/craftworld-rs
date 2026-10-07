//! Read-only route oracle comparison and bounded diffuse projection census.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 5 || (args.len() == 6 && args[5] == "--animation-base")) {
        return Err("diffuse_parity SNAPSHOT GAME-DIR ORACLE NEW-JSON".into());
    }
    let output = Path::new(&args[4]);
    let animation_base = args.len() == 6;
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
    if oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9"
        || oracle["scope"] != "PS3_GMT_graph_metadata_no_shader_bytes"
    {
        return Err("oracle pin/scope differs".into());
    }
    let samples = oracle["samples"].as_array().ok_or("samples required")?;
    if samples.is_empty() || samples.len() > 48 {
        return Err("sample budget".into());
    }
    let mut rows = vec![];
    let (mut bytes_read, mut mismatches, mut projected) = (0u64, 0usize, 0usize);
    for sample in samples {
        if sample["status"] != "parsed_graph" || sample.get("diffuse_route").is_none() {
            return Err("route evidence absent".into());
        }
        let sha = sample["sha1"].as_str().ok_or("sha required")?;
        let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
        if key.size > lbp_resource::material::MAX_INPUT as u64 {
            return Err("material budget".into());
        }
        bytes_read = bytes_read.checked_add(key.size).ok_or("total overflow")?;
        if bytes_read > 16 * 1024 * 1024 {
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
        let graph = lbp_resource::material::inspect_graph(&data, || control.check().is_err())
            .map_err(|e| e.to_string())?;
        let wraps_equal = json!(graph.material.prefix.wrap_s_raw) == sample["prefix"]["wrap_s_raw"]
            && json!(graph.material.prefix.wrap_t_raw) == sample["prefix"]["wrap_t_raw"];
        mismatches += usize::from(!wraps_equal);
        let output = graph
            .graph
            .boxes
            .iter()
            .position(|b| b.type_raw == 0)
            .ok_or("output absent")?;
        let wire = graph
            .graph
            .wires
            .iter()
            .find(|w| w.box_to as usize == output && w.port_to_raw == 0);
        let Some(wire) = wire else {
            // Unwired diffuse: oracle route fields are null; projection must be zero.
            let mut route =
                json!({"output_index":output,"source_index":null,"source_type":null,"slot":null});
            if sample["diffuse_route"].get("uv_channel_raw").is_some() {
                route["uv_channel_raw"] = Value::Null;
            }
            let projection = match lbp_preview::select_solid_diffuse(&graph.graph) {
                Ok(Some(s)) if s.unwired_diffuse => {
                    mismatches += usize::from(
                        route != sample["diffuse_route"]
                            || s.output_index != output
                            || s.color.bits != [0; 4],
                    );
                    projected += 1;
                    json!({"status":"unwired_projected","output_index":s.output_index,"color_bits":s.color.bits,"ignored_nodes":s.ignored_nodes,"ignored_wires":s.ignored_wires})
                }
                Ok(_) => {
                    mismatches += 1;
                    json!({"status":"unwired_not_selected"})
                }
                Err(e) => {
                    mismatches += usize::from(route != sample["diffuse_route"]);
                    json!({"status":"unsupported","reason":e})
                }
            };
            rows.push(json!({"sha1":sha,"route_equal":route == sample["diffuse_route"],"wraps_equal":wraps_equal,"route":route,"projection":projection}));
            continue;
        };
        let node = graph
            .graph
            .boxes
            .get(wire.box_from as usize)
            .ok_or("source absent")?;
        let mut route = json!({"output_index":output,"source_index":wire.box_from,"source_type":node.type_raw,"slot":if node.type_raw==1 {Some(node.parameters_raw[5])} else {None}});
        if sample["diffuse_route"].get("uv_channel_raw").is_some() {
            route["uv_channel_raw"] = json!(if node.type_raw == 1 {
                Some(node.parameters_raw[4])
            } else {
                None
            });
        }
        let equal = route == sample["diffuse_route"];
        mismatches += usize::from(!equal);
        let solid = lbp_preview::select_solid_diffuse(&graph.graph);
        let projection = if let Ok(Some(s)) = solid {
            if s.output_index != output
                || s.color_index != wire.box_from as usize
                || s.color.bits.iter().enumerate().any(|(i, b)| {
                    sample["graph"]["boxes"][s.color_index]["parameters_raw"][i].as_u64()
                        != Some(u64::from(*b))
                })
            {
                mismatches += 1;
            }
            projected += 1;
            json!({"status":"solid_projected","output_index":s.output_index,"color_index":s.color_index,"color_bits":s.color.bits,"texture":null,"uv_channel":null,"ignored_nodes":s.ignored_nodes,"ignored_wires":s.ignored_wires})
        } else {
            match lbp_preview::select_diffuse_with_animation_base(&graph.graph, animation_base) {
                Ok(s) => {
                    let sample_node = &graph.graph.boxes[s.sample_index];
                    let multiply_equal = if let Some(mix) = s.alpha_mix {
                        let root = mix.mix_index;
                        let color = s.color_index.ok_or("alpha MIX color index missing")?;
                        let nodes = &sample["graph"]["boxes"];
                        let wires = sample["graph"]["wires"]
                            .as_array()
                            .ok_or("oracle wires required")?;
                        let mixed: Vec<_> = wires
                            .iter()
                            .filter(|w| w["box_to"].as_u64() == Some(root as u64))
                            .collect();
                        let edge = |source: usize, port: u64, swizzle: Value| {
                            mixed.iter().any(|w| {
                                w["box_from"].as_u64() == Some(source as u64)
                                    && w["port_to_raw"].as_u64() == Some(port)
                                    && w["port_from_raw"] == 0
                                    && w["swizzle_raw"] == swizzle
                            })
                        };
                        root == wire.box_from as usize
                            && mixed.len() == 3
                            && s.second_sample.is_none()
                            && s.multiply_index.is_none()
                            && s.blend.is_none()
                            && edge(
                                color,
                                if mix.color_first { 0 } else { 1 },
                                json!([0, 0, 0, 0, 0]),
                            )
                            && edge(
                                s.sample_index,
                                if mix.color_first { 1 } else { 0 },
                                json!([0, 0, 0, 0, 0]),
                            )
                            && edge(s.sample_index, 2, json!([97, 0, 0, 0, 0]))
                            && nodes[root]["type_raw"] == 12
                            && nodes[root]["parameters_raw"][0].as_u64()
                                == Some(u64::from(mix.unused_weight_bits))
                            && nodes[color]["type_raw"] == 3
                            && mix.color.bits.iter().enumerate().all(|(i, b)| {
                                nodes[color]["parameters_raw"][i].as_u64() == Some(u64::from(*b))
                            })
                    } else if let Some(blend) = s.blend.filter(|_| s.second_sample.is_none()) {
                        let root = blend.mix_index;
                        let color = s.color_index.ok_or("single MIX color index missing")?;
                        let nodes = &sample["graph"]["boxes"];
                        let wires = sample["graph"]["wires"]
                            .as_array()
                            .ok_or("oracle wires required")?;
                        let mixed: Vec<_> = wires
                            .iter()
                            .filter(|w| w["box_to"].as_u64() == Some(root as u64))
                            .collect();
                        let edge = |source: usize, port: u64| {
                            mixed.iter().any(|w| {
                                w["box_from"].as_u64() == Some(source as u64)
                                    && w["port_to_raw"].as_u64() == Some(port)
                                    && w["port_from_raw"] == 0
                                    && w["swizzle_raw"] == json!([0, 0, 0, 0, 0])
                            })
                        };
                        root == wire.box_from as usize
                            && mixed.len() == 2
                            && s.multiply_index.is_none()
                            && edge(color, if blend.color_first { 0 } else { 1 })
                            && edge(s.sample_index, if blend.color_first { 1 } else { 0 })
                            && nodes[root]["type_raw"] == 12
                            && nodes[root]["parameters_raw"][0].as_u64()
                                == Some(u64::from(blend.weight_bits))
                            && nodes[color]["type_raw"] == 3
                            && blend.color.bits.iter().enumerate().all(|(i, b)| {
                                nodes[color]["parameters_raw"][i].as_u64() == Some(u64::from(*b))
                            })
                    } else if let Some(blend) = s.blend {
                        let root = blend.mix_index;
                        let color = s.color_index.ok_or("MIX color index missing")?;
                        let multiply = s.multiply_index.ok_or("MIX multiply index missing")?;
                        let second = s.second_sample.ok_or("MIX second sample missing")?;
                        let wires = sample["graph"]["wires"]
                            .as_array()
                            .ok_or("oracle wires required")?;
                        let inputs = |target: usize| {
                            wires
                                .iter()
                                .filter(|w| w["box_to"].as_u64() == Some(target as u64))
                                .collect::<Vec<_>>()
                        };
                        let mixed = inputs(root);
                        let product = inputs(multiply);
                        let edge = |list: &Vec<&Value>, source: usize, port: u64| {
                            list.iter().any(|w| {
                                w["box_from"].as_u64() == Some(source as u64)
                                    && w["port_to_raw"].as_u64() == Some(port)
                            })
                        };
                        let second_node = &sample["graph"]["boxes"][second.sample_index];
                        root == wire.box_from as usize
                            && mixed.len() == 2
                            && product.len() == 2
                            && edge(&mixed, color, if blend.color_first { 0 } else { 1 })
                            && edge(&mixed, multiply, if blend.color_first { 1 } else { 0 })
                            && product
                                .iter()
                                .any(|w| w["box_from"].as_u64() == Some(s.sample_index as u64))
                            && product
                                .iter()
                                .any(|w| w["box_from"].as_u64() == Some(second.sample_index as u64))
                            && sample["graph"]["boxes"][root]["type_raw"] == 12
                            && sample["graph"]["boxes"][root]["parameters_raw"][0].as_u64()
                                == Some(u64::from(blend.weight_bits))
                            && sample["graph"]["boxes"][color]["type_raw"] == 3
                            && blend.color.bits.iter().enumerate().all(|(i, b)| {
                                sample["graph"]["boxes"][color]["parameters_raw"][i].as_u64()
                                    == Some(u64::from(*b))
                            })
                            && sample["graph"]["boxes"][multiply]["type_raw"] == 10
                            && second_node["type_raw"] == 1
                            && second_node["subtype_raw"].as_u64()
                                == Some(u64::from(second.uv_transform.subtype()))
                            && second_node["parameters_raw"][4].as_u64()
                                == Some(second.uv_channel as u64)
                            && second_node["parameters_raw"][5].as_u64() == Some(second.slot as u64)
                            && second
                                .uv_transform
                                .parameter_indices()
                                .iter()
                                .zip(second.uv_transform.bits())
                                .all(|(i, b)| {
                                    second_node["parameters_raw"][*i].as_u64()
                                        == Some(u64::from(*b))
                                })
                    } else if let (Some(inner), Some(third), Some(second), Some(root)) = (
                        s.inner_multiply_index,
                        s.third_sample,
                        s.second_sample,
                        s.multiply_index,
                    ) {
                        let oracle_wires = sample["graph"]["wires"]
                            .as_array()
                            .ok_or("oracle wires required")?;
                        let inputs = |target: usize| {
                            oracle_wires
                                .iter()
                                .filter(|w| w["box_to"].as_u64() == Some(target as u64))
                                .collect::<Vec<_>>()
                        };
                        let has = |edges: &Vec<&Value>, source: usize| {
                            edges
                                .iter()
                                .any(|w| w["box_from"].as_u64() == Some(source as u64))
                        };
                        let root_inputs = inputs(root);
                        let inner_inputs = inputs(inner);
                        let chain_equal = if let (Some(deep), Some(fourth)) =
                            (s.deep_multiply_index, s.fourth_sample)
                        {
                            let bottom = inputs(deep);
                            bottom.len() == 2
                                && has(&root_inputs, inner)
                                && has(&root_inputs, fourth.sample_index)
                                && has(&inner_inputs, deep)
                                && has(&inner_inputs, third.sample_index)
                                && has(&bottom, s.sample_index)
                                && has(&bottom, second.sample_index)
                                && sample["graph"]["boxes"][deep]["type_raw"] == 10
                                && sample["graph"]["boxes"][deep]["subtype_raw"] == 0
                        } else {
                            has(&root_inputs, inner)
                                && has(&root_inputs, third.sample_index)
                                && has(&inner_inputs, s.sample_index)
                                && has(&inner_inputs, second.sample_index)
                        };
                        root == wire.box_from as usize
                            && root_inputs.len() == 2
                            && inner_inputs.len() == 2
                            && chain_equal
                            && [root, inner].iter().all(|i| {
                                sample["graph"]["boxes"][*i]["type_raw"] == 10
                                    && sample["graph"]["boxes"][*i]["subtype_raw"] == 0
                            })
                            && [second, third]
                                .iter()
                                .chain(s.fourth_sample.iter())
                                .all(|leaf| {
                                    let node = &sample["graph"]["boxes"][leaf.sample_index];
                                    node["type_raw"] == 1
                                        && node["subtype_raw"].as_u64()
                                            == Some(u64::from(leaf.uv_transform.subtype()))
                                        && node["parameters_raw"][4].as_u64()
                                            == Some(leaf.uv_channel as u64)
                                        && node["parameters_raw"][5].as_u64()
                                            == Some(leaf.slot as u64)
                                        && leaf
                                            .uv_transform
                                            .parameter_indices()
                                            .iter()
                                            .zip(leaf.uv_transform.bits())
                                            .all(|(i, b)| {
                                                node["parameters_raw"][*i].as_u64()
                                                    == Some(u64::from(*b))
                                            })
                                })
                    } else {
                        match (s.multiply_index, s.color_index) {
                            (Some(multiply), Some(color)) => {
                                sample["static_diffuse_multiply"]
                                    == json!({"multiply_index":multiply,"sample_index":s.sample_index,"color_index":color,"tint_bits":s.tint.bits})
                            }
                            (None, None) => {
                                s.sample_index == wire.box_from as usize
                                    && s.tint == lbp_preview::DiffuseTint::default()
                            }
                            (Some(multiply), None) => s.second_sample.is_some_and(|second| {
                                let oracle_wires = sample["graph"]["wires"].as_array().unwrap();
                                let inputs: Vec<_> = oracle_wires
                                    .iter()
                                    .filter(|w| w["box_to"].as_u64() == Some(multiply as u64))
                                    .collect();
                                let node = &sample["graph"]["boxes"][second.sample_index];
                                multiply == wire.box_from as usize
                                    && inputs.len() == 2
                                    && inputs.iter().any(|w| {
                                        w["box_from"].as_u64() == Some(s.sample_index as u64)
                                    })
                                    && inputs.iter().any(|w| {
                                        w["box_from"].as_u64() == Some(second.sample_index as u64)
                                    })
                                    && node["type_raw"] == 1
                                    && node["subtype_raw"].as_u64()
                                        == Some(u64::from(second.uv_transform.subtype()))
                                    && node["parameters_raw"][4].as_u64()
                                        == Some(second.uv_channel as u64)
                                    && node["parameters_raw"][5].as_u64()
                                        == Some(second.slot as u64)
                                    && second
                                        .uv_transform
                                        .parameter_indices()
                                        .iter()
                                        .zip(second.uv_transform.bits())
                                        .all(|(i, b)| {
                                            node["parameters_raw"][*i].as_u64()
                                                == Some(u64::from(*b))
                                        })
                            }),
                            _ => false,
                        }
                    };
                    let base_equal =
                        s.animation_base.is_none_or(|base| {
                            let animations =
                                &sample["graph"]["boxes"][s.sample_index]["animations"];
                            [0, 1, 2, 3, 6, 7].iter().zip(base.static_matrix_bits).all(
                                |(i, bits)| {
                                    sample["graph"]["boxes"][s.sample_index]["parameters_raw"][*i]
                                        .as_u64()
                                        == Some(u64::from(bits))
                                },
                            ) && (0..2).all(|row| {
                                animations[row]["derived_key_count"].as_u64()
                                    == Some(base.key_counts[row] as u64)
                                    && (0..4).all(|c| {
                                        animations[row]["base_value_bits"][c].as_u64()
                                            == Some(u64::from(base.row_bits[row][c]))
                                    })
                            }) && s.uv_transform.bits()
                                == if base.subtype_raw == 1 {
                                    vec![
                                        base.row_bits[0][0],
                                        base.row_bits[0][1],
                                        base.row_bits[0][2],
                                        base.row_bits[1][0],
                                        base.row_bits[1][1],
                                        base.row_bits[1][2],
                                    ]
                                } else {
                                    // Independent recomputation of (uv - zw) * xy.
                                    let [x, y, z, w] = base.row_bits[0].map(f32::from_bits);
                                    vec![x, y, -(z * x), -(w * y)]
                                        .into_iter()
                                        .map(f32::to_bits)
                                        .collect()
                                }
                        });
                    if s.output_index != output
                        || !base_equal
                        || !multiply_equal
                        || s.slot != sample_node.parameters_raw[5] as usize
                        || s.uv_channel != sample_node.parameters_raw[4] as usize
                        || s.uv_transform.subtype() != sample_node.subtype_raw
                        || (s.animation_base.is_none()
                            && s.uv_transform
                                .parameter_indices()
                                .iter()
                                .zip(s.uv_transform.bits())
                                .any(|(i, bits)| {
                                    sample["graph"]["boxes"][s.sample_index]["parameters_raw"][*i]
                                        .as_u64()
                                        != Some(u64::from(*bits))
                                }))
                    {
                        mismatches += 1;
                    }
                    projected += 1;
                    // Versioned diagnostics retain all three original selectors/root indices.
                    json!({"status":"projected","alpha_mix":s.alpha_mix.map(|m|json!({"mix_index":m.mix_index,"color_bits":m.color.bits,"color_first":m.color_first,"unused_weight_bits":m.unused_weight_bits,"factor":"same_sample_alpha","filter":"nearest"})),"animation_base":s.animation_base.map(|b|json!({"subtype_raw":b.subtype_raw,"static_matrix_bits":b.static_matrix_bits,"row_bits":b.row_bits,"key_counts":b.key_counts,"time":null})),"blend":s.blend.map(|b|json!({"mix_index":b.mix_index,"weight_bits":b.weight_bits,"color_bits":b.color.bits,"color_first":b.color_first})),"deep_multiply_index":s.deep_multiply_index,"fourth_sample":s.fourth_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"inner_multiply_index":s.inner_multiply_index,"third_sample":s.third_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"second_sample":s.second_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"output_index":s.output_index,"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"uv_transform_subtype":s.uv_transform.subtype(),"uv_transform_bits":s.uv_transform.bits(),"sampling":s.uv_transform.label(),"tint_bits":s.tint.bits,"multiply_index":s.multiply_index,"color_index":s.color_index,"ignored_nodes":s.ignored_nodes,"ignored_wires":s.ignored_wires})
                }
                Err(e) => json!({"status":"unsupported","reason":e}),
            }
        };
        rows.push(json!({"sha1":sha,"route_equal":equal,"wraps_equal":wraps_equal,"wrap_s_raw":graph.material.prefix.wrap_s_raw,"wrap_t_raw":graph.material.prefix.wrap_t_raw,"route":route,"projection":projection}));
    }
    source.check_stable()?;
    let report = json!({"schema_version":11,"animation_base_enabled":animation_base,"scope":"diffuse_route_oracle_and_projection_census","compared":samples.len(),"mismatches":mismatches,"projected":projected,"unsupported":samples.len()-projected,"archived_bytes_read":bytes_read,"samples":rows});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "Diffuse routes: {} compared, {mismatches} mismatches; {projected} projected",
        samples.len()
    );
    if mismatches != 0 {
        Err("route comparison failed".into())
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
