//! Opt-in local animation base-value snapshot, not a game-time sample.
//! Sources/bounds/unknown timing: docs/animation-base-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationBaseSnapshot {
    /// 1: two matrix-row curves; 0: one scale/offset curve, second row empty.
    pub subtype_raw: u32,
    /// Graph index of the animated sampler; a product leaf when D78 selects it.
    pub sample_index: usize,
    pub global_rows_verified: bool,
    pub static_matrix_bits: [u32; 6],
    pub row_bits: [[u32; 4]; 2],
    pub key_counts: [usize; 2],
}

/// Distinct animated product leaves in first-use order (D78 one, D92 up to four).
/// Each leaf keeps its own base-row snapshot; a sampler seen twice (squared) is one.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AnimatedLeaves([Option<AnimationBaseSnapshot>; 4]);
impl AnimatedLeaves {
    pub(crate) fn insert(&mut self, snapshot: AnimationBaseSnapshot) -> Result<(), String> {
        if self
            .0
            .iter()
            .flatten()
            .any(|b| b.sample_index == snapshot.sample_index)
        {
            return Ok(());
        }
        let free =
            self.0.iter_mut().find(|b| b.is_none()).ok_or(
                "Unsupported: animation base allows at most four animated product samplers",
            )?;
        *free = Some(snapshot);
        Ok(())
    }
    /// The first animated leaf (`DiffuseSelection::animation_base`).
    pub(crate) fn primary(&self) -> Option<AnimationBaseSnapshot> {
        self.0[0]
    }
    /// D92: the second and later distinct animated leaves.
    pub(crate) fn more(&self) -> [Option<AnimationBaseSnapshot>; 3] {
        [self.0[1], self.0[2], self.0[3]]
    }
}

/// Exactly one animated texture leaf × one COLOR under the root MULTIPLY: no further
/// leaf, nested product or MIX/BLEND/ADD/tree/sampler-scale state. D88 accepts this
/// shape with an instance tint; D102 also with a plain static COLOR.
pub(crate) fn single_tinted_leaf(s: &crate::DiffuseSelection) -> bool {
    s.animation_base.is_some()
        && s.animation_base_more == [None; 3]
        && s.multiply_index.is_some()
        && s.color_index.is_some()
        && s.second_sample.is_none()
        && s.third_sample.is_none()
        && s.fourth_sample.is_none()
        && s.inner_multiply_index.is_none()
        && s.deep_multiply_index.is_none()
        && s.blend.is_none()
        && s.blend_mask.is_none()
        && s.texture_blend.is_none()
        && s.alpha_mix.is_none()
        && s.texture_alpha_mix.is_none()
        && s.texture_add.is_none()
        && s.product_tree.is_none()
        && s.sampler_scale.is_none()
}

/// Scope of the animation-base product search: D78/D92 products of two or more
/// texture samples, and D102 one animated texture × static COLOR.
pub(crate) fn product_scope(s: crate::DiffuseSelection) -> Result<crate::DiffuseSelection, String> {
    if s.animation_base.is_some() && (s.second_sample.is_some() || single_tinted_leaf(&s)) {
        Ok(s)
    } else {
        Err("Unsupported: animation base product needs 2+ textures and an animated sampler".into())
    }
}

