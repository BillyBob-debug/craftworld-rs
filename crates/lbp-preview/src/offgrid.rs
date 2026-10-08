//! D103: off-grid bakes for the D100 graph evaluator (docs/d103-offgrid-bake.md).
//! Node semantics are the D100 evaluator's (Toolkit `GfxAssembler.resolve`, restated in
//! the D-docs). The resampling here is authored *preview policy*, not a claim about game
//! behaviour, and every selection carrying it is labelled as an approximation:
//!
//! - **Resampled**: all texture sources on one UV channel but off the integer Repeat
//!   grid (non-integer scales, offsets, Mirror). Each leaf is sampled bilinearly in
//!   linear light (box-averaged up to 4×4 when minified) at the texel centres of a
//!   common periodic UV tile `[Pu, Pv]` (smallest integers ≤ 16 that make every leaf
//!   repeat), the graph is evaluated per bake texel, and the bake is shown on
//!   `uv / P` with Repeat and linear filtering.
//! - **Split**: sources on UV0 and UV1 whose graph is a separable product (MULTIPLY
//!   tree whose factors each depend on one channel or none). Each channel group's
//!   product is resampled as above and the two bakes are multiplied per pixel by the
//!   existing Bevy dual path (linear values). Each factor is clamped to 0..1 at 8-bit
//!   encoding before the product; the legacy viewer keeps its dual refusal.
use crate::grid_graph::{GraphOp, GraphProgram, LoadContext, Source, Step};

/// Largest tile period (UV units) per axis.
const MAX_PERIOD: u32 = 16;
/// Bake extent per axis and the total pixel budget (same as the D100 grid).
const MAX_AXIS: u64 = 4096;
/// Box-filter samples per axis per leaf when the bake minifies a leaf.
const MAX_SUBSAMPLES: u64 = 4;
/// Bound on |scale| and |offset| for the bake (keeps texel positions small integers).
const MAX_TRANSFORM: f64 = 4096.;
/// Integer test tolerance for `period × scale`.
const PERIOD_EPSILON: f64 = 1e-4;
/// Factor visits when flattening the root product.
const MAX_FACTORS: usize = 64;

/// Selection marker for an off-grid evaluated graph; every variant is a labelled preview
/// approximation (bilinear resampling on a common tile), not game behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffGrid {
    /// One UV channel; all leaves resampled on a common `period` UV tile.
    Resampled { channel: u8, period: [u8; 2] },
    /// Separable product of a UV-channel group (primary) and the other channel's group
    /// (Bevy dual multiply), each resampled on its own tile.
    Split {
        channel: u8,
        period: [u8; 2],
        second_channel: u8,
        second_period: [u8; 2],
    },
}
impl OffGrid {
    pub fn label(self) -> String {
        match self {
            Self::Resampled { channel, period } => format!(
                "preview approximation (not game behaviour): off-grid UV{channel} leaves \
                 resampled bilinear in linear light on a common {period:?} UV tile"
            ),
            Self::Split {
                channel,
                period,
                second_channel,
                second_period,
            } => format!(
                "preview approximation (not game behaviour): separable product split by UV \
                 channel, UV{channel} factors on a {period:?} tile × UV{second_channel} \
                 factors on a {second_period:?} tile (bilinear, linear light; each factor \
                 clamped to 0..1 before the dual multiply)"
            ),
        }
    }
}

pub(crate) struct Plan {
    pub(crate) kind: OffGrid,
    /// Split only: box index of the first sample on the second channel.
    pub(crate) second_sample: Option<usize>,
}

/// One baked texture: the product of `factors` (step positions with optional lanes),
/// evaluating only `needed` steps, over the sources on `channel`.
#[derive(Debug)]
struct Target {
    channel: usize,
    factors: Vec<(usize, Option<u8>)>,
    needed: Vec<bool>,
    sources: Vec<usize>,
}

fn scale_offset(source: &Source) -> Result<[f64; 4], String> {
    let crate::SamplingTransform::ScaleOffset(t) = source.transform else {
        return Err(
            "Unsupported: graph evaluator off-grid bake needs scale/offset samplers".into(),
        );
    };
    t.validate()?;
    let v = t.values().map(f64::from);
    if v.iter().any(|x| x.abs() > MAX_TRANSFORM) {
        return Err("Unsupported: graph evaluator off-grid transform budget".into());
    }
    Ok(v)
}

