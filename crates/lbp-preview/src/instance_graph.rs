//! D105: the D100 graph evaluator on the plain supplied-instance substitution.
//! Supplied-color opt-in only. When the D86/D88/D96 projection's rewritten graph has
//! no loadable selection (a constant fold, nested tint or operand fold the fixed routes
//! still refuse, a D99 product tree or a D89 ADD the instance guards refuse), the
//! ThingColor nodes of the original graph are replaced by the supplied COLOR and
//! nothing else is rewritten, so every former fold is the evaluator's own COLOR and
//! MULTIPLY/MIX nodes. Wired samplers (UV alpha subtraction, sampler-port inputs)
//! stay refused by the evaluator's sampler rule. Evidence: docs/d105-instance-graphs.md.
use lbp_resource::material::GraphMetadata;

const REFUSED: &str = "Unsupported: supplied instance graph evaluator";

/// The original graph with only the projected ThingColor nodes turned into the
/// supplied COLOR, and a projection that records no rewrite (`evaluated` set).
fn plain(
    graph: &GraphMetadata,
    p: &crate::InstanceProjection,
) -> (GraphMetadata, crate::InstanceProjection) {
    let mut derived = graph.clone();
    for &index in &p.color_nodes {
        derived.boxes[index].type_raw = 3;
        derived.boxes[index].parameters_raw[..4].copy_from_slice(&p.color.bits);
    }
    let projection = crate::InstanceProjection {
        color: p.color,
        nested_tint: None,
        folded_sampler: None,
        color_nodes: p.color_nodes.clone(),
        alpha_subtract_samplers: vec![],
        folded_root: None,
        swizzled_color_edges: vec![],
        folded_operand: None,
        sampler_port_folds: vec![],
        evaluated: true,
    };
    (derived, projection)
}

/// Evaluator selection on a plain substituted graph, with the evaluator's own compile
/// reason when it refuses (the D100 fallback keeps the fixed route's reason instead).
pub fn select_evaluated(plain: &GraphMetadata) -> Result<crate::DiffuseSelection, String> {
    let output: Vec<_> = (0..plain.boxes.len())
        .filter(|&i| plain.boxes[i].type_raw == 0)
        .collect();
    let root = match output.as_slice() {
        [o] => plain
            .wires
            .iter()
            .filter(|w| w.box_to as usize == *o && w.port_to_raw == 0)
            .map(|w| w.box_from)
            .collect::<Vec<_>>(),
        _ => vec![],
    };
    if let [root] = root.as_slice() {
        if *root >= 0 && (*root as usize) < plain.boxes.len() {
            crate::grid_graph_program(plain, *root as usize)
                .map_err(|e| format!("{REFUSED}: {e}"))?;
        }
    }
    crate::grid_graph::select(plain, false, REFUSED.into())
}

/// Supplied-color projection for the loader and the censuses. The D86..D96 projection
/// is kept whenever it already selects (solid, or a loadable texture route); otherwise
/// the plain substitution is returned when the evaluator bakes it.
pub fn project_instance_graph(
    graph: &GraphMetadata,
    color: crate::DiffuseTint,
    animation_base: bool,
) -> Result<(GraphMetadata, Option<crate::InstanceProjection>), String> {
    let (derived, projection) = crate::project_instance_color(graph, color)?;
    let Some(p) = projection else {
        return Ok((derived, None));
    };
    if !matches!(crate::select_solid_diffuse(&derived), Ok(None))
        // D89 ADD stays refused by the loader under a supplied colour; the evaluator
        // may take the same graph instead.
        || crate::select_instance_diffuse(&derived, &p, animation_base)
            .is_ok_and(|s| s.texture_add.is_none())
    {
        return Ok((derived, Some(p)));
    }
    let (evaluated, q) = plain(graph, &p);
    if matches!(crate::select_solid_diffuse(&evaluated), Ok(None))
        && select_evaluated(&evaluated).is_ok()
    {
        return Ok((evaluated, Some(q)));
    }
    Ok((derived, Some(p)))
}

