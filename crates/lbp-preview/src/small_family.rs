//! D87 small "Other" diffuse families reusing existing render paths: CONST4 alone and
//! COLOR × COLOR as solid colors, and one static sampler whose UV is scaled by a
//! CONST/CONST2 on sampler port 1 (folded into its scale/offset). Independent rule
//! statements, Toolkit pin and limitations: docs/d87-small-families.md.
use crate::{DiffuseSelection, DiffuseTint, SolidSelection};
use lbp_resource::material::{BoxMetadata, GraphMetadata, WireMetadata};

/// Sampler port-1 UV scale folded into the selected sampler's scale/offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerScale {
    /// CONST (4) or CONST2 (5) node wired to sampler port 1.
    pub const_index: usize,
    /// X/Y factor bits; a scalar CONST repeats its value.
    pub factor_bits: [u32; 2],
    /// Sampler's own scale X/Y, offset X/Y bits before folding.
    pub source_bits: [u32; 4],
}

fn unanimated(node: &BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
}
fn plain(wire: &WireMetadata) -> bool {
    wire.port_from_raw == 0 && wire.swizzle_raw == [0; 5]
}
fn inputs(graph: &GraphMetadata, index: usize) -> Vec<&WireMetadata> {
    graph
        .wires
        .iter()
        .filter(|w| w.box_to >= 0 && w.box_to as usize == index)
        .collect()
}
/// The single static diffuse wire, or None when the graph is outside the basic bounds
/// (the existing routes report those reasons).
fn diffuse_source(graph: &GraphMetadata) -> Option<(usize, usize)> {
    let n = graph.boxes.len();
    if n > 1024
        || graph.wires.len() > 4096
        || graph.wires.iter().any(|w| {
            w.box_from < 0 || w.box_to < 0 || w.box_from as usize >= n || w.box_to as usize >= n
        })
    {
        return None;
    }
    let mut outputs = graph
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0);
    let (output_index, output) = outputs.next()?;
    if outputs.next().is_some() || !unanimated(output) {
        return None;
    }
    let diffuse: Vec<_> = inputs(graph, output_index)
        .into_iter()
        .filter(|w| w.port_to_raw == 0)
        .collect();
    match diffuse[..] {
        [wire] if plain(wire) => Some((output_index, wire.box_from as usize)),
        _ => None,
    }
}

/// CONST4 alone or COLOR × COLOR on the diffuse wire. `Ok(None)`: not these shapes.
/// Callers apply it to the original graph only (not an instance-color derived graph).
pub fn select_small_solid(graph: &GraphMetadata) -> Result<Option<SolidSelection>, String> {
    let Some((output_index, source)) = diffuse_source(graph) else {
        return Ok(None);
    };
    let node = &graph.boxes[source];
    let static_leaf =
        |index: usize| unanimated(&graph.boxes[index]) && inputs(graph, index).is_empty();
    let (color, used_nodes, used_wires, product_of) = match node.type_raw {
        7 => {
            if !static_leaf(source) {
                return Err(
                    "Unsupported: CONST4 diffuse must be static, unwired and subtype zero".into(),
                );
            }
            let color = DiffuseTint {
                bits: node.parameters_raw[..4].try_into().unwrap(),
            };
            (color, 2, 1, None)
        }
        10 => {
            let incoming = inputs(graph, source);
            let [a, b] = match incoming[..] {
                [x, y] => [x, y],
                _ => return Ok(None),
            };
            let colors = [a, b].map(|w| graph.boxes[w.box_from as usize].type_raw == 3);
            if colors != [true, true] {
                return Ok(None);
            }
            let (left, right) = if a.port_to_raw == 0 { (a, b) } else { (b, a) };
            if left.port_to_raw != 0
                || right.port_to_raw != 1
                || !plain(left)
                || !plain(right)
                || !unanimated(node)
                || ![left, right]
                    .iter()
                    .all(|w| static_leaf(w.box_from as usize))
            {
                return Err("Unsupported: COLOR × COLOR needs a static multiply of two unwired unanimated COLORs on ports 0/1".into());
            }
            let operands = [left.box_from as usize, right.box_from as usize];
            let [l, r] = operands.map(|i| DiffuseTint {
                bits: graph.boxes[i].parameters_raw[..4].try_into().unwrap(),
            });
            l.validate()?;
            r.validate()?;
            let (lv, rv) = (l.values(), r.values());
            let color = DiffuseTint {
                bits: std::array::from_fn(|i| (lv[i] * rv[i]).to_bits()),
            };
            let distinct = 2 + usize::from(operands[0] != operands[1]);
            (color, 1 + distinct, 3, Some(operands))
        }
        _ => return Ok(None),
    };
    color.validate()?;
    Ok(Some(SolidSelection {
        output_index,
        color_index: source,
        color,
        ignored_nodes: graph.boxes.len() - used_nodes,
        ignored_wires: graph.wires.len() - used_wires,
        unwired_diffuse: false,
        product_of,
    }))
}