pub(crate) fn select(
    graph: &lbp_resource::material::GraphMetadata,
) -> Result<crate::DiffuseSelection, String> {
    if graph.boxes.len() > 1024
        || graph.wires.len() > 4096
        || graph.wires.iter().any(|w| {
            w.box_from < 0
                || w.box_to < 0
                || w.box_from as usize >= graph.boxes.len()
                || w.box_to as usize >= graph.boxes.len()
        })
    {
        return Err("Unsupported: animation base graph budget/endpoints".into());
    }
    let outputs = graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0)
        .collect::<Vec<_>>();
    if outputs.len() != 1 {
        return Err("Unsupported: animation base needs one output".into());
    }
    let (output_index, output) = outputs[0];
    let static_node = |b: &lbp_resource::material::BoxMetadata| {
        b.subtype_raw == 0
            && b.animations
                .iter()
                .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|v| v == b'0'))
    };
    let diffuse = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == output_index && w.port_to_raw == 0)
        .collect::<Vec<_>>();
    if !static_node(output) || diffuse.len() != 1 {
        return Err("Unsupported: animation base needs static direct diffuse output".into());
    }
    let wire = diffuse[0];
    if wire.port_from_raw != 0 || wire.swizzle_raw != [0; 5] {
        return Err(
            "Unsupported: animation base only supports direct unwired UV0/UV1 sampler".into(),
        );
    }
    let (selected, snapshot) = sampler(graph, wire.box_from as usize)?;
    Ok(crate::DiffuseSelection {
        animation_base: Some(snapshot),
        output_index,
        sample_index: selected.sample_index,
        slot: selected.slot,
        uv_channel: selected.uv_channel,
        uv_transform: selected.uv_transform,
        ignored_nodes: graph.boxes.len() - 2,
        ignored_wires: graph.wires.len() - 1,
        ..Default::default()
    })
}

/// One unwired UV0/UV1 sampler with named base rows. Endpoints must already be
/// validated. Also used for one animated leaf of a static product (D78).
pub(crate) fn sampler(
    graph: &lbp_resource::material::GraphMetadata,
    sample_index: usize,
) -> Result<(crate::linked::SamplerSelection, AnimationBaseSnapshot), String> {
    let sample = &graph.boxes[sample_index];
    if sample.type_raw != 1
        || sample.subtype_raw > 1
        || crate::sampler_uv_channel(sample.parameters_raw[4]).is_none()
        || graph
            .wires
            .iter()
            .any(|w| w.box_to as usize == sample_index)
    {
        return Err(
            "Unsupported: animation base only supports direct unwired UV0/UV1 sampler".into(),
        );
    }
    // Subtype 1: two XYZ matrix-row curves. Subtype 0: one curve whose base
    // vector is (scale X/Y, subtracted offset X/Y); the second binding is empty.
    let named = if sample.subtype_raw == 1 { 2 } else { 1 };
    for a in &sample.animations[..named] {
        let name = a.name_hex.as_bytes();
        let mask = a.components_raw;
        let lanes = mask.count_ones() as usize;
        // D102 (docs/d102-sampler-rest.md): a matrix row reads the whole base `.xyz`
        // whichever of its lanes are keyed, so any nonempty XYZ subset is accepted.
        let valid_mask = if sample.subtype_raw == 1 {
            (1..=7).contains(&mask)
        } else {
            (1..=15).contains(&mask)
        };
        if name.len() != 6
            || &name[4..] != b"00"
            || !name[..4].iter().all(u8::is_ascii_hexdigit)
            || !valid_mask
            || a.component_value_count == 0
            || a.component_value_count > 4096
            || !a.component_value_count.is_multiple_of(lanes)
            || a.derived_key_count != a.component_value_count / lanes
        {
            return Err("Unsupported: animation base needs named masked curves".into());
        }
        for offset in [0, 2] {
            let value = u8::from_str_radix(&a.name_hex[offset..offset + 2], 16)
                .map_err(|_| "Invalid: animation name")?;
            if !value.is_ascii_alphanumeric() {
                return Err("Unsupported: animation base name".into());
            }
        }
    }
    let rows = sample.animations.each_ref().map(|a| a.base_value_bits);
    let uv_transform = if sample.subtype_raw == 1 {
        if sample.animations[0].name_hex == sample.animations[1].name_hex {
            return Err("Unsupported: animation base row names must differ".into());
        }
        crate::SamplingTransform::Matrix(crate::UvMatrix {
            bits: [
                rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2],
            ],
        })
    } else {
        let empty = &sample.animations[1];
        if empty.name_hex.bytes().any(|b| b != b'0')
            || empty.component_value_count != 0
            || empty.components_raw != 0
            || empty.derived_key_count != 0
            || empty.base_value_bits != [0; 4]
        {
            return Err("Unsupported: scale/offset animation base needs one curve".into());
        }
        // (uv - zw) * xy, expressed as uv * xy + (-(z * x), -(w * y)). A single
        // rounded product per axis; no fused-operation or GPU rounding claim.
        let [sx, sy, zx, zy] = rows[0].map(f32::from_bits);
        crate::SamplingTransform::ScaleOffset(crate::UvTransform {
            bits: [sx, sy, -(zx * sx), -(zy * sy)].map(f32::to_bits),
        })
    };
    uv_transform.validate()?;
    Ok((
        crate::linked::SamplerSelection {
            sample_index,
            slot: sample.parameters_raw[5] as usize,
            uv_channel: crate::sampler_uv_channel(sample.parameters_raw[4]).unwrap_or_default(),
            uv_transform,
        },
        AnimationBaseSnapshot {
            subtype_raw: sample.subtype_raw,
            sample_index,
            global_rows_verified: false,
            static_matrix_bits: [
                sample.parameters_raw[0],
                sample.parameters_raw[1],
                sample.parameters_raw[2],
                sample.parameters_raw[3],
                sample.parameters_raw[6],
                sample.parameters_raw[7],
            ],
            row_bits: rows,
            key_counts: sample.animations.each_ref().map(|a| a.derived_key_count),
        },
    ))
}

