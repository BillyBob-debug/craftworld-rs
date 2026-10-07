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
        || sample.parameters_raw[4] > 1
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
        let valid_mask = if sample.subtype_raw == 1 {
            mask == 7
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
            uv_channel: sample.parameters_raw[4] as usize,
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

/// Reconcile both local bindings against unique global curve declarations.
/// Matching metadata establishes input identity, never a game time or playback rule.
pub(crate) fn verify_global(
    graph: &lbp_resource::material::GraphMetadata,
    selection: &mut crate::DiffuseSelection,
    globals: &[lbp_resource::material::AnimationMetadata],
) -> Result<(), String> {
    let Some(snapshot) = selection.animation_base.as_mut() else {
        return Ok(());
    };
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
