//! D99: static MULTIPLY trees whose leaves are texture samples and static COLOR
//! boxes, in any nesting. The component-wise product is commutative and
//! associative, so all COLOR leaves fold into one tint and the 1-4 texture leaves
//! (with multiplicity) map onto the existing single, two-texture, three-texture
//! and four-chain render roles. Runs only after the fixed product routes refused a
//! graph. Evidence, scope and limits: docs/d99-product-trees.md.
use lbp_resource::material::{BoxMetadata, GraphMetadata};

/// Bounded walk over the diffuse MULTIPLY tree (a DAG is expanded with multiplicity).
const MAX_VISITS: usize = 64;
const MAX_DEPTH: usize = 8;
const MAX_TEXTURE_LEAVES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductTree {
    /// Root MULTIPLY (the box wired to the diffuse port).
    pub root: usize,
    /// Distinct MULTIPLY boxes in the tree.
    pub multiply_nodes: u8,
    /// Texture leaves counted with multiplicity (a squared sampler counts twice).
    pub texture_leaves: u8,
    /// COLOR leaves counted with multiplicity; their product is `DiffuseSelection::tint`.
    pub color_leaves: u8,
}
impl ProductTree {
    pub fn label(self) -> String {
        format!(
            "product tree: {} texture leaves × {} folded COLOR leaves under {} MULTIPLY",
            self.texture_leaves, self.color_leaves, self.multiply_nodes
        )
    }
}

fn unanimated(node: &BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
}

/// Leaves of the tree under `root`, in walk order, with multiplicity.
pub(crate) struct Flat {
    pub(crate) multiplies: Vec<usize>,
    pub(crate) textures: Vec<usize>,
    pub(crate) colors: Vec<usize>,
}

/// `Ok(None)`: not a pure MULTIPLY tree of TEX/COLOR leaves (the caller keeps its
/// own reason). `Err`: a tree whose leaves or budget are unsupported.
pub(crate) fn flatten(graph: &GraphMetadata, root: usize) -> Result<Option<Flat>, String> {
    let boxes = &graph.boxes;
    if boxes.get(root).is_none_or(|b| b.type_raw != 10) {
        return Ok(None);
    }
    let mut flat = Flat {
        multiplies: vec![],
        textures: vec![],
        colors: vec![],
    };
    let mut pending = vec![(root, 0usize)];
    let mut visits = 0usize;
    while let Some((index, depth)) = pending.pop() {
        visits += 1;
        if visits > MAX_VISITS || depth > MAX_DEPTH {
            return Err("Unsupported: product tree depth or visit budget".into());
        }
        let node = &boxes[index];
        match node.type_raw {
            10 => {
                let mut incoming: Vec<_> = graph
                    .wires
                    .iter()
                    .filter(|w| w.box_to as usize == index)
                    .collect();
                incoming.sort_by_key(|w| w.port_to_raw);
                if !unanimated(node)
                    || incoming.len() != 2
                    || incoming[0].port_to_raw != 0
                    || incoming[1].port_to_raw != 1
                    || incoming
                        .iter()
                        .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
                {
                    return Ok(None);
                }
                if !flat.multiplies.contains(&index) {
                    flat.multiplies.push(index);
                }
                // Port 1 pushed first so port 0 is walked first (deterministic order).
                for w in incoming.iter().rev() {
                    pending.push((w.box_from as usize, depth + 1));
                }
            }
            1 => flat.textures.push(index),
            3 => {
                if !unanimated(node) || graph.wires.iter().any(|w| w.box_to as usize == index) {
                    return Err(
                        "Unsupported: product tree COLOR must be unanimated and unwired".into(),
                    );
                }
                flat.colors.push(index);
            }
            _ => return Ok(None),
        }
    }
    Ok(Some(flat))
}

/// Component-wise f32 product of the COLOR leaves, in ascending box index order
/// (with multiplicity), starting from 1. `diffuse_parity` recomputes it independently.
pub(crate) fn fold_colors(rows: impl IntoIterator<Item = [u32; 4]>) -> crate::DiffuseTint {
    let mut value = [1f32; 4];
    for row in rows {
        for (v, bits) in value.iter_mut().zip(row) {
            *v *= f32::from_bits(bits);
        }
    }
    crate::DiffuseTint {
        bits: value.map(f32::to_bits),
    }
}

fn same(a: crate::linked::SamplerSelection, b: crate::linked::SamplerSelection) -> bool {
    a.uv_channel == b.uv_channel && a.uv_transform == b.uv_transform
}

