//! D89 diffuse ADD (box 11) shapes that reuse existing bake paths: two texture samples
//! summed on the nearest Repeat product grid, and one texture (optionally × COLOR) plus
//! a static COLOR baked per pixel. Rule statements, Toolkit pin and limitations:
//! docs/d89-add-uv2.md. Sums are linear-RGB preview policy clamped at 8-bit encoding.
use crate::{DiffuseSelection, DiffuseTint, SamplingTransform};
use lbp_resource::material::{BoxMetadata, GraphMetadata, WireMetadata};

/// ADD root of the selected diffuse route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffuseAdd {
    pub add_index: usize,
    /// Static COLOR on the other ADD port; `None` when ports 0 and 1 are the two
    /// samples (`sample_index` on port 0, `second_sample` on port 1).
    pub addend: Option<DiffuseTint>,
    pub addend_index: Option<usize>,
}

fn linear_table() -> [f64; 256] {
    std::array::from_fn(|v| {
        let c = v as f64 / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}
fn encode(c: f64) -> u8 {
    let c = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    };
    (c * 255.).round().clamp(0., 255.) as u8
}

impl DiffuseAdd {
    pub fn label(self) -> String {
        match self.addend {
            None => "two-texture ADD (linear sum, clamped) • nearest grid".into(),
            Some(c) => format!("ADD static linear color {:?} (clamped)", c.values()),
        }
    }
    /// `linear(sample) × tint + addend` per RGB channel, alpha `a × tint.a + addend.a`,
    /// clamped to 0..1 at 8-bit encoding.
    pub fn pixels(
        self,
        tint: DiffuseTint,
        rgba: &[u8],
        cancelled: impl Fn() -> bool,
    ) -> Result<Vec<u8>, String> {
        let addend = self
            .addend
            .ok_or("Unsupported: ADD pixel bake needs a COLOR")?;
        tint.validate()?;
        addend.validate()?;
        if !rgba.len().is_multiple_of(4) || rgba.len() > 4 * lbp_texture::MAX_PIXELS {
            return Err("Invalid: ADD pixel extent/budget".into());
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        let (t, c) = (tint.values().map(f64::from), addend.values().map(f64::from));
        let linear = linear_table();
        let tables: [[u8; 256]; 3] =
            std::array::from_fn(|k| std::array::from_fn(|v| encode(linear[v] * t[k] + c[k])));
        let alpha: [u8; 256] = std::array::from_fn(|v| {
            ((v as f64 / 255. * t[3] + c[3]) * 255.)
                .round()
                .clamp(0., 255.) as u8
        });
        let mut result = Vec::with_capacity(rgba.len());
        for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if i % 4096 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            result.extend_from_slice(&[
                tables[0][p[0] as usize],
                tables[1][p[1] as usize],
                tables[2][p[2] as usize],
                alpha[p[3] as usize],
            ]);
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(result)
    }
}

/// Two-texture ADD on the nearest LCM product grid (same grid as D49/D83 products):
/// RGB `linear(a) + linear(b)` and alpha `a + b`, clamped at encoding. `a` is port 0.
pub(crate) fn pixels_sum(
    a: &lbp_texture::Texture,
    b: &lbp_texture::Texture,
    ta: SamplingTransform,
    tb: SamplingTransform,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    bake_sum(
        [
            [a.header.width, a.header.height],
            [b.header.width, b.header.height],
        ],
        [&a.rgba, &b.rgba],
        [crate::product::scales(ta)?, crate::product::scales(tb)?],
        cancelled,
    )
}
fn bake_sum(
    dimensions: [[u32; 2]; 2],
    sources: [&[u8]; 2],
    scales: [[u32; 2]; 2],
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    let (width, height, count) = crate::product::extent(dimensions, sources, scales)?;
    if cancelled() {
        return Err("Cancelled".into());
    }
    let linear = linear_table();
    // 65536-entry encoded sum table bounds the nonlinear work.
    let mut table = vec![0u8; 65536];
    for x in 0..256 {
        if x % 16 == 0 && cancelled() {
            return Err("Cancelled".into());
        }
        for y in 0..256 {
            table[x * 256 + y] = encode(linear[x] + linear[y]);
        }
    }
    let [da, db] = dimensions;
    let [a, b] = sources;
    let [sa, sb] = scales;
    let mut output = Vec::with_capacity(count as usize * 4);
    for y in 0..height {
        for x in 0..width {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            let index = |d: [u32; 2], s: [u32; 2]| {
                let px = ((u64::from(x) * u64::from(d[0]) * u64::from(s[0]) / u64::from(width))
                    % u64::from(d[0])) as usize;
                let py = ((u64::from(y) * u64::from(d[1]) * u64::from(s[1]) / u64::from(height))
                    % u64::from(d[1])) as usize;
                (py * d[0] as usize + px) * 4
            };
            let (i, j) = (index(da, sa), index(db, sb));
            for c in 0..3 {
                output.push(table[a[i + c] as usize * 256 + b[j + c] as usize]);
            }
            output.push((u16::from(a[i + 3]) + u16::from(b[j + 3])).min(255) as u8);
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((width, height, output))
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
/// Incoming wires of `index`, sorted by destination port; endpoints already checked.
fn inputs(graph: &GraphMetadata, index: usize) -> Vec<&WireMetadata> {
    let mut wires: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == index)
        .collect();
    wires.sort_by_key(|w| w.port_to_raw);
    wires
}
/// Two plain inputs on ports 0 and 1 of an unanimated node: `[port 0, port 1]` sources.
fn binary(graph: &GraphMetadata, index: usize) -> Option<[usize; 2]> {
    match inputs(graph, index)[..] {
        [l, r]
            if unanimated(&graph.boxes[index])
                && l.port_to_raw == 0
                && r.port_to_raw == 1
                && plain(l)
                && plain(r) =>
        {
            Some([l.box_from as usize, r.box_from as usize])
        }
        _ => None,
    }
}
/// Unwired, unanimated COLOR within the 0..1 preview bounds.
fn static_color(graph: &GraphMetadata, index: usize) -> Result<Option<DiffuseTint>, String> {
    let node = &graph.boxes[index];
    if node.type_raw != 3 {
        return Ok(None);
    }
    if !unanimated(node) || !inputs(graph, index).is_empty() {
        return Err("Unsupported: ADD COLOR must be static, unwired and subtype zero".into());
    }
    let color = DiffuseTint {
        bits: node.parameters_raw[..4].try_into().unwrap(),
    };
    color.validate()?;
    Ok(Some(color))
}

/// Diffuse wire from an ADD (box 11) at `root`; the output/diffuse wire is already
/// validated by `select_products`. Every rejection names the unsupported ADD form.
pub(crate) fn select_add(
    graph: &GraphMetadata,
    root: usize,
    output: usize,
) -> Result<DiffuseSelection, String> {
    let Some([left, right]) = binary(graph, root) else {
        return Err(
            "Unsupported: ADD needs a static node with unswizzled inputs on ports 0 and 1".into(),
        );
    };
    let kind = |i: usize| graph.boxes[i].type_raw;
    let add = |addend, addend_index| DiffuseAdd {
        add_index: root,
        addend,
        addend_index,
    };
    match (kind(left), kind(right)) {
        (1, 1) if left != right => {
            let first = crate::linked::sampler(graph, left)?;
            let second = crate::linked::sampler(graph, right)?;
            let selection = DiffuseSelection {
                texture_add: Some(add(None, None)),
                output_index: output,
                sample_index: left,
                slot: first.slot,
                uv_channel: first.uv_channel,
                uv_transform: first.uv_transform,
                second_sample: Some(second),
                ignored_nodes: graph.boxes.len() - 4,
                ignored_wires: graph.wires.len() - 3,
                ..Default::default()
            };
            if !crate::linked::uses_grid(selection) {
                return Err(
                    "Unsupported: two-texture ADD requires same-channel integer Repeat grid samplers"
                        .into(),
                );
            }
            Ok(selection)
        }
        (3, other) | (other, 3) if matches!(other, 1 | 10) => {
            let (color, operand) = if kind(left) == 3 {
                (left, right)
            } else {
                (right, left)
            };
            let addend = static_color(graph, color)?.ok_or("COLOR expected")?;
            let (sample_index, tint, multiply_index, color_index, used) = if other == 1 {
                (operand, DiffuseTint::default(), None, None, (4, 3))
            } else {
                let Some([a, b]) = binary(graph, operand) else {
                    return Err("Unsupported: ADD operand MULTIPLY needs static unswizzled inputs on ports 0 and 1".into());
                };
                let (sample, tint_index) = match (kind(a), kind(b)) {
                    (1, 3) => (a, b),
                    (3, 1) => (b, a),
                    _ => return Err("Unsupported: ADD of COLOR and a MULTIPLY needs one texture sample × one COLOR".into()),
                };
                let tint = static_color(graph, tint_index)?.ok_or("COLOR expected")?;
                (sample, tint, Some(operand), Some(tint_index), (6, 5))
            };
            if color_index == Some(color) {
                return Err("Unsupported: ADD reuses its COLOR inside the product".into());
            }
            let sampled = crate::linked::sampler(graph, sample_index)?;
            Ok(DiffuseSelection {
                texture_add: Some(add(Some(addend), Some(color))),
                output_index: output,
                sample_index,
                slot: sampled.slot,
                uv_channel: sampled.uv_channel,
                uv_transform: sampled.uv_transform,
                tint,
                multiply_index,
                color_index,
                ignored_nodes: graph.boxes.len() - used.0,
                ignored_wires: graph.wires.len() - used.1,
                ..Default::default()
            })
        }
        (1, 10) | (10, 1) => Err(
            "Unsupported: ADD of a texture and a product needs a three-texture bake (no render path)"
                .into(),
        ),
        _ => Err(
            "Unsupported: ADD supports two texture samples, or a texture (optionally × COLOR) plus COLOR"
                .into(),
        ),
    }
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
    fn sample(scale: f32, slot: u32) -> BoxMetadata {
        let mut n = node(1, [scale, scale, 0., -0.]);
        n.parameters_raw[5] = slot;
        n
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
    fn two_texture_add_selects_grid_samples_on_ports_zero_and_one() {
        // Output, ADD, port-1 sample (scale 4, slot 0), port-0 sample (slot 1), extra
        // sample on another output port (ignored).
        let base = graph(
            vec![
                node(0, [0.; 4]),
                node(11, [0.; 4]),
                sample(4., 0),
                sample(1., 1),
                sample(1., 2),
            ],
            vec![wire(3, 1, 0), wire(2, 1, 1), wire(1, 0, 0), wire(4, 0, 3)],
        );
        let s = crate::select_diffuse(&base).unwrap();
        let add = s.texture_add.unwrap();
        assert_eq!(
            (add.add_index, add.addend, add.addend_index),
            (1, None, None)
        );
        assert_eq!((s.sample_index, s.slot), (3, 1));
        let second = s.second_sample.unwrap();
        assert_eq!((second.sample_index, second.slot), (2, 0));
        assert_eq!((s.ignored_nodes, s.ignored_wires), (1, 1));
        assert!(s.multiply_index.is_none() && s.tint == DiffuseTint::default());
        // Non-integer scale cannot use the grid: D103 resampled approximation. A
        // different UV channel under an ADD is not a separable product: refused.
        let mut g = base.clone();
        g.boxes[2].parameters_raw[0] = 1.5f32.to_bits();
        assert!(crate::select_diffuse(&g)
            .unwrap()
            .grid_graph
            .unwrap()
            .off_grid
            .is_some());
        let mut g = base.clone();
        g.boxes[2].parameters_raw[4] = 1;
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("outside a separable product"));
        // D104: UV channel word 2 samples UV0, so the grid sum still applies; an
        // unobserved word stays unsupported.
        let mut g = base.clone();
        g.boxes[2].parameters_raw[4] = 2;
        assert_eq!(
            crate::select_diffuse(&g).unwrap(),
            crate::select_diffuse(&base).unwrap()
        );
        g.boxes[2].parameters_raw[4] = 3;
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("sampler must be static"));
        // Swizzle, missing port, animated ADD, same sample twice.
        let mut g = base.clone();
        g.wires[0].swizzle_raw[0] = b'x';
        // D100: a one-component swizzle broadcasts; the graph evaluator bakes it.
        assert!(crate::select_diffuse(&g).unwrap().grid_graph.is_some());
        g.wires[0].swizzle_raw[1] = b'y';
        assert!(crate::select_diffuse(&g).unwrap_err().contains("ADD needs"));
        let mut g = base.clone();
        g.wires[1].port_to_raw = 2;
        assert!(crate::select_diffuse(&g).unwrap_err().contains("ADD needs"));
        let mut g = base.clone();
        g.boxes[1].animations[0].name_hex = "410000".into();
        assert!(crate::select_diffuse(&g).unwrap_err().contains("ADD needs"));
        // D100: the same sample twice is one source, evaluated per texel.
        let mut g = base;
        g.wires[1].box_from = 3;
        assert_eq!(
            crate::select_diffuse(&g)
                .unwrap()
                .grid_graph
                .unwrap()
                .sources,
            1
        );
    }
    #[test]
    fn texture_times_color_plus_color_and_three_texture_add() {
        let c1 = [0.4117647f32, 0.39215687, 0.37254903, 1.];
        let c2 = [0.16470589f32, 0.08627451, 0., 1.];
        // Output, sample (scale 6), COLOR c2, MULTIPLY, COLOR c1, ADD.
        let base = graph(
            vec![
                node(0, [0.; 4]),
                sample(6., 0),
                node(3, c2),
                node(10, [0.; 4]),
                node(3, c1),
                node(11, [0.; 4]),
            ],
            vec![
                wire(1, 3, 0),
                wire(4, 3, 1),
                wire(3, 5, 0),
                wire(2, 5, 1),
                wire(5, 0, 0),
            ],
        );
        let s = crate::select_diffuse(&base).unwrap();
        let add = s.texture_add.unwrap();
        assert_eq!(add.addend.unwrap().values(), c2);
        assert_eq!((add.add_index, add.addend_index), (5, Some(2)));
        assert_eq!(s.tint.values(), c1);
        assert_eq!(
            (s.sample_index, s.multiply_index, s.color_index),
            (1, Some(3), Some(4))
        );
        assert!(s.second_sample.is_none());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        // TEX + COLOR directly (identity tint).
        let mut g = base.clone();
        g.wires[2].box_from = 1;
        let s = crate::select_diffuse(&g).unwrap();
        assert_eq!((s.sample_index, s.multiply_index), (1, None));
        assert_eq!(s.tint, DiffuseTint::default());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (2, 2));
        // COLOR outside 0..1 (the football net has alpha -0.236).
        let mut g = base.clone();
        g.boxes[2].parameters_raw[3] = (-0.236f32).to_bits();
        assert!(crate::select_diffuse(&g).unwrap_err().contains("0..1"));
        // Multiply of two textures under the ADD needs a three-texture bake.
        let mut g = base.clone();
        g.boxes[4] = sample(2., 1);
        g.boxes[2] = sample(1., 2);
        // D100: the graph evaluator bakes it on the Repeat grid.
        assert_eq!(
            crate::select_diffuse(&g)
                .unwrap()
                .grid_graph
                .unwrap()
                .sources,
            3
        );
        // D103: off the grid (scale 1.5 on one axis) a labelled resampled bake.
        g.boxes[2].parameters_raw[0] = 1.5f32.to_bits();
        assert_eq!(
            crate::select_diffuse(&g)
                .unwrap()
                .grid_graph
                .unwrap()
                .off_grid,
            Some(crate::OffGrid::Resampled {
                channel: 0,
                period: [2, 1]
            })
        );
        // Animated COLOR addend.
        let mut g = base;
        g.boxes[2].animations[1].name_hex = "410000".into();
        assert!(crate::select_diffuse(&g).unwrap_err().contains("ADD COLOR"));
    }
    #[test]
    fn add_bakes_clamp_linear_sums_and_cancel() {
        let add = DiffuseAdd {
            add_index: 0,
            addend: Some(DiffuseTint {
                bits: [0.5f32, 0., 1., 0.25].map(f32::to_bits),
            }),
            addend_index: Some(1),
        };
        let tint = DiffuseTint {
            bits: [1f32, 0.5, 1., 1.].map(f32::to_bits),
        };
        let out = add.pixels(tint, &[0, 255, 128, 255], || false).unwrap();
        // R: 0 + 0.5 linear -> sRGB 188; G: 1.0 * 0.5 -> 188; B clamps; A clamps.
        assert_eq!(out, [188, 188, 255, 255]);
        let out = add.pixels(tint, &[0, 0, 0, 0], || false).unwrap();
        assert_eq!(out, [188, 0, 255, 64]);
        assert!(add.pixels(tint, &[0; 4], || true).is_err());
        assert!(add.pixels(tint, &[0; 3], || false).is_err());
        let none = DiffuseAdd {
            addend: None,
            ..add
        };
        assert!(none.pixels(tint, &[0; 4], || false).is_err());
        // Grid: 1x1 red (scale 2) + 2x1 [black, green] (scale 1) -> 2x1 grid.
        let red = [255, 0, 0, 200];
        let pair = [0, 0, 0, 100, 0, 128, 0, 10];
        let (w, h, rgba) =
            bake_sum([[1, 1], [2, 1]], [&red, &pair], [[2, 1], [1, 1]], || false).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(rgba, [255, 0, 0, 255, 255, 128, 0, 210]);
        assert!(bake_sum([[1, 1]; 2], [&red, &red], [[1, 1]; 2], || true).is_err());
        assert!(bake_sum([[1, 1]; 2], [&red, &red], [[17, 1], [1, 1]], || false).is_err());
    }
}
