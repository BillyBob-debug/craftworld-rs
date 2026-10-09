//! Supplied linear RGBA input; D188 adds an opt-out standalone white default (StandaloneDefaults).
//! Independent projection policy and pinned operation evidence: docs/instance-preview.md.
use lbp_resource::material::GraphMetadata;

#[derive(Debug, Clone, Copy, Default)]
pub struct PreviewOptions {
    pub animation_base: bool,
    pub instance_color: Option<crate::DiffuseTint>,
    /// P2q: also load bump/gloss extras (docs/p2q-bump-gloss.md).
    pub surface: bool,
}

/// D188 standalone-preview defaults (docs/d188-absent-material-primitives.md).
/// Used only where no Thing supplies inputs (single-mesh preview, mesh censuses);
/// `PreviewOptions::default()` stays source-only for every other caller.
/// - White instance colour (1,1,1,1): 96% of placed RENDER_MESH Things carry
///   editorColor `ffffffff` (instance_color_census, `CONFIRMED_FROM_LOCAL_DATA`);
///   Toolkit `PRenderMesh.editorColor` defaults to -1 = opaque white
///   (docs/d80-thing-color.md E7, `CONFIRMED_FROM_COMMUNITY_CODE`). That editorColor
///   feeds the THING_COLOR shader input is `INFERRED` (no capture shows it).
/// - Animation base values (t = base, no game time): the existing D61 snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandaloneDefaults {
    pub instance_white: bool,
    pub animation_base: bool,
}
impl Default for StandaloneDefaults {
    fn default() -> Self {
        Self::ON
    }
}
/// Which standalone defaults actually filled an input the caller left unset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AppliedDefaults {
    pub instance_white: bool,
    pub animation_base: bool,
}
impl AppliedDefaults {
    pub fn any(self) -> bool {
        self.instance_white || self.animation_base
    }
}
impl StandaloneDefaults {
    pub const ON: Self = Self {
        instance_white: true,
        animation_base: true,
    };
    pub const OFF: Self = Self {
        instance_white: false,
        animation_base: false,
    };
    /// Fills unset inputs of `explicit`; explicit values always win.
    pub fn apply(self, explicit: PreviewOptions) -> (PreviewOptions, AppliedDefaults) {
        let mut options = explicit;
        let mut applied = AppliedDefaults::default();
        if options.instance_color.is_none() && self.instance_white {
            options.instance_color = Some(crate::DiffuseTint::default());
            applied.instance_white = true;
        }
        if !options.animation_base && self.animation_base {
            options.animation_base = true;
            applied.animation_base = true;
        }
        (options, applied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceProjection {
    pub color: crate::DiffuseTint,
    pub nested_tint: Option<crate::DiffuseTint>,
    /// Derived sampler node and its original source sampler node.
    pub folded_sampler: Option<[usize; 2]>,
    pub color_nodes: Vec<usize>,
    pub alpha_subtract_samplers: Vec<usize>,
    /// D86: diffuse root MULTIPLY/MIX folded into one derived COLOR (constant inputs only).
    pub folded_root: Option<usize>,
    /// D86: (ThingColor node, swizzle byte) broadcast as one component into the fold.
    pub swizzled_color_edges: Vec<(usize, u8)>,
    /// D88: constant ThingColor MULTIPLY/MIX operand of a texture product root, folded
    /// into a derived COLOR so the existing texture × COLOR routes can tint with it.
    pub folded_operand: Option<usize>,
    /// D96: samplers whose static port 0/1/2 inputs were folded into their scale/offset.
    pub sampler_port_folds: Vec<crate::SamplerPortFold>,
    /// D105: the derived graph is the plain ThingColor-to-COLOR substitution and is
    /// selected by the graph evaluator only (docs/d105-instance-graphs.md).
    pub evaluated: bool,
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
    if diffuse.is_empty() {
        // Nothing to project; the unwired-diffuse route decides support.
        return Ok((graph.clone(), None));
    }
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
        folded_root: None,
        swizzled_color_edges: vec![],
        folded_operand: None,
        sampler_port_folds: vec![],
        evaluated: false,
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
    // D96: without a ThingColor in the diffuse ancestry the opt-in has nothing to
    // project; the original graph and the default selectors decide (e.g. D87 scale).
    if projection.color_nodes.is_empty() {
        return Ok((graph.clone(), None));
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
        let alpha_subtract = inputs.len() == 1
            && node.subtype_raw == 0
            && inputs[0].port_to_raw == 2
            && inputs[0].port_from_raw == 0
            && inputs[0].swizzle_raw == [b'a', 0, 0, 0, 0]
            && projection
                .color_nodes
                .contains(&(inputs[0].box_from as usize));
        if !alpha_subtract {
            // D96 (docs/d96-thing-sampler-ports.md): other static port inputs fold into
            // the derived sampler's scale/offset, or name the unsupported input.
            let fold = crate::instance_sampler::fold_sampler_ports(&mut derived, index)
                .map_err(|e| format!("{e} (supplied instance sampler port)"))?;
            projection.sampler_port_folds.extend(fold);
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
    if !projection.color_nodes.is_empty()
        && projection.folded_sampler.is_none()
        && !crate::instance_fold::fold_constant_root(&mut derived, root, &mut projection)?
    {
        crate::instance_fold::fold_constant_operand(&mut derived, root, &mut projection)?;
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
    if coordinates.len() > crate::MAX_VERTICES {
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
        // No diffuse wire: nothing projected, left to the unwired-diffuse route.
        assert!(project_instance_color(&bad, color()).unwrap().1.is_none());
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
        assert!(subtract_alpha(
            &mut vec![[0.; 2]; crate::MAX_VERTICES + 1],
            1,
            Some(&p),
            || false
        )
        .is_err());
        assert!(subtract_alpha(&mut [[f32::NAN, 0.]], 1, Some(&p), || false).is_err());
        // D96: other static ThingColor port inputs fold into the derived sampler.
        original.wires[3].swizzle_raw = [b'r', 0, 0, 0, 0];
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert!(p.alpha_subtract_samplers.is_empty());
        assert_eq!(p.sampler_port_folds.len(), 1);
        assert_eq!(uv_params(&derived, 1), [1., 1., -0.2, -0.2]);
        original.wires[3].swizzle_raw = [b'a', 0, 0, 0, 0];
        original.wires.push(edge(2, 1, 0));
        let (derived, _) = project_instance_color(&original, color()).unwrap();
        assert_eq!(uv_params(&derived, 1), [1., 1., 0.2 - 0.25, 0.4 - 0.25]);
        // A non-constant input (another sampler) stays unsupported.
        original.boxes.push(node(1));
        original.wires.push(edge(4, 1, 1));
        let err = project_instance_color(&original, color()).unwrap_err();
        assert!(err.contains("box type 1 "), "{err}");
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
    fn solid(g: &GraphMetadata) -> Option<[f32; 4]> {
        crate::select_solid_diffuse(g)
            .ok()
            .flatten()
            .map(|s| s.color.values())
    }
    fn swizzled(mut w: WireMetadata, c: u8) -> WireMetadata {
        w.swizzle_raw = [c, 0, 0, 0, 0];
        w
    }
    #[test]
    fn d86_constant_multiply_folds_only_under_supplied_color() {
        // COLOR × THING: the default route stays unsupported; supplied color folds.
        let mut original = graph(
            &[0, 10, 3, 2],
            vec![edge(2, 1, 0), edge(3, 1, 1), edge(1, 0, 0)],
        );
        original.boxes[2].parameters_raw[..4]
            .copy_from_slice(&[0.5f32, 1., 0.25, 1.].map(f32::to_bits));
        let before = serde_json::to_value(&original).unwrap();
        assert!(crate::select_solid_diffuse(&original).unwrap().is_none());
        assert!(crate::select_diffuse(&original).is_err());
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        let p = p.unwrap();
        assert_eq!(p.folded_root, Some(1));
        assert!(p.swizzled_color_edges.is_empty());
        assert_eq!(solid(&derived), Some([0.5 * 0.2, 0.4, 0.25 * 0.8, 0.25]));
        // Wire order does not change the projection.
        let mut permuted = original.clone();
        permuted.wires.reverse();
        let (again, q) = project_instance_color(&permuted, color()).unwrap();
        assert_eq!(q.unwrap(), p);
        assert_eq!(solid(&again), solid(&derived));
        // CONSTANT (box 4) broadcasts its scalar word 0.
        let mut constant = original.clone();
        constant.boxes[2].type_raw = 4;
        constant.boxes[2].parameters_raw = [0.5f32.to_bits(), 0, 0, 0, 0, 0, 0, 0];
        let (derived, _) = project_instance_color(&constant, color()).unwrap();
        assert_eq!(solid(&derived), Some([0.1, 0.2, 0.4, 0.125]));
        // Out of preview bounds is an explicit reason, not a silent clamp.
        let mut bright = original.clone();
        bright.boxes[2].parameters_raw[0] = 8f32.to_bits();
        let err = project_instance_color(&bright, color()).unwrap_err();
        assert!(err.contains("folded supplied instance color"), "{err}");
        // Not folded: animated root, wired operand, no ThingColor operand, extra input.
        let mut animated = original.clone();
        animated.boxes[1].animations[0].name_hex = "0001ab".into();
        assert!(project_instance_color(&animated, color())
            .unwrap()
            .1
            .unwrap()
            .folded_root
            .is_none());
        let mut wired = original.clone();
        wired.wires.push(edge(3, 2, 0));
        assert!(project_instance_color(&wired, color())
            .map(|(d, _)| solid(&d).is_none())
            .unwrap_or(true));
        let colors = graph(
            &[0, 10, 3, 3, 2],
            vec![edge(2, 1, 0), edge(3, 1, 1), edge(1, 0, 0), edge(4, 0, 4)],
        );
        let (derived, _) = project_instance_color(&colors, color()).unwrap();
        assert!(solid(&derived).is_none());
        let mut third = original.clone();
        third.wires.push(edge(2, 1, 2));
        let (derived, p) = project_instance_color(&third, color()).unwrap();
        assert!(p.unwrap().folded_root.is_none() && solid(&derived).is_none());
    }
    #[test]
    fn d86_single_component_thing_swizzle_broadcasts_into_the_fold() {
        let original = graph(
            &[0, 10, 2],
            vec![edge(2, 1, 0), swizzled(edge(2, 1, 1), b'a'), edge(1, 0, 0)],
        );
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert_eq!(p.swizzled_color_edges, vec![(2, b'a')]);
        assert_eq!(
            solid(&derived),
            Some([0.2, 0.4, 0.8, 0.25].map(|v: f32| v * 0.25))
        );
        let x = graph(
            &[0, 10, 2],
            vec![edge(2, 1, 0), swizzled(edge(2, 1, 1), b'x'), edge(1, 0, 0)],
        );
        let r = graph(
            &[0, 10, 2],
            vec![edge(2, 1, 0), swizzled(edge(2, 1, 1), b'r'), edge(1, 0, 0)],
        );
        assert_eq!(
            solid(&project_instance_color(&x, color()).unwrap().0),
            solid(&project_instance_color(&r, color()).unwrap().0)
        );
        for bad in [
            [b'x', b'y', 0, 0, 0],
            [b'a', b'x', 0, 0, 0],
            [b'q', 0, 0, 0, 0],
            [b'a', 0, 0, 0, b'x'],
        ] {
            let mut g = original.clone();
            g.wires[1].swizzle_raw = bad;
            let (derived, p) = project_instance_color(&g, color()).unwrap();
            assert!(p.unwrap().folded_root.is_none());
            assert!(solid(&derived).is_none());
        }
        // Two scalars have no observed color widening rule.
        let both = graph(
            &[0, 10, 2],
            vec![
                swizzled(edge(2, 1, 0), b'a'),
                swizzled(edge(2, 1, 1), b'a'),
                edge(1, 0, 0),
            ],
        );
        assert!(solid(&project_instance_color(&both, color()).unwrap().0).is_none());
        // THING × THING without swizzle squares the supplied color.
        let square = graph(
            &[0, 10, 2],
            vec![edge(2, 1, 0), edge(2, 1, 1), edge(1, 0, 0)],
        );
        assert_eq!(
            solid(&project_instance_color(&square, color()).unwrap().0),
            Some([0.2f32, 0.4, 0.8, 0.25].map(|v| v * v))
        );
    }
    #[test]
    fn d190_rgb_identity_output_swizzle_reaches_the_solid_route() {
        // THING × THING.a with the diffuse wire swizzled (the g21845 shape).
        let shape = |swizzle: [u8; 5]| {
            let mut out = edge(1, 0, 0);
            out.swizzle_raw = swizzle;
            graph(
                &[0, 10, 2],
                vec![edge(2, 1, 0), swizzled(edge(2, 1, 1), b'a'), out],
            )
        };
        let white = crate::DiffuseTint::default();
        for ok in [*b"rgb\0\0", *b"xyz\0\0", *b"rgba\0", *b"xyzw\0"] {
            let (derived, p) = crate::project_instance_graph(&shape(ok), white, true).unwrap();
            assert!(p.unwrap().folded_root.is_some());
            let s = crate::select_solid_diffuse(&derived).unwrap().unwrap();
            assert_eq!(s.color, white);
            assert_eq!(s.output_swizzle, ok);
            assert_eq!(solid(&derived), Some([1.; 4]));
        }
        // Any swizzle that moves or drops an RGB component stays refused.
        for bad in [
            *b"a\0\0\0\0",
            *b"bgr\0\0",
            *b"rg\0\0\0",
            *b"xyzx\0",
            *b"rgbaa",
        ] {
            let (derived, _) = crate::project_instance_graph(&shape(bad), white, true).unwrap();
            assert!(crate::select_solid_diffuse(&derived)
                .unwrap_err()
                .contains("subtype-zero COLOR"));
        }
        // Unswizzled output keeps zero swizzle metadata.
        let (derived, _) = project_instance_color(&shape([0; 5]), white).unwrap();
        let s = crate::select_solid_diffuse(&derived).unwrap().unwrap();
        assert_eq!(s.output_swizzle, [0; 5]);
    }
    #[test]
    fn d86_static_constant_mix_folds_with_parameter_weight_only() {
        // Toolkit MIX form ((r - l) * w) + l, l on port 0, r on port 1, w = word 0.
        let mut original = graph(
            &[0, 12, 2, 3],
            vec![edge(2, 1, 1), edge(3, 1, 0), edge(1, 0, 0)],
        );
        original.boxes[1].parameters_raw[0] = 0.25f32.to_bits();
        original.boxes[3].parameters_raw[..4]
            .copy_from_slice(&[1f32, 0., 0., 1.].map(f32::to_bits));
        assert!(crate::select_diffuse(&original).is_err());
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(p.unwrap().folded_root, Some(1));
        let l = [1f32, 0., 0., 1.];
        let r = [0.2f32, 0.4, 0.8, 0.25];
        assert_eq!(
            solid(&derived),
            Some(std::array::from_fn(|i| (r[i] - l[i]) * 0.25 + l[i]))
        );
        let mut wired_weight = original.clone();
        wired_weight.boxes.push(node(4));
        wired_weight.wires.push(edge(4, 1, 2));
        let (derived, p) = project_instance_color(&wired_weight, color()).unwrap();
        assert!(p.unwrap().folded_root.is_none() && solid(&derived).is_none());
        let mut nan = original.clone();
        nan.boxes[1].parameters_raw[0] = f32::NAN.to_bits();
        assert!(project_instance_color(&nan, color()).is_err());
    }
    fn animate(g: &mut GraphMetadata, index: usize) {
        let a = &mut g.boxes[index].animations[0];
        a.name_hex = "616200".into();
        a.components_raw = 15;
        a.component_value_count = 4;
        a.derived_key_count = 1;
        a.base_value_bits = [2f32, 2., 0., 0.].map(f32::to_bits);
    }
    #[test]
    fn d88_animated_texture_times_thing_needs_both_opt_ins() {
        // THING × animated sampler: D78 leaves single-texture tints out.
        let mut original = graph(
            &[0, 10, 2, 1],
            vec![edge(2, 1, 0), edge(3, 1, 1), edge(1, 0, 0)],
        );
        animate(&mut original, 3);
        assert!(crate::select_diffuse(&original).is_err());
        assert!(crate::select_diffuse_with_animation_base(&original, true).is_err());
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert!(p.folded_root.is_none() && p.folded_operand.is_none());
        // D102: the plain animation-base selector also takes one animated sample × COLOR.
        assert!(crate::select_diffuse_with_animation_base(&derived, true).is_ok());
        assert!(crate::select_instance_diffuse(&derived, &p, false).is_err());
        let s = crate::select_instance_diffuse(&derived, &p, true).unwrap();
        assert_eq!(s.tint, color());
        assert_eq!((s.sample_index, s.color_index), (3, Some(2)));
        assert_eq!(s.multiply_index, Some(1));
        assert_eq!(s.animation_base.unwrap().sample_index, 3);
        assert!(s.second_sample.is_none());
        // A plain static COLOR is not projected, so no instance route applies.
        let mut plain = original.clone();
        plain.boxes[2] = node(3);
        assert!(project_instance_color(&plain, color()).unwrap().1.is_none());
        let s = crate::select_diffuse_with_animation_base(&plain, true).unwrap();
        assert_eq!((s.sample_index, s.color_index), (3, Some(2)));
        assert!(crate::select_diffuse(&plain).is_err());
        // Rejected: animated MULTIPLY; static sampler stays on the existing route.
        let mut animated_root = original.clone();
        animate(&mut animated_root, 1);
        let (d, q) = project_instance_color(&animated_root, color()).unwrap();
        assert!(crate::select_instance_diffuse(&d, &q.unwrap(), true).is_err());
        let still = graph(
            &[0, 10, 2, 1],
            vec![edge(2, 1, 0), edge(3, 1, 1), edge(1, 0, 0)],
        );
        let (d, q) = project_instance_color(&still, color()).unwrap();
        let s = crate::select_instance_diffuse(&d, &q.unwrap(), true).unwrap();
        assert!(s.animation_base.is_none() && s.tint == color());
        // D92: two animated leaves under THING are the D75 tinted product, each leaf
        // on its own base rows, tinted by the supplied color (both opt-ins).
        let mut two = graph(
            &[0, 10, 2, 10, 1, 1],
            vec![
                edge(2, 1, 0),
                edge(3, 1, 1),
                edge(4, 3, 0),
                edge(5, 3, 1),
                edge(1, 0, 0),
            ],
        );
        animate(&mut two, 4);
        animate(&mut two, 5);
        two.boxes[5].animations[0].name_hex = "636400".into();
        let (d, q) = project_instance_color(&two, color()).unwrap();
        let q = q.unwrap();
        assert!(crate::select_instance_diffuse(&d, &q, false).is_err());
        let s = crate::select_instance_diffuse(&d, &q, true).unwrap();
        assert_eq!((s.tint, s.color_index), (color(), Some(2)));
        let leaves = [s.animation_base, s.animation_base_more[0]].map(|b| b.unwrap().sample_index);
        assert_eq!(
            leaves
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            2
        );
        assert!(leaves.iter().all(|l| [4, 5].contains(l)));
        assert!(s.animation_base_more[1].is_none());
        // A nested (T × THING) × T folds the sampler (nested tint): one animated leaf
        // keeps the existing D78 route, two animated leaves stay unsupported.
        let mut nested = graph(
            &[0, 10, 10, 2, 1, 1],
            vec![
                edge(4, 2, 0),
                edge(3, 2, 1),
                edge(2, 1, 0),
                edge(5, 1, 1),
                edge(1, 0, 0),
            ],
        );
        animate(&mut nested, 5);
        let (d, q) = project_instance_color(&nested, color()).unwrap();
        let q = q.unwrap();
        assert!(q.folded_sampler.is_some());
        assert!(crate::select_instance_diffuse(&d, &q, true).is_ok());
        animate(&mut nested, 4);
        nested.boxes[4].animations[0].name_hex = "636400".into();
        let (d, q) = project_instance_color(&nested, color()).unwrap();
        assert!(crate::select_instance_diffuse(&d, &q.unwrap(), true).is_err());
    }
    #[test]
    fn d88_constant_thing_operand_folds_into_texture_tint() {
        // texture × (THING × COLOR): the operand folds; texture × COLOR selects.
        let mut original = graph(
            &[0, 10, 10, 3, 2, 1],
            vec![
                edge(2, 1, 1),
                edge(4, 2, 0),
                edge(3, 2, 1),
                edge(5, 1, 0),
                edge(1, 0, 0),
            ],
        );
        original.boxes[3].parameters_raw[..4]
            .copy_from_slice(&[0.5f32, 1., 0.25, 1.].map(f32::to_bits));
        let before = serde_json::to_value(&original).unwrap();
        assert!(crate::select_diffuse(&original).is_err());
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        let p = p.unwrap();
        assert_eq!((p.folded_root, p.folded_operand), (None, Some(2)));
        let s = crate::select_diffuse(&derived).unwrap();
        assert_eq!(s.tint.values(), [0.2 * 0.5, 0.4, 0.8 * 0.25, 0.25]);
        assert_eq!((s.sample_index, s.color_index), (5, Some(2)));
        assert_eq!(
            crate::select_instance_diffuse(&derived, &p, false).unwrap(),
            s
        );
        // With an animated sampler the folded operand also tints (both opt-ins).
        let mut moving = original.clone();
        animate(&mut moving, 5);
        let (d, q) = project_instance_color(&moving, color()).unwrap();
        let q = q.unwrap();
        assert!(crate::select_diffuse_with_animation_base(&d, true).is_ok());
        let s = crate::select_instance_diffuse(&d, &q, true).unwrap();
        assert_eq!(s.color_index, Some(2));
        assert!(s.animation_base.is_some());
        // Not folded: operand also feeds another channel; swizzled root input.
        let mut shared = original.clone();
        shared.wires.push(edge(2, 0, 4));
        let (d, q) = project_instance_color(&shared, color()).unwrap();
        let q = q.unwrap();
        assert!(q.folded_operand.is_none());
        // D99: the derived graph is a product tree, refused under the supplied color.
        assert!(crate::select_instance_diffuse(&d, &q, true)
            .unwrap_err()
            .contains("product tree under a supplied instance color"));
        let mut swizzle = original.clone();
        swizzle.wires[0].swizzle_raw = [b'a', 0, 0, 0, 0];
        let (_, q) = project_instance_color(&swizzle, color()).unwrap();
        assert!(q.unwrap().folded_operand.is_none());
        // Out of preview bounds is an explicit reason.
        let mut bright = original.clone();
        bright.boxes[3].parameters_raw[0] = 8f32.to_bits();
        let err = project_instance_color(&bright, color()).unwrap_err();
        assert!(err.contains("folded supplied instance color"), "{err}");
    }
    fn uv_params(g: &GraphMetadata, i: usize) -> [f32; 4] {
        std::array::from_fn(|k| f32::from_bits(g.boxes[i].parameters_raw[k]))
    }
    fn constant(g: &mut GraphMetadata, kind: u32, values: &[f32]) -> i32 {
        let mut n = node(kind);
        for (k, v) in values.iter().enumerate() {
            n.parameters_raw[k] = v.to_bits();
        }
        g.boxes.push(n);
        g.box_count = g.boxes.len();
        g.boxes.len() as i32 - 1
    }
    #[test]
    fn d96_thing_times_const_scaled_sampler_folds_scale_under_supplied_color_only() {
        // OUTPUT, MULTIPLY(THING port 0, TEX port 1), CONST on TEX port 1.
        let mut original = graph(
            &[0, 10, 1, 2],
            vec![edge(1, 0, 0), edge(3, 1, 0), edge(2, 1, 1)],
        );
        original.boxes[2].parameters_raw[..4].copy_from_slice(&[1., 2., 0.5, 0.].map(f32::to_bits));
        let c = constant(&mut original, 4, &[4.]);
        original.wires.push(edge(c, 2, 1));
        let before = serde_json::to_value(&original).unwrap();
        assert!(crate::select_diffuse(&original).is_err());
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        assert_eq!(uv_params(&derived, 2), [4., 8., 2., 0.]);
        let fold = &p.sampler_port_folds[0];
        assert_eq!(fold.sample_index, 2);
        assert_eq!(fold.port_bits[1], Some([4f32.to_bits(); 2]));
        assert_eq!(fold.port_bits[0], None);
        let s = crate::select_instance_diffuse(&derived, &p, false).unwrap();
        assert_eq!((s.sample_index, s.tint), (2, color()));
        assert_eq!(
            s.uv_transform.bits(),
            vec![4., 8., 2., 0.]
                .into_iter()
                .map(f32::to_bits)
                .collect::<Vec<_>>()
        );
        // Zero scale, a FRESNEL input and an animated sampler stay unsupported.
        let mut zero = original.clone();
        zero.boxes[c as usize].parameters_raw[0] = 0;
        assert!(project_instance_color(&zero, color())
            .unwrap_err()
            .contains("zero"));
        let mut fresnel = original.clone();
        fresnel.boxes[c as usize].type_raw = 18;
        let err = project_instance_color(&fresnel, color()).unwrap_err();
        assert!(err.contains("box type 18"), "{err}");
        let mut moving = original.clone();
        animate(&mut moving, 2);
        let err = project_instance_color(&moving, color()).unwrap_err();
        assert!(err.contains("static subtype-0"), "{err}");
        // Without a ThingColor the opt-in leaves the graph to the default (D87) route.
        let mut plain = original.clone();
        plain.wires.retain(|w| w.box_from != 3);
        plain.boxes[1].type_raw = 3;
        let (same, none) = project_instance_color(&plain, color()).unwrap();
        assert!(none.is_none());
        assert_eq!(
            serde_json::to_value(&same).unwrap(),
            serde_json::to_value(&plain).unwrap()
        );
    }
    #[test]
    fn d96_thing_offsets_fold_into_both_product_samplers() {
        // MULTIPLY(TEX a, TEX b); ThingColor unswizzled on port 2 of both samplers.
        let original = graph(
            &[0, 10, 1, 2, 1],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 1),
                edge(4, 1, 0),
                edge(3, 2, 2),
                edge(3, 4, 2),
            ],
        );
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert_eq!(p.sampler_port_folds.len(), 2);
        assert_eq!(uv_params(&derived, 2), [1., 1., -0.2, -0.4]);
        assert_eq!(uv_params(&derived, 4), [1., 1., -0.2, -0.4]);
        let s = crate::select_instance_diffuse(&derived, &p, false).unwrap();
        assert!(s.second_sample.is_some());
    }
    #[test]
    fn d96_constant_expressions_of_thing_alpha_fold_into_the_offset() {
        // TEX(port 2 = CONST × THING.a) × THING.
        let mut original = graph(
            &[0, 10, 1, 10, 2],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(4, 1, 1), edge(3, 2, 2)],
        );
        let k = constant(&mut original, 4, &[2.]);
        original.wires.push(edge(k, 3, 0));
        original.wires.push(swizzled(edge(4, 3, 1), b'a'));
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        assert_eq!(uv_params(&derived, 2), [1., 1., -0.5, -0.5]);
        let s = crate::select_instance_diffuse(&derived, &p.unwrap(), false).unwrap();
        assert_eq!(s.tint, color());
        // MAKE_FLOAT2(CONST × THING.a, CONST) on port 2.
        let mut two = original.clone();
        two.wires.retain(|w| !(w.box_from == 3 && w.box_to == 2));
        let m = constant(&mut two, 13, &[0., 0.]);
        let c = constant(&mut two, 4, &[0.75]);
        two.wires
            .extend([edge(3, m, 0), edge(c, m, 1), edge(m, 2, 2)]);
        let (derived, _) = project_instance_color(&two, color()).unwrap();
        assert_eq!(uv_params(&derived, 2), [1., 1., -0.5, -0.75]);
        // A vector input to MAKE_FLOAT2 is reduced by Toolkit; not folded.
        let mut vector = two.clone();
        vector.wires.push(edge(4, m, 1));
        vector.wires.retain(|w| !(w.box_from == c && w.box_to == m));
        let err = project_instance_color(&vector, color()).unwrap_err();
        assert!(err.contains("scalar inputs"), "{err}");
        // MIX(CONST4, CONST4, THING.a) on port 2: (r - l) × a + l, first two lanes.
        let mut mix = original.clone();
        mix.wires.retain(|w| !(w.box_from == 3 && w.box_to == 2));
        let x = constant(&mut mix, 12, &[0.]);
        let l = constant(&mut mix, 7, &[0., 1., 0., 0.]);
        let r = constant(&mut mix, 7, &[1., 0., 0., 0.]);
        mix.wires.extend([
            edge(l, x, 0),
            edge(r, x, 1),
            swizzled(edge(4, x, 2), b'a'),
            edge(x, 2, 2),
        ]);
        let (derived, _) = project_instance_color(&mix, color()).unwrap();
        assert_eq!(uv_params(&derived, 2), [1., 1., -0.25, -0.75]);
    }
    #[test]
    fn d193_port_folded_sampler_composes_with_the_nested_tint() {
        // TEX a(port 2 = MIX(CONST4, CONST4, THING.a)) × (THING × animated TEX b),
        // the 25ff1aa3 shape (D96 table). Fold a's offset, fold b under the tint.
        let mut original = graph(
            &[0, 10, 1, 10, 2, 1],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(3, 1, 1),
                edge(4, 3, 0),
                edge(5, 3, 1),
            ],
        );
        animate(&mut original, 5);
        let x = constant(&mut original, 12, &[0.]);
        let l = constant(&mut original, 7, &[0., 1., 0., 0.]);
        let r = constant(&mut original, 7, &[1., 0., 0., 0.]);
        original.wires.extend([
            edge(l, x, 0),
            edge(r, x, 1),
            swizzled(edge(4, x, 2), b'a'),
            edge(x, 2, 2),
        ]);
        let before = serde_json::to_value(&original).unwrap();
        let (derived, p) = project_instance_color(&original, color()).unwrap();
        let p = p.unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), before);
        assert_eq!(p.folded_sampler, Some([3, 5]));
        assert_eq!(p.nested_tint, Some(color()));
        assert_eq!(p.sampler_port_folds.len(), 1);
        assert_eq!(p.sampler_port_folds[0].sample_index, 2);
        assert_eq!(uv_params(&derived, 2), [1., 1., -0.25, -0.75]);
        assert!(crate::select_instance_diffuse(&derived, &p, false).is_err());
        let s = crate::select_instance_diffuse(&derived, &p, true).unwrap();
        assert!(s.second_sample.is_some());
        let samples = [s.sample_index, s.second_sample.unwrap().sample_index];
        assert!(samples.contains(&2) && samples.contains(&3), "{samples:?}");
        // A port-folded sampler must stay static (D96 rule 1).
        let mut both = original.clone();
        animate(&mut both, 2);
        assert!(project_instance_color(&both, color()).is_err());
    }
}
