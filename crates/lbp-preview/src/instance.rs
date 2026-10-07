//! Explicit supplied linear RGBA input, never an inferred game-instance default.
//! Independent projection policy and pinned operation evidence: docs/instance-preview.md.
use lbp_resource::material::GraphMetadata;

#[derive(Debug, Clone, Copy, Default)]
pub struct PreviewOptions {
    pub animation_base: bool,
    pub instance_color: Option<crate::DiffuseTint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceProjection {
    pub color: crate::DiffuseTint,
    pub nested_tint: Option<crate::DiffuseTint>,
    /// Derived sampler node and its original source sampler node.
    pub folded_sampler: Option<[usize; 2]>,
    pub color_nodes: Vec<usize>,
    pub alpha_subtract_samplers: Vec<usize>,
}

/// Bound and validate the selected diffuse ancestry before deriving a graph.
/// The original graph, other branches, and source bytes remain intact.
pub fn project_instance_color(
    graph: &GraphMetadata,
    color: crate::DiffuseTint,
) -> Result<(GraphMetadata, Option<InstanceProjection>), String> {
    color.validate()?;
    if graph.boxes.len() > 1024
        || graph.wires.len() > 4096
        || graph.wires.iter().any(|w| {
            w.box_from < 0
                || w.box_to < 0
                || w.box_from as usize >= graph.boxes.len()
                || w.box_to as usize >= graph.boxes.len()
        })
    {
        return Err("Unsupported: instance input graph bounds/endpoints".into());
    }
    let outputs: Vec<_> = graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0)
        .collect();
    if outputs.len() != 1 {
        return Err("Unsupported: instance input requires one output".into());
    }
    let diffuse: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == outputs[0].0 && w.port_to_raw == 0)
        .collect();
    if diffuse.len() != 1 {
        return Err("Unsupported: no unique diffuse connection; other material channels require separate support".into());
    }
    let mut selected = vec![false; graph.boxes.len()];
    let mut pending = vec![diffuse[0].box_from as usize];
    while let Some(index) = pending.pop() {
        if selected[index] {
            continue;
        }
        selected[index] = true;
        pending.extend(
            graph
                .wires
                .iter()
                .filter(|w| w.box_to as usize == index)
                .map(|w| w.box_from as usize),
        );
    }
    let mut derived = graph.clone();
    let mut projection = InstanceProjection {
        color,
        nested_tint: None,
        folded_sampler: None,
        color_nodes: vec![],
        alpha_subtract_samplers: vec![],
    };
    for (index, node) in graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(i, b)| selected[*i] && b.type_raw == 2)
    {
        if node.subtype_raw != 0
            || node.parameters_raw != [8, 0, 0, 0, 0, 0, 0, 0]
            || node
                .animations
                .iter()
                .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|b| b != b'0'))
            || graph.wires.iter().any(|w| w.box_to as usize == index)
        {
            return Err(
                "Unsupported: instance input needs an unwired static ThingColor variant 8".into(),
            );
        }
        projection.color_nodes.push(index);
        derived.boxes[index].type_raw = 3;
        derived.boxes[index].parameters_raw[..4].copy_from_slice(&color.bits);
    }
    for (index, node) in graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(i, b)| selected[*i] && b.type_raw == 1)
    {
        let inputs: Vec<_> = graph
            .wires
            .iter()
            .filter(|w| w.box_to as usize == index)
            .collect();
        if inputs.is_empty() {
            continue;
        }
        if inputs.len() != 1
            || node.subtype_raw != 0
            || inputs[0].port_to_raw != 2
            || inputs[0].port_from_raw != 0
            || inputs[0].swizzle_raw != [b'a', 0, 0, 0, 0]
            || !projection
                .color_nodes
                .contains(&(inputs[0].box_from as usize))
        {
            return Err("Unsupported: supplied instance input only supports scalar alpha subtraction on sampler port 2".into());
        }
        projection.alpha_subtract_samplers.push(index);
    }
    derived.wires.retain(|w| {
        !projection
            .alpha_subtract_samplers
            .contains(&(w.box_to as usize))
    });
    // Exactly A × (supplied COLOR × B), independently reduced to two
    // samplers and a retained tint. No recursive graph evaluation.
    let root = diffuse[0].box_from as usize;
    fn inputs(g: &GraphMetadata, index: usize) -> Vec<&lbp_resource::material::WireMetadata> {
        g.wires
            .iter()
            .filter(|w| w.box_to as usize == index)
            .collect()
    }
    let static_product = |g: &GraphMetadata, index: usize| {
        let b = &g.boxes[index];
        b.type_raw == 10
            && b.subtype_raw == 0
            && b.animations
                .iter()
                .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|v| v == b'0'))
    };
    if static_product(&derived, root) {
        let outer = inputs(&derived, root);
        if outer.len() == 2
            && outer
                .iter()
                .all(|w| w.port_from_raw == 0 && w.swizzle_raw == [0; 5])
            && outer[0].port_to_raw != outer[1].port_to_raw
            && outer.iter().all(|w| w.port_to_raw < 2)
        {
            for edge in outer {
                let inner = edge.box_from as usize;
                if !static_product(&derived, inner) {
                    continue;
                }
                let children = inputs(&derived, inner);
                if children.len() != 2
                    || children.iter().any(|w| {
                        w.port_from_raw != 0 || w.swizzle_raw != [0; 5] || w.port_to_raw > 1
                    })
                    || children[0].port_to_raw == children[1].port_to_raw
                {
                    continue;
                }
                let color_edge = children
                    .iter()
                    .find(|w| projection.color_nodes.contains(&(w.box_from as usize)));
                let texture_edge = children
                    .iter()
                    .find(|w| derived.boxes[w.box_from as usize].type_raw == 1);
                if let (Some(c), Some(t)) = (color_edge, texture_edge) {
                    let texture_index = t.box_from as usize;
                    if !inputs(&derived, texture_index).is_empty()
                        || projection.alpha_subtract_samplers.contains(&texture_index)
                    {
                        continue;
                    }
                    let tint = crate::DiffuseTint {
                        bits: derived.boxes[c.box_from as usize].parameters_raw[..4]
                            .try_into()
                            .unwrap(),
                    };
                    tint.validate()?;
                    let replacement = derived.boxes[texture_index].clone();
                    derived.boxes[inner] = replacement;
                    derived.wires.retain(|w| w.box_to as usize != inner);
                    projection.nested_tint = Some(tint);
                    projection.folded_sampler = Some([inner, texture_index]);
                    break;
                }
            }
        }
    }
    derived.wire_count = derived.wires.len();
    let used = !projection.color_nodes.is_empty();
    Ok((derived, used.then_some(projection)))
}

