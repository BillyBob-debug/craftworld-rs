//! Bounded static COLOR → DIFFUSE projection. Evidence/pin: docs/solid-color-preview.md.
//! Unwired diffuse (zero per Toolkit's generator, game unverified): docs/unwired-diffuse-preview.md.
use crate::DiffuseTint;
use lbp_resource::material::GraphMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidSelection {
    pub output_index: usize,
    pub color_index: usize,
    pub color: DiffuseTint,
    pub ignored_nodes: usize,
    pub ignored_wires: usize,
    /// No diffuse wire: `color` is zero and `color_index` equals `output_index`.
    pub unwired_diffuse: bool,
    /// D87 COLOR × COLOR: the two operand nodes; `color_index` is then the multiply.
    pub product_of: Option<[usize; 2]>,
    /// D101: COLOR-only MULTIPLY tree folded to `color`; `color_index` is its root.
    pub product_tree: Option<crate::ProductTree>,
    /// D190: raw swizzle of the diffuse wire into the output; all zero when none.
    /// Only the RGB-identity prefixes `xyz`/`rgb`/`xyzw`/`rgba` are accepted.
    pub output_swizzle: [u8; 5],
}
/// D190 (docs/d190-small-material-fixes.md): diffuse-wire swizzles whose displayed RGB
/// is the source RGB under both readings of the Toolkit pin: GfxAssembler `setupPath`
/// resolves the output's source box without the wire swizzle, while `getWithSwizzle`
/// (used for every other wire) appends it (`.rgb` then `float4(v, 1.0)`). The two
/// readings differ only in alpha, which the solid preview does not display.
pub(crate) fn rgb_identity_swizzle(swizzle: [u8; 5]) -> bool {
    matches!(&swizzle, b"xyz\0\0" | b"rgb\0\0" | b"xyzw\0" | b"rgba\0")
}
pub fn select_solid_diffuse(graph: &GraphMetadata) -> Result<Option<SolidSelection>, String> {
    let boxes = &graph.boxes;
    let wires = &graph.wires;
    if boxes.len() > 1024 || wires.len() > 4096 {
        return Err("Unsupported: solid diffuse graph budget".into());
    }
    if wires.iter().any(|w| {
        w.box_from < 0
            || w.box_to < 0
            || w.box_from as usize >= boxes.len()
            || w.box_to as usize >= boxes.len()
    }) {
        return Err("Unsupported: solid diffuse wire endpoints".into());
    }
    let outputs: Vec<_> = boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0)
        .collect();
    if outputs.len() != 1 {
        return Err("Unsupported: solid diffuse requires one output".into());
    }
    let (output_index, output) = outputs[0];
    let diffuse: Vec<_> = wires
        .iter()
        .filter(|w| w.box_to as usize == output_index && w.port_to_raw == 0)
        .collect();
    if diffuse.is_empty() {
        // Only a static, parameterless output; Blender-family nodes (128+) carry
        // output-parameter colors in the reference reader and stay unsupported.
        if output.subtype_raw != 0
            || output.parameters_raw != [0; 8]
            || output
                .animations
                .iter()
                .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|v| v != b'0'))
            || boxes.iter().any(|b| b.type_raw >= 128)
        {
            return Err("Unsupported: unwired diffuse needs a static parameterless output".into());
        }
        return Ok(Some(SolidSelection {
            output_index,
            color_index: output_index,
            color: DiffuseTint { bits: [0; 4] },
            ignored_nodes: boxes.len() - 1,
            ignored_wires: wires.len(),
            unwired_diffuse: true,
            product_of: None,
            product_tree: None,
            output_swizzle: [0; 5],
        }));
    }
    if diffuse.len() != 1 {
        return Err("Unsupported: solid diffuse requires one diffuse wire".into());
    }
    let wire = diffuse[0];
    let color_index = wire.box_from as usize;
    let node = &boxes[color_index];
    if node.type_raw != 3 {
        return Ok(None);
    }
    if wire.port_from_raw != 0
        || (wire.swizzle_raw != [0; 5] && !rgb_identity_swizzle(wire.swizzle_raw))
        || [node, output].iter().any(|b| {
            b.subtype_raw != 0
                || b.animations
                    .iter()
                    .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|v| v != b'0'))
        })
        || wires.iter().any(|w| w.box_to as usize == color_index)
    {
        return Err(
            "Unsupported: solid diffuse needs an unwired unanimated subtype-zero COLOR".into(),
        );
    }
    let color = DiffuseTint {
        bits: node.parameters_raw[..4].try_into().unwrap(),
    };
    color.validate()?;
    Ok(Some(SolidSelection {
        output_index,
        color_index,
        color,
        ignored_nodes: boxes.len() - 2,
        ignored_wires: wires.len() - 1,
        unwired_diffuse: false,
        product_of: None,
        product_tree: None,
        output_swizzle: wire.swizzle_raw,
    }))
}