/// Smallest tile period `P` (1..=16) per axis with `P × |scale| / repeat` integral for
/// every source (`repeat` = 1 for Repeat, 2 for Mirror per axis).
fn period(transforms: &[([f64; 4], [u32; 2])]) -> Result<[u32; 2], String> {
    let axis = |a: usize| {
        (1..=MAX_PERIOD).find(|&p| {
            transforms.iter().all(|(t, repeat)| {
                let cycles = f64::from(p) * t[a].abs() / f64::from(repeat[a]);
                (cycles - cycles.round()).abs() <= PERIOD_EPSILON
            })
        })
    };
    match (axis(0), axis(1)) {
        (Some(u), Some(v)) => Ok([u, v]),
        _ => Err(
            "Unsupported: graph evaluator off-grid scales have no common period within 16 repeats"
                .into(),
        ),
    }
}

/// Bake targets in display order (primary channel first). `repeat` gives each source's
/// per-axis repeat length (1 Repeat, 2 Mirror).
fn targets(
    program: &GraphProgram,
    sources: &[Source],
    primary_channel: usize,
) -> Result<(Vec<Target>, Option<usize>), String> {
    for s in sources {
        scale_offset(s)?;
    }
    let position = |index: usize| {
        program
            .nodes
            .iter()
            .position(|n| n.index == index)
            .ok_or_else(|| "Invalid: graph evaluator input".to_string())
    };
    // Channel bit mask per step (bit c = depends on UV channel c).
    let mut masks: Vec<u8> = Vec::with_capacity(program.nodes.len());
    for node in &program.nodes {
        let inputs: Vec<crate::GraphInput> = match node.op {
            GraphOp::Sample { uv_channel, .. } => {
                if uv_channel > 1 {
                    return Err("Unsupported: graph evaluator off-grid UV channel".into());
                }
                masks.push(1 << uv_channel);
                continue;
            }
            GraphOp::Color(_) | GraphOp::Constant(_) => vec![],
            GraphOp::Multiply(i) | GraphOp::Add(i) => i.to_vec(),
            GraphOp::Mix { inputs, factor } => {
                let mut v = inputs.to_vec();
                if let crate::MixFactor::Input(f) = factor {
                    v.push(f);
                }
                v
            }
            GraphOp::Blend { inputs, .. } => inputs.to_vec(),
        };
        let mut mask = 0;
        for i in inputs {
            mask |= masks[position(i.node)?];
        }
        masks.push(mask);
    }
    let root = program
        .nodes
        .len()
        .checked_sub(1)
        .ok_or("Invalid: graph evaluator root")?;
    let mut groups: [Vec<(usize, Option<u8>)>; 2] = [vec![], vec![]];
    let mut constants = vec![];
    if masks[root] == 1 << primary_channel {
        groups[primary_channel].push((root, None));
    } else if masks[root] == 3 {
        // Flatten the root product; every factor must depend on one channel or none.
        let mut pending = vec![(root, None::<u8>)];
        let mut visits = 0;
        while let Some((p, lane)) = pending.pop() {
            visits += 1;
            if visits > MAX_FACTORS {
                return Err("Unsupported: graph evaluator off-grid factor budget".into());
            }
            if let GraphOp::Multiply([a, b]) = program.nodes[p].op {
                // (a × b).lane = a'.lane × b'.lane; an input swizzle takes precedence.
                for i in [b, a] {
                    pending.push((position(i.node)?, i.lane.or(lane)));
                }
                continue;
            }
            match masks[p] {
                0 => constants.push((p, lane)),
                1 => groups[0].push((p, lane)),
                2 => groups[1].push((p, lane)),
                _ => {
                    return Err(
                        "Unsupported: graph evaluator off-grid samplers mix UV channels \
                                outside a separable product"
                            .into(),
                    )
                }
            }
        }
    } else {
        return Err("Invalid: graph evaluator primary channel".into());
    }
    groups[primary_channel].extend(constants);
    let order = [primary_channel, 1 - primary_channel];
    let mut result = vec![];
    for channel in order {
        let factors = std::mem::take(&mut groups[channel]);
        if factors.is_empty() {
            continue;
        }
        let mut needed = vec![false; program.nodes.len()];
        for &(p, _) in &factors {
            needed[p] = true;
        }
        for k in (0..program.nodes.len()).rev() {
            if !needed[k] {
                continue;
            }
            let inputs: Vec<crate::GraphInput> = match program.nodes[k].op {
                GraphOp::Multiply(i) | GraphOp::Add(i) => i.to_vec(),
                GraphOp::Mix { inputs, factor } => {
                    let mut v = inputs.to_vec();
                    if let crate::MixFactor::Input(f) = factor {
                        v.push(f);
                    }
                    v
                }
                GraphOp::Blend { inputs, .. } => inputs.to_vec(),
                _ => vec![],
            };
            for i in inputs {
                needed[position(i.node)?] = true;
            }
        }
        let sources = (0..sources.len())
            .filter(|&i| sources[i].uv_channel == channel)
            .collect();
        result.push(Target {
            channel,
            factors,
            needed,
            sources,
        });
    }
    let second = match result.get(1) {
        Some(t) => Some(
            program
                .nodes
                .iter()
                .find(|n| matches!(n.op, GraphOp::Sample { uv_channel, .. } if uv_channel == t.channel))
                .ok_or("Invalid: graph evaluator second channel")?
                .index,
        ),
        None => None,
    };
    if result
        .first()
        .is_none_or(|t| t.channel != primary_channel || t.sources.is_empty())
    {
        return Err("Invalid: graph evaluator primary channel".into());
    }
    Ok((result, second))
}

