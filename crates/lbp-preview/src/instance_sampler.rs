//! D96: under the supplied-instance-color opt-in only, fold static constant inputs on
//! TEXTURE_SAMPLE ports 0 (UV add), 1 (UV multiply) and 2 (UV subtract) into the
//! sampler's own scale/offset on the derived graph, so the existing sampler routes see
//! an unwired static sampler. Inputs are projected ThingColor (now a COLOR), COLOR,
//! CONSTANT..CONSTANT4, MULTIPLY, MIX and MAKE_FLOAT2 of those, evaluated once.
//! Rule statements, Toolkit pin, evidence labels and limits: docs/d96-thing-sampler-ports.md.
use lbp_resource::material::{BoxMetadata, GraphMetadata, WireMetadata};

/// One sampler whose port inputs were folded into its scale/offset (derived graph only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplerPortFold {
    pub sample_index: usize,
    /// Sampler scale X/Y, offset X/Y bits before folding.
    pub source_bits: [u32; 4],
    /// Folded scale X/Y, offset X/Y bits written to the derived sampler.
    pub folded_bits: [u32; 4],
    /// Evaluated (x, y) bits of ports 0 (add), 1 (multiply), 2 (subtract); None = unwired.
    pub port_bits: [Option<[u32; 2]>; 3],
}

/// Evaluated constant: width 1 (scalar, broadcast) to 4.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Value {
    width: usize,
    v: [f32; 4],
}

impl Value {
    fn get(self, i: usize) -> f32 {
        if self.width == 1 {
            self.v[0]
        } else {
            self.v[i]
        }
    }
    /// The UV operand: a scalar broadcasts, a vector contributes its first two lanes.
    fn uv(self) -> Option<[f32; 2]> {
        let out = [self.get(0), self.get(1)];
        (self.width != 0 && out.iter().all(|v| v.is_finite())).then_some(out)
    }
}

const MAX_DEPTH: usize = 8;
const MAX_VISITS: usize = 64;

fn unanimated(node: &BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
}

fn into(graph: &GraphMetadata, index: usize) -> Vec<&WireMetadata> {
    graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == index)
        .collect()
}

fn swizzle(value: Value, raw: [u8; 5]) -> Result<Value, String> {
    if raw == [0; 5] {
        return Ok(value);
    }
    let names: Vec<u8> = raw.iter().copied().take_while(|&c| c != 0).collect();
    if names.len() > 4 || raw[names.len()..].iter().any(|&c| c != 0) || value.width == 1 {
        return Err("Unsupported: sampler port input swizzle".into());
    }
    let mut out = Value {
        width: names.len(),
        v: [0.; 4],
    };
    for (slot, name) in names.iter().enumerate() {
        let lane = match name {
            b'x' | b'r' => 0,
            b'y' | b'g' => 1,
            b'z' | b'b' => 2,
            b'w' | b'a' => 3,
            _ => return Err("Unsupported: sampler port input swizzle".into()),
        };
        if lane >= value.width {
            return Err("Unsupported: sampler port input swizzle".into());
        }
        out.v[slot] = value.v[lane];
    }
    Ok(out)
}

fn params(node: &BoxMetadata, width: usize) -> Value {
    Value {
        width,
        v: std::array::from_fn(|i| f32::from_bits(node.parameters_raw[i])),
    }
}

fn binary(l: Value, r: Value, f: impl Fn(f32, f32) -> f32) -> Value {
    let width = l.width.max(r.width);
    Value {
        width,
        v: std::array::from_fn(|i| if i < width { f(l.get(i), r.get(i)) } else { 0. }),
    }
}

/// Value carried by `wire` (source port 0, swizzle applied), static constants only.
fn wire_value(
    graph: &GraphMetadata,
    wire: &WireMetadata,
    depth: usize,
    visits: &mut usize,
) -> Result<Value, String> {
    *visits += 1;
    if depth > MAX_DEPTH || *visits > MAX_VISITS {
        return Err("Unsupported: sampler port input depth budget".into());
    }
    if wire.port_from_raw != 0 {
        return Err("Unsupported: sampler port input from a non-zero source port".into());
    }
    let index = wire.box_from as usize;
    let node = &graph.boxes[index];
    if !unanimated(node) {
        return Err(
            "Unsupported: sampler port input must be static, unanimated and subtype zero".into(),
        );
    }
    let incoming = into(graph, index);
    let mut port = |p: u8| -> Result<Option<Value>, String> {
        let wires: Vec<_> = incoming.iter().filter(|w| w.port_to_raw == p).collect();
        match wires[..] {
            [] => Ok(None),
            [w] => wire_value(graph, w, depth + 1, visits).map(Some),
            _ => Err("Unsupported: sampler port input with a doubly wired port".into()),
        }
    };
    let leaf = |width| {
        if incoming.is_empty() {
            Ok(params(node, width))
        } else {
            Err("Unsupported: sampler port constant must be unwired".to_string())
        }
    };
    let value = match node.type_raw {
        // COLOR, including each projected ThingColor (already the supplied bits).
        3 => leaf(4)?,
        4 => leaf(1)?,
        5 => leaf(2)?,
        6 => leaf(3)?,
        7 => leaf(4)?,
        10 | 12 => {
            if incoming.iter().any(|w| w.port_to_raw > 2)
                || (node.type_raw == 10 && incoming.iter().any(|w| w.port_to_raw == 2))
            {
                return Err(
                    "Unsupported: sampler port MULTIPLY/MIX input on an unknown port".into(),
                );
            }
            let (Some(l), Some(r)) = (port(0)?, port(1)?) else {
                return Err("Unsupported: sampler port MULTIPLY/MIX needs both operands".into());
            };
            if node.type_raw == 10 {
                binary(l, r, |a, b| a * b)
            } else {
                // MIX: (r - l) × weight + l; weight from port 2 or parameter word 0.
                let w = port(2)?.unwrap_or(params(node, 1));
                let d = binary(r, l, |a, b| a - b);
                let p = binary(d, w, |a, b| a * b);
                binary(p, l, |a, b| a + b)
            }
        }
        13 => {
            if incoming.iter().any(|w| w.port_to_raw > 1) {
                return Err(
                    "Unsupported: sampler port MAKE_FLOAT2 input on an unknown port".into(),
                );
            }
            let mut out = params(node, 2);
            for lane in 0..2u8 {
                if let Some(v) = port(lane)? {
                    // Vector inputs are reduced (average/luminance) in Toolkit; not folded.
                    if v.width != 1 {
                        return Err(
                            "Unsupported: sampler port MAKE_FLOAT2 needs scalar inputs".into()
                        );
                    }
                    out.v[lane as usize] = v.v[0];
                }
            }
            out.v[2..].fill(0.);
            out
        }
        other => {
            return Err(format!(
                "Unsupported: sampler port input from box type {other} is not a static constant"
            ))
        }
    };
    let value = swizzle(value, wire.swizzle_raw)?;
    if value.v[..value.width].iter().any(|v| !v.is_finite()) {
        return Err("Unsupported: sampler port input must be finite".into());
    }
    Ok(value)
}