/// One static subtype-0 sampler on the diffuse wire whose only input is a static
/// CONST/CONST2 on port 1 (UV scale). `Ok(None)`: not this shape.
pub(crate) fn select_scaled_sampler(
    graph: &GraphMetadata,
) -> Result<Option<DiffuseSelection>, String> {
    let Some((output_index, sample_index)) = diffuse_source(graph) else {
        return Ok(None);
    };
    let sample = &graph.boxes[sample_index];
    let incoming = inputs(graph, sample_index);
    let [wire] = incoming[..] else {
        return Ok(None);
    };
    let const_index = wire.box_from as usize;
    let constant = &graph.boxes[const_index];
    if sample.type_raw != 1 || wire.port_to_raw != 1 || !matches!(constant.type_raw, 4 | 5) {
        return Ok(None);
    }
    let p = &sample.parameters_raw;
    if !plain(wire)
        || !unanimated(sample)
        || p[4] > 1
        || !unanimated(constant)
        || !inputs(graph, const_index).is_empty()
        || const_index == sample_index
    {
        return Err("Unsupported: sampler UV scale needs a static subtype-0 UV0/UV1 sampler and an unwired static CONST/CONST2 on port 1".into());
    }
    let factor_bits = if constant.type_raw == 4 {
        [constant.parameters_raw[0]; 2]
    } else {
        [constant.parameters_raw[0], constant.parameters_raw[1]]
    };
    let factor = factor_bits.map(f32::from_bits);
    if factor.iter().any(|v| !v.is_finite()) {
        return Err("Unsupported: sampler UV scale factor must be finite".into());
    }
    let source_bits = [p[0], p[1], p[2], p[3]];
    let source = crate::UvTransform { bits: source_bits };
    source.validate()?;
    let [sx, sy, ox, oy] = source.values();
    // ((uv × s) + o) × c, regrouped as uv × (s × c) + (o × c); each product rounded once.
    let folded = crate::UvTransform {
        bits: [
            sx * factor[0],
            sy * factor[1],
            ox * factor[0],
            oy * factor[1],
        ]
        .map(f32::to_bits),
    };
    let uv_transform = crate::SamplingTransform::ScaleOffset(folded);
    uv_transform.validate()?;
    Ok(Some(DiffuseSelection {
        sampler_scale: Some(SamplerScale {
            const_index,
            factor_bits,
            source_bits,
        }),
        output_index,
        sample_index,
        slot: p[5] as usize,
        uv_channel: p[4] as usize,
        uv_transform,
        ignored_nodes: graph.boxes.len() - 3,
        ignored_wires: graph.wires.len() - 2,
        ..Default::default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::AnimationMetadata;
    fn node(kind: u32, params: [f32; 4]) -> BoxMetadata {
        let mut raw = [0; 8];
        raw[..4].copy_from_slice(&params.map(f32::to_bits));
        BoxMetadata {
            type_raw: kind,
            declared_parameter_count: 8,
            parameters_raw: raw,
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
    #[test]
    fn const4_alone_is_a_solid_color_and_guards_static_inputs() {
        let base = graph(
            vec![node(0, [0.; 4]), node(7, [0.15, 0.15, 0.15, 1.])],
            vec![wire(1, 0, 0), wire(1, 0, 7)],
        );
        let s = select_small_solid(&base).unwrap().unwrap();
        assert_eq!(s.color.values(), [0.15, 0.15, 0.15, 1.]);
        assert_eq!((s.color_index, s.product_of), (1, None));
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 1));
        // The existing solid route leaves CONST4 to this family.
        assert!(crate::select_solid_diffuse(&base).unwrap().is_none());
        let mut g = base.clone();
        g.boxes[1].animations[0].name_hex = "410000".into();
        assert!(select_small_solid(&g).is_err());
        let mut g = base.clone();
        g.boxes[1].parameters_raw[3] = 1.5f32.to_bits();
        assert!(select_small_solid(&g).unwrap_err().contains("0..1"));
        let mut g = base.clone();
        g.wires[0].swizzle_raw[0] = b'x';
        assert!(select_small_solid(&g).unwrap().is_none());
        let mut g = base;
        g.boxes[1].type_raw = 6; // CONST3 stays outside D87.
        assert!(select_small_solid(&g).unwrap().is_none());
    }
    #[test]
    fn color_times_color_multiplies_componentwise_in_f32() {
        let a = [0.7059f32, 0.6078, 0.2157, 1.];
        let b = [0.5922f32, 0.5, 1., 0.25];
        let base = graph(
            vec![node(0, [0.; 4]), node(3, a), node(3, b), node(10, [0.; 4])],
            vec![wire(1, 3, 0), wire(2, 3, 1), wire(3, 0, 0)],
        );
        let s = select_small_solid(&base).unwrap().unwrap();
        assert_eq!(s.color.values(), std::array::from_fn(|i| a[i] * b[i]));
        assert_eq!((s.color_index, s.product_of), (3, Some([1, 2])));
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        assert!(crate::select_solid_diffuse(&base).unwrap().is_none());
        let mut g = base.clone();
        g.boxes[2].type_raw = 1; // texture × COLOR is the existing tint route
        assert!(select_small_solid(&g).unwrap().is_none());
        let mut g = base.clone();
        g.boxes[2].type_raw = 2; // THING_COLOR stays out of scope
        assert!(select_small_solid(&g).unwrap().is_none());
        let mut g = base.clone();
        g.wires[1].port_to_raw = 0;
        assert!(select_small_solid(&g).is_err());
        let mut g = base.clone();
        g.wires.push(wire(1, 2, 0));
        assert!(select_small_solid(&g).is_err());
        let mut g = base.clone();
        g.boxes[3].animations[1].component_value_count = 2;
        assert!(select_small_solid(&g).is_err());
        let mut g = base;
        g.boxes[1].parameters_raw[0] = (-0.01f32).to_bits();
        assert!(select_small_solid(&g).is_err());
    }
    #[test]
    fn port_one_constant_folds_into_sampler_scale_and_offset() {
        let mut sampler = node(1, [1.6, 1.5, 0.25, -0.5]);
        sampler.parameters_raw[5] = 3;
        let base = graph(
            vec![node(0, [0.; 4]), sampler, node(4, [0.75, 0., 0., 0.])],
            vec![wire(1, 0, 0), wire(2, 1, 1)],
        );
        let s = select_scaled_sampler(&base).unwrap().unwrap();
        assert_eq!((s.sample_index, s.slot, s.uv_channel), (1, 3, 0));
        let crate::SamplingTransform::ScaleOffset(t) = s.uv_transform else {
            panic!("scale/offset expected")
        };
        assert_eq!(
            t.values(),
            [1.6 * 0.75, 1.5 * 0.75, 0.25 * 0.75, -0.5 * 0.75]
        );
        let scale = s.sampler_scale.unwrap();
        assert_eq!(
            (scale.const_index, scale.factor_bits),
            (2, [0.75f32.to_bits(); 2])
        );
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        // Also reachable through the public diffuse selector.
        assert_eq!(crate::select_diffuse(&base).unwrap(), s);
        // CONST2 scales per axis.
        let mut g = base.clone();
        g.boxes[2] = node(5, [1., 0.6, 0., 0.]);
        let t = select_scaled_sampler(&g)
            .unwrap()
            .unwrap()
            .uv_transform
            .bits()
            .to_vec();
        assert_eq!(t, [1.6f32, 1.5 * 0.6, 0.25, -0.5 * 0.6].map(f32::to_bits));
        // Rejections: other ports, swizzle, CONST3, matrix sampler, UV2, animation.
        for port in [0, 2, 3] {
            let mut g = base.clone();
            g.wires[1].port_to_raw = port;
            assert!(crate::select_diffuse(&g)
                .unwrap_err()
                .contains("sampler must be static"));
        }
        let mut g = base.clone();
        g.wires[1].swizzle_raw[0] = b'x';
        assert!(select_scaled_sampler(&g).is_err());
        let mut g = base.clone();
        g.boxes[2].type_raw = 6;
        assert!(select_scaled_sampler(&g).unwrap().is_none());
        let mut g = base.clone();
        g.boxes[1].subtype_raw = 1;
        assert!(select_scaled_sampler(&g).is_err());
        let mut g = base.clone();
        g.boxes[1].parameters_raw[4] = 2;
        assert!(select_scaled_sampler(&g).is_err());
        let mut g = base.clone();
        g.boxes[2].animations[0].name_hex = "410000".into();
        assert!(select_scaled_sampler(&g).is_err());
        let mut g = base.clone();
        g.boxes[2].parameters_raw[0] = f32::NAN.to_bits();
        assert!(select_scaled_sampler(&g).is_err());
        let mut g = base;
        g.boxes.push(node(4, [2., 0., 0., 0.]));
        g.wires.push(wire(3, 1, 0));
        assert!(select_scaled_sampler(&g).unwrap().is_none());
    }
}