/// Fallback after the fixed routes refused `graph`. Returns `fixed_error` unchanged for
/// anything that is not a pure static MULTIPLY tree of TEX/COLOR leaves.
pub(crate) fn select(
    graph: &GraphMetadata,
    animated: bool,
    fixed_error: String,
) -> Result<crate::DiffuseSelection, String> {
    let boxes = &graph.boxes;
    let Some(output) = boxes.iter().position(|b| b.type_raw == 0) else {
        return Err(fixed_error);
    };
    let diffuse: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == output && w.port_to_raw == 0)
        .collect();
    if boxes.iter().filter(|b| b.type_raw == 0).count() != 1
        || diffuse.len() != 1
        || diffuse[0].port_from_raw != 0
        || diffuse[0].swizzle_raw != [0; 5]
        || !unanimated(&boxes[output])
    {
        return Err(fixed_error);
    }
    let root = diffuse[0].box_from as usize;
    let flat = match flatten(graph, root)? {
        Some(flat) => flat,
        None => return Err(fixed_error),
    };
    if flat.textures.is_empty() {
        return Err("Unsupported: product tree without texture leaves (COLOR-only product)".into());
    }
    if flat.textures.len() > MAX_TEXTURE_LEAVES {
        return Err("Unsupported: product tree with more than four texture leaves".into());
    }
    let mut colors = flat.colors.clone();
    colors.sort_unstable();
    let tint = fold_colors(colors.iter().map(|&c| {
        let p = &boxes[c].parameters_raw;
        [p[0], p[1], p[2], p[3]]
    }));
    tint.validate()?;
    let mut leaves = crate::linked::Leaves::new(animated);
    let mut sampled = vec![];
    for &index in &flat.textures {
        sampled.push(leaves.sample(graph, index)?);
    }
    let (first, [second, third, fourth]) = roles(&sampled)?;
    let distinct = |list: &[usize]| {
        let mut list = list.to_vec();
        list.sort_unstable();
        list.dedup();
        list.len()
    };
    let used_nodes = 1 + flat.multiplies.len() + distinct(&flat.textures) + distinct(&flat.colors);
    let used_wires = 1 + 2 * flat.multiplies.len();
    let selection = crate::DiffuseSelection {
        animation_base: leaves.base.primary(),
        animation_base_more: leaves.base.more(),
        product_tree: Some(ProductTree {
            root,
            multiply_nodes: flat.multiplies.len() as u8,
            texture_leaves: flat.textures.len() as u8,
            color_leaves: flat.colors.len() as u8,
        }),
        second_sample: second,
        third_sample: third,
        fourth_sample: fourth,
        output_index: output,
        sample_index: first.sample_index,
        slot: first.slot,
        uv_channel: first.uv_channel,
        uv_transform: first.uv_transform,
        tint,
        multiply_index: Some(root),
        ignored_nodes: boxes.len().saturating_sub(used_nodes),
        ignored_wires: graph.wires.len().saturating_sub(used_wires),
        ..Default::default()
    };
    if selection.third_sample.is_some() {
        // Same rule the loader applies (D65): fail at selection so census reasons match.
        crate::linked::relative_third(&selection, &[])?;
    }
    Ok(selection)
}

type Leaf = crate::linked::SamplerSelection;

fn trial(first: Leaf, second: Leaf, third: Leaf) -> crate::DiffuseSelection {
    crate::DiffuseSelection {
        second_sample: Some(second),
        third_sample: Some(third),
        sample_index: first.sample_index,
        slot: first.slot,
        uv_channel: first.uv_channel,
        uv_transform: first.uv_transform,
        ..Default::default()
    }
}

