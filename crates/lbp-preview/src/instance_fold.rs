//! D86: under the supplied-instance-color opt-in only, fold a diffuse root MULTIPLY or
//! MIX whose two inputs are all constants (projected ThingColor, static COLOR, CONSTANT,
//! optionally one-component swizzled) into one derived COLOR for the solid route.
//! Operation forms, evidence labels, pin and limits: docs/d86-thing-color.md.
//! D88 applies the same fold to one direct operand of a texture product root.
use lbp_resource::material::GraphMetadata;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Operand {
    Scalar(f32),
    Vector([f32; 4]),
}

fn is_static(node: &lbp_resource::material::BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|v| v == b'0'))
}

/// One component name (`x y z w` or `r g b a`) followed by NULs; anything else is None.
fn single_component(swizzle: [u8; 5]) -> Option<Option<usize>> {
    if swizzle == [0; 5] {
        return Some(None);
    }
    if swizzle[1..] != [0; 4] {
        return None;
    }
    match swizzle[0] {
        b'x' | b'r' => Some(Some(0)),
        b'y' | b'g' => Some(Some(1)),
        b'z' | b'b' => Some(Some(2)),
        b'w' | b'a' => Some(Some(3)),
        _ => None,
    }
}

fn operand(
    graph: &GraphMetadata,
    wire: &lbp_resource::material::WireMetadata,
    color_nodes: &[usize],
) -> Option<(Operand, Option<u8>)> {
    let index = wire.box_from as usize;
    let node = &graph.boxes[index];
    if wire.port_from_raw != 0
        || !is_static(node)
        || graph.wires.iter().any(|w| w.box_to as usize == index)
    {
        return None;
    }
    let value = match node.type_raw {
        // Projected ThingColor nodes already carry the supplied bits as a COLOR.
        3 => Operand::Vector(std::array::from_fn(|i| {
            f32::from_bits(node.parameters_raw[i])
        })),
        4 => Operand::Scalar(f32::from_bits(node.parameters_raw[0])),
        _ => return None,
    };
    let component = single_component(wire.swizzle_raw)?;
    let value = match (value, component) {
        (v, None) => v,
        (Operand::Vector(v), Some(c)) => Operand::Scalar(v[c]),
        // Swizzling a scalar is not observed; keep it unsupported.
        (Operand::Scalar(_), Some(_)) => return None,
    };
    let thing_component = component
        .filter(|_| color_nodes.contains(&index))
        .map(|_| wire.swizzle_raw[0]);
    Some((value, thing_component))
}

fn broadcast(value: Operand) -> [f32; 4] {
    match value {
        Operand::Scalar(s) => [s; 4],
        Operand::Vector(v) => v,
    }
}

/// Returns `Ok(true)` when the root was folded. A non-matching shape is left unchanged
/// (existing selectors decide); a matching shape outside 0..1 is an explicit error.
pub(crate) fn fold_constant_root(
    derived: &mut GraphMetadata,
    root: usize,
    projection: &mut crate::InstanceProjection,
) -> Result<bool, String> {
    let Some(swizzled) = fold_constant_node(derived, root, &projection.color_nodes)? else {
        return Ok(false);
    };
    projection.folded_root = Some(root);
    projection.swizzled_color_edges = swizzled;
    Ok(true)
}