/// Folds every input of `sample` on the derived graph. `Ok(None)` when the sampler has
/// no inputs; an error names the first unsupported input.
pub(crate) fn fold_sampler_ports(
    derived: &mut GraphMetadata,
    sample: usize,
) -> Result<Option<SamplerPortFold>, String> {
    let incoming = into(derived, sample);
    if incoming.is_empty() {
        return Ok(None);
    }
    let node = &derived.boxes[sample];
    // Toolkit only applies sampler ports on a subtype-0 sampler without named animation.
    if !unanimated(node) || crate::sampler_uv_channel(node.parameters_raw[4]).is_none() {
        return Err(
            "Unsupported: sampler port inputs need a static subtype-0 UV0/UV1 sampler".into(),
        );
    }
    let mut ports: [Option<[f32; 2]>; 3] = [None; 3];
    let mut visits = 0;
    for wire in &incoming {
        let p = wire.port_to_raw as usize;
        if p > 2 || ports[p].is_some() {
            return Err("Unsupported: sampler input outside ports 0..2 or doubly wired".into());
        }
        if wire.box_from as usize == sample {
            return Err("Unsupported: sampler port input cycle".into());
        }
        let value = wire_value(derived, wire, 0, &mut visits)?;
        ports[p] = Some(
            value
                .uv()
                .ok_or("Unsupported: sampler port input must be finite")?,
        );
    }
    let p = &node.parameters_raw;
    let source_bits = [p[0], p[1], p[2], p[3]];
    let [sx, sy, ox, oy] = source_bits.map(f32::from_bits);
    let (mut scale, mut offset) = ([sx, sy], [ox, oy]);
    // ((uv × s + o) + a) × m − b, regrouped as uv × (s × m) + ((o + a) × m − b);
    // each operation applied only when its port is wired.
    for axis in 0..2 {
        if let Some(a) = ports[0] {
            offset[axis] += a[axis];
        }
        if let Some(m) = ports[1] {
            scale[axis] *= m[axis];
            offset[axis] *= m[axis];
        }
        if let Some(b) = ports[2] {
            offset[axis] -= b[axis];
        }
    }
    let folded = crate::UvTransform {
        bits: [scale[0], scale[1], offset[0], offset[1]].map(f32::to_bits),
    };
    folded.validate()?;
    if scale.contains(&0.) {
        return Err("Unsupported: folded sampler UV scale is zero".into());
    }
    let node = &mut derived.boxes[sample];
    node.parameters_raw[..4].copy_from_slice(&folded.bits);
    derived.wires.retain(|w| w.box_to as usize != sample);
    Ok(Some(SamplerPortFold {
        sample_index: sample,
        source_bits,
        folded_bits: folded.bits,
        port_bits: ports.map(|p| p.map(|v| v.map(f32::to_bits))),
    }))
}

#[cfg(test)]
mod tests {
    use super::swizzle;
    use super::Value;
    #[test]
    fn swizzle_selects_named_lanes_and_rejects_the_rest() {
        let v = Value {
            width: 4,
            v: [1., 2., 3., 4.],
        };
        assert_eq!(swizzle(v, [b'a', 0, 0, 0, 0]).unwrap().v[0], 4.);
        assert_eq!(swizzle(v, [b'a', 0, 0, 0, 0]).unwrap().width, 1);
        assert_eq!(swizzle(v, [b'y', b'x', 0, 0, 0]).unwrap().v[..2], [2., 1.]);
        assert!(swizzle(v, [b'q', 0, 0, 0, 0]).is_err());
        assert!(swizzle(v, [0, b'a', 0, 0, 0]).is_err());
        let s = Value {
            width: 1,
            v: [5., 0., 0., 0.],
        };
        assert!(swizzle(s, [b'x', 0, 0, 0, 0]).is_err());
        let two = Value {
            width: 2,
            v: [1., 2., 0., 0.],
        };
        assert!(swizzle(two, [b'z', 0, 0, 0, 0]).is_err());
    }
}