fn target_period(
    target: &Target,
    sources: &[Source],
    repeat: &[[u32; 2]],
) -> Result<[u32; 2], String> {
    let transforms = target
        .sources
        .iter()
        .map(|&i| Ok((scale_offset(&sources[i])?, repeat[i])))
        .collect::<Result<Vec<_>, String>>()?;
    period(&transforms)
}

fn narrow(p: [u32; 2]) -> [u8; 2] {
    p.map(|v| v as u8)
}

/// Selection-time plan, assuming Repeat on every texture (checked again at load).
pub(crate) fn plan(
    program: &GraphProgram,
    sources: &[Source],
    primary_channel: usize,
) -> Result<Plan, String> {
    let (targets, second_sample) = targets(program, sources, primary_channel)?;
    let repeat = vec![[1, 1]; sources.len()];
    let first = target_period(&targets[0], sources, &repeat)?;
    let kind = match targets.get(1) {
        None => OffGrid::Resampled {
            channel: primary_channel as u8,
            period: narrow(first),
        },
        Some(t) => OffGrid::Split {
            channel: primary_channel as u8,
            period: narrow(first),
            second_channel: t.channel as u8,
            second_period: narrow(target_period(t, sources, &repeat)?),
        },
    };
    Ok(Plan {
        kind,
        second_sample,
    })
}

/// One decoded leaf texture with its transform and per-axis Mirror flag.
struct Leaf<'a> {
    extent: [u32; 2],
    rgba: &'a [u8],
    transform: [f64; 4],
    mirror: [bool; 2],
}

fn wrap(i: i64, n: i64, mirror: bool) -> usize {
    if mirror {
        let m = i.rem_euclid(2 * n);
        (if m >= n { 2 * n - 1 - m } else { m }) as usize
    } else {
        i.rem_euclid(n) as usize
    }
}

/// Bilinear sample at texture coordinates `t` (texel centres at (i + 0.5) / w), RGB
/// already sRGB-decoded through `linear`, alpha linear.
fn bilinear(leaf: &Leaf, linear: &[f64; 256], t: [f64; 2]) -> [f64; 4] {
    let [w, h] = leaf.extent.map(i64::from);
    let x = t[0] * w as f64 - 0.5;
    let y = t[1] * h as f64 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let texel = |xi: i64, yi: i64| -> [f64; 4] {
        let i = (wrap(yi, h, leaf.mirror[1]) * w as usize + wrap(xi, w, leaf.mirror[0])) * 4;
        let p = &leaf.rgba[i..i + 4];
        [
            linear[usize::from(p[0])],
            linear[usize::from(p[1])],
            linear[usize::from(p[2])],
            f64::from(p[3]) / 255.,
        ]
    };
    let (a, b, c, d) = (
        texel(x0, y0),
        texel(x0 + 1, y0),
        texel(x0, y0 + 1),
        texel(x0 + 1, y0 + 1),
    );
    std::array::from_fn(|k| {
        let top = a[k] + (b[k] - a[k]) * fx;
        let bottom = c[k] + (d[k] - c[k]) * fx;
        top + (bottom - top) * fy
    })
}