/// Reconcile both local bindings of every animated leaf (D92: each leaf on its own)
/// against unique global curve declarations. Matching metadata establishes input
/// identity, never a game time or playback rule.
pub(crate) fn verify_global(
    graph: &lbp_resource::material::GraphMetadata,
    selection: &mut crate::DiffuseSelection,
    globals: &[lbp_resource::material::AnimationMetadata],
) -> Result<(), String> {
    let mut leaves = [selection.animation_base]
        .into_iter()
        .chain(selection.animation_base_more)
        .collect::<Vec<_>>();
    for snapshot in leaves.iter_mut().flatten() {
        verify_leaf(graph, snapshot, globals)?;
    }
    selection.animation_base = leaves[0];
    selection.animation_base_more = [leaves[1], leaves[2], leaves[3]];
    Ok(())
}

fn verify_leaf(
    graph: &lbp_resource::material::GraphMetadata,
    snapshot: &mut AnimationBaseSnapshot,
    globals: &[lbp_resource::material::AnimationMetadata],
) -> Result<(), String> {
    let named = if snapshot.subtype_raw == 1 { 2 } else { 1 };
    for local in &graph.boxes[snapshot.sample_index].animations[..named] {
        let matches = globals
            .iter()
            .filter(|a| a.name_hex == local.name_hex)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("Unsupported: animation row needs one matching global declaration".into());
        }
        let global = matches[0];
        if global.base_value_bits != local.base_value_bits
            || global.components_raw != local.components_raw
            || global.component_value_count != local.component_value_count
            || global.derived_key_count != local.derived_key_count
            || global.component_bytes_sha256 != local.component_bytes_sha256
        {
            return Err(
                "Unsupported: local/global material animation declarations conflict".into(),
            );
        }
    }
    snapshot.global_rows_verified = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_global;
    use crate::select_diffuse_with_animation_base as select;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, GraphMetadata, WireMetadata};

    fn node(kind: u32, slot: u32) -> BoxMetadata {
        let mut parameters_raw = [0; 8];
        if kind == 1 {
            parameters_raw = [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, slot, 0, 0];
        } else if kind == 3 {
            parameters_raw[..4].copy_from_slice(&[0.5f32, 1., 0.25, 1.].map(f32::to_bits));
        }
        BoxMetadata {
            type_raw: kind,
            declared_parameter_count: 8,
            parameters_raw,
            position_bits: [0; 4],
            subtype_raw: 0,
            animations: std::array::from_fn(|_| AnimationMetadata {
                base_value_bits: [0; 4],
                component_value_count: 0,
                component_bytes_sha256: String::new(),
                name_hex: "000000".into(),
                components_raw: 0,
                derived_key_count: 0,
            }),
            texture_slot_raw: None,
            texture_slot_in_range: None,
        }
    }
    fn wire(from: i32, to: i32, port: u8) -> WireMetadata {
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
    fn graph(kinds: &[u32], wires: Vec<WireMetadata>) -> GraphMetadata {
        GraphMetadata {
            box_count: kinds.len(),
            wire_count: wires.len(),
            boxes: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| node(*k, i as u32))
                .collect(),
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        }
    }
    /// Subtype 0: one scale/offset curve (D62), base (x, y, z, w).
    fn scale_offset(g: &mut GraphMetadata, index: usize, name: &str, base: [f32; 4]) {
        let a = &mut g.boxes[index].animations[0];
        a.name_hex = name.into();
        (
            a.components_raw,
            a.component_value_count,
            a.derived_key_count,
        ) = (15, 8, 2);
        a.base_value_bits = base.map(f32::to_bits);
    }
    /// Subtype 1: two named XYZ matrix-row curves (D56).
    fn matrix(g: &mut GraphMetadata, index: usize, names: [&str; 2]) {
        g.boxes[index].subtype_raw = 1;
        for (row, name) in names.into_iter().enumerate() {
            let a = &mut g.boxes[index].animations[row];
            a.name_hex = name.into();
            (
                a.components_raw,
                a.component_value_count,
                a.derived_key_count,
            ) = (7, 3, 1);
            a.base_value_bits = [1. + row as f32, 0.5, 0.25 * row as f32, 0.].map(f32::to_bits);
        }
    }
    fn globals(g: &GraphMetadata) -> Vec<AnimationMetadata> {
        let mut all = g
            .boxes
            .iter()
            .flat_map(|b| b.animations.iter())
            .filter(|a| a.name_hex.bytes().any(|c| c != b'0'))
            .cloned()
            .collect::<Vec<_>>();
        all.sort_by(|a, b| a.name_hex.cmp(&b.name_hex));
        all.dedup_by(|a, b| a.name_hex == b.name_hex);
        all
    }
    fn animated_indices(s: &crate::DiffuseSelection) -> Vec<usize> {
        let mut v = [s.animation_base]
            .into_iter()
            .chain(s.animation_base_more)
            .flatten()
            .map(|b| b.sample_index)
            .collect::<Vec<_>>();
        v.sort_unstable();
        v
    }

    #[test]
    fn d92_each_animated_product_leaf_uses_its_own_base_rows() {
        // T1 × T2 (boxes 1/2, root multiply 3), both animated with distinct curves.
        let product = || {
            graph(
                &[0, 1, 1, 10],
                vec![wire(1, 3, 0), wire(2, 3, 1), wire(3, 0, 0)],
            )
        };
        let mut g = product();
        scale_offset(&mut g, 1, "613100", [2., 4., 0.5, -3.]);
        matrix(&mut g, 2, ["623100", "633100"]);
        assert!(crate::select_diffuse(&g).is_err());
        assert!(select(&g, false).is_err());
        let s = select(&g, true).unwrap();
        assert_eq!(animated_indices(&s), [1, 2]);
        let leaf = |i: usize| {
            [s.animation_base]
                .into_iter()
                .chain(s.animation_base_more)
                .flatten()
                .find(|b| b.sample_index == i)
                .unwrap()
        };
        assert_eq!((leaf(1).subtype_raw, leaf(2).subtype_raw), (0, 1));
        let transform = |i: usize| {
            if s.sample_index == i {
                s.uv_transform
            } else {
                s.second_sample.unwrap().uv_transform
            }
        };
        assert_eq!(
            transform(1).bits(),
            [2f32, 4., -(0.5f32 * 2.), -(-3f32 * 4.)].map(f32::to_bits)
        );
        let rows = leaf(2).row_bits;
        assert_eq!(
            transform(2).bits(),
            [rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2]]
        );
        assert!(!crate::linked::uses_grid(s));
        // Every leaf is reconciled on its own; one missing global fails the whole.
        let mut verified = s;
        verify_global(&g, &mut verified, &globals(&g)).unwrap();
        assert!(verified.animation_base.unwrap().global_rows_verified);
        assert!(
            verified.animation_base_more[0]
                .unwrap()
                .global_rows_verified
        );
        let partial = globals(&g)
            .into_iter()
            .filter(|a| a.name_hex != "633100")
            .collect::<Vec<_>>();
        assert!(verify_global(&g, &mut s.clone(), &partial).is_err());
        // Both leaves on one shared curve: identical transforms, still two leaves.
        let mut shared = product();
        scale_offset(&mut shared, 1, "613100", [2., 2., 0., 0.]);
        scale_offset(&mut shared, 2, "613100", [2., 2., 0., 0.]);
        let s = select(&shared, true).unwrap();
        assert_eq!(animated_indices(&s), [1, 2]);
        assert_eq!(s.uv_transform, s.second_sample.unwrap().uv_transform);
        verify_global(&shared, &mut s.clone(), &globals(&shared)).unwrap();
        // Per-leaf rules still apply to every animated leaf.
        let mut uneven = g.clone();
        uneven.boxes[2].animations[1].name_hex = "000000".into();
        assert!(select(&uneven, true).is_err());
        let mut uv2 = g.clone();
        uv2.boxes[2].parameters_raw[4] = 3;
        assert!(select(&uv2, true).is_err());
        let mut wired = g.clone();
        wired.wires.push(wire(1, 2, 0));
        assert!(select(&wired, true).is_err());
        let mut moving_root = g.clone();
        scale_offset(&mut moving_root, 3, "643100", [1., 1., 0., 0.]);
        assert!(select(&moving_root, true).is_err());
    }

    #[test]
    fn d102_partial_row_masks_and_single_animated_tint() {
        // Direct subtype-1 sampler whose rows key only Z (mask 4): base rows used whole.
        let mut direct = graph(&[0, 1], vec![wire(1, 0, 0)]);
        matrix(&mut direct, 1, ["623100", "633100"]);
        for a in &mut direct.boxes[1].animations {
            (
                a.components_raw,
                a.component_value_count,
                a.derived_key_count,
            ) = (4, 5, 5);
        }
        assert!(crate::select_diffuse(&direct).is_err());
        let s = select(&direct, true).unwrap();
        let rows = s.animation_base.unwrap().row_bits;
        assert_eq!(
            s.uv_transform.bits(),
            [rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2]]
        );
        verify_global(&direct, &mut s.clone(), &globals(&direct)).unwrap();
        // Masks outside XYZ, or an empty mask, stay refused for matrix rows.
        for bad in [0u8, 8, 0x0c, 0x87] {
            let mut g = direct.clone();
            g.boxes[1].animations[1].components_raw = bad;
            assert!(select(&g, true).is_err(), "mask {bad}");
        }
        // One animated sample × static COLOR (either port order) under the opt-in.
        for (tex_port, color_port) in [(0, 1), (1, 0)] {
            let mut tinted = graph(
                &[0, 1, 3, 10],
                vec![wire(1, 3, tex_port), wire(2, 3, color_port), wire(3, 0, 0)],
            );
            scale_offset(&mut tinted, 1, "613100", [2., 2., 0.5, 0.]);
            assert!(crate::select_diffuse(&tinted).is_err());
            assert!(select(&tinted, false).is_err());
            let s = select(&tinted, true).unwrap();
            assert_eq!((s.sample_index, s.color_index), (1, Some(2)));
            assert_eq!(s.multiply_index, Some(3));
            assert_eq!(s.tint.values(), [0.5, 1., 0.25, 1.]);
            assert!(super::single_tinted_leaf(&s) && s.second_sample.is_none());
            // The rejections of the per-leaf rule are unchanged.
            let mut wired = tinted.clone();
            wired.wires.push(wire(2, 1, 1));
            assert!(select(&wired, true).is_err());
            let mut uv2 = tinted.clone();
            uv2.boxes[1].parameters_raw[4] = 3;
            assert!(select(&uv2, true).is_err());
            let mut moving_root = tinted.clone();
            scale_offset(&mut moving_root, 3, "643100", [1., 1., 0., 0.]);
            assert!(select(&moving_root, true).is_err());
        }
        // Animated sample × COLOR × COLOR is a D99 product tree, not this route.
        let mut chain = graph(
            &[0, 1, 3, 10, 3, 10],
            vec![
                wire(1, 3, 0),
                wire(2, 3, 1),
                wire(3, 5, 0),
                wire(4, 5, 1),
                wire(5, 0, 0),
            ],
        );
        scale_offset(&mut chain, 1, "613100", [2., 2., 0.5, 0.]);
        assert!(!select(&chain, true).is_ok_and(|s| super::single_tinted_leaf(&s)));
    }

    #[test]
    fn d92_tinted_three_and_four_leaf_products() {
        let nested = |third: u32| {
            graph(
                &[0, 10, 1, 1, third, 10],
                vec![
                    wire(2, 1, 0),
                    wire(3, 1, 1),
                    wire(1, 5, 0),
                    wire(4, 5, 1),
                    wire(5, 0, 0),
                ],
            )
        };
        // (T1 × T2) × COLOR, both leaves animated (D75 tint).
        let mut tinted = nested(3);
        scale_offset(&mut tinted, 2, "613100", [2., 2., 0.5, 0.]);
        scale_offset(&mut tinted, 3, "623100", [3., 3., 0., 0.25]);
        let s = select(&tinted, true).unwrap();
        assert_eq!(s.tint.values(), [0.5, 1., 0.25, 1.]);
        assert_eq!(animated_indices(&s), [2, 3]);
        // (T1 × T2) × T3 with an independent third: all three animated, subtype 0.
        let mut three = nested(1);
        scale_offset(&mut three, 2, "613100", [2., 2., 0.5, 0.]);
        scale_offset(&mut three, 3, "623100", [3., 3., 0., 0.25]);
        scale_offset(&mut three, 4, "633100", [4., 4., 0., 0.]);
        let s = select(&three, true).unwrap();
        assert!(s.third_sample.is_some());
        assert_eq!(animated_indices(&s), [2, 3, 4]);
        verify_global(&three, &mut s.clone(), &globals(&three)).unwrap();
        // An independent subtype-1 third keeps the D65 scale/offset rejection.
        let mut matrix_third = three.clone();
        matrix(&mut matrix_third, 4, ["643100", "653100"]);
        assert!(select(&matrix_third, true).is_err());
        // ((A × B) × C) × D: D shares A's curve and C shares B's, so the exact
        // coordinate sharing holds; four leaves, each with its own snapshot.
        let mut four = graph(
            &[0, 10, 10, 10, 1, 1, 1, 1],
            vec![
                wire(4, 3, 0),
                wire(5, 3, 1),
                wire(3, 2, 0),
                wire(6, 2, 1),
                wire(2, 1, 0),
                wire(7, 1, 1),
                wire(1, 0, 0),
            ],
        );
        for (index, name, v) in [
            (4, "613100", 2.),
            (5, "623100", 3.),
            (6, "623100", 3.),
            (7, "613100", 2.),
        ] {
            scale_offset(&mut four, index, name, [v, v, 0.5, 0.]);
        }
        assert!(select(&four, false).is_err());
        let s = select(&four, true).unwrap();
        assert!(s.fourth_sample.is_some());
        assert_eq!(animated_indices(&s), [4, 5, 6, 7]);
        verify_global(&four, &mut s.clone(), &globals(&four)).unwrap();
    }
}