const PERMUTATIONS_3: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// Assign texture leaves (walk order) to the primary/second/third/fourth render roles.
/// Three leaves: exact second/third sharing first (a squared sampler always shares),
/// then any order whose third has a D65 scale/offset base on its channel. Four leaves:
/// the D60 chain pairing (first/fourth and second/third share exact coordinates).
fn roles(leaves: &[Leaf]) -> Result<(Leaf, [Option<Leaf>; 3]), String> {
    match *leaves {
        [a] => Ok((a, [None; 3])),
        [a, b] => Ok((a, [Some(b), None, None])),
        [_, _, _] => {
            let pick = |p: [usize; 3]| (leaves[p[0]], leaves[p[1]], leaves[p[2]]);
            let shared = PERMUTATIONS_3
                .iter()
                .map(|&p| pick(p))
                .find(|&(_, b, c)| same(b, c));
            // D65: without exact sharing, every leaf must be a nonzero scale/offset.
            let scale_offset = |s: &Leaf| {
                matches!(s.uv_transform, crate::SamplingTransform::ScaleOffset(t)
                    if t.values()[..2].iter().all(|v| *v != 0.))
            };
            if shared.is_none() && !leaves.iter().all(scale_offset) {
                return Err(
                    "Unsupported: independent third texture needs nonzero scale/offset samplers"
                        .into(),
                );
            }
            let chosen = shared.or_else(|| {
                PERMUTATIONS_3
                    .iter()
                    .map(|&p| pick(p))
                    .find(|&(a, b, c)| crate::linked::relative_third(&trial(a, b, c), &[]).is_ok())
            });
            let (a, b, c) = chosen.ok_or(
                "Unsupported: product tree third texture needs a scale/offset sample on its channel",
            )?;
            Ok((a, [Some(b), Some(c), None]))
        }
        [_, _, _, _] => {
            for p in PERMUTATIONS_3 {
                // Leaf 0 is always first or fourth in some valid pairing; try it as first.
                let [b, c, d] = p.map(|i| leaves[i + 1]);
                if same(leaves[0], d) && same(b, c) {
                    return Ok((leaves[0], [Some(b), Some(c), Some(d)]));
                }
            }
            Err(
                "Unsupported: product tree four textures need two exact coordinate-sharing pairs"
                    .into(),
            )
        }
        _ => Err("Unsupported: product tree texture leaf count".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, WireMetadata};
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
    fn sample(scale: f32, slot: u32, channel: u32) -> BoxMetadata {
        let mut n = node(1, [scale, scale, 0., 0.]);
        n.parameters_raw[4] = channel;
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
    const MUL: u32 = 10;
    #[test]
    fn d99_single_texture_under_nested_colors_folds_the_tint() {
        // Output; (T × C1) × C2: 1 sample, 2 C1, 3 inner, 4 C2, 5 root.
        let c1 = [0.5f32, 0.25, 1., 1.];
        let c2 = [0.5f32, 1., 0.75, 0.5];
        let mut g = graph(
            vec![
                node(0, [0.; 4]),
                sample(3., 0, 0),
                node(3, c1),
                node(MUL, [0.; 4]),
                node(3, c2),
                node(MUL, [0.; 4]),
            ],
            vec![
                wire(1, 3, 0),
                wire(2, 3, 1),
                wire(3, 5, 0),
                wire(4, 5, 1),
                wire(5, 0, 0),
            ],
        );
        let s = crate::select_diffuse(&g).unwrap();
        let tree = s.product_tree.unwrap();
        assert_eq!(
            (
                tree.root,
                tree.multiply_nodes,
                tree.texture_leaves,
                tree.color_leaves
            ),
            (5, 2, 1, 2)
        );
        assert_eq!(s.tint.values(), [0.25, 0.25, 0.75, 0.5]);
        assert_eq!(
            (s.sample_index, s.multiply_index, s.color_index),
            (1, Some(5), None)
        );
        assert!(s.second_sample.is_none() && s.third_sample.is_none());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        // Mirrored nesting C2 × (C1 × T) folds to the same tint.
        for w in &mut g.wires {
            if w.box_to != 0 {
                w.port_to_raw = 1 - w.port_to_raw;
            }
        }
        assert_eq!(crate::select_diffuse(&g).unwrap().tint, s.tint);
        // The same COLOR on both inner ports counts twice (DAG multiplicity).
        let mut dag = g.clone();
        dag.wires[0].box_from = 2;
        dag.wires[3].box_from = 1;
        let d = crate::select_diffuse(&dag).unwrap();
        assert_eq!(d.tint.values(), [0.25, 0.0625, 1., 1.]);
        assert_eq!(d.product_tree.unwrap().color_leaves, 2);
        // A folded value outside 0..1 is refused with the tint reason.
        g.boxes[2].parameters_raw[0] = 4f32.to_bits();
        assert!(crate::select_diffuse(&g).unwrap_err().contains("0..1"));
        g.boxes[2].parameters_raw[0] = 0.5f32.to_bits();
        // Animated COLOR leaves are refused explicitly.
        g.boxes[4].animations[0].name_hex = "653100".into();
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("COLOR must be unanimated"));
        g.boxes[4].animations[0].name_hex = "000000".into();
        // COLOR-only products and non-tree inputs keep explicit reasons.
        let mut colors = g.clone();
        colors.boxes[1] = node(3, c1);
        assert!(crate::select_diffuse(&colors)
            .unwrap_err()
            .contains("COLOR-only"));
        let mut add = g.clone();
        add.boxes[3].type_raw = 11;
        let fixed = crate::select_diffuse(&add).unwrap_err();
        assert!(!fixed.contains("product tree"), "{fixed}");
        let mut swizzled = g.clone();
        swizzled.wires[0].swizzle_raw[0] = b'r';
        assert!(!crate::select_diffuse(&swizzled)
            .unwrap_err()
            .contains("product tree"));
    }
    #[test]
    fn d99_two_and_three_texture_trees_take_existing_roles() {
        // (T1 × C1) × (T2 × C2): 1/2 samples, 3/4 colors, 5/6 inner, 7 root.
        let mut g = graph(
            vec![
                node(0, [0.; 4]),
                sample(1., 0, 0),
                sample(0.3, 1, 0),
                node(3, [0.5, 0.5, 0.5, 1.]),
                node(3, [0.5, 1., 0.5, 1.]),
                node(MUL, [0.; 4]),
                node(MUL, [0.; 4]),
                node(MUL, [0.; 4]),
            ],
            vec![
                wire(1, 5, 0),
                wire(3, 5, 1),
                wire(2, 6, 0),
                wire(4, 6, 1),
                wire(5, 7, 0),
                wire(6, 7, 1),
                wire(7, 0, 0),
            ],
        );
        let s = crate::select_diffuse(&g).unwrap();
        assert_eq!(
            (s.sample_index, s.second_sample.unwrap().sample_index),
            (1, 2)
        );
        assert_eq!(s.tint.values(), [0.25, 0.5, 0.25, 1.]);
        assert!(!crate::linked::uses_grid(s)); // 0.3 scale: dual path
                                               // Three leaves: C2 becomes a third sample sharing T1's coordinates.
        g.boxes[4] = sample(1., 2, 0);
        let three = crate::select_diffuse(&g).unwrap();
        let (second, third) = (three.second_sample.unwrap(), three.third_sample.unwrap());
        assert_eq!(
            (three.sample_index, second.sample_index, third.sample_index),
            (2, 1, 4)
        );
        assert!(crate::linked::relative_third(&three, &[])
            .unwrap()
            .is_identity());
        assert_eq!(three.tint.values(), [0.5, 0.5, 0.5, 1.]);
        assert_eq!(three.product_tree.unwrap().texture_leaves, 3);
        // No exact sharing: D65 needs nonzero scale/offset for every leaf.
        g.boxes[4] = sample(2., 2, 0);
        assert!(crate::select_diffuse(&g).unwrap().third_sample.is_some());
        g.boxes[4].parameters_raw[0] = 0;
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("nonzero scale/offset"));
    }
    #[test]
    fn d99_four_texture_trees_need_two_sharing_pairs() {
        // Balanced (T1 × T2) × (T3 × T4): 1..4 samples, 5/6 inner, 7 root.
        let mut g = graph(
            vec![
                node(0, [0.; 4]),
                sample(1., 0, 1),
                sample(3., 1, 0),
                sample(1., 2, 1),
                sample(3., 3, 0),
                node(MUL, [0.; 4]),
                node(MUL, [0.; 4]),
                node(MUL, [0.; 4]),
            ],
            vec![
                wire(1, 5, 0),
                wire(2, 5, 1),
                wire(3, 6, 0),
                wire(4, 6, 1),
                wire(5, 7, 0),
                wire(6, 7, 1),
                wire(7, 0, 0),
            ],
        );
        let s = crate::select_diffuse(&g).unwrap();
        let [second, third, fourth] =
            [s.second_sample, s.third_sample, s.fourth_sample].map(Option::unwrap);
        assert_eq!(
            (
                s.sample_index,
                second.sample_index,
                third.sample_index,
                fourth.sample_index
            ),
            (1, 2, 4, 3)
        );
        assert_eq!(s.tint, crate::DiffuseTint::default());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        g.boxes[4].parameters_raw[0] = 2f32.to_bits();
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("two exact coordinate-sharing pairs"));
        // Five texture leaves exceed the render roles.
        g.boxes[4].parameters_raw[0] = 3f32.to_bits();
        g.boxes.extend([sample(1., 4, 0), node(MUL, [0.; 4])]);
        g.wires.pop();
        g.wires
            .extend([wire(7, 9, 0), wire(8, 9, 1), wire(9, 0, 0)]);
        assert!(crate::select_diffuse(&g)
            .unwrap_err()
            .contains("more than four"));
    }
    #[test]
    fn d99_fold_is_an_ascending_f32_product() {
        let rows = [[0.1f32, 1., 0.5, 2.], [0.3, 0.5, 0.5, 0.25]].map(|r| r.map(f32::to_bits));
        let t = fold_colors(rows);
        assert_eq!(t.values(), [1f32 * 0.1 * 0.3, 0.5, 0.25, 0.5]);
        assert_eq!(fold_colors([]), crate::DiffuseTint::default());
    }
}