/// Why the plain substitution is not evaluated (census diagnostics), or `None` when
/// there is no projected ThingColor.
pub fn instance_graph_refusal(graph: &GraphMetadata, color: crate::DiffuseTint) -> Option<String> {
    let (_, Some(p)) = crate::project_instance_color(graph, color).ok()? else {
        return None;
    };
    select_evaluated(&plain(graph, &p).0).err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};

    fn node(kind: u32, params: [f32; 8]) -> BoxMetadata {
        BoxMetadata {
            type_raw: kind,
            subtype_raw: 0,
            declared_parameter_count: 8,
            parameters_raw: params.map(f32::to_bits),
            position_bits: [0; 4],
            texture_slot_raw: None,
            texture_slot_in_range: None,
            animations: std::array::from_fn(|_| AnimationMetadata {
                base_value_bits: [0; 4],
                component_value_count: 0,
                component_bytes_sha256: String::new(),
                name_hex: "000000".into(),
                components_raw: 0,
                derived_key_count: 0,
            }),
        }
    }
    fn sample(slot: u32, scale: f32) -> BoxMetadata {
        let mut b = node(1, [scale, scale, 0., 0., 0., 0., 0., 0.]);
        b.parameters_raw[4] = 0;
        b.parameters_raw[5] = slot;
        b
    }
    fn thing() -> BoxMetadata {
        let mut b = node(2, [0.; 8]);
        b.parameters_raw[0] = 8;
        b
    }
    fn edge(from: i32, to: i32, port: u8) -> WireMetadata {
        WireMetadata {
            box_from: from,
            port_from_raw: 0,
            box_to: to,
            port_to_raw: port,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        }
    }
    fn graph(boxes: Vec<BoxMetadata>, wires: Vec<WireMetadata>) -> GraphMetadata {
        GraphMetadata {
            box_count: boxes.len(),
            wire_count: wires.len(),
            boxes,
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        }
    }
    const HALF: crate::DiffuseTint = crate::DiffuseTint {
        bits: [0x3f00_0000; 4],
    };

    /// `(THING × T0) × T1` folds into a nested tint that the fixed routes take; the
    /// D105 path must leave that projection alone.
    #[test]
    fn d105_existing_instance_selections_are_kept() {
        let g = graph(
            vec![
                node(0, [0.; 8]),
                node(10, [0.; 8]),
                node(10, [0.; 8]),
                thing(),
                sample(0, 1.),
                sample(1, 1.),
            ],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(5, 1, 1),
                edge(3, 2, 0),
                edge(4, 2, 1),
            ],
        );
        let (_, kept) = crate::project_instance_color(&g, HALF).unwrap();
        let (_, p) = project_instance_graph(&g, HALF, false).unwrap();
        assert_eq!(p, kept);
        assert!(!p.unwrap().evaluated);
    }

    /// `BLEND(T1, T2, T3.r) × (THING × T0)`: the projection folds the inner product into
    /// a nested tint, which the D100 guard refuses; the plain graph evaluates on the grid
    /// with the ThingColor as a COLOR node under the original MULTIPLY.
    fn nested_blend(scale: f32) -> GraphMetadata {
        let mut mask = edge(8, 5, 2);
        mask.swizzle_raw = [b'r', 0, 0, 0, 0];
        graph(
            vec![
                node(0, [0.; 8]),
                node(10, [0.; 8]),
                node(10, [0.; 8]),
                thing(),
                sample(0, 1.),
                node(16, [0.15, 0.16, 0., 0., 0., 0., 0., 0.]),
                sample(1, 1.),
                sample(2, scale),
                sample(3, 1.),
            ],
            vec![
                edge(1, 0, 0),
                edge(5, 1, 0),
                edge(2, 1, 1),
                edge(3, 2, 0),
                edge(4, 2, 1),
                edge(6, 5, 0),
                edge(7, 5, 1),
                mask,
            ],
        )
    }

    #[test]
    fn d105_nested_instance_tint_evaluates_the_plain_graph() {
        let g = nested_blend(2.);
        assert!(crate::select_diffuse(&g).is_err());
        let (rewritten, q) = crate::project_instance_color(&g, HALF).unwrap();
        let q = q.unwrap();
        assert!(q.nested_tint.is_some());
        let refused = crate::select_instance_diffuse(&rewritten, &q, false).unwrap_err();
        assert!(refused.contains("folded or rewritten"), "{refused}");
        let (derived, p) = project_instance_graph(&g, HALF, false).unwrap();
        let p = p.unwrap();
        assert!(p.evaluated && crate::grid_graph::instance_supported(&p));
        assert_eq!(derived.boxes[3].type_raw, 3);
        assert_eq!(derived.boxes[3].parameters_raw[..4], HALF.bits);
        assert_eq!(format!("{:?}", derived.wires), format!("{:?}", g.wires));
        let s = crate::select_instance_diffuse(&derived, &p, false).unwrap();
        let marker = s.grid_graph.unwrap();
        assert_eq!((marker.nodes, marker.samples, marker.sources), (8, 4, 4));
        let program = crate::grid_graph_program(&derived, 1).unwrap();
        assert!(program
            .nodes
            .iter()
            .any(|n| n.index == 3 && n.op == crate::GraphOp::Color(HALF.bits)));
        assert_eq!(instance_graph_refusal(&g, HALF), None);
        // Default route (no supplied color) is unchanged.
        assert!(crate::select_diffuse(&g).is_err());
    }

    /// Off-grid leaves and wired samplers stay refused with the evaluator's reason;
    /// the original projection (and its reason) is returned.
    #[test]
    fn d105_refusals_keep_the_projection_and_name_the_reason() {
        // Off-grid scale 1.5: since D103 the evaluator resamples it (labelled preview
        // approximation), so it is evaluated rather than refused.
        let g = nested_blend(1.5);
        let (_, p) = project_instance_graph(&g, HALF, false).unwrap();
        assert!(p.unwrap().evaluated);
        assert_eq!(instance_graph_refusal(&g, HALF), None);
        // ThingColor alpha on a sampler UV port: the plain graph keeps the wire.
        let mut port = edge(3, 6, 2);
        port.swizzle_raw = [b'a', 0, 0, 0, 0];
        let mut h = nested_blend(2.);
        h.wires.push(port);
        let reason = instance_graph_refusal(&h, HALF).unwrap();
        assert!(reason.starts_with(REFUSED), "{reason}");
        let (_, p) = project_instance_graph(&h, HALF, false).unwrap();
        assert!(!p.unwrap().evaluated);
    }
}