/// D88 (docs/d88-thing-color-textured.md): one direct operand of a static diffuse root
/// MULTIPLY that is itself a foldable constant MULTIPLY/MIX holding a projected
/// ThingColor, e.g. `texture × (ThingColor × COLOR)`. The operand must feed only the
/// root on port 0 or 1, unswizzled, so no other branch sees the derived COLOR. The
/// existing texture × COLOR selectors then decide; nothing else is evaluated.
pub(crate) fn fold_constant_operand(
    derived: &mut GraphMetadata,
    root: usize,
    projection: &mut crate::InstanceProjection,
) -> Result<bool, String> {
    let node = &derived.boxes[root];
    if node.type_raw != 10 || !is_static(node) {
        return Ok(false);
    }
    let inputs: Vec<_> = derived
        .wires
        .iter()
        .filter(|w| w.box_to as usize == root)
        .collect();
    let mut ports: Vec<_> = inputs.iter().map(|w| w.port_to_raw).collect();
    ports.sort_unstable();
    if ports != [0, 1]
        || inputs
            .iter()
            .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
    {
        return Ok(false);
    }
    let candidates: Vec<usize> = inputs
        .iter()
        .map(|w| w.box_from as usize)
        .filter(|&i| {
            i != root
                && matches!(derived.boxes[i].type_raw, 10 | 12)
                && derived
                    .wires
                    .iter()
                    .filter(|w| w.box_from as usize == i)
                    .count()
                    == 1
        })
        .collect();
    for operand in candidates {
        if let Some(swizzled) = fold_constant_node(derived, operand, &projection.color_nodes)? {
            projection.folded_operand = Some(operand);
            projection.swizzled_color_edges = swizzled;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Folds `root` in place into a derived COLOR when it is a static two-input
/// MULTIPLY/MIX of constants with at least one projected ThingColor. Returns the
/// swizzled ThingColor edges, or None when the shape does not match.
fn fold_constant_node(
    derived: &mut GraphMetadata,
    root: usize,
    color_nodes: &[usize],
) -> Result<Option<Vec<(usize, u8)>>, String> {
    let node = &derived.boxes[root];
    if !(node.type_raw == 10 || node.type_raw == 12) || !is_static(node) {
        return Ok(None);
    }
    let inputs: Vec<_> = derived
        .wires
        .iter()
        .filter(|w| w.box_to as usize == root)
        .collect();
    let mut ports: Vec<_> = inputs.iter().map(|w| w.port_to_raw).collect();
    ports.sort_unstable();
    // Both operands wired once; a wired MIX weight (port 2) is not folded.
    if ports != [0, 1] {
        return Ok(None);
    }
    if !inputs
        .iter()
        .any(|w| color_nodes.contains(&(w.box_from as usize)))
    {
        return Ok(None);
    }
    let mut operands = [None, None];
    let mut swizzled = vec![];
    for wire in &inputs {
        let Some((value, thing)) = operand(derived, wire, color_nodes) else {
            return Ok(None);
        };
        operands[wire.port_to_raw as usize] = Some(value);
        if let Some(component) = thing {
            swizzled.push((wire.box_from as usize, component));
        }
    }
    let [Some(l), Some(r)] = operands else {
        return Ok(None);
    };
    // A scalar result has no observed widening rule to a color; keep it unsupported.
    if matches!((l, r), (Operand::Scalar(_), Operand::Scalar(_))) {
        return Ok(None);
    }
    let (l, r) = (broadcast(l), broadcast(r));
    let weight = f32::from_bits(node.parameters_raw[0]);
    let value: [f32; 4] = std::array::from_fn(|i| {
        if node.type_raw == 10 {
            l[i] * r[i]
        } else {
            (r[i] - l[i]) * weight + l[i]
        }
    });
    let color = crate::DiffuseTint {
        bits: value.map(f32::to_bits),
    };
    color.validate().map_err(|_| {
        "Unsupported: folded supplied instance color outside finite 0..1 preview bounds".to_string()
    })?;
    let folded = &mut derived.boxes[root];
    folded.type_raw = 3;
    folded.parameters_raw = [0; 8];
    folded.parameters_raw[..4].copy_from_slice(&color.bits);
    derived.wires.retain(|w| w.box_to as usize != root);
    swizzled.sort_unstable();
    swizzled.dedup();
    Ok(Some(swizzled))
}

#[cfg(test)]
mod tests {
    use super::single_component;
    #[test]
    fn swizzle_accepts_one_named_component_only() {
        assert_eq!(single_component([0; 5]), Some(None));
        assert_eq!(single_component([b'a', 0, 0, 0, 0]), Some(Some(3)));
        assert_eq!(single_component([b'w', 0, 0, 0, 0]), Some(Some(3)));
        assert_eq!(single_component([b'r', 0, 0, 0, 0]), Some(Some(0)));
        assert_eq!(single_component([b'x', b'y', 0, 0, 0]), None);
        assert_eq!(single_component([b'a', b'x', 0, 0, 0]), None);
        assert_eq!(single_component([b'q', 0, 0, 0, 0]), None);
        assert_eq!(single_component([b'a', 0, 0, 0, b'x']), None);
        assert_eq!(single_component([0, b'a', 0, 0, 0]), None);
    }
}
