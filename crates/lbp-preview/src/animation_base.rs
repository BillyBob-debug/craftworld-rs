//! Opt-in local animation base-value snapshot, not a game-time sample.
//! Sources/bounds/unknown timing: docs/animation-base-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationBaseSnapshot {
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
    let sample_index = wire.box_from as usize;
    let sample = &graph.boxes[sample_index];
    if wire.port_from_raw != 0
        || wire.swizzle_raw != [0; 5]
        || sample.type_raw != 1
        || sample.subtype_raw != 1
        || sample.parameters_raw[4] > 1
        || graph
            .wires
            .iter()
            .any(|w| w.box_to as usize == sample_index)
    {
        return Err(
            "Unsupported: animation base only supports direct unwired matrix UV0/UV1 sampler"
                .into(),
        );
    }
    for a in &sample.animations {
        let name = a.name_hex.as_bytes();
        if name.len() != 6
            || &name[4..] != b"00"
            || !name[..4].iter().all(u8::is_ascii_hexdigit)
            || a.components_raw != 7
            || a.component_value_count == 0
            || a.component_value_count > 4096
            || !a.component_value_count.is_multiple_of(3)
            || a.derived_key_count != a.component_value_count / 3
        {
            return Err("Unsupported: animation base needs two named XYZ row curves".into());
        }
        for offset in [0, 2] {
            let value = u8::from_str_radix(&a.name_hex[offset..offset + 2], 16)
                .map_err(|_| "Invalid: animation name")?;
            if !value.is_ascii_alphanumeric() {
                return Err("Unsupported: animation base name".into());
            }
        }
    }
    if sample.animations[0].name_hex == sample.animations[1].name_hex {
        return Err("Unsupported: animation base row names must differ".into());
    }
    let rows = sample.animations.each_ref().map(|a| a.base_value_bits);
    let uv_transform = crate::SamplingTransform::Matrix(crate::UvMatrix {
        bits: [
            rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2],
        ],
    });
    uv_transform.validate()?;
    Ok(crate::DiffuseSelection {
        animation_base: Some(AnimationBaseSnapshot {
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
        }),
        output_index,
        sample_index,
        slot: sample.parameters_raw[5] as usize,
        uv_channel: sample.parameters_raw[4] as usize,
        uv_transform,
        ignored_nodes: graph.boxes.len() - 2,
        ignored_wires: graph.wires.len() - 1,
        ..Default::default()
    })
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
    for local in &graph.boxes[selection.sample_index].animations {
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