pub(crate) fn subtract_alpha(
    coordinates: &mut [[f32; 2]],
    sample: usize,
    projection: Option<&InstanceProjection>,
    cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    let Some(p) = projection.filter(|p| p.alpha_subtract_samplers.contains(&sample)) else {
        return Ok(());
    };
    if coordinates.len() > 100_000 {
        return Err("Instance UV vertex budget".into());
    }
    let alpha = p.color.values()[3];
    for (i, point) in coordinates.iter_mut().enumerate() {
        if i % 256 == 0 && cancelled() {
            return Err("Cancelled".into());
        }
        for value in point {
            *value -= alpha;
            if !value.is_finite() || value.abs() > 1_000_000. {
                return Err("Instance UV finite preview bounds".into());
            }
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok(())
}

/// Known texture samplers reachable from output channels, excluding displayed slots.
/// Unconnected descriptors do not establish a texture requirement.
pub(crate) fn additional_texture_slots(
    graph: &GraphMetadata,
    displayed: &[usize],
) -> Result<Vec<usize>, String> {
    if graph.boxes.len() > 1024
        || graph.wires.len() > 4096
        || graph.wires.iter().any(|w| {
            w.box_from < 0
                || w.box_to < 0
                || w.box_from as usize >= graph.boxes.len()
                || w.box_to as usize >= graph.boxes.len()
        })
    {
        return Err("Unsupported: texture review graph bounds/endpoints".into());
    }
    let mut selected = vec![false; graph.boxes.len()];
    let mut pending: Vec<_> = graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0)
        .map(|(i, _)| i)
        .collect();
    let mut missing = std::collections::BTreeSet::new();
    while let Some(index) = pending.pop() {
        if selected[index] {
            continue;
        }
        selected[index] = true;
        let node = &graph.boxes[index];
        if node.type_raw == 1 {
            let slot = node.parameters_raw[5] as usize;
            if slot >= 8 {
                return Err("Unsupported: texture review sampler slot outside bounds".into());
            }
            if !displayed.contains(&slot) {
                missing.insert(slot);
            }
        }
        pending.extend(
            graph
                .wires
                .iter()
                .filter(|w| w.box_to as usize == index)
                .map(|w| w.box_from as usize),
        );
    }
    Ok(missing.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};
    fn node(kind: u32) -> BoxMetadata {
        BoxMetadata {
            type_raw: kind,
            subtype_raw: 0,
            declared_parameter_count: 8,
            parameters_raw: match kind {
                1 => [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, 0, 0, 0],
                2 => [8, 0, 0, 0, 0, 0, 0, 0],
                _ => [0; 8],
            },
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
    fn edge(from: i32, to: i32, port: u8) -> WireMetadata {
        WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw: port,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        }
    }
    fn graph(types: &[u32], wires: Vec<WireMetadata>) -> GraphMetadata {
        GraphMetadata {
            box_count: types.len(),
            wire_count: wires.len(),
            boxes: types.iter().copied().map(node).collect(),
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        }
    }
    fn color() -> crate::DiffuseTint {
        crate::DiffuseTint {
            bits: [0.2, 0.4, 0.8, 0.25].map(f32::to_bits),
        }
    }
    #[test]
    fn texture_review_tracks_connected_other_channels_but_not_unused_descriptors() {
        let mut g = graph(&[0, 1, 1, 1], vec![edge(1, 0, 0), edge(2, 0, 4)]);
        g.boxes[2].parameters_raw[5] = 1;
        g.boxes[3].parameters_raw[5] = 2;
        assert_eq!(additional_texture_slots(&g, &[0]).unwrap(), vec![1]);
        assert!(additional_texture_slots(&g, &[0, 1]).unwrap().is_empty());
        g.wires.push(edge(0, 2, 0)); // bounded cycle does not loop
        assert_eq!(additional_texture_slots(&g, &[0]).unwrap(), vec![1]);
        g.boxes[2].parameters_raw[5] = 8;
        assert!(additional_texture_slots(&g, &[0]).is_err());
    }
    #[test]
    fn explicit_color_preserves_source_and_ignores_unselected_branches() {
        let original = graph(&[0, 2, 2], vec![edge(1, 0, 0), edge(2, 0, 4)]);
        let before = serde_json::to_value(&original).unwrap();
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        assert_eq!(p.unwrap().color_nodes, vec![1]);
        assert_eq!(derived.boxes[2].type_raw, 2);
        assert_eq!(
            crate::select_solid_diffuse(&derived)
                .unwrap()
                .unwrap()
                .color,
            color()
        );
        let plain = graph(&[0, 1, 2], vec![edge(1, 0, 0), edge(2, 0, 4)]);
        assert!(project_instance_color(&plain, color()).unwrap().1.is_none());
        let mut bad = original.clone();
        bad.boxes[1].parameters_raw[0] = 9;
        assert!(project_instance_color(&bad, color()).is_err());
        bad = original.clone();
        bad.wires[0].box_from = -1;
        assert!(project_instance_color(&bad, color()).is_err());
        bad = original.clone();
        bad.wires[0].port_to_raw = 4;
        assert!(project_instance_color(&bad, color()).is_err());
        assert!(project_instance_color(
            &original,
            crate::DiffuseTint {
                bits: [f32::NAN.to_bits(); 4]
            }
        )
        .is_err());
    }
    #[test]
    fn scalar_alpha_subtraction_keeps_operation_order_and_guards_inputs() {
        let mut original = graph(
            &[0, 1, 2, 10],
            vec![edge(1, 3, 1), edge(2, 3, 0), edge(3, 0, 0), edge(2, 1, 2)],
        );
        original.wires[3].swizzle_raw = [b'a', 0, 0, 0, 0];
        let before = serde_json::to_value(&original).unwrap();
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert_eq!(crate::select_diffuse(&derived).unwrap().tint, color());
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        let mut uv = vec![[1., 0.5], [-0.5, 0.]];
        subtract_alpha(&mut uv, 1, Some(&p), || false).unwrap();
        assert_eq!(uv, vec![[0.75, 0.25], [-0.75, -0.25]]);
        assert!(subtract_alpha(&mut uv, 1, Some(&p), || true).is_err());
        assert!(subtract_alpha(&mut vec![[0.; 2]; 100_001], 1, Some(&p), || false).is_err());
        assert!(subtract_alpha(&mut [[f32::NAN, 0.]], 1, Some(&p), || false).is_err());
        original.wires[3].swizzle_raw = [b'r', 0, 0, 0, 0];
        assert!(project_instance_color(&original, color()).is_err());
        original.wires[3].swizzle_raw = [b'a', 0, 0, 0, 0];
        original.wires.push(edge(2, 1, 0));
        assert!(project_instance_color(&original, color()).is_err());
    }
    #[test]
    fn fixed_nested_instance_tint_reduces_to_two_samplers_without_cycles() {
        let mut original = graph(
            &[0, 1, 2, 1, 10, 10],
            vec![
                edge(2, 4, 0),
                edge(3, 4, 1),
                edge(1, 5, 0),
                edge(4, 5, 1),
                edge(5, 0, 0),
            ],
        );
        original.boxes[3].parameters_raw[5] = 1;
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(p.unwrap().nested_tint, Some(color()));
        let selected = crate::select_diffuse(&derived).unwrap();
        assert!(selected.second_sample.is_some());
        assert!(selected.third_sample.is_none());
        original.wires[1].box_from = 5;
        let (derived, _) = project_instance_color(&original, color()).unwrap();
        assert!(crate::select_diffuse(&derived).is_err());
    }
}