/// Bake one target on its `period` tile. Leaves are indexed by source (None = another
/// channel's source). Extent: the densest leaf (`period × |scale| × texels`) per axis,
/// capped at 4096 per axis and 4 Mi pixels (larger axis halved until it fits).
fn resample(
    steps: &[Step],
    target: &Target,
    leaves: &[Option<Leaf>],
    period: [u32; 2],
    cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    if period.iter().any(|p| !(1..=MAX_PERIOD).contains(p)) {
        return Err("Unsupported: graph evaluator off-grid period budget".into());
    }
    let mut density = [1f64; 2];
    for &s in &target.sources {
        let leaf = leaves
            .get(s)
            .and_then(Option::as_ref)
            .ok_or("Invalid: graph evaluator off-grid leaf")?;
        let [w, h] = leaf.extent;
        if w == 0
            || h == 0
            || w > 4096
            || h > 4096
            || u64::from(w) * u64::from(h) * 4 != leaf.rgba.len() as u64
        {
            return Err("Invalid: graph evaluator source extent".into());
        }
        for a in 0..2 {
            let d = f64::from(period[a]) * leaf.transform[a].abs() * f64::from(leaf.extent[a]);
            density[a] = density[a].max(d);
        }
    }
    let mut n = density.map(|d| (d.ceil() as u64).clamp(1, MAX_AXIS));
    while n[0] * n[1] > lbp_texture::MAX_PIXELS as u64 {
        let a = usize::from(n[1] > n[0]);
        n[a] = n[a].div_ceil(2);
    }
    // Box samples per leaf per axis when the bake minifies it.
    let subsamples: Vec<[u64; 2]> = (0..leaves.len())
        .map(|s| match &leaves[s] {
            Some(leaf) if target.sources.contains(&s) => std::array::from_fn(|a| {
                let d = f64::from(period[a]) * leaf.transform[a].abs() * f64::from(leaf.extent[a]);
                ((d / n[a] as f64).ceil() as u64).clamp(1, MAX_SUBSAMPLES)
            }),
            _ => [1, 1],
        })
        .collect();
    if cancelled() {
        return Err("Cancelled".into());
    }
    let linear: [f64; 256] = std::array::from_fn(|v| crate::blend_mask::srgb_decode(v as u8));
    let mut values = vec![[0f64; 4]; steps.len()];
    let mut cache = vec![[0f64; 4]; leaves.len()];
    let mut output = Vec::with_capacity((n[0] * n[1]) as usize * 4);
    for y in 0..n[1] {
        for x in 0..n[0] {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            for &s in &target.sources {
                let leaf = leaves[s].as_ref().expect("checked leaf");
                let [ku, kv] = subsamples[s];
                let mut sum = [0f64; 4];
                for j in 0..kv {
                    let v = (y as f64 + (j as f64 + 0.5) / kv as f64) / n[1] as f64
                        * f64::from(period[1]);
                    for i in 0..ku {
                        let u = (x as f64 + (i as f64 + 0.5) / ku as f64) / n[0] as f64
                            * f64::from(period[0]);
                        let t = [
                            u * leaf.transform[0] + leaf.transform[2],
                            v * leaf.transform[1] + leaf.transform[3],
                        ];
                        let texel = bilinear(leaf, &linear, t);
                        for c in 0..4 {
                            sum[c] += texel[c];
                        }
                    }
                }
                let count = (ku * kv) as f64;
                cache[s] = sum.map(|c| c / count);
            }
            crate::grid_graph::evaluate(steps, &mut values, Some(&target.needed), &|s| cache[s]);
            let mut product = [1f64; 4];
            for &factor in &target.factors {
                let v = crate::grid_graph::lane_value(&values, factor);
                for c in 0..4 {
                    product[c] *= v[c];
                }
            }
            output.extend_from_slice(&crate::grid_graph::encode_texel(product)?);
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((n[0] as u32, n[1] as u32, output))
}

fn tile_uv(
    uv: &[[f32; 2]],
    period: [u32; 2],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<[f32; 2]>, String> {
    crate::SamplingTransform::ScaleOffset(crate::UvTransform {
        bits: [
            (1. / period[0] as f32).to_bits(),
            (1. / period[1] as f32).to_bits(),
            0,
            0,
        ],
    })
    .sampling(uv, cancelled)
}

/// Loader branch for an `off_grid` selection (called from `grid_graph::load`).
pub(crate) fn load(
    cx: &LoadContext,
    material_key: lbp_catalog::attachment::LocationKey,
    instance_projection: Option<crate::InstanceProjection>,
    selection: crate::DiffuseSelection,
    program: &GraphProgram,
    sources: &[Source],
    kind: OffGrid,
) -> Result<crate::MaterialAsset, String> {
    let (targets, _) = targets(program, sources, selection.uv_channel)?;
    let prefix = &cx.graph.material.prefix;
    let repeat = sources
        .iter()
        .map(|s| {
            let axis = |raw: Option<&u8>| match raw {
                Some(1) => Ok(1),
                Some(2) => Ok(2),
                _ => Err("Unsupported: graph evaluator off-grid bake needs Repeat or Mirror on every texture".to_string()),
            };
            Ok([axis(prefix.wrap_s_raw.get(s.slot))?, axis(prefix.wrap_t_raw.get(s.slot))?])
        })
        .collect::<Result<Vec<[u32; 2]>, String>>()?;
    let periods = targets
        .iter()
        .map(|t| target_period(t, sources, &repeat).map(narrow))
        .collect::<Result<Vec<_>, String>>()?;
    let expected = match kind {
        OffGrid::Resampled { period, .. } => vec![period],
        OffGrid::Split {
            period,
            second_period,
            ..
        } => vec![period, second_period],
    };
    if periods != expected {
        return Err("Unsupported: graph evaluator off-grid Mirror changes the bake period".into());
    }
    let mut slots: Vec<usize> = vec![];
    for s in sources {
        if !slots.contains(&s.slot) {
            slots.push(s.slot);
        }
    }
    if slots[0] != selection.slot {
        return Err("Invalid: graph evaluator primary slot".into());
    }
    let additional_texture_slots =
        crate::instance::additional_texture_slots(cx.original_graph, &slots)?;
    let coordinates = cx.uv.decode(cx.control)?;
    let vertices = cx.vertices;
    let mut decoded = vec![];
    for &slot in &slots {
        decoded.push(Some(crate::grid_graph::decode(cx, slot, false)?));
    }
    let cancelled = || cx.control.check().is_err();
    let steps = crate::grid_graph::steps(program, sources)?;
    let mut bakes = vec![];
    {
        let leaves: Vec<Option<Leaf>> = sources
            .iter()
            .zip(&repeat)
            .map(|(s, r)| {
                let p = slots.iter().position(|&slot| slot == s.slot)?;
                let t = &decoded[p].as_ref()?.1;
                Some(Leaf {
                    extent: [t.header.width, t.header.height],
                    rgba: t.rgba.as_slice(),
                    transform: scale_offset(s).ok()?,
                    mirror: r.map(|v| v == 2),
                })
            })
            .collect();
        if leaves.iter().any(Option::is_none) {
            return Err("Invalid: graph evaluator off-grid leaves".into());
        }
        for (target, period) in targets.iter().zip(&periods) {
            let period = period.map(u32::from);
            let uv = crate::linked::selected_uv(&coordinates, target.channel, vertices)?;
            let sampling = tile_uv(&uv, period, &cancelled)?;
            let bake = resample(&steps, target, &leaves, period, &cancelled)?;
            bakes.push((uv, sampling, bake));
        }
    }
    let first_slot_of = |target: &Target| sources[target.sources[0]].slot;
    let mut take = |slot: usize| {
        let p = slots.iter().position(|&s| s == slot)?;
        decoded[p].take()
    };
    let (texture_key, texture) = take(selection.slot).ok_or("Invalid: graph evaluator textures")?;
    let dual = match (targets.get(1), bakes.get(1)) {
        (Some(target), Some((_, sampling, (w, h, rgba)))) => {
            let slot = first_slot_of(target);
            let (key, original) = match take(slot) {
                Some(pair) => pair,
                None => crate::grid_graph::decode(cx, slot, false)?,
            };
            let mut header = original.header.clone();
            header.width = *w;
            header.height = *h;
            Some(Box::new(crate::linked::DualAsset {
                texture: lbp_texture::Texture {
                    header,
                    expanded_bytes: original.expanded_bytes,
                    top_mip_block_bytes: original.top_mip_block_bytes,
                    unparsed_after_top_mip: original.unparsed_after_top_mip,
                    rgba: rgba.clone(),
                },
                texture_key: key,
                sampling_uv: sampling.clone(),
                wrap: crate::TextureWrapping::uniform(true),
            }))
        }
        _ => None,
    };
    let (uv, sampling_uv, (width, height, rgba)) = bakes
        .into_iter()
        .next()
        .ok_or("Invalid: graph evaluator bakes")?;
    // The product keeps a second decoded texture of the primary group (as the D100 grid).
    let second_slot = targets[0]
        .sources
        .iter()
        .map(|&i| sources[i].slot)
        .find(|&s| s != selection.slot)
        .unwrap_or(selection.slot);
    let (key, second) = match take(second_slot) {
        Some(pair) => pair,
        None => crate::grid_graph::decode(cx, second_slot, false)?,
    };
    let product = Some(Box::new(crate::linked::ProductAsset {
        texture: second,
        texture_key: key,
        width,
        height,
        rgba,
    }));
    cx.control.check()?;
    cx.source.check_stable()?;
    Ok(crate::MaterialAsset::Texture(Box::new(crate::Asset {
        instance_projection,
        additional_texture_slots,
        border_domain: None,
        fourth: None,
        third: None,
        dual,
        product,
        tinted_rgba: None,
        sampling_uv,
        uv,
        uv_metadata: coordinates.metadata,
        texture,
        material_key,
        texture_key,
        wrap: crate::TextureWrapping::uniform(true),
        selection,
    })))
}

/// Display label for an off-grid asset.
pub(crate) fn label(a: &crate::Asset, g: crate::GridGraph) -> String {
    let second = a.dual.as_ref().map_or(String::new(), |d| {
        format!(
            " × {:?} texels (Bevy dual multiply)",
            [d.texture.header.width, d.texture.header.height]
        )
    });
    format!(
        "Diffuse only • {} • bake {:?} texels{second} • Repeat, linear display • ignores {} nodes / {} wires",
        g.label(),
        a.display_extent(),
        a.selection.ignored_nodes,
        a.selection.ignored_wires
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, GraphMetadata, WireMetadata};

    fn node(kind: u32, params: [f32; 8]) -> BoxMetadata {
        BoxMetadata {
            type_raw: kind,
            subtype_raw: 0,
            declared_parameter_count: 8,
            parameters_raw: params.map(f32::to_bits),
            position_bits: [0; 4],
            texture_slot_raw: None,
            texture_slot_in_range: None,
            animations: std::array::from_fn(|_| AnimationMetadata {
                base_value_bits: [0; 4],
                component_value_count: 0,
                component_bytes_sha256: String::new(),
                name_hex: "000000".into(),
                components_raw: 0,
                derived_key_count: 0,
            }),
        }
    }
    /// Sampler with scale, offset, UV channel and slot.
    fn sample(slot: u32, channel: u32, scale: [f32; 2], offset: [f32; 2]) -> BoxMetadata {
        let mut b = node(
            1,
            [scale[0], scale[1], offset[0], offset[1], 0., 0., 0., 0.],
        );
        b.parameters_raw[4] = channel;
        b.parameters_raw[5] = slot;
        b
    }
    fn edge(from: i32, to: i32, port: u8) -> WireMetadata {
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
    const NO: [f32; 8] = [0.; 8];
    const ONE: [f32; 2] = [1., 1.];
    const ZERO: [f32; 2] = [0., 0.];

    /// BLEND(T0 × T1, T0, mask T2), all UV0; T1 at a non-integer scale.
    fn blend_offgrid(scale: f32) -> GraphMetadata {
        graph(
            vec![
                node(0, NO),
                node(16, [0., 1., 0., 0., 0., 0., 0., 0.]),
                sample(0, 0, ONE, ZERO),
                sample(1, 0, [scale, scale], ZERO),
                node(10, NO),
                sample(2, 0, ONE, ZERO),
            ],
            vec![
                edge(1, 0, 0),
                edge(4, 1, 0),
                edge(2, 1, 1),
                edge(5, 1, 2),
                edge(2, 4, 0),
                edge(3, 4, 1),
            ],
        )
    }

    #[test]
    fn d103_same_channel_offgrid_graphs_select_as_labelled_resamples() {
        let s = crate::select_diffuse(&blend_offgrid(2.5)).unwrap();
        let g = s.grid_graph.unwrap();
        assert_eq!(
            g.off_grid,
            Some(OffGrid::Resampled {
                channel: 0,
                period: [2, 2]
            })
        );
        assert!(g.label().contains("preview approximation"));
        assert!(s.second_sample.is_none());
        // On the integer grid the D100 marker is unchanged (no approximation).
        let on = crate::select_diffuse(&blend_offgrid(3.)).unwrap();
        assert_eq!(on.grid_graph.unwrap().off_grid, None);
        // Offsets alone keep period 1; no common period within 16 is refused.
        let mut offset = blend_offgrid(1.);
        offset.boxes[3].parameters_raw[2] = (-1.5f32).to_bits();
        assert_eq!(
            crate::select_diffuse(&offset)
                .unwrap()
                .grid_graph
                .unwrap()
                .off_grid,
            Some(OffGrid::Resampled {
                channel: 0,
                period: [1, 1]
            })
        );
        let err = crate::select_diffuse(&blend_offgrid(0.37)).unwrap_err();
        assert!(err.ends_with(
            "(graph evaluator: off-grid scales have no common period within 16 repeats)"
        ));
        // Matrix samplers stay refused off the grid.
        let mut matrix = blend_offgrid(2.5);
        matrix.boxes[3].subtype_raw = 1;
        assert!(crate::select_diffuse(&matrix)
            .unwrap_err()
            .ends_with("(graph evaluator: off-grid bake needs scale/offset samplers)"));
    }

    #[test]
    fn d103_separable_products_split_by_channel_and_others_are_refused() {
        // (ADD(T0 UV1, T2 UV1) × T1 UV0) × COLOR: no product tree (ADD inside), primary
        // channel = first sample's (UV1).
        let g = graph(
            vec![
                node(0, NO),
                node(10, NO),
                node(10, NO),
                node(11, NO),
                sample(0, 1, [2., 1.], ZERO),
                sample(1, 0, ONE, ZERO),
                node(3, [0.5, 0.5, 0.5, 1., 0., 0., 0., 0.]),
                sample(2, 1, ONE, [0., -0.1]),
            ],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(6, 1, 1),
                edge(3, 2, 0),
                edge(5, 2, 1),
                edge(4, 3, 0),
                edge(7, 3, 1),
            ],
        );
        let s = crate::select_diffuse(&g).unwrap();
        assert_eq!(
            s.grid_graph.unwrap().off_grid,
            Some(OffGrid::Split {
                channel: 1,
                period: [1, 1],
                second_channel: 0,
                second_period: [1, 1]
            })
        );
        assert_eq!(
            (s.uv_channel, s.second_sample.unwrap().sample_index),
            (1, 5)
        );
        let program = crate::grid_graph_program(&g, 1).unwrap();
        let (t, second) = targets(&program, &program_sources(&program), 1).unwrap();
        assert_eq!(second, Some(5));
        assert_eq!(
            (t[0].channel, t[0].factors.len(), t[0].sources.len()),
            (1, 2, 2)
        );
        assert_eq!(
            (t[1].channel, t[1].factors.len(), t[1].sources.len()),
            (0, 1, 1)
        );
        // The COLOR joins the primary group; the UV0 leaf is not evaluated there.
        let position = |i: usize| program.nodes.iter().position(|n| n.index == i).unwrap();
        assert!(t[0].needed[position(6)] && !t[0].needed[position(5)]);
        // A cross-channel BLEND is not separable.
        let mut blend = blend_offgrid(2.5);
        blend.boxes[5].parameters_raw[4] = 1;
        assert!(crate::select_diffuse(&blend).unwrap_err().ends_with(
            "(graph evaluator: off-grid samplers mix UV channels outside a separable product)"
        ));
    }

    fn program_sources(program: &GraphProgram) -> Vec<Source> {
        let mut out = vec![];
        for n in &program.nodes {
            if let GraphOp::Sample {
                slot,
                uv_channel,
                transform,
            } = n.op
            {
                let s = Source {
                    slot,
                    uv_channel,
                    transform,
                };
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    }

    #[test]
    fn d103_resample_reproduces_texels_and_evaluates_the_graph() {
        // One leaf at scale 1: the bake samples texel centres, so bytes round-trip.
        let g = graph(
            vec![
                node(0, NO),
                node(10, NO),
                sample(0, 0, ONE, ZERO),
                node(3, [1., 1., 1., 1., 0., 0., 0., 0.]),
            ],
            vec![edge(1, 0, 0), edge(2, 1, 0), edge(3, 1, 1)],
        );
        let program = crate::grid_graph_program(&g, 1).unwrap();
        let sources = program_sources(&program);
        let (t, _) = targets(&program, &sources, 0).unwrap();
        let steps = crate::grid_graph::steps(&program, &sources).unwrap();
        let rgba: Vec<u8> = (0..16u8)
            .flat_map(|i| [i * 16, 255 - i * 16, 7, i * 8])
            .collect();
        let leaf = |transform: [f64; 4], mirror| Leaf {
            extent: [4, 4],
            rgba: &rgba,
            transform,
            mirror,
        };
        let (w, h, out) = resample(
            &steps,
            &t[0],
            &[Some(leaf([1., 1., 0., 0.], [false; 2]))],
            [1, 1],
            &|| false,
        )
        .unwrap();
        assert_eq!((w, h), (4, 4));
        assert_eq!(out, rgba);
        // Scale 2.5 on a period-2 tile: 20 texels per axis, values interpolated between
        // neighbours (bilinear in linear light); Mirror flips the second repeat.
        let (w, _, out) = resample(
            &steps,
            &t[0],
            &[Some(leaf([2.5, 2.5, 0., 0.], [false; 2]))],
            [2, 2],
            &|| false,
        )
        .unwrap();
        assert_eq!(w, 20);
        // Bake texel 0 maps to texture coordinate 0.125 = texel centre 0 exactly.
        assert_eq!(&out[..4], &rgba[..4]);
        let (_, _, mirrored) = resample(
            &steps,
            &t[0],
            &[Some(leaf([1., 1., 0., 0.], [true, false]))],
            [2, 1],
            &|| false,
        )
        .unwrap();
        // Columns 4..8 repeat columns 3..0 (mirror).
        assert_eq!(&mirrored[4 * 4..4 * 5], &rgba[3 * 4..4 * 4]);
        // Minified leaf (scale 4096 on a 4×4 texture) averages a 4×4 box.
        let tall = resample(
            &steps,
            &t[0],
            &[Some(leaf([1., 4096., 0., 0.], [false; 2]))],
            [1, 1],
            &|| false,
        )
        .unwrap();
        assert_eq!((tall.0, tall.1), (4, 4096));
        assert_eq!(
            resample(
                &steps,
                &t[0],
                &[Some(leaf([1., 1., 0., 0.], [false; 2]))],
                [1, 1],
                &|| true
            )
            .unwrap_err(),
            "Cancelled"
        );
        assert!(resample(
            &steps,
            &t[0],
            &[Some(leaf([1.; 4], [false; 2]))],
            [17, 1],
            &|| false
        )
        .is_err());
    }

    #[test]
    fn d103_split_bakes_multiply_back_to_the_whole_graph() {
        // T0 UV0 × T1 UV1 with COLOR: the product of the two bakes equals the evaluated
        // graph where both leaves are sampled at texel centres.
        let g = graph(
            vec![
                node(0, NO),
                node(10, NO),
                node(10, NO),
                sample(0, 0, ONE, ZERO),
                sample(1, 1, [1.5, 1.5], ZERO),
                node(3, [0.5, 1., 1., 1., 0., 0., 0., 0.]),
            ],
            vec![
                edge(1, 0, 0),
                edge(2, 1, 0),
                edge(4, 1, 1),
                edge(3, 2, 0),
                edge(5, 2, 1),
            ],
        );
        let program = crate::grid_graph_program(&g, 1).unwrap();
        let sources = program_sources(&program);
        let (t, _) = targets(&program, &sources, 0).unwrap();
        assert_eq!(
            target_period(&t[1], &sources, &[[1, 1]; 2]).unwrap(),
            [2, 2]
        );
        let steps = crate::grid_graph::steps(&program, &sources).unwrap();
        let a = [255u8, 128, 0, 255, 64, 64, 64, 255];
        let b = [255u8, 255, 255, 255, 0, 255, 0, 255];
        let leaves = [
            Some(Leaf {
                extent: [2, 1],
                rgba: &a,
                transform: [1., 1., 0., 0.],
                mirror: [false; 2],
            }),
            Some(Leaf {
                extent: [2, 1],
                rgba: &b,
                transform: [1.5, 1.5, 0., 0.],
                mirror: [false; 2],
            }),
        ];
        let (w0, _, first) = resample(&steps, &t[0], &leaves, [1, 1], &|| false).unwrap();
        assert_eq!(w0, 2);
        // COLOR (0.5, 1, 1) folds into the UV0 bake: red 255 → linear 0.5 → sRGB 188.
        assert_eq!(&first[..4], &[188, 128, 0, 255]);
        let (w1, h1, second) = resample(&steps, &t[1], &leaves, [2, 2], &|| false).unwrap();
        assert_eq!((w1, h1), (6, 3));
        assert_eq!(second.len(), 6 * 3 * 4);
        // Cancellation and the encoded factor stay within 0..255.
        assert!(resample(&steps, &t[1], &leaves, [2, 2], &|| true).is_err());
    }
}
