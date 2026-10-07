//! Bounded static COLOR → DIFFUSE projection. Evidence/pin: docs/solid-color-preview.md.
use crate::DiffuseTint;
use lbp_resource::material::GraphMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidSelection {
    pub output_index: usize,
    pub color_index: usize,
    pub color: DiffuseTint,
    pub ignored_nodes: usize,
    pub ignored_wires: usize,
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
        || wire.swizzle_raw != [0; 5]
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
    }))
}
