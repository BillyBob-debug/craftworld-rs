//! Read-only route oracle comparison and bounded diffuse projection census.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let flags = args.get(5..).unwrap_or_default();
    if args.len() < 5
        || flags.len() > 2
        || flags
            .iter()
            .any(|f| !["--animation-base", "--instance-white"].contains(&f.as_str()))
        || (flags.len() == 2 && flags[0] == flags[1])
    {
        return Err(
            "diffuse_parity SNAPSHOT GAME-DIR ORACLE NEW-JSON [--animation-base] [--instance-white]"
                .into(),
        );
    }
    let output = Path::new(&args[4]);
    let animation_base = flags.iter().any(|f| f == "--animation-base");
    // D86: supplied white instance color; folded roots are recomputed from oracle rows.
    // D88: single texture x instance tint (ThingColor or folded operand).
    let instance_white = flags.iter().any(|f| f == "--instance-white");
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
        let mut instance_selection = None;
        let mut instance_expected = Value::Null;
        if instance_white {
            let white = lbp_preview::DiffuseTint {
                bits: [1f32.to_bits(); 4],
            };
            if let Ok((derived, Some(p))) = lbp_preview::project_instance_color(&graph.graph, white)
            {
                if let Some(root) = p.folded_root {
                    let selected = lbp_preview::select_solid_diffuse(&derived)
                        .ok()
                        .flatten()
                        .map(|s| s.color.bits);
                    let expected = oracle_constant_fold(sample, root);
                    mismatches += usize::from(
                        root != wire.box_from as usize
                            || selected.is_none()
                            || selected != expected,
                    );
                    projected += 1;
                    let projection = json!({"status":"instance_white_folded","folded_root":root,"color_bits":selected,"oracle_fold_bits":expected,"swizzled_color_edges":p.swizzled_color_edges.iter().map(|(n,c)|json!([n,(*c as char).to_string()])).collect::<Vec<_>>()});
                    rows.push(json!({"sha1":sha,"route_equal":equal,"wraps_equal":wraps_equal,"route":route,"projection":projection}));
                    continue;
                }
                if p.folded_sampler.is_none() && p.alpha_subtract_samplers.is_empty() {
                    if let Ok(s) =
                        lbp_preview::select_instance_diffuse(&derived, &p, animation_base)
                    {
                        if let (Some(m), Some(c), None, None) = (
                            s.multiply_index,
                            s.color_index,
                            s.second_sample,
                            s.inner_multiply_index,
                        ) {
                            instance_expected =
                                oracle_instance_tint(sample, &p, m, c, s.sample_index)
                                    .map_or(Value::Null, |bits| json!({"multiply_index":m,"sample_index":s.sample_index,"color_index":c,"tint_bits":bits}));
                            instance_selection = Some(s);
                        }
                    }
                }
            }
        }
        let solid = lbp_preview::select_solid_diffuse(&graph.graph);
        // D87: CONST4 alone and COLOR × COLOR, original graph only.
        let solid = match solid {
            Ok(None) => lbp_preview::select_small_solid(&graph.graph),
            other => other,
        };
        let projection = if let Ok(Some(s)) = solid {
            let nodes = &sample["graph"]["boxes"];
            let param = |node: usize, i: usize| {
                nodes[node]["parameters_raw"][i]
                    .as_u64()
                    .map(|v| f32::from_bits(v as u32))
            };
            let color_equal = if let Some([l, r]) = s.product_of {
                // Independent f32 recomputation of the oracle's two COLOR inputs.
                let oracle_wires = sample["graph"]["wires"]
                    .as_array()
                    .ok_or("oracle wires required")?;
                let inputs: Vec<_> = oracle_wires
                    .iter()
                    .filter(|w| w["box_to"].as_u64() == Some(s.color_index as u64))
                    .collect();
                let edge = |source: usize, port: u64| {
                    inputs.iter().any(|w| {
                        w["box_from"].as_u64() == Some(source as u64)
                            && w["port_to_raw"].as_u64() == Some(port)
                            && w["port_from_raw"] == 0
                            && w["swizzle_raw"] == json!([0, 0, 0, 0, 0])
                    })
                };
                inputs.len() == 2
                    && edge(l, 0)
                    && edge(r, 1)
                    && nodes[s.color_index]["type_raw"] == 10
                    && nodes[l]["type_raw"] == 3
                    && nodes[r]["type_raw"] == 3
                    && (0..4).all(|i| match (param(l, i), param(r, i)) {
                        (Some(a), Some(b)) => (a * b).to_bits() == s.color.bits[i],
                        _ => false,
                    })
            } else {
                matches!(nodes[s.color_index]["type_raw"].as_u64(), Some(3 | 7))
                    && s.color.bits.iter().enumerate().all(|(i, b)| {
                        nodes[s.color_index]["parameters_raw"][i].as_u64() == Some(u64::from(*b))
                    })
            };
            if s.output_index != output || s.color_index != wire.box_from as usize || !color_equal {
                mismatches += 1;
            }
            projected += 1;
            json!({"status":"solid_projected","output_index":s.output_index,"color_index":s.color_index,"product_of":s.product_of,"color_bits":s.color.bits,"texture":null,"uv_channel":null,"ignored_nodes":s.ignored_nodes,"ignored_wires":s.ignored_wires})
        } else {
            let selected = match instance_selection {
                Some(s) => Ok(s),
                None => {
                    lbp_preview::select_diffuse_with_animation_base(&graph.graph, animation_base)
                }
            };
            match selected {
                Ok(s) => {
                    let sample_node = &graph.graph.boxes[s.sample_index];
                    // D78: the one animated leaf, in any product role, must equal its
                    // base-row transform; every static leaf must equal its parameters.
                    let transform_equal =
                        |index: usize, transform: lbp_preview::SamplingTransform| {
                            match s.animation_base {
                                Some(base) if base.sample_index == index => {
                                    transform.bits()
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
                                }
                                // D87: sampler port-1 CONST/CONST2 scale, recomputed
                                // from the oracle's sampler and constant parameters.
                                _ if s.sampler_scale.is_some() && index == s.sample_index => {
                                    let scale = s.sampler_scale.unwrap();
                                    let nodes = &sample["graph"]["boxes"];
                                    let raw = |node: usize, i: usize| {
                                        nodes[node]["parameters_raw"][i]
                                            .as_u64()
                                            .map(|v| f32::from_bits(v as u32))
                                    };
                                    let c = scale.const_index;
                                    let factor = match nodes[c]["type_raw"].as_u64() {
                                        Some(4) => raw(c, 0).zip(raw(c, 0)),
                                        Some(5) => raw(c, 0).zip(raw(c, 1)),
                                        _ => None,
                                    };
                                    let wired =
                                        sample["graph"]["wires"].as_array().is_some_and(|w| {
                                            let into: Vec<_> = w
                                                .iter()
                                                .filter(|w| {
                                                    w["box_to"].as_u64() == Some(index as u64)
                                                })
                                                .collect();
                                            into.len() == 1
                                                && into[0]["box_from"].as_u64() == Some(c as u64)
                                                && into[0]["port_to_raw"] == 1
                                                && into[0]["swizzle_raw"] == json!([0, 0, 0, 0, 0])
                                        });
                                    match (
                                        factor,
                                        (0..4).map(|i| raw(index, i)).collect::<Option<Vec<_>>>(),
                                    ) {
                                        (Some((fx, fy)), Some(p)) if wired => {
                                            transform.bits()
                                                == [p[0] * fx, p[1] * fy, p[2] * fx, p[3] * fy]
                                                    .map(f32::to_bits)
                                        }
                                        _ => false,
                                    }
                                }
                                _ => transform
                                    .parameter_indices()
                                    .iter()
                                    .zip(transform.bits())
                                    .all(|(i, b)| {
                                        sample["graph"]["boxes"][index]["parameters_raw"][*i]
                                            .as_u64()
                                            == Some(u64::from(*b))
                                    }),
                            }
                        };
                    let multiply_equal = if let Some(add) = s.texture_add {
                        add_equal(sample, &s, add, wire.box_from as usize, &transform_equal)?
                    } else if let Some(mix) = s.alpha_mix {
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
                    } else if let Some(mix) = s.texture_alpha_mix {
                        // D83: port-0 sample, port-1 sample, factor = alpha of one of them.
                        let root = mix.mix_index;
                        let second = s.second_sample.ok_or("alpha MIX second sample missing")?;
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
                        let factor = if mix.factor_port == 0 {
                            s.sample_index
                        } else {
                            second.sample_index
                        };
                        let second_node = &nodes[second.sample_index];
                        root == wire.box_from as usize
                            && mixed.len() == 3
                            && s.sample_index != second.sample_index
                            && s.alpha_mix.is_none()
                            && s.blend.is_none()
                            && s.multiply_index.is_none()
                            && s.third_sample.is_none()
                            && edge(s.sample_index, 0, json!([0, 0, 0, 0, 0]))
                            && edge(second.sample_index, 1, json!([0, 0, 0, 0, 0]))
                            && edge(factor, 2, json!([97, 0, 0, 0, 0]))
                            && nodes[root]["type_raw"] == 12
                            && nodes[root]["subtype_raw"] == 0
                            && nodes[root]["parameters_raw"][0].as_u64()
                                == Some(u64::from(mix.unused_weight_bits))
                            && second_node["type_raw"] == 1
                            && second_node["subtype_raw"].as_u64()
                                == Some(u64::from(second.uv_transform.subtype()))
                            && second_node["parameters_raw"][4].as_u64()
                                == Some(second.uv_channel as u64)
                            && second_node["parameters_raw"][5].as_u64() == Some(second.slot as u64)
                            && transform_equal(second.sample_index, second.uv_transform)
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
                            && transform_equal(second.sample_index, second.uv_transform)
                    } else if let (Some(inner), None, Some(color), Some(second), Some(root)) = (
                        s.inner_multiply_index,
                        s.third_sample,
                        s.color_index,
                        s.second_sample,
                        s.multiply_index,
                    ) {
                        // D75: two samples and one static COLOR under two multiplies in
                        // either nesting; leaves must equal the oracle's root + inner inputs.
                        let oracle_wires = sample["graph"]["wires"]
                            .as_array()
                            .ok_or("oracle wires required")?;
                        let nodes = &sample["graph"]["boxes"];
                        let inputs = |target: usize| {
                            oracle_wires
                                .iter()
                                .filter(|w| w["box_to"].as_u64() == Some(target as u64))
                                .collect::<Vec<_>>()
                        };
                        let root_inputs = inputs(root);
                        let inner_inputs = inputs(inner);
                        let mut leaves = [s.sample_index, second.sample_index, color];
                        leaves.sort_unstable();
                        let mut graph_leaves: Vec<usize> = root_inputs
                            .iter()
                            .chain(inner_inputs.iter())
                            .filter_map(|w| w["box_from"].as_u64())
                            .map(|v| v as usize)
                            .filter(|&v| v != inner)
                            .collect();
                        graph_leaves.sort_unstable();
                        let second_node = &nodes[second.sample_index];
                        root == wire.box_from as usize
                            && root_inputs.len() == 2
                            && inner_inputs.len() == 2
                            && root_inputs
                                .iter()
                                .any(|w| w["box_from"].as_u64() == Some(inner as u64))
                            && graph_leaves == leaves
                            && [root, inner].iter().all(|i| {
                                nodes[*i]["type_raw"] == 10 && nodes[*i]["subtype_raw"] == 0
                            })
                            && nodes[color]["type_raw"] == 3
                            && s.tint.bits.iter().enumerate().all(|(i, b)| {
                                nodes[color]["parameters_raw"][i].as_u64() == Some(u64::from(*b))
                            })
                            && second_node["type_raw"] == 1
                            && second_node["subtype_raw"].as_u64()
                                == Some(u64::from(second.uv_transform.subtype()))
                            && second_node["parameters_raw"][4].as_u64()
                                == Some(second.uv_channel as u64)
                            && second_node["parameters_raw"][5].as_u64() == Some(second.slot as u64)
                            && transform_equal(second.sample_index, second.uv_transform)
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
                            // D67: roles may be permuted (commutative product); the three
                            // leaves must equal the outer leaf plus both inner inputs.
                            let mut leaves =
                                [s.sample_index, second.sample_index, third.sample_index];
                            leaves.sort_unstable();
                            let mut graph_leaves: Vec<usize> = root_inputs
                                .iter()
                                .chain(inner_inputs.iter())
                                .filter_map(|w| w["box_from"].as_u64())
                                .map(|v| v as usize)
                                .filter(|&v| v != inner)
                                .collect();
                            graph_leaves.sort_unstable();
                            has(&root_inputs, inner) && graph_leaves == leaves
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
                                        && transform_equal(leaf.sample_index, leaf.uv_transform)
                                })
                    } else {
                        match (s.multiply_index, s.color_index) {
                            (Some(multiply), Some(color)) => {
                                (if instance_expected.is_null() {
                                    &sample["static_diffuse_multiply"]
                                } else {
                                    &instance_expected
                                }) == &json!({"multiply_index":multiply,"sample_index":s.sample_index,"color_index":color,"tint_bits":s.tint.bits})
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
                                    && transform_equal(second.sample_index, second.uv_transform)
                            }),
                            _ => false,
                        }
                    };
                    let base_equal = s.animation_base.is_none_or(|base| {
                        let leaves = [Some(s.sample_index)].into_iter().chain(
                            [s.second_sample, s.third_sample, s.fourth_sample]
                                .map(|l| l.map(|l| l.sample_index)),
                        );
                        let node = &sample["graph"]["boxes"][base.sample_index];
                        leaves.flatten().any(|l| l == base.sample_index)
                            && node["subtype_raw"].as_u64() == Some(u64::from(base.subtype_raw))
                            && [0, 1, 2, 3, 6, 7].iter().zip(base.static_matrix_bits).all(
                                |(i, bits)| {
                                    node["parameters_raw"][*i].as_u64() == Some(u64::from(bits))
                                },
                            )
                            && (0..2).all(|row| {
                                node["animations"][row]["derived_key_count"].as_u64()
                                    == Some(base.key_counts[row] as u64)
                                    && (0..4).all(|c| {
                                        node["animations"][row]["base_value_bits"][c].as_u64()
                                            == Some(u64::from(base.row_bits[row][c]))
                                    })
                            })
                    });
                    if s.output_index != output
                        || !base_equal
                        || !multiply_equal
                        || s.slot != sample_node.parameters_raw[5] as usize
                        || s.uv_channel != sample_node.parameters_raw[4] as usize
                        || s.uv_transform.subtype() != sample_node.subtype_raw
                        || !transform_equal(s.sample_index, s.uv_transform)
                    {
                        mismatches += 1;
                    }
                    projected += 1;
                    // Versioned diagnostics retain all three original selectors/root indices.
                    json!({"status":"projected","texture_add":s.texture_add.map(|a|json!({"add_index":a.add_index,"addend_index":a.addend_index,"addend_bits":a.addend.map(|c|c.bits),"policy":"linear_sum_clamped"})),"sampler_scale":s.sampler_scale.map(|c|json!({"const_index":c.const_index,"factor_bits":c.factor_bits,"source_bits":c.source_bits})),"alpha_mix":s.alpha_mix.map(|m|json!({"mix_index":m.mix_index,"color_bits":m.color.bits,"color_first":m.color_first,"unused_weight_bits":m.unused_weight_bits,"factor":"same_sample_alpha","filter":"nearest"})),"texture_alpha_mix":s.texture_alpha_mix.map(|m|json!({"mix_index":m.mix_index,"factor_port":m.factor_port,"unused_weight_bits":m.unused_weight_bits,"filter":"nearest_grid"})),"animation_base":s.animation_base.map(|b|json!({"subtype_raw":b.subtype_raw,"sample_index":b.sample_index,"static_matrix_bits":b.static_matrix_bits,"row_bits":b.row_bits,"key_counts":b.key_counts,"time":null})),"blend":s.blend.map(|b|json!({"mix_index":b.mix_index,"weight_bits":b.weight_bits,"color_bits":b.color.bits,"color_first":b.color_first})),"deep_multiply_index":s.deep_multiply_index,"fourth_sample":s.fourth_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"inner_multiply_index":s.inner_multiply_index,"third_sample":s.third_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"second_sample":s.second_sample.map(|s|json!({"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"transform_bits":s.uv_transform.bits()})),"output_index":s.output_index,"sample_index":s.sample_index,"slot":s.slot,"uv_channel":s.uv_channel,"uv_transform_subtype":s.uv_transform.subtype(),"uv_transform_bits":s.uv_transform.bits(),"sampling":s.uv_transform.label(),"tint_bits":s.tint.bits,"multiply_index":s.multiply_index,"color_index":s.color_index,"ignored_nodes":s.ignored_nodes,"ignored_wires":s.ignored_wires})
                }
                Err(e) => json!({"status":"unsupported","reason":e}),
            }
        };
        rows.push(json!({"sha1":sha,"route_equal":equal,"wraps_equal":wraps_equal,"wrap_s_raw":graph.material.prefix.wrap_s_raw,"wrap_t_raw":graph.material.prefix.wrap_t_raw,"route":route,"projection":projection,"instance_white_expected":instance_expected}));
    }
    source.check_stable()?;
    let report = json!({"schema_version":18,"animation_base_enabled":animation_base,"instance_white_enabled":instance_white,"scope":"diffuse_route_oracle_and_projection_census","compared":samples.len(),"mismatches":mismatches,"projected":projected,"unsupported":samples.len()-projected,"archived_bytes_read":bytes_read,"samples":rows});
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
/// D89: the ADD root and its operands re-read from the oracle's graph rows (Toolkit
/// GfxAssembler ADD = port 0 + port 1): two samples on ports 0/1, or COLOR on one port
/// and the sample (directly or through a sample x COLOR MULTIPLY) on the other.
fn add_equal(
    sample: &Value,
    s: &lbp_preview::DiffuseSelection,
    add: lbp_preview::DiffuseAdd,
    root: usize,
    transform_equal: &dyn Fn(usize, lbp_preview::SamplingTransform) -> bool,
) -> Result<bool, String> {
    let nodes = &sample["graph"]["boxes"];
    let wires = sample["graph"]["wires"]
        .as_array()
        .ok_or("oracle wires required")?;
    let into = |target: usize| -> Vec<(u64, u64)> {
        let mut v: Vec<_> = wires
            .iter()
            .filter(|w| w["box_to"].as_u64() == Some(target as u64))
            .filter(|w| w["port_from_raw"] == 0 && w["swizzle_raw"] == json!([0, 0, 0, 0, 0]))
            .filter_map(|w| Some((w["port_to_raw"].as_u64()?, w["box_from"].as_u64()?)))
            .collect();
        v.sort();
        v
    };
    let all_in = |target: usize| {
        wires
            .iter()
            .filter(|w| w["box_to"].as_u64() == Some(target as u64))
            .count()
    };
    let color_equal = |index: usize, bits: [u32; 4]| {
        nodes[index]["type_raw"] == 3
            && all_in(index) == 0
            && bits
                .iter()
                .enumerate()
                .all(|(i, b)| nodes[index]["parameters_raw"][i].as_u64() == Some(u64::from(*b)))
    };
    let leaf_equal =
        |index: usize, slot: usize, channel: usize, t: lbp_preview::SamplingTransform| {
            nodes[index]["type_raw"] == 1
                && nodes[index]["subtype_raw"].as_u64() == Some(u64::from(t.subtype()))
                && nodes[index]["parameters_raw"][4].as_u64() == Some(channel as u64)
                && nodes[index]["parameters_raw"][5].as_u64() == Some(slot as u64)
                && all_in(index) == 0
                && transform_equal(index, t)
        };
    let operands = into(root);
    let mut ok = add.add_index == root
        && nodes[root]["type_raw"] == 11
        && nodes[root]["subtype_raw"] == 0
        && all_in(root) == 2
        && operands.len() == 2
        && operands[0].0 == 0
        && operands[1].0 == 1
        && leaf_equal(s.sample_index, s.slot, s.uv_channel, s.uv_transform);
    match (add.addend, add.addend_index, s.second_sample) {
        (None, None, Some(second)) => {
            ok &= operands[0].1 == s.sample_index as u64
                && operands[1].1 == second.sample_index as u64
                && s.multiply_index.is_none()
                && leaf_equal(
                    second.sample_index,
                    second.slot,
                    second.uv_channel,
                    second.uv_transform,
                );
        }
        (Some(addend), Some(color), None) if ok => {
            let other = if operands[0].1 == color as u64 {
                operands[1].1
            } else {
                operands[0].1
            };
            ok &= operands.iter().any(|o| o.1 == color as u64) && color_equal(color, addend.bits);
            match (s.multiply_index, s.color_index) {
                (None, None) => {
                    ok &= other == s.sample_index as u64 && s.tint.bits == [1f32.to_bits(); 4]
                }
                (Some(m), Some(c)) => {
                    let factors: Vec<_> = into(m).iter().map(|o| o.1).collect();
                    ok &= other == m as u64
                        && nodes[m]["type_raw"] == 10
                        && nodes[m]["subtype_raw"] == 0
                        && all_in(m) == 2
                        && factors.len() == 2
                        && factors.contains(&(s.sample_index as u64))
                        && factors.contains(&(c as u64))
                        && c != color
                        && color_equal(c, s.tint.bits);
                }
                _ => ok = false,
            }
        }
        _ => ok = false,
    }
    Ok(ok)
}
/// D86 independent recomputation from the oracle's graph rows only (Toolkit
/// GfxAssembler forms: THING_COLOR = supplied white, COLOR words 0..3, CONSTANT word 0,
/// MULTIPLY l*r, MIX ((r-l)*w)+l with w = word 0, one-component swizzle).
fn oracle_constant_fold(sample: &Value, root: usize) -> Option<[u32; 4]> {
    let boxes = &sample["graph"]["boxes"];
    let word = |i: u64, k: usize| {
        boxes[i as usize]["parameters_raw"][k]
            .as_u64()
            .map(|v| f32::from_bits(v as u32))
    };
    let kind = boxes[root]["type_raw"].as_u64()?;
    let mut operands: [Option<[f32; 4]>; 2] = [None, None];
    let mut scalars = 0;
    for w in sample["graph"]["wires"].as_array()? {
        if w["box_to"].as_u64() != Some(root as u64) {
            continue;
        }
        let from = w["box_from"].as_u64()?;
        let port = w["port_to_raw"].as_u64()? as usize;
        let mut value: [f32; 4] = match boxes[from as usize]["type_raw"].as_u64()? {
            2 => [1.; 4],
            3 => [
                word(from, 0)?,
                word(from, 1)?,
                word(from, 2)?,
                word(from, 3)?,
            ],
            4 => {
                scalars += 1;
                [word(from, 0)?; 4]
            }
            _ => return None,
        };
        let swizzle: Vec<u64> = w["swizzle_raw"]
            .as_array()?
            .iter()
            .filter_map(Value::as_u64)
            .filter(|&c| c != 0)
            .collect();
        if let [c] = swizzle[..] {
            let i = "xyzw"
                .find(c as u8 as char)
                .or("rgba".find(c as u8 as char))?;
            value = [value[i]; 4];
            scalars += 1;
        } else if !swizzle.is_empty() {
            return None;
        }
        *operands.get_mut(port)? = Some(value);
    }
    let ([Some(l), Some(r)], true) = (operands, scalars < 2) else {
        return None;
    };
    let weight = word(root as u64, 0)?;
    Some(std::array::from_fn(|i| match kind {
        10 => (l[i] * r[i]).to_bits(),
        12 => ((r[i] - l[i]) * weight + l[i]).to_bits(),
        _ => u32::MAX,
    }))
}
/// D88 independent expectation from the oracle's graph rows only: the root MULTIPLY has
/// exactly the tint node and the sample on ports 0/1, unswizzled; the tint is the
/// supplied white for THING_COLOR (type 2) or the oracle-row constant fold of the
/// folded operand.
fn oracle_instance_tint(
    sample: &Value,
    projection: &lbp_preview::InstanceProjection,
    multiply: usize,
    color: usize,
    texture: usize,
) -> Option<[u32; 4]> {
    let boxes = &sample["graph"]["boxes"];
    let inputs: Vec<_> = sample["graph"]["wires"]
        .as_array()?
        .iter()
        .filter(|w| w["box_to"].as_u64() == Some(multiply as u64))
        .collect();
    let edge = |from: usize| {
        inputs.iter().any(|w| {
            w["box_from"].as_u64() == Some(from as u64)
                && w["port_from_raw"] == 0
                && w["swizzle_raw"] == json!([0, 0, 0, 0, 0])
        })
    };
    let mut ports: Vec<_> = inputs
        .iter()
        .filter_map(|w| w["port_to_raw"].as_u64())
        .collect();
    ports.sort_unstable();
    if boxes[multiply]["type_raw"] != 10
        || ports != [0, 1]
        || !edge(color)
        || !edge(texture)
        || boxes[texture]["type_raw"] != 1
    {
        return None;
    }
    match boxes[color]["type_raw"].as_u64()? {
        2 => Some([1f32.to_bits(); 4]),
        10 | 12 if projection.folded_operand == Some(color) => oracle_constant_fold(sample, color),
        _ => None,
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
