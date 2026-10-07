//! Bounded CPU-only direct-diffuse slice. Independent interpretation; no shader code.
//! Evidence/pin/applicability/conflicts: docs/textured-mesh.md and docs/affine-preview.md.
//! Static COLOR × texture routes: docs/tinted-preview.md (independent bounded projection).
//! Two-texture nearest-grid products: docs/product-preview.md.
//! Independent static matrix/affine products: docs/dual-preview.md.
//! Bounded three-texture nested product: docs/nested-preview.md.
//! Bounded four-texture chain: docs/four-product-preview.md.
//! Independent single-texture U/V addressing: docs/axis-wrap-preview.md.
//! Static COLOR/product MIX: docs/static-mix-preview.md.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
use lbp_mesh::{Geometry, UvMetadata};
use lbp_resource::material::{GraphInspection, GraphMetadata, TextureDescriptor};
use std::collections::{BTreeSet, HashSet};

pub struct Asset {
    pub instance_projection: Option<crate::InstanceProjection>,
    pub additional_texture_slots: Vec<usize>,
    /// Source BORDER axes retained separately from guarded display addressing.
    pub border_domain: Option<crate::BorderDomain>,
    /// Fourth source shares the primary sampler coordinates.
    pub fourth: Option<Box<SharedSampleAsset>>,
    /// Third source samples second or primary coordinates via `relative`; no fallback.
    pub third: Option<Box<SharedSampleAsset>>,
    pub dual: Option<Box<DualAsset>>,
    pub product: Option<Box<ProductAsset>>,
    /// Optional derived display pixels; original decoded texture pixels remain intact.
    pub tinted_rgba: Option<Vec<u8>>,
    /// Renderer sampling coordinates, separate from the selected raw UV array.
    pub sampling_uv: Vec<[f32; 2]>,
    pub uv: Vec<[f32; 2]>,
    pub uv_metadata: UvMetadata,
    pub texture: lbp_texture::Texture,
    pub material_key: LocationKey,
    pub texture_key: LocationKey,
    /// Effective display addressing; source_wrap_raw() retains guarded BORDER axes.
    pub wrap: crate::TextureWrapping,
    pub selection: DiffuseSelection,
}
pub struct SharedSampleAsset {
    pub texture: lbp_texture::Texture,
    pub texture_key: LocationKey,
    /// Third sample only: coordinates as a scale/offset of a displayed set.
    pub relative: Option<RelativeSampling>,
}
/// Sampling coordinates re-derived from an already displayed coordinate set on the
/// same raw channel: `base * scale + offset`. Computed in f64 from two scale/offset
/// transforms and rounded once to f32; no bit-exact or GPU rounding claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeSampling {
    /// true: primary sampling coordinates; false: second-sample coordinates.
    pub from_primary: bool,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
impl RelativeSampling {
    pub fn is_identity(&self) -> bool {
        self.scale == [1., 1.] && self.offset == [0., 0.]
    }
}
/// D65: the third sample shares a raw channel with the primary or second sample.
/// Exact sharing keeps the second coordinates; otherwise both transforms must be
/// finite scale/offset with nonzero scale and no instance UV subtraction.
pub(crate) fn relative_third(
    selection: &DiffuseSelection,
    subtracted: &[usize],
) -> Result<RelativeSampling, String> {
    let third = selection.third_sample.ok_or("Unsupported: third sample")?;
    let second = selection
        .second_sample
        .ok_or("Unsupported: third sample needs a second sample")?;
    if second.uv_channel == third.uv_channel && second.uv_transform == third.uv_transform {
        return Ok(RelativeSampling {
            from_primary: false,
            scale: [1., 1.],
            offset: [0., 0.],
        });
    }
    let primary = SamplerSelection {
        sample_index: selection.sample_index,
        slot: selection.slot,
        uv_channel: selection.uv_channel,
        uv_transform: selection.uv_transform,
    };
    let crate::SamplingTransform::ScaleOffset(target) = third.uv_transform else {
        return Err("Unsupported: independent third texture needs scale/offset".into());
    };
    for (base, from_primary) in [(second, false), (primary, true)] {
        let crate::SamplingTransform::ScaleOffset(t) = base.uv_transform else {
            continue;
        };
        if base.uv_channel != third.uv_channel
            || subtracted.contains(&base.sample_index)
            || subtracted.contains(&third.sample_index)
        {
            continue;
        }
        let [bx, by, box_, boy] = t.values().map(f64::from);
        let [tx, ty, tox, toy] = target.values().map(f64::from);
        if bx == 0. || by == 0. {
            continue;
        }
        let (rx, ry) = (tx / bx, ty / by);
        let values = [rx, ry, tox - box_ * rx, toy - boy * ry].map(|v| v as f32);
        if values
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        {
            return Err("Unsupported: third texture relative transform bounds".into());
        }
        return Ok(RelativeSampling {
            from_primary,
            scale: [values[0], values[1]],
            offset: [values[2], values[3]],
        });
    }
    Err("Unsupported: third texture needs a scale/offset sample on its channel".into())
}
pub struct DualAsset {
    pub texture: lbp_texture::Texture,
    pub texture_key: LocationKey,
    pub sampling_uv: Vec<[f32; 2]>,
    pub wrap: crate::TextureWrapping,
}
pub struct ProductAsset {
    pub texture: lbp_texture::Texture,
    pub texture_key: LocationKey,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl Asset {
    pub fn source_wrap_raw(&self) -> [u8; 2] {
        self.border_domain
            .map_or_else(|| self.wrap.raw(), |b| b.source_wrap_raw)
    }
    pub fn nearest_sampling(&self) -> bool {
        self.product.is_some() || self.selection.alpha_mix.is_some()
    }
    pub fn display_extent(&self) -> [u32; 2] {
        self.product.as_ref().map_or(
            [self.texture.header.width, self.texture.header.height],
            |p| [p.width, p.height],
        )
    }
    pub fn display_pixels(&self) -> &[u8] {
        self.product.as_ref().map_or_else(
            || self.tinted_rgba.as_deref().unwrap_or(&self.texture.rgba),
            |p| p.rgba.as_slice(),
        )
    }
}
pub struct SolidAsset {
    pub instance_projection: Option<crate::InstanceProjection>,
    pub additional_texture_slots: Vec<usize>,
    pub material_key: LocationKey,
    pub selection: crate::SolidSelection,
}
/// Explicit no-texture material, distinct from a failed binding or invented image.
pub enum MaterialAsset {
    Texture(Box<Asset>),
    Solid(Box<SolidAsset>),
}
impl MaterialAsset {
    /// Heap bytes retained for display: decoded and derived pixels plus coordinate
    /// arrays. Used for the per-mesh aggregate budget; small metadata is ignored.
    pub fn retained_bytes(&self) -> usize {
        let Self::Texture(a) = self else {
            return 0;
        };
        let uv = |v: &Vec<[f32; 2]>| v.len() * std::mem::size_of::<[f32; 2]>();
        a.texture.rgba.len()
            + a.tinted_rgba.as_ref().map_or(0, Vec::len)
            + uv(&a.sampling_uv)
            + uv(&a.uv)
            + a.product
                .as_ref()
                .map_or(0, |p| p.texture.rgba.len() + p.rgba.len())
            + a.dual
                .as_ref()
                .map_or(0, |d| d.texture.rgba.len() + uv(&d.sampling_uv))
            + a.third.as_ref().map_or(0, |t| t.texture.rgba.len())
            + a.fourth.as_ref().map_or(0, |t| t.texture.rgba.len())
    }
    pub fn texture(&self) -> Option<&Asset> {
        match self {
            Self::Texture(a) => Some(a),
            Self::Solid(_) => None,
        }
    }
    pub fn material_key(&self) -> &LocationKey {
        match self {
            Self::Texture(a) => &a.material_key,
            Self::Solid(a) => &a.material_key,
        }
    }
    pub fn sampling_uv(&self) -> Option<&[[f32; 2]]> {
        self.texture().map(|a| a.sampling_uv.as_slice())
    }
    pub fn label(&self) -> String {
        let mut label = self.source_label();
        if !self.additional_texture_slots().is_empty() {
            label.push_str(&format!(
                " | Other required texture slots unrendered: {:?}",
                self.additional_texture_slots()
            ));
        }
        if let Some(p) = self.instance_projection() {
            format!(
                "Supplied instance RGBA {:?} (unverified) | {label}",
                p.color.values()
            )
        } else {
            label
        }
    }
    pub fn additional_texture_slots(&self) -> &[usize] {
        match self {
            Self::Texture(a) => &a.additional_texture_slots,
            Self::Solid(a) => &a.additional_texture_slots,
        }
    }
    pub fn instance_projection(&self) -> Option<&crate::InstanceProjection> {
        match self {
            Self::Texture(a) => a.instance_projection.as_ref(),
            Self::Solid(a) => a.instance_projection.as_ref(),
        }
    }
    fn source_label(&self) -> String {
        match self {
            Self::Texture(a) if a.selection.alpha_mix.is_some() => format!("Diffuse only • {} • UV{} {} • {} • ignores {} nodes / {} wires",a.selection.alpha_mix.unwrap().label(),a.selection.uv_channel,a.selection.uv_transform.label(),a.border_domain.map_or_else(||a.wrap.label(),|b|b.label()),a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Texture(a) if a.selection.animation_base.is_some()=>format!("Animation base-value snapshot • local/global rows verified • {} keys per row {:?} • UV{} {} • time/motion unverified • diffuse only",a.selection.animation_base.unwrap().key_counts[0],a.selection.animation_base.unwrap().key_counts,a.selection.uv_channel,a.selection.uv_transform.label()),
            Self::Texture(a) if a.fourth.is_some() => format!("Diffuse only • four-texture nested product • UV{} {} × UV{} {} × UV{} {} × UV{} {} • linear filtering • ignores {} nodes / {} wires",a.selection.uv_channel,a.selection.uv_transform.label(),a.selection.second_sample.unwrap().uv_channel,a.selection.second_sample.unwrap().uv_transform.label(),a.selection.third_sample.unwrap().uv_channel,a.selection.third_sample.unwrap().uv_transform.label(),a.selection.fourth_sample.unwrap().uv_channel,a.selection.fourth_sample.unwrap().uv_transform.label(),a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Texture(a) if a.third.is_some() => format!("Diffuse only • three-texture nested product{} • UV{} {} × UV{} {} × UV{} {} • linear filtering • ignores {} nodes / {} wires",a.third.as_ref().and_then(|t| t.relative).filter(|r| !r.is_identity() || r.from_primary).map_or(String::new(), |r| format!(" (third from {} coordinates × {:?} + {:?})", if r.from_primary {"primary"} else {"second"}, r.scale, r.offset)),a.selection.uv_channel,a.selection.uv_transform.label(),a.selection.second_sample.unwrap().uv_channel,a.selection.second_sample.unwrap().uv_transform.label(),a.selection.third_sample.unwrap().uv_channel,a.selection.third_sample.unwrap().uv_transform.label(),a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Texture(a) if a.dual.is_some() => format!("Diffuse only • independent two-texture sampling • UV{} {} ({}) / UV{} {} ({}) | linear filtering • ignores {} nodes / {} wires",a.selection.uv_channel,a.selection.uv_transform.label(),a.wrap.label(),a.selection.second_sample.unwrap().uv_channel,a.selection.second_sample.unwrap().uv_transform.label(),a.dual.as_ref().unwrap().wrap.label(),a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Texture(a) => format!("Diffuse only • UV{} • {} • {} • {}×{} {} • {} • {} • ignores {} nodes / {} wires", a.selection.uv_channel,a.selection.uv_transform.label(),a.selection.blend.map_or_else(||a.selection.tint.label(),|blend|blend.label()),a.texture.header.width,a.texture.header.height,a.texture.header.pixel_format,a.wrap.label(),if let Some(second)=a.selection.second_sample {format!("two-texture product, second {}, nearest grid {:?}",second.uv_transform.label(),a.display_extent())} else {"single texture".into()},a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Solid(a) if a.selection.unwired_diffuse => format!("No diffuse wire • black (0,0,0,0) per Toolkit shader generator; game default unverified • other channels not rendered • ignores {} nodes / {} wires",a.selection.ignored_nodes,a.selection.ignored_wires),
            Self::Solid(a) => format!("Solid diffuse color {:?} • no diffuse texture required • ignores {} nodes / {} wires",a.selection.color.values(),a.selection.ignored_nodes,a.selection.ignored_wires),
        }
    }
}
/// Diffuse-only projection; other nodes/wires are deliberately not evaluated.
/// Rule evidence, pin, applicability and limitations: docs/diffuse-projection.md.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffuseSelection {
    pub alpha_mix: Option<crate::AlphaDiffuseMix>,
    pub animation_base: Option<crate::AnimationBaseSnapshot>,
    pub blend: Option<crate::DiffuseBlend>,
    pub fourth_sample: Option<SamplerSelection>,
    pub deep_multiply_index: Option<usize>,
    pub third_sample: Option<SamplerSelection>,
    pub inner_multiply_index: Option<usize>,
    pub second_sample: Option<SamplerSelection>,
    pub output_index: usize,
    pub sample_index: usize,
    pub slot: usize,
    /// Original material channel; renderer feeds its coordinates into its sampling attribute.
    pub uv_channel: usize,
    pub uv_transform: crate::SamplingTransform,
    pub tint: crate::DiffuseTint,
    pub multiply_index: Option<usize>,
    pub color_index: Option<usize>,
    pub ignored_nodes: usize,
    pub ignored_wires: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerSelection {
    pub sample_index: usize,
    pub slot: usize,
    pub uv_channel: usize,
    pub uv_transform: crate::SamplingTransform,
}
fn uses_grid(selection: DiffuseSelection) -> bool {
    selection.third_sample.is_none()
        && selection.second_sample.is_some_and(|second| {
            selection.uv_channel == second.uv_channel
                && crate::product::scales(selection.uv_transform).is_ok()
                && crate::product::scales(second.uv_transform).is_ok()
        })
}
// Only Repeat is periodic on the authored LCM product grid. Mirror and Clamp
// retain their independent GPU samplers; constant product MIX still needs a grid.
fn grid_with_wrapping(
    selection: DiffuseSelection,
    primary: crate::TextureWrapping,
    secondary: Option<crate::TextureWrapping>,
) -> bool {
    uses_grid(selection)
        && primary == crate::TextureWrapping::uniform(true)
        && secondary == Some(crate::TextureWrapping::uniform(true))
}
fn sampler(graph: &GraphMetadata, index: usize) -> Result<SamplerSelection, String> {
    let sample = &graph.boxes[index];
    let p = &sample.parameters_raw;
    if sample.type_raw != 1
        || p[4] > 1
        || sample
            .animations
            .iter()
            .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|v| v != b'0'))
        || graph.wires.iter().any(|w| w.box_to as usize == index)
    {
        return Err("Unsupported: sampler must be static, unwired UV0/UV1".into());
    }
    let uv_transform = match sample.subtype_raw {
        0 => crate::SamplingTransform::ScaleOffset(crate::UvTransform {
            bits: [p[0], p[1], p[2], p[3]],
        }),
        1 => crate::SamplingTransform::Matrix(crate::UvMatrix {
            bits: [p[0], p[1], p[2], p[3], p[6], p[7]],
        }),
        _ => return Err("Unsupported: unknown texture sampler subtype".into()),
    };
    uv_transform.validate()?;
    Ok(SamplerSelection {
        sample_index: index,
        slot: p[5] as usize,
        uv_channel: p[4] as usize,
        uv_transform,
    })
}
// Structurally fixed chain: ((A × B) × C) × D. No recursive traversal.
// D shares A's exact coordinates, C shares B's. Evidence: docs/four-product-preview.md.
fn four_chain(
    graph: &GraphMetadata,
    root: usize,
    output: usize,
) -> Result<Option<DiffuseSelection>, String> {
    let children = |index: usize| {
        graph
            .wires
            .iter()
            .filter(|w| w.box_to as usize == index)
            .collect::<Vec<_>>()
    };
    let chain = |index: usize| {
        let incoming = children(index);
        if incoming.len() != 2 {
            return None;
        }
        let a = incoming[0].box_from as usize;
        let b = incoming[1].box_from as usize;
        match (graph.boxes[a].type_raw, graph.boxes[b].type_raw) {
            (10, 1) => Some((a, b)),
            (1, 10) => Some((b, a)),
            _ => None,
        }
    };
    let Some((middle, fourth)) = chain(root) else {
        return Ok(None);
    };
    let Some((deep, third)) = chain(middle) else {
        return Ok(None);
    };
    let selected_nodes = [root, middle, deep, third, fourth];
    if selected_nodes.into_iter().collect::<BTreeSet<_>>().len() != 5 {
        return Err("Unsupported: four-texture chain cycle or repeated node".into());
    }
    for index in [root, middle, deep] {
        let incoming = children(index);
        let node = &graph.boxes[index];
        if node.type_raw != 10
            || node.subtype_raw != 0
            || node
                .animations
                .iter()
                .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|b| b != b'0'))
            || incoming.len() != 2
            || incoming
                .iter()
                .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
            || !incoming.iter().any(|w| w.port_to_raw == 0)
            || !incoming.iter().any(|w| w.port_to_raw == 1)
        {
            return Err("Unsupported: four-texture chain requires unique static unswizzled inputs on ports 0/1".into());
        }
    }
    let bottom = children(deep);
    let mut first = sampler(graph, bottom[0].box_from as usize)?;
    let mut second = sampler(graph, bottom[1].box_from as usize)?;
    let third = sampler(graph, third)?;
    let fourth = sampler(graph, fourth)?;
    if [
        first.sample_index,
        second.sample_index,
        third.sample_index,
        fourth.sample_index,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>()
    .len()
        != 4
    {
        return Err("Unsupported: four-texture chain requires four distinct texture leaves".into());
    }
    let matches = |a: SamplerSelection, b: SamplerSelection| {
        a.uv_channel == b.uv_channel && a.uv_transform == b.uv_transform
    };
    if !matches(first, fourth) || !matches(second, third) {
        std::mem::swap(&mut first, &mut second);
    }
    if !matches(first, fourth) || !matches(second, third) {
        return Err("Unsupported: four-texture chain needs exact first/fourth and second/third coordinate sharing".into());
    }
    Ok(Some(DiffuseSelection {
        fourth_sample: Some(fourth),
        deep_multiply_index: Some(deep),
        third_sample: Some(third),
        inner_multiply_index: Some(middle),
        second_sample: Some(second),
        output_index: output,
        sample_index: first.sample_index,
        slot: first.slot,
        uv_channel: first.uv_channel,
        uv_transform: first.uv_transform,
        multiply_index: Some(root),
        ignored_nodes: graph.boxes.len() - 8,
        ignored_wires: graph.wires.len() - 7,
        ..Default::default()
    }))
}
// Static MIX: COLOR plus a single sample or two samples under one MULTIPLY.
// Fixed same-sample alpha factor: docs/alpha-mix-preview.md.
fn select_alpha_mix(
    graph: &GraphMetadata,
    root: usize,
    output: usize,
) -> Result<DiffuseSelection, String> {
    let static_node = |node: &lbp_resource::material::BoxMetadata| {
        node.subtype_raw == 0
            && node
                .animations
                .iter()
                .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
    };
    let mut edges: Vec<_> = graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == root)
        .collect();
    edges.sort_by_key(|w| w.port_to_raw);
    if !static_node(&graph.boxes[root])
        || edges.len() != 3
        || edges.iter().enumerate().any(|(i, w)| {
            w.port_to_raw as usize != i
                || w.port_from_raw != 0
                || w.swizzle_raw != if i == 2 { [b'a', 0, 0, 0, 0] } else { [0; 5] }
        })
    {
        return Err(
            "Unsupported: alpha MIX requires static ports 0/1 and same-sample alpha on port 2"
                .into(),
        );
    }
    let left = edges[0].box_from as usize;
    let right = edges[1].box_from as usize;
    let (color, sample, color_first) =
        match (graph.boxes[left].type_raw, graph.boxes[right].type_raw) {
            (3, 1) => (left, right, true),
            (1, 3) => (right, left, false),
            _ => return Err("Unsupported: alpha MIX needs COLOR and one texture sample".into()),
        };
    if edges[2].box_from as usize != sample
        || !static_node(&graph.boxes[color])
        || graph.wires.iter().any(|w| w.box_to as usize == color)
    {
        return Err(
            "Unsupported: alpha MIX factor must reuse sample; COLOR must be static and unwired"
                .into(),
        );
    }
    let selected = sampler(graph, sample)?;
    let mix = crate::AlphaDiffuseMix {
        mix_index: root,
        color: crate::DiffuseTint {
            bits: graph.boxes[color].parameters_raw[..4].try_into().unwrap(),
        },
        color_first,
        unused_weight_bits: graph.boxes[root].parameters_raw[0],
    };
    mix.color.validate()?;
    Ok(DiffuseSelection {
        alpha_mix: Some(mix),
        output_index: output,
        sample_index: sample,
        slot: selected.slot,
        uv_channel: selected.uv_channel,
        uv_transform: selected.uv_transform,
        color_index: Some(color),
        ignored_nodes: graph.boxes.len() - 4,
        ignored_wires: graph.wires.len() - 4,
        ..Default::default()
    })
}
// Single-sample evidence and derived-pixel policy: docs/single-mix-preview.md.
// Endpoints/output already validated. Depth is fixed; no recursive graph evaluation.
fn select_static_mix(
    graph: &GraphMetadata,
    root: usize,
    output: usize,
) -> Result<DiffuseSelection, String> {
    if graph
        .wires
        .iter()
        .filter(|w| w.box_to as usize == root)
        .count()
        == 3
    {
        return select_alpha_mix(graph, root, output);
    }
    let inputs = |index: usize, kind: u32| -> Result<[usize; 2], String> {
        let node = &graph.boxes[index];
        let mut edges = graph
            .wires
            .iter()
            .filter(|w| w.box_to as usize == index)
            .collect::<Vec<_>>();
        if node.type_raw != kind
            || node.subtype_raw != 0
            || node
                .animations
                .iter()
                .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|b| b != b'0'))
            || edges.len() != 2
            || edges
                .iter()
                .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
            || !edges.iter().any(|w| w.port_to_raw == 0)
            || !edges.iter().any(|w| w.port_to_raw == 1)
        {
            return Err("Unsupported: static MIX/product needs unique unswizzled inputs on ports 0/1 and no animated or wired weight".into());
        }
        edges.sort_by_key(|w| w.port_to_raw);
        Ok([edges[0].box_from as usize, edges[1].box_from as usize])
    };
    let [left, right] = inputs(root, 12)?;
    let (color, multiply, color_first) = match (
        graph.boxes[left].type_raw,
        graph.boxes[right].type_raw,
    ) {
        (3, 10) => (left, right, true),
        (10, 3) => (right, left, false),
        (3, 1) => (left, right, true),
        (1, 3) => (right, left, false),
        _ => return Err(
            "Unsupported: static MIX requires COLOR and a texture sample or two-texture MULTIPLY"
                .into(),
        ),
    };
    let color_node = &graph.boxes[color];
    if color_node.subtype_raw != 0
        || color_node
            .animations
            .iter()
            .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|b| b != b'0'))
        || graph.wires.iter().any(|w| w.box_to as usize == color)
    {
        return Err("Unsupported: MIX COLOR must be static and unwired".into());
    }
    let blend = crate::DiffuseBlend {
        mix_index: root,
        color: crate::DiffuseTint {
            bits: color_node.parameters_raw[..4].try_into().unwrap(),
        },
        weight_bits: graph.boxes[root].parameters_raw[0],
        color_first,
    };
    blend.validate()?;
    if graph.boxes[multiply].type_raw == 1 {
        let sample = sampler(graph, multiply)?;
        return Ok(DiffuseSelection {
            blend: Some(blend),
            output_index: output,
            sample_index: sample.sample_index,
            slot: sample.slot,
            uv_channel: sample.uv_channel,
            uv_transform: sample.uv_transform,
            color_index: Some(color),
            ignored_nodes: graph.boxes.len() - 4,
            ignored_wires: graph.wires.len() - 3,
            ..Default::default()
        });
    }
    let [first, second] = inputs(multiply, 10)?;
    let first = sampler(graph, first)?;
    let second = sampler(graph, second)?;
    if first.sample_index == second.sample_index {
        return Err("Unsupported: MIX needs two distinct texture leaves".into());
    }
    let selection = DiffuseSelection {
        blend: Some(blend),
        output_index: output,
        sample_index: first.sample_index,
        slot: first.slot,
        uv_channel: first.uv_channel,
        uv_transform: first.uv_transform,
        second_sample: Some(second),
        multiply_index: Some(multiply),
        color_index: Some(color),
        ignored_nodes: graph.boxes.len() - 6,
        ignored_wires: graph.wires.len() - 5,
        ..Default::default()
    };
    if !uses_grid(selection) {
        return Err(
            "Unsupported: static MIX requires same-channel integer Repeat grid samplers".into(),
        );
    }
    Ok(selection)
}
pub fn select_diffuse(graph: &GraphMetadata) -> Result<DiffuseSelection, String> {
    let boxes = &graph.boxes;
    let wires = &graph.wires;
    if boxes.len() > 1024 || wires.len() > 4096 {
        return Err("Unsupported: diffuse graph budget".into());
    }
    if wires.iter().any(|w| {
        w.box_from < 0
            || w.box_to < 0
            || w.box_from as usize >= boxes.len()
            || w.box_to as usize >= boxes.len()
    }) {
        return Err("Unsupported: diffuse graph wire endpoints".into());
    }
    let outputs: Vec<_> = boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.type_raw == 0)
        .collect();
    if outputs.len() != 1 {
        return Err("Unsupported: diffuse projection requires one output node".into());
    }
    let (output_index, output) = outputs[0];
    let diffuse: Vec<_> = wires
        .iter()
        .filter(|w| w.box_to as usize == output_index && w.port_to_raw == 0)
        .collect();
    if diffuse.len() != 1 {
        return Err("Unsupported: diffuse projection requires one diffuse wire".into());
    }
    let wire = diffuse[0];
    if wire.port_from_raw != 0
        || wire.swizzle_raw != [0; 5]
        || output.subtype_raw != 0
        || output
            .animations
            .iter()
            .any(|a| a.component_value_count != 0 || a.name_hex.bytes().any(|b| b != b'0'))
    {
        return Err("Unsupported: diffuse output must be static and unswizzled".into());
    }
    if boxes[wire.box_from as usize].type_raw == 10 {
        if let Some(selection) = four_chain(graph, wire.box_from as usize, output_index)? {
            return Ok(selection);
        }
    }
    if boxes[wire.box_from as usize].type_raw == 12 {
        return select_static_mix(graph, wire.box_from as usize, output_index);
    }
    let mut sample_index = wire.box_from as usize;
    let mut tint = crate::DiffuseTint::default();
    let (mut multiply_index, mut color_index) = (None, None);
    let mut second_sample = None;
    let mut third_sample = None;
    let mut inner_multiply_index = None;
    let mut used_nodes = 2;
    let mut used_wires = 1;
    let unanimated = |node: &lbp_resource::material::BoxMetadata| {
        node.subtype_raw == 0
            && node
                .animations
                .iter()
                .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|b| b == b'0'))
    };
    if boxes[sample_index].type_raw == 10 {
        let multiply = sample_index;
        let incoming: Vec<_> = wires
            .iter()
            .filter(|w| w.box_to as usize == multiply)
            .collect();
        if !unanimated(&boxes[multiply])
            || incoming.len() != 2
            || incoming
                .iter()
                .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
            || !incoming.iter().any(|w| w.port_to_raw == 0)
            || !incoming.iter().any(|w| w.port_to_raw == 1)
        {
            return Err("Unsupported: diffuse multiply requires unique unswizzled static inputs on ports 0/1".into());
        }
        let left = incoming[0].box_from as usize;
        let right = incoming[1].box_from as usize;
        // Inner multiply of a product of a product; children keep wire order.
        let nested_inputs = |inner: usize| -> Result<[usize; 2], String> {
            let nested: Vec<_> = wires
                .iter()
                .filter(|w| w.box_to as usize == inner)
                .collect();
            if inner == multiply
                || !unanimated(&boxes[inner])
                || nested.len() != 2
                || nested
                    .iter()
                    .any(|w| w.port_from_raw != 0 || w.swizzle_raw != [0; 5])
                || !nested.iter().any(|w| w.port_to_raw == 0)
                || !nested.iter().any(|w| w.port_to_raw == 1)
            {
                return Err("Unsupported: nested product needs one static multiply with unique inputs on ports 0/1".into());
            }
            Ok([nested[0].box_from as usize, nested[1].box_from as usize])
        };
        let (sample, color) = match (boxes[left].type_raw, boxes[right].type_raw) {
            // D75: (texture × COLOR) × texture. The component-wise product is
            // commutative, so this is the two-texture product with a static tint.
            (10, 1) | (1, 10)
                if {
                    let inner = if boxes[left].type_raw == 10 { left } else { right };
                    let [a, b] = nested_inputs(inner)?;
                    matches!(
                        (boxes[a].type_raw, boxes[b].type_raw),
                        (1, 3) | (3, 1)
                    )
                } =>
            {
                let (inner, outer) = if boxes[left].type_raw == 10 { (left, right) } else { (right, left) };
                let [a, b] = nested_inputs(inner)?;
                let (inner_sample, inner_color) = if boxes[a].type_raw == 1 { (a, b) } else { (b, a) };
                sampler(graph, inner_sample)?;
                second_sample = Some(sampler(graph, outer)?);
                inner_multiply_index = Some(inner);
                (inner_sample, Some(inner_color))
            }
            (10, 1) | (1, 10) => {
                let (inner, third) = if boxes[left].type_raw == 10 { (left, right) } else { (right, left) };
                let [mut first, mut second] = nested_inputs(inner)?;
                if first == second || first == third || second == third {
                    return Err("Unsupported: nested product requires three distinct texture leaves".into());
                }
                let first_selection = sampler(graph, first)?;
                let mut second_selection = sampler(graph, second)?;
                let mut third = sampler(graph, third)?;
                let shares = |s: &SamplerSelection| s.uv_channel == third.uv_channel && s.uv_transform == third.uv_transform;
                if !shares(&second_selection) && shares(&first_selection) {
                    std::mem::swap(&mut first, &mut second);
                    second_selection = first_selection;
                }
                // D65: otherwise the third texture keeps its own scale/offset, derived
                // from the displayed coordinates on its channel (relative_third).
                let scale_offset = |s: &SamplerSelection| matches!(s.uv_transform, crate::SamplingTransform::ScaleOffset(t) if t.values()[..2].iter().all(|v| *v != 0.));
                if !shares(&second_selection) && ![first_selection, second_selection, third].iter().all(scale_offset) {
                    return Err("Unsupported: independent third texture needs nonzero scale/offset samplers".into());
                }
                // D67: the componentwise product is commutative. When both inner leaves
                // use one channel and the outer leaf the other, swap the second and
                // third roles so the displayed coordinates cover both channels.
                if third.uv_channel != first_selection.uv_channel && third.uv_channel != second_selection.uv_channel {
                    std::mem::swap(&mut second_selection, &mut third);
                }
                second_sample = Some(second_selection);
                third_sample = Some(third);
                inner_multiply_index = Some(inner);
                (first, None)
            }
            (1, 1) => {
                // Same sample on both ports is that sample squared: identical
                // coordinates, so the existing product path is exact per texel.
                sampler(graph, left)?;
                let second = sampler(graph, right)?;
                second_sample = Some(second);
                (left, None)
            }
            // D75: (texture × texture) × COLOR; one sample on both inner ports is squared.
            (10, 3) | (3, 10) => {
                let (inner, color) = if boxes[left].type_raw == 10 { (left, right) } else { (right, left) };
                let [a, b] = nested_inputs(inner)?;
                if boxes[a].type_raw != 1 || boxes[b].type_raw != 1 {
                    return Err("Unsupported: product × COLOR needs two direct texture samples under the inner multiply".into());
                }
                sampler(graph, a)?;
                second_sample = Some(sampler(graph, b)?);
                inner_multiply_index = Some(inner);
                (a, Some(color))
            }
            (1, 3) => (left, Some(right)),
            (3, 1) => (right, Some(left)),
            _ => {
                return Err(
                    "Unsupported: diffuse multiply requires two direct textures or one texture and static COLOR"
                        .into(),
                )
            }
        };
        if let Some(color) = color {
            if !unanimated(&boxes[color]) || wires.iter().any(|w| w.box_to as usize == color) {
                return Err("Unsupported: diffuse COLOR must be unanimated and unwired".into());
            }
            tint.bits.copy_from_slice(&boxes[color].parameters_raw[..4]);
            tint.validate()?;
        }
        sample_index = sample;
        multiply_index = Some(multiply);
        color_index = color;
        let squared = second_sample.is_some_and(|s| s.sample_index == sample);
        // Nested routes add the inner multiply, one more leaf and two wires.
        let nested = usize::from(inner_multiply_index.is_some());
        used_nodes = if third_sample.is_some() {
            6
        } else {
            4 - usize::from(squared) + 2 * nested
        };
        used_wires = 3 + 2 * nested;
    }
    let selected = sampler(graph, sample_index)?;
    if wire.port_from_raw != 0
        || wire.swizzle_raw != [0; 5]
        || output.subtype_raw != 0
        || !unanimated(output)
    {
        return Err("Unsupported: diffuse projection needs a direct unanimated static UV0/UV1 texture without swizzle or wired sampler inputs".into());
    }
    Ok(DiffuseSelection {
        alpha_mix: None,
        animation_base: None,
        blend: None,
        fourth_sample: None,
        deep_multiply_index: None,
        third_sample,
        inner_multiply_index,
        second_sample,
        output_index,
        sample_index,
        slot: selected.slot,
        uv_channel: selected.uv_channel,
        uv_transform: selected.uv_transform,
        tint,
        multiply_index,
        color_index,
        ignored_nodes: boxes.len() - used_nodes,
        ignored_wires: wires.len() - used_wires,
    })
}
#[cfg(test)]
fn sample_slot(graph: &GraphMetadata) -> Result<usize, String> {
    select_diffuse(graph).map(|s| s.slot)
}
pub fn select_diffuse_with_animation_base(
    graph: &GraphMetadata,
    animation_base: bool,
) -> Result<DiffuseSelection, String> {
    match select_diffuse(graph) {
        Ok(selection) => Ok(selection),
        Err(error) if !animation_base => Err(error),
        Err(_) => crate::animation_base::select(graph),
    }
}
fn binding(
    g: &GraphInspection,
    animation_base: bool,
) -> Result<(&TextureDescriptor, crate::TextureWrapping, DiffuseSelection), String> {
    let selection = select_diffuse_with_animation_base(&g.graph, animation_base)?;
    let slot = selection.slot;
    let prefix = &g.material.prefix;
    let texture = prefix
        .textures
        .get(slot)
        .ok_or("Unsupported: diffuse texture slot")?;
    let s = prefix.wrap_s_raw.get(slot).copied();
    let t = prefix.wrap_t_raw.get(slot).copied();
    let wrap = crate::border_domain::addressing([s, t], selection.alpha_mix.is_some())?;
    if texture.kind == 0 || texture.flags != 0 {
        return Err("Unsupported: absent texture or texture descriptor flags".into());
    }
    Ok((texture, wrap, selection))
}
fn load_shared_sample(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    slot: usize,
    control: &LoadControl,
) -> Result<Box<SharedSampleAsset>, String> {
    control.check()?;
    let prefix = &graph.material.prefix;
    let descriptor = prefix
        .textures
        .get(slot)
        .ok_or("Unsupported: shared texture slot")?;
    if descriptor.kind == 0
        || descriptor.flags != 0
        || prefix.wrap_s_raw.get(slot) != Some(&1)
        || prefix.wrap_t_raw.get(slot) != Some(&1)
    {
        return Err("Unsupported: shared texture requires present unflagged Repeat texture".into());
    }
    let texture_key = resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
    let texture = lbp_texture::decode_reader(
        texture_key.size,
        |offset, count| {
            source
                .read_hex_checked(&texture_key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    Ok(Box::new(SharedSampleAsset {
        texture,
        texture_key,
        relative: None,
    }))
}
pub fn resolve(
    catalog: &Catalog,
    guid: Option<u32>,
    hash: Option<&str>,
) -> Result<LocationKey, String> {
    let mut groups = Vec::new();
    if let Some(guid) = guid {
        if guid == 0 {
            return Err("Unresolved: null GUID".into());
        }
        groups.push(
            catalog
                .by_guid
                .get(&format!("{guid:08x}"))
                .ok_or("Unresolved: GUID")?,
        );
    }
    if let Some(hash) = hash {
        if hash.bytes().all(|b| b == b'0') {
            return Err("Unresolved: null hash".into());
        }
        groups.push(catalog.by_hash.get(hash).ok_or("Unresolved: hash")?);
    }
    if groups.is_empty() {
        return Err("Unresolved: absent resource identifier".into());
    }
    // Preserve conflicting identifiers even if only one candidate has available bytes.
    let mut hashes = BTreeSet::new();
    let mut keys = HashSet::new();
    let mut unavailable = false;
    for rows in groups {
        for &row in rows {
            let r = &catalog.snapshot.resources[row];
            hashes.insert(&r.map_row.sha1);
            unavailable |= r.archive_candidates.is_empty() && r.loose_candidates.is_empty();
            keys.extend(r.archive_candidates.iter().map(|c| LocationKey {
                path: c.archive.clone(),
                offset: c.offset,
                size: u64::from(c.size),
            }));
            keys.extend(r.loose_candidates.iter().map(|c| LocationKey {
                path: c.path.clone(),
                offset: 0,
                size: c.size,
            }));
        }
    }
    if hashes.len() != 1 || keys.len() > 1 {
        return Err("Ambiguous: conflicting linked resource candidates".into());
    }
    if unavailable || keys.is_empty() {
        return Err("Unresolved: linked bytes unavailable".into());
    }
    Ok(keys.into_iter().next().unwrap())
}
pub fn load(
    source: &AttachedSource,
    catalog: &Catalog,
    geometry: &Geometry,
    mesh_bytes: &[u8],
    control: &LoadControl,
) -> Result<MaterialAsset, String> {
    control.check()?;
    if geometry.metadata.primitives.len() != 1 {
        return Err("Unsupported: textured slice requires one primitive".into());
    }
    load_primitive(
        source,
        catalog,
        geometry,
        mesh_bytes,
        &geometry.metadata.primitives[0],
        control,
        crate::PreviewOptions::default(),
    )
}
pub(crate) fn load_primitive(
    source: &AttachedSource,
    catalog: &Catalog,
    geometry: &Geometry,
    mesh_bytes: &[u8],
    primitive: &lbp_mesh::Primitive,
    control: &LoadControl,
    options: crate::PreviewOptions,
) -> Result<MaterialAsset, String> {
    control.check()?;
    if primitive.material_flags != 0
        || primitive.alternatives_kind != 0
        || primitive.alternatives_flags != 0
    {
        return Err("Unsupported: material flags or alternative materials".into());
    }
    let material_key = resolve(
        catalog,
        primitive.material_guid,
        primitive.material_hash.as_deref(),
    )?;
    if material_key.size > lbp_resource::material::MAX_INPUT as u64 {
        return Err("Material input budget exceeded".into());
    }
    let mut bytes = Vec::with_capacity(material_key.size as usize);
    while bytes.len() < material_key.size as usize {
        control.check()?;
        let count = (material_key.size as usize - bytes.len()).min(4096);
        let page = source
            .read_hex_checked(&material_key, bytes.len() as u64, count, control)?
            .bytes;
        if page.len() != count {
            return Err("Short material page".into());
        }
        bytes.extend(page);
    }
    let mut graph = lbp_resource::material::inspect_graph(&bytes, || control.check().is_err())
        .map_err(|e| e.to_string())?;
    let original_graph = graph.graph.clone();
    let instance_projection = if let Some(color) = options.instance_color {
        let (derived, projection) = crate::project_instance_color(&graph.graph, color)?;
        graph.graph = derived;
        projection
    } else {
        None
    };
    if let Some(selection) = crate::select_solid_diffuse(&graph.graph)? {
        control.check()?;
        source.check_stable()?;
        return Ok(MaterialAsset::Solid(Box::new(SolidAsset {
            material_key,
            selection,
            instance_projection,
            additional_texture_slots: crate::instance::additional_texture_slots(
                &original_graph,
                &[],
            )?,
        })));
    }
    let (descriptor, wrap, mut selection) = binding(&graph, options.animation_base)?;
    if selection.animation_base.is_some() {
        let tail = lbp_resource::material::inspect_tail(&bytes, || control.check().is_err())
            .map_err(|e| e.to_string())?;
        crate::animation_base::verify_global(
            &graph.graph,
            &mut selection,
            &tail.tail.parameter_animations,
        )?;
    }
    let displayed_slots = [
        Some(selection.slot),
        selection.second_sample.map(|s| s.slot),
        selection.third_sample.map(|s| s.slot),
        selection.fourth_sample.map(|s| s.slot),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let additional_texture_slots =
        crate::instance::additional_texture_slots(&original_graph, &displayed_slots)?;
    if let Some(tint) = instance_projection.as_ref().and_then(|p| p.nested_tint) {
        if selection.second_sample.is_none()
            || selection.tint != crate::DiffuseTint::default()
            || selection.third_sample.is_some()
        {
            return Err(
                "Unsupported: instance nested tint needs exactly two direct samples".into(),
            );
        }
        selection.tint = tint;
    }
    let coordinates =
        lbp_mesh::decode_uvs(mesh_bytes, || control.check().is_err()).map_err(|e| e.to_string())?;
    let uv = selected_uv(&coordinates, selection.uv_channel, geometry.positions.len())?;
    let second_wrap = selection
        .second_sample
        .map(|second| {
            let prefix = &graph.material.prefix;
            crate::TextureWrapping::from_raw(
                prefix.wrap_s_raw.get(second.slot).copied(),
                prefix.wrap_t_raw.get(second.slot).copied(),
            )
        })
        .transpose()?;
    let use_grid = grid_with_wrapping(selection, wrap, second_wrap)
        && instance_projection
            .as_ref()
            .is_none_or(|p| p.alpha_subtract_samplers.is_empty() && p.nested_tint.is_none());
    if selection.blend.is_some() && selection.second_sample.is_some() && !use_grid {
        return Err("Unsupported: product MIX requires Repeat grid".into());
    }
    let mut sampling_uv = if use_grid {
        crate::SamplingTransform::default()
    } else {
        selection.uv_transform
    }
    .sampling(&uv, || control.check().is_err())?;
    crate::instance::subtract_alpha(
        &mut sampling_uv,
        selection.sample_index,
        instance_projection.as_ref(),
        || control.check().is_err(),
    )?;
    let border_domain = crate::BorderDomain::confirm(
        [
            graph
                .material
                .prefix
                .wrap_s_raw
                .get(selection.slot)
                .copied(),
            graph
                .material
                .prefix
                .wrap_t_raw
                .get(selection.slot)
                .copied(),
        ],
        &sampling_uv,
        || control.check().is_err(),
    )?;
    let texture_key = resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
    let texture = lbp_texture::decode_reader(
        texture_key.size,
        |offset, count| {
            source
                .read_hex_checked(&texture_key, offset, count, control)
                .map(|p| p.bytes)
                .map_err(lbp_texture::Error::Input)
        },
        || control.check().is_err(),
    )
    .map_err(|e| e.to_string())?;
    let tinted_rgba = if let Some(mix) = selection.alpha_mix {
        Some(mix.pixels(&texture.rgba, || control.check().is_err())?)
    } else if let Some(blend) = selection
        .blend
        .filter(|_| selection.second_sample.is_none())
    {
        blend.pixels(&texture.rgba, || control.check().is_err())?
    } else {
        selection
            .tint
            .pixels(&texture.rgba, || control.check().is_err())?
    };
    let mut dual = None;
    let product = if let Some(second) = selection.second_sample {
        if selection.third_sample.is_some() && wrap != crate::TextureWrapping::uniform(true) {
            return Err("Unsupported: texture product requires Repeat on both axes".into());
        }
        let prefix = &graph.material.prefix;
        let descriptor = prefix
            .textures
            .get(second.slot)
            .ok_or("Unsupported: second texture slot")?;
        if descriptor.kind == 0
            || descriptor.flags != 0
            || (selection.third_sample.is_some()
                && second_wrap != Some(crate::TextureWrapping::uniform(true)))
        {
            return Err(
                "Unsupported: texture product requires present unflagged Repeat texture".into(),
            );
        }
        let second_key = resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
        let second_texture = lbp_texture::decode_reader(
            second_key.size,
            |offset, count| {
                source
                    .read_hex_checked(&second_key, offset, count, control)
                    .map(|p| p.bytes)
                    .map_err(lbp_texture::Error::Input)
            },
            || control.check().is_err(),
        )
        .map_err(|e| e.to_string())?;
        if use_grid {
            let (width, height, rgba) = if let Some(blend) = selection.blend {
                crate::product::pixels_blended(
                    &texture,
                    &second_texture,
                    selection.uv_transform,
                    second.uv_transform,
                    blend,
                    || control.check().is_err(),
                )?
            } else {
                let (width, height, rgba) = crate::product::pixels(
                    &texture,
                    &second_texture,
                    selection.uv_transform,
                    second.uv_transform,
                    || control.check().is_err(),
                )?;
                // D75: static COLOR of a product of a product, applied to the baked grid.
                let rgba = selection
                    .tint
                    .pixels(&rgba, || control.check().is_err())?
                    .unwrap_or(rgba);
                (width, height, rgba)
            };
            Some(Box::new(ProductAsset {
                texture: second_texture,
                texture_key: second_key,
                width,
                height,
                rgba,
            }))
        } else {
            let raw_second =
                selected_uv(&coordinates, second.uv_channel, geometry.positions.len())?;
            let mut sampling_uv = second
                .uv_transform
                .sampling(&raw_second, || control.check().is_err())?;
            crate::instance::subtract_alpha(
                &mut sampling_uv,
                second.sample_index,
                instance_projection.as_ref(),
                || control.check().is_err(),
            )?;
            dual = Some(Box::new(DualAsset {
                texture: second_texture,
                texture_key: second_key,
                sampling_uv,
                wrap: second_wrap.ok_or("Missing second texture wrapping")?,
            }));
            None
        }
    } else {
        None
    };
    let third = selection
        .third_sample
        .map(|s| {
            let subtracted = instance_projection
                .as_ref()
                .map_or(&[][..], |p| &p.alpha_subtract_samplers[..]);
            let relative = relative_third(&selection, subtracted)?;
            let mut shared = load_shared_sample(source, catalog, &graph, s.slot, control)?;
            shared.relative = Some(relative);
            Ok::<_, String>(shared)
        })
        .transpose()?;
    let fourth = selection
        .fourth_sample
        .map(|s| load_shared_sample(source, catalog, &graph, s.slot, control))
        .transpose()?;
    control.check()?;
    source.check_stable()?;
    Ok(MaterialAsset::Texture(Box::new(Asset {
        instance_projection,
        additional_texture_slots,
        border_domain,
        fourth,
        third,
        dual,
        product,
        tinted_rgba,
        sampling_uv,
        uv,
        uv_metadata: coordinates.metadata,
        texture,
        material_key,
        texture_key,
        wrap,
        selection,
    })))
}

// Clone the selected raw coordinates; never mutate, reorder or substitute another channel.
fn selected_uv(
    coordinates: &lbp_mesh::UvCoordinates,
    channel: usize,
    vertices: usize,
) -> Result<Vec<[f32; 2]>, String> {
    if channel > 1 {
        return Err("Unsupported: diffuse UV channel outside UV0/UV1".into());
    }
    let uv = coordinates
        .channels
        .get(channel)
        .ok_or_else(|| format!("Unsupported: mesh has no requested UV{channel}"))?;
    if uv.len() != vertices || uv.iter().flatten().any(|v| !v.is_finite()) {
        return Err("Invalid: selected UV/vertex correspondence".into());
    }
    Ok(uv.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diffuse_projection_requires_bounded_static_routes_and_labels_ignored_branches() {
        use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};
        let node = |kind| BoxMetadata {
            type_raw: kind,
            declared_parameter_count: 8,
            parameters_raw: [0; 8],
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
        };
        let mut graph = GraphMetadata {
            box_count: 2,
            wire_count: 1,
            boxes: vec![node(0), node(1)],
            wires: vec![WireMetadata {
                box_from: 1,
                box_to: 0,
                port_from_raw: 0,
                port_to_raw: 0,
                swizzle_raw: [0; 5],
                from_in_range: true,
                to_in_range: true,
            }],
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        };
        graph.boxes[1].parameters_raw = [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, 2, 0, 0];
        assert_eq!(sample_slot(&graph).unwrap(), 2);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            graph.boxes[1].parameters_raw[2] = value.to_bits();
            assert!(select_diffuse(&graph).is_err());
        }
        for value in [f32::MIN_POSITIVE, -1.] {
            graph.boxes[1].parameters_raw[2] = value.to_bits();
            assert!(select_diffuse(&graph).is_ok());
        }
        graph.boxes[1].parameters_raw[2] = 0;
        for kind in [3, 10, 12] {
            graph.boxes[1].type_raw = kind;
            assert!(select_diffuse(&graph).is_err());
        }
        graph.boxes[1].type_raw = 1;
        graph.boxes[1].subtype_raw = 1;
        graph.boxes[1].parameters_raw = [1f32.to_bits(), 0, 0, 0, 0, 2, 1f32.to_bits(), 0];
        assert_eq!(select_diffuse(&graph).unwrap().uv_transform.subtype(), 1);
        graph.boxes[1].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[1].animations[0].component_value_count = 0;
        graph.boxes[1].animations[0].name_hex = "41".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[1].animations[0].name_hex.clear();
        graph.boxes[1].subtype_raw = 2;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[1].subtype_raw = 0;
        graph.boxes[1].parameters_raw = [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, 2, 0, 0];
        graph.wires[0].port_from_raw = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.wires[0].port_from_raw = 0;
        graph.boxes[0].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[0].animations[0].component_value_count = 0;
        graph.boxes[1].animations[0].name_hex = "410000".into();
        assert!(sample_slot(&graph).is_err());
        graph.boxes[1].animations[0].name_hex.clear();
        graph.boxes[1].parameters_raw[4] = 1;
        assert_eq!(select_diffuse(&graph).unwrap().uv_channel, 1);
        graph.boxes[1].parameters_raw[4] = 2;
        assert!(sample_slot(&graph).is_err());
        graph.boxes[1].parameters_raw[4] = 0;
        graph.wires[0].swizzle_raw[0] = b'x';
        assert!(sample_slot(&graph).is_err());
        graph.wires[0].swizzle_raw = [0; 5];
        graph.wires[0].box_from = -1;
        assert!(sample_slot(&graph).is_err());
        graph.wires[0].box_from = 1;
        graph.wires[0].port_to_raw = 1;
        assert!(sample_slot(&graph).is_err());
        graph.wires[0].port_to_raw = 0;
        graph.boxes.push(node(1));
        assert_eq!(sample_slot(&graph).unwrap(), 2);
        graph.boxes[1].parameters_raw[3] = (-0f32).to_bits();
        assert_eq!(sample_slot(&graph).unwrap(), 2);
        let choice = select_diffuse(&graph).unwrap();
        assert_eq!((choice.ignored_nodes, choice.ignored_wires), (1, 0));
        // Unrelated graph cycles/animations are not evaluated; selected sampler inputs reject.
        graph.boxes[2].animations[0].name_hex = "41".into();
        let wire = |from, to, port| WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw: port,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        };
        graph.wires.push(wire(2, 2, 9));
        assert_eq!(select_diffuse(&graph).unwrap().ignored_wires, 1);
        graph.wires[1].box_to = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.wires.push(wire(1, 0, 0));
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.boxes[2].type_raw = 0;
        assert!(select_diffuse(&graph).is_err());

        // One bounded multiply: texture × unwired static color in either input order.
        graph.boxes[2] = node(3);
        graph.boxes[2].parameters_raw[..4].copy_from_slice(&[0.5, 1., 0.25, 1.].map(f32::to_bits));
        graph.boxes.push(node(10));
        graph.wires[0].box_from = 3;
        graph.wires.extend([wire(1, 3, 0), wire(2, 3, 1)]);
        let s = select_diffuse(&graph).unwrap();
        assert_eq!(
            (s.sample_index, s.multiply_index, s.color_index),
            (1, Some(3), Some(2))
        );
        assert_eq!(s.tint.values(), [0.5, 1., 0.25, 1.]);
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        graph.wires[1].box_from = 2;
        graph.wires[2].box_from = 1;
        assert_eq!(select_diffuse(&graph).unwrap().tint, s.tint);
        graph.wires[2].port_to_raw = 0;
        assert!(select_diffuse(&graph).is_err());
        graph.wires[2].port_to_raw = 1;
        graph.wires[1].swizzle_raw[0] = b'r';
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].swizzle_raw = [0; 5];
        graph.boxes[2].animations[0].name_hex = "41".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].animations[0].name_hex = "00".into();
        graph.boxes[3].animations[1].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[3].animations[1].component_value_count = 0;
        graph.wires.push(wire(1, 2, 0));
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.wires[1].box_from = 1;
        // D64: the same sample on both ports is supported as that sample squared.
        let same = select_diffuse(&graph).unwrap();
        assert_eq!(same.second_sample.unwrap().sample_index, same.sample_index);
        graph.wires[1].box_from = 3;
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].box_from = 2;
        graph.boxes[2].parameters_raw[0] = f32::NAN.to_bits();
        assert!(select_diffuse(&graph).is_err());
        // Two direct textures with independent integer Repeat scales on one channel.
        graph.boxes[2] = node(1);
        graph.boxes[2].parameters_raw = [3f32.to_bits(), 2f32.to_bits(), 0, 0, 0, 1, 0, 0];
        let product = select_diffuse(&graph).unwrap();
        assert!(uses_grid(product));
        let repeat = crate::TextureWrapping::uniform(true);
        let mirror = crate::TextureWrapping::from_raw(Some(2), Some(1)).unwrap();
        assert!(grid_with_wrapping(product, repeat, Some(repeat)));
        assert!(!grid_with_wrapping(product, repeat, Some(mirror)));
        assert!(!grid_with_wrapping(product, mirror, Some(repeat)));
        assert!(!grid_with_wrapping(product, repeat, None));
        assert_eq!(product.second_sample.unwrap().slot, 2);
        assert!(product.color_index.is_none());
        graph.boxes[1].parameters_raw[4] = 1;
        let pair = select_diffuse(&graph).unwrap();
        assert_eq!(
            (pair.uv_channel, pair.second_sample.unwrap().uv_channel),
            (0, 1)
        );
        assert!(!uses_grid(pair)); // Equal/integer transforms cannot bake different raw channels.
        graph.boxes[1].parameters_raw[4] = 0;
        graph.boxes[1].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[1].animations[0].component_value_count = 0;
        graph.boxes[2].parameters_raw[0] = 0.5f32.to_bits();
        assert!(select_diffuse(&graph).is_ok()); // independent sampling, not grid eligibility
        graph.boxes[2].subtype_raw = 1;
        graph.boxes[2].parameters_raw = [
            2f32.to_bits(),
            3f32.to_bits(),
            0.5f32.to_bits(),
            (-4f32).to_bits(),
            0,
            1,
            5f32.to_bits(),
            (-0.25f32).to_bits(),
        ];
        let matrix_route = select_diffuse(&graph).unwrap();
        assert_eq!(
            matrix_route.uv_transform.bits(),
            &[2., 3., 0.5, -4., 5., -0.25].map(f32::to_bits)
        );
        assert!(matrix_route.second_sample.is_some());
        graph.boxes[2].parameters_raw[6] = f32::NAN.to_bits();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].type_raw = 10;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2] = node(3);
        // Static solid color is supported without sampler/UV/texture placeholders.
        graph.boxes[2].parameters_raw[..4]
            .copy_from_slice(&[0.25, 0.5, 1., 0.75].map(f32::to_bits));
        graph.wires = vec![wire(2, 0, 0)];
        let solid = crate::select_solid_diffuse(&graph).unwrap().unwrap();
        let material = MaterialAsset::Solid(Box::new(SolidAsset {
            instance_projection: None,
            additional_texture_slots: vec![],
            material_key: LocationKey {
                path: "authored-material".into(),
                offset: 0,
                size: 1,
            },
            selection: solid,
        }));
        assert!(material.texture().is_none());
        assert!(material.sampling_uv().is_none());
        assert_eq!(solid.color.values(), [0.25, 0.5, 1., 0.75]);
        assert_eq!(
            (solid.color_index, solid.ignored_nodes, solid.ignored_wires),
            (2, 2, 0)
        );
        graph.boxes[2].animations[0].component_value_count = 1;
        assert!(crate::select_solid_diffuse(&graph).is_err());
        graph.boxes[2].animations[0].component_value_count = 0;
        graph.wires.push(wire(2, 0, 0));
        assert!(crate::select_solid_diffuse(&graph).is_err());
        graph.wires.pop();
        assert!(!solid.unwired_diffuse);
        // No diffuse wire: explicit zero, other channels ignored, never a texture.
        let mut unwired = graph.clone();
        unwired.wires = vec![wire(2, 0, 4), wire(1, 0, 6)];
        let black = crate::select_solid_diffuse(&unwired).unwrap().unwrap();
        assert!(black.unwired_diffuse);
        assert_eq!(black.color.bits, [0; 4]);
        assert_eq!(
            (black.color_index, black.ignored_nodes, black.ignored_wires),
            (black.output_index, unwired.boxes.len() - 1, 2)
        );
        let (projected, p) =
            crate::project_instance_color(&unwired, crate::DiffuseTint { bits: [0; 4] }).unwrap();
        assert!(p.is_none());
        assert_eq!(projected.wires.len(), 2);
        unwired.boxes[0].parameters_raw[0] = 1;
        assert!(crate::select_solid_diffuse(&unwired).is_err());
        unwired.boxes[0].parameters_raw[0] = 0;
        unwired.boxes[1].type_raw = 130;
        assert!(crate::select_solid_diffuse(&unwired).is_err());
        unwired.boxes[1].type_raw = 1;
        unwired.boxes[0].animations[0].name_hex = "623100".into();
        assert!(crate::select_solid_diffuse(&unwired).is_err());
        // One sample feeding both multiply ports is that sample squared.
        let mut square = graph.clone();
        square.boxes = vec![node(0), node(1), node(10)];
        square.wires = vec![wire(1, 2, 0), wire(1, 2, 1), wire(2, 0, 0)];
        let squared = select_diffuse(&square).unwrap();
        let second = squared.second_sample.unwrap();
        assert_eq!((squared.sample_index, second.sample_index), (1, 1));
        assert_eq!(second.uv_transform, squared.uv_transform);
        assert_eq!((squared.ignored_nodes, squared.ignored_wires), (0, 0));
        square.wires[1].swizzle_raw[0] = b'a';
        assert!(select_diffuse(&square).is_err());
        square.wires[1].swizzle_raw[0] = 0;
        square.wires[1].port_to_raw = 0;
        assert!(select_diffuse(&square).is_err());
        graph.wires.push(wire(1, 2, 0));
        assert!(crate::select_solid_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.wires[0].swizzle_raw[0] = b'r';
        assert!(crate::select_solid_diffuse(&graph).is_err());
        graph.wires[0].swizzle_raw = [0; 5];
        graph.boxes[2].parameters_raw[0] = f32::INFINITY.to_bits();
        assert!(crate::select_solid_diffuse(&graph).is_err());
        graph.boxes[2].type_raw = 2; // Instance-dependent ThingColor remains unsupported.
        assert!(crate::select_solid_diffuse(&graph).unwrap().is_none());
        graph.wires[0].box_from = 1;
        assert!(crate::select_solid_diffuse(&graph).unwrap().is_none());
        // Exactly two multiplies/three distinct static samplers; third shares UVs with second.
        graph.boxes = vec![node(0), node(1), node(1), node(10), node(1), node(10)];
        for i in [1, 2, 4] {
            graph.boxes[i].parameters_raw =
                [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, i as u32, 0, 0];
        }
        graph.boxes[1].parameters_raw[4] = 1;
        graph.wires = vec![
            wire(1, 3, 0),
            wire(2, 3, 1),
            wire(3, 5, 0),
            wire(4, 5, 1),
            wire(5, 0, 0),
        ];
        let triple = select_diffuse(&graph).unwrap();
        assert_eq!(
            (
                triple.sample_index,
                triple.second_sample.unwrap().sample_index,
                triple.third_sample.unwrap().sample_index
            ),
            (1, 2, 4)
        );
        assert_eq!(
            (triple.inner_multiply_index, triple.multiply_index),
            (Some(3), Some(5))
        );
        assert_eq!((triple.ignored_nodes, triple.ignored_wires), (0, 0));
        assert!(!uses_grid(triple));
        graph.wires.reverse();
        assert_eq!(select_diffuse(&graph).unwrap().sample_index, 1);
        graph.wires[1].port_to_raw = 0;
        graph.wires[2].port_to_raw = 1;
        assert!(select_diffuse(&graph).is_ok()); // either outer input order
        graph.wires[1].port_to_raw = 1;
        graph.wires[2].port_to_raw = 0;
        graph.wires[1].port_to_raw = 0; // duplicate root port
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].port_to_raw = 1;
        // D65: a third transform differing from the second (here only by -0) is an
        // independent scale/offset re-derived from coordinates on its channel.
        graph.boxes[4].parameters_raw[3] = (-0f32).to_bits();
        let independent = select_diffuse(&graph).unwrap();
        let r = relative_third(&independent, &[]).unwrap();
        // The base is whichever displayed sample shares the channel (box 2 on UV0).
        assert!(r.is_identity());
        assert_eq!(r.from_primary, independent.sample_index == 2);
        assert!(relative_third(&independent, &[4]).is_err());
        graph.boxes[4].parameters_raw[..4]
            .copy_from_slice(&[4f32, 2., 0.5, 0.25].map(f32::to_bits));
        graph.boxes[2].parameters_raw[..4].copy_from_slice(&[2f32, 4., 0.25, 0.].map(f32::to_bits));
        let selected = select_diffuse(&graph).unwrap();
        let r = relative_third(&selected, &[]).unwrap();
        // raw = (b - (0.25, 0)) / (2, 4); third = raw × (4, 2) + (0.5, 0.25)
        assert_eq!(
            (r.from_primary, r.scale, r.offset),
            (selected.sample_index == 2, [2., 0.5], [0., 0.25])
        );
        graph.boxes[4].parameters_raw[4] = 1; // UV1: only box 1 shares it
        let selected = select_diffuse(&graph).unwrap();
        let r = relative_third(&selected, &[]).unwrap();
        assert_eq!(
            (r.from_primary, r.scale, r.offset),
            (selected.sample_index == 1, [4., 2.], [0.5, 0.25])
        );
        graph.boxes[4].parameters_raw[4] = 0;
        graph.boxes[2].parameters_raw[0] = 0; // zero base scale is not invertible
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].parameters_raw[..4].copy_from_slice(&[1f32, 1., 0., 0.].map(f32::to_bits));
        // D67: both inner leaves on UV0 and the outer leaf on UV1: the second and
        // third roles swap so displayed coordinates cover both channels.
        let mut split = graph.clone();
        split.boxes[1].parameters_raw[4] = 0;
        split.boxes[4].parameters_raw[4] = 1;
        let swapped = select_diffuse(&split).unwrap();
        assert_eq!(swapped.second_sample.unwrap().sample_index, 4);
        assert_eq!(swapped.second_sample.unwrap().uv_channel, 1);
        let moved = swapped.third_sample.unwrap();
        assert_eq!(moved.uv_channel, 0);
        assert!([1, 2].contains(&moved.sample_index));
        assert_ne!(moved.sample_index, swapped.sample_index);
        assert!(relative_third(&swapped, &[]).unwrap().from_primary);
        graph.boxes[4].subtype_raw = 1; // matrix third stays unsupported unless shared
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[4].subtype_raw = 0;
        graph.boxes[4].parameters_raw[..4].copy_from_slice(&[1f32, 1., 0., 0.].map(f32::to_bits));
        graph.boxes[4].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[4].animations[0].component_value_count = 0;
        graph.wires[3].box_from = 5; // cycle through outer multiply
        assert!(select_diffuse(&graph).is_err());
        graph.wires[3].box_from = 2;
        graph.boxes[2].type_raw = 10; // deeper nesting
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].type_raw = 1;
        graph.wires[1].box_from = 2; // shared leaf
        assert!(select_diffuse(&graph).is_err());
        // Four-leaf chain: first/fourth UV1, second/third UV0.
        graph.boxes = vec![
            node(0),
            node(1),
            node(1),
            node(10),
            node(1),
            node(10),
            node(1),
            node(10),
        ];
        for i in [1, 2, 4, 6] {
            graph.boxes[i].parameters_raw =
                [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, i as u32, 0, 0];
        }
        graph.boxes[1].parameters_raw[4] = 1;
        graph.boxes[6].parameters_raw[4] = 1;
        graph.wires = vec![
            wire(1, 3, 0),
            wire(2, 3, 1),
            wire(3, 5, 0),
            wire(4, 5, 1),
            wire(5, 7, 0),
            wire(6, 7, 1),
            wire(7, 0, 0),
        ];
        let four = select_diffuse(&graph).unwrap();
        assert_eq!(
            (
                four.sample_index,
                four.second_sample.unwrap().sample_index,
                four.third_sample.unwrap().sample_index,
                four.fourth_sample.unwrap().sample_index
            ),
            (1, 2, 4, 6)
        );
        assert_eq!(
            (
                four.multiply_index,
                four.inner_multiply_index,
                four.deep_multiply_index
            ),
            (Some(7), Some(5), Some(3))
        );
        assert_eq!((four.ignored_nodes, four.ignored_wires), (0, 0));
        assert!(!uses_grid(four));
        graph.wires.reverse();
        assert_eq!(select_diffuse(&graph).unwrap(), four);
        for w in &mut graph.wires {
            if w.box_to != 0 {
                w.port_to_raw = 1 - w.port_to_raw;
            }
        }
        assert_eq!(select_diffuse(&graph).unwrap(), four);
        graph.boxes[6].parameters_raw[0] = 2f32.to_bits();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[6].parameters_raw[0] = 1f32.to_bits();
        graph.boxes[6].parameters_raw[3] = (-0f32).to_bits();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[6].parameters_raw[3] = 0;
        graph.boxes[6].animations[0].name_hex = "41".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[6].animations[0].name_hex = "00".into();
        graph.wires[1].port_from_raw = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].port_from_raw = 0;
        graph.wires[1].box_from = 1; // duplicate fourth/first leaf
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].box_from = 6;
        graph.wires[4].box_from = 7; // cycle instead of deepest multiply
        assert!(select_diffuse(&graph).is_err());
        graph.boxes = vec![node(0), node(3), node(1), node(1), node(10), node(12)];
        graph.boxes[1].parameters_raw[..4].copy_from_slice(&[0.25, 0.5, 1., 1.].map(f32::to_bits));
        for i in [2, 3] {
            graph.boxes[i].parameters_raw =
                [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, i as u32, 0, 0];
        }
        graph.boxes[3].parameters_raw[0] = 3f32.to_bits();
        graph.boxes[5].parameters_raw[0] = 0.6f32.to_bits();
        graph.wires = vec![
            wire(2, 4, 0),
            wire(3, 4, 1),
            wire(1, 5, 0),
            wire(4, 5, 1),
            wire(5, 0, 0),
        ];
        let mixed = select_diffuse(&graph).unwrap();
        assert!(uses_grid(mixed));
        assert_eq!(
            (
                mixed.sample_index,
                mixed.second_sample.unwrap().sample_index,
                mixed.multiply_index,
                mixed.color_index
            ),
            (2, 3, Some(4), Some(1))
        );
        assert_eq!(mixed.blend.unwrap().weight_bits, 0.6f32.to_bits());
        assert_eq!((mixed.ignored_nodes, mixed.ignored_wires), (0, 0));
        graph.wires.reverse();
        assert_eq!(select_diffuse(&graph).unwrap(), mixed);
        graph.wires[1].port_to_raw = 0;
        graph.wires[2].port_to_raw = 1;
        assert!(!select_diffuse(&graph).unwrap().blend.unwrap().color_first);
        graph.boxes[5].parameters_raw[0] = f32::NAN.to_bits();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[5].parameters_raw[0] = 0.6f32.to_bits();
        graph.wires.push(wire(2, 5, 2));
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.boxes[3].parameters_raw[4] = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[3].parameters_raw[4] = 0;
        graph.boxes[1].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[1].animations[0].component_value_count = 0;
        graph.wires[1].swizzle_raw[0] = b'r';
        assert!(select_diffuse(&graph).is_err());
        graph.wires[1].swizzle_raw = [0; 5];
        graph.wires[3].box_from = 5;
        assert!(select_diffuse(&graph).is_err());
        // Constant COLOR/single-sample MIX supports independent sampler transforms.
        graph.boxes = vec![node(0), node(3), node(1), node(12)];
        graph.boxes[1].parameters_raw[..4].copy_from_slice(&[0.25, 0.5, 1., 1.].map(f32::to_bits));
        graph.boxes[2].subtype_raw = 1;
        graph.boxes[2].parameters_raw = [
            2f32.to_bits(),
            3f32.to_bits(),
            0.5f32.to_bits(),
            (-4f32).to_bits(),
            1,
            7,
            5f32.to_bits(),
            (-0f32).to_bits(),
        ];
        graph.boxes[3].parameters_raw[0] = 0.25f32.to_bits();
        graph.wires = vec![wire(1, 3, 0), wire(2, 3, 1), wire(3, 0, 0)];
        let single = select_diffuse(&graph).unwrap();
        assert_eq!(
            (single.slot, single.uv_channel, single.color_index),
            (7, 1, Some(1))
        );
        assert!(
            single.second_sample.is_none() && single.multiply_index.is_none() && !uses_grid(single)
        );
        assert_eq!((single.ignored_nodes, single.ignored_wires), (0, 0));
        graph.wires.reverse();
        assert_eq!(select_diffuse(&graph).unwrap(), single);
        graph.wires[1].port_to_raw = 0;
        graph.wires[2].port_to_raw = 1;
        assert!(!select_diffuse(&graph).unwrap().blend.unwrap().color_first);
        graph.wires.push(wire(2, 3, 2));
        graph.wires.last_mut().unwrap().swizzle_raw[0] = b'a';
        let alpha = select_diffuse(&graph).unwrap();
        assert!(alpha.alpha_mix.is_some() && alpha.blend.is_none());
        assert_eq!((alpha.ignored_nodes, alpha.ignored_wires), (0, 0));
        graph.boxes[3].parameters_raw[0] = f32::NAN.to_bits();
        assert_eq!(
            select_diffuse(&graph)
                .unwrap()
                .alpha_mix
                .unwrap()
                .unused_weight_bits,
            f32::NAN.to_bits()
        );
        graph.wires.reverse();
        assert!(select_diffuse(&graph).is_ok());
        graph.wires[0].swizzle_raw[0] = b'r';
        assert!(select_diffuse(&graph).is_err());
        graph.wires[0].swizzle_raw[0] = b'a';
        graph.wires[0].box_from = 1;
        assert!(select_diffuse(&graph).is_err());
        graph.wires.reverse();
        graph.wires.pop();
        graph.boxes[3].parameters_raw[0] = 0.25f32.to_bits();
        graph.boxes[2].animations[0].name_hex = "623100".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].animations[0].name_hex = "000000".into();
        graph.wires.push(wire(1, 2, 0));
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.boxes[1].type_raw = 2;
        assert!(select_diffuse(&graph).is_err());
        // Opt-in stored local base rows, not a sampled game time or static fallback.
        graph.boxes = vec![node(0), node(1)];
        graph.boxes[1].subtype_raw = 1;
        graph.boxes[1].parameters_raw = [1f32.to_bits(), 0, 0, 0, 1, 2, 1f32.to_bits(), 0];
        for (i, name) in ["623100", "633100"].into_iter().enumerate() {
            let a = &mut graph.boxes[1].animations[i];
            a.name_hex = name.into();
            a.components_raw = 7;
            a.component_value_count = 3;
            a.derived_key_count = 1;
        }
        let rows = [[2., 3., 0.5, -0.], [-4., 5., -0.25, 0.]].map(|r| r.map(f32::to_bits));
        for (a, row) in graph.boxes[1].animations.iter_mut().zip(rows) {
            a.base_value_bits = row;
        }
        graph.wires = vec![wire(1, 0, 0)];
        assert!(select_diffuse_with_animation_base(&graph, false).is_err());
        let snapshot = select_diffuse_with_animation_base(&graph, true).unwrap();
        assert_eq!(snapshot.animation_base.unwrap().row_bits, rows);
        let mut verified = snapshot;
        let globals = graph.boxes[1].animations.to_vec();
        crate::animation_base::verify_global(&graph, &mut verified, &globals).unwrap();
        assert!(verified.animation_base.unwrap().global_rows_verified);
        let mut duplicate = globals.clone();
        duplicate.push(globals[0].clone());
        assert!(crate::animation_base::verify_global(&graph, &mut verified, &duplicate).is_err());
        assert!(
            crate::animation_base::verify_global(&graph, &mut verified, &globals[..1]).is_err()
        );
        let mut conflict = globals.clone();
        conflict[0].base_value_bits[0] ^= 1;
        assert!(crate::animation_base::verify_global(&graph, &mut verified, &conflict).is_err());
        conflict = globals.clone();
        conflict[0].component_bytes_sha256.push('f');
        assert!(crate::animation_base::verify_global(&graph, &mut verified, &conflict).is_err());

        assert_eq!(
            snapshot.animation_base.unwrap().static_matrix_bits,
            [1.0f32.to_bits(), 0, 0, 0, 1.0f32.to_bits(), 0]
        );
        assert_eq!(snapshot.uv_channel, 1);
        assert_eq!(
            snapshot
                .uv_transform
                .sampling(&[[1., -0.]], || false)
                .unwrap(),
            [[2.5, -4.25]]
        );
        assert_eq!(graph.boxes[1].parameters_raw[0], 1f32.to_bits());
        graph.boxes[1].animations[0].base_value_bits[0] = f32::NAN.to_bits();
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[0].base_value_bits = rows[0];
        graph.boxes[1].animations[0].components_raw = 0x87;
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[0].components_raw = 7;
        graph.boxes[1].animations[0].name_hex = "zz3100".into();
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[0].name_hex = "623100".into();
        graph.wires.push(wire(0, 1, 0));
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.wires.pop();
        graph.wires[0].box_from = -1;
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());

        // Subtype 0: one masked curve, base (scale X/Y, subtracted offset X/Y).
        graph.wires[0].box_from = 1;
        graph.boxes[1].subtype_raw = 0;
        graph.boxes[1].parameters_raw = [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, 0, 0, 0];
        graph.boxes[1].animations[1] = node(1).animations[1].clone();
        let a = &mut graph.boxes[1].animations[0];
        (
            a.components_raw,
            a.component_value_count,
            a.derived_key_count,
        ) = (8, 5, 5);
        a.base_value_bits = [2., 4., 0.5, -3.].map(f32::to_bits);
        let base = select_diffuse_with_animation_base(&graph, true).unwrap();
        assert!(select_diffuse(&graph).is_err());
        assert_eq!(base.uv_transform.subtype(), 0);
        assert_eq!(base.animation_base.unwrap().subtype_raw, 0);
        assert_eq!(
            base.uv_transform
                .sampling(&[[1., -0.5], [0.5, 3.]], || false)
                .unwrap(),
            [[1., 10.], [0., 24.]]
        );
        let mut verified = base;
        let globals = graph.boxes[1].animations[..1].to_vec();
        crate::animation_base::verify_global(&graph, &mut verified, &globals).unwrap();
        assert!(verified.animation_base.unwrap().global_rows_verified);
        assert!(crate::animation_base::verify_global(&graph, &mut verified, &[]).is_err());
        // Uneven lanes, a populated second binding, or wired inputs stay unsupported.
        graph.boxes[1].animations[0].components_raw = 3;
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[0].components_raw = 0x18;
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[0].components_raw = 8;
        graph.boxes[1].animations[1].name_hex = "633100".into();
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[1].name_hex = "000000".into();
        graph.boxes[1].animations[1].base_value_bits[0] = 1;
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.boxes[1].animations[1].base_value_bits[0] = 0;
        assert!(select_diffuse_with_animation_base(&graph, true).is_ok());
        graph.wires.push(wire(0, 1, 2));
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
        graph.wires.pop();
        graph.boxes[1].animations[0].base_value_bits[0] = f32::INFINITY.to_bits();
        assert!(select_diffuse_with_animation_base(&graph, true).is_err());
    }
    #[test]
    fn two_textures_and_static_color_under_two_multiplies_select_a_tinted_product() {
        use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};
        let node = |kind, slot: u32| {
            let mut parameters_raw = [0; 8];
            if kind == 1 {
                parameters_raw = [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, slot, 0, 0];
            } else if kind == 3 {
                parameters_raw[..4].copy_from_slice(&[0.5f32, 1., 0.25, 1.].map(f32::to_bits));
            }
            BoxMetadata {
                type_raw: kind,
                declared_parameter_count: 8,
                parameters_raw,
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
        };
        let wire = |from, to, port| WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw: port,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        };
        let tint = [0.5f32, 1., 0.25, 1.];
        // (T1 × T2) × COLOR: boxes 1/2 samples, 3 inner, 4 COLOR, 5 root.
        let mut graph = GraphMetadata {
            box_count: 6,
            wire_count: 5,
            boxes: vec![
                node(0, 0),
                node(1, 0),
                node(1, 1),
                node(10, 0),
                node(3, 0),
                node(10, 0),
            ],
            wires: vec![
                wire(1, 3, 0),
                wire(2, 3, 1),
                wire(3, 5, 0),
                wire(4, 5, 1),
                wire(5, 0, 0),
            ],
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        };
        let s = select_diffuse(&graph).unwrap();
        assert_eq!(
            (s.sample_index, s.second_sample.unwrap().sample_index),
            (1, 2)
        );
        assert_eq!(
            (s.multiply_index, s.inner_multiply_index, s.color_index),
            (Some(5), Some(3), Some(4))
        );
        assert_eq!(s.tint.values(), tint);
        assert!(s.third_sample.is_none());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        assert!(uses_grid(s)); // same channel, integer scales: Repeat grid bakes the tint
                               // Either outer input order.
        graph.wires[2].port_to_raw = 1;
        graph.wires[3].port_to_raw = 0;
        assert_eq!(select_diffuse(&graph).unwrap().tint, s.tint);
        // One sample on both inner ports is that sample squared, then tinted.
        graph.wires[1].box_from = 1;
        let squared = select_diffuse(&graph).unwrap();
        assert_eq!(squared.second_sample.unwrap().sample_index, 1);
        assert_eq!((squared.ignored_nodes, squared.ignored_wires), (1, 0));
        graph.wires[1].box_from = 2;
        // Rejections: animated/wired COLOR, swizzled inner edge, COLOR inside the
        // inner product with a second COLOR, animated sampler, UV2.
        graph.boxes[4].animations[0].name_hex = "41".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[4].animations[0].name_hex = "00".into();
        graph.wires.push(wire(2, 4, 0));
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.wires[0].swizzle_raw[0] = b'r';
        assert!(select_diffuse(&graph).is_err());
        graph.wires[0].swizzle_raw = [0; 5];
        graph.boxes[2].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph)
            .unwrap_err()
            .contains("sampler must be static"));
        graph.boxes[2].animations[0].component_value_count = 0;
        graph.boxes[2].parameters_raw[4] = 2;
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2].parameters_raw[4] = 0;
        graph.boxes[2] = node(3, 0);
        assert!(select_diffuse(&graph)
            .unwrap_err()
            .contains("two direct texture samples"));
        graph.boxes[2] = node(1, 1);
        graph.wires[0].box_from = 5; // cycle through the root
        assert!(select_diffuse(&graph).is_err());
        graph.wires[0].box_from = 1;

        // (T1 × COLOR) × T2: the inner COLOR used to reach the sampler rule.
        graph.boxes = vec![
            node(0, 0),
            node(1, 0),
            node(3, 0),
            node(10, 0),
            node(1, 1),
            node(10, 0),
        ];
        graph.wires = vec![
            wire(1, 3, 0),
            wire(2, 3, 1),
            wire(3, 5, 0),
            wire(4, 5, 1),
            wire(5, 0, 0),
        ];
        let s = select_diffuse(&graph).unwrap();
        assert_eq!(
            (s.sample_index, s.second_sample.unwrap().sample_index),
            (1, 4)
        );
        assert_eq!(
            (s.multiply_index, s.inner_multiply_index, s.color_index),
            (Some(5), Some(3), Some(2))
        );
        assert_eq!(s.tint.values(), tint);
        assert!(s.third_sample.is_none());
        assert_eq!((s.ignored_nodes, s.ignored_wires), (0, 0));
        // Inner port order and outer port order do not change the selection.
        graph.wires[0].port_to_raw = 1;
        graph.wires[1].port_to_raw = 0;
        graph.wires[2].port_to_raw = 1;
        graph.wires[3].port_to_raw = 0;
        let swapped = select_diffuse(&graph).unwrap();
        assert_eq!(
            (swapped.sample_index, swapped.color_index, swapped.tint),
            (1, Some(2), s.tint)
        );
        // Different channels stay on independent dual sampling.
        graph.boxes[4].parameters_raw[4] = 1;
        let pair = select_diffuse(&graph).unwrap();
        assert_eq!(pair.second_sample.unwrap().uv_channel, 1);
        assert!(!uses_grid(pair));
        graph.boxes[4].parameters_raw[4] = 0;
        // (T × COLOR) × T with one sample: squared and tinted.
        graph.wires[3].box_from = 1;
        let squared = select_diffuse(&graph).unwrap();
        assert_eq!(squared.second_sample.unwrap().sample_index, 1);
        assert_eq!((squared.ignored_nodes, squared.ignored_wires), (1, 0));
        graph.wires[3].box_from = 4;
        // Genuine sampler-rule failures stay unsupported.
        for (index, raw) in [(1, 2u32), (4, 2)] {
            graph.boxes[index].parameters_raw[4] = raw;
            assert!(select_diffuse(&graph)
                .unwrap_err()
                .contains("sampler must be static"));
            graph.boxes[index].parameters_raw[4] = 0;
        }
        graph.boxes[4].animations[1].name_hex = "41".into();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[4].animations[1].name_hex = "00".into();
        graph.boxes[2].animations[0].component_value_count = 1;
        assert!(select_diffuse(&graph)
            .unwrap_err()
            .contains("COLOR must be unanimated"));
        graph.boxes[2].animations[0].component_value_count = 0;
        graph.boxes[2].parameters_raw[0] = f32::NAN.to_bits();
        assert!(select_diffuse(&graph).is_err());
        graph.boxes[2] = node(3, 0);
        graph.wires.push(wire(4, 1, 0)); // wired sampler input
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        graph.wires.push(wire(4, 3, 0)); // third inner input
        assert!(select_diffuse(&graph).is_err());
        graph.wires.pop();
        assert!(select_diffuse(&graph).is_ok());
    }
    #[test]
    fn requested_uv1_preserves_raw_channels_and_never_falls_back_to_uv0() {
        let mut coords = lbp_mesh::UvCoordinates {
            metadata: lbp_mesh::UvMetadata {
                vertex_count: 3,
                channel_count: 2,
                channels: vec![],
            },
            channels: vec![
                vec![[0., 0.], [0., 1.], [1., 0.]],
                vec![[1., -0.], [1., 0.], [0., 1.]],
            ],
        };
        let raw = coords
            .channels
            .iter()
            .map(|v| v.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let chosen = selected_uv(&coords, 1, 3).unwrap();
        assert_eq!(chosen[0].map(f32::to_bits), [1f32.to_bits(), 0x80000000]);
        assert_ne!(chosen, selected_uv(&coords, 0, 3).unwrap());
        let transform = crate::UvTransform {
            bits: [2., 0.5, 0.25, 0.5].map(f32::to_bits),
        };
        assert_eq!(
            transform.sampling(&chosen, || false).unwrap(),
            vec![[2.25, 0.5], [2.25, 0.5], [0.25, 1.]]
        );
        assert_eq!(
            raw,
            coords
                .channels
                .iter()
                .map(|v| v.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert!(selected_uv(&coords, 2, 3).is_err());
        assert!(selected_uv(&coords, 1, 2).is_err());
        coords.channels.pop();
        assert!(selected_uv(&coords, 1, 3)
            .unwrap_err()
            .contains("requested UV1"));
        assert!(selected_uv(&coords, 0, 3).is_ok());
        coords.channels[0][0][0] = f32::NAN;
        assert!(selected_uv(&coords, 0, 3).is_err());
    }
    #[test]
    fn aliases_deduplicate_but_conflicting_candidates_and_missing_ids_do_not_select() {
        let mut catalog =
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap();
        let guid = u32::from_str_radix(&catalog.snapshot.resources[0].map_row.guid, 16).unwrap();
        let hash = catalog.snapshot.resources[0].map_row.sha1.clone();
        let first = resolve(&catalog, Some(guid), Some(&hash));
        assert!(first.is_ok());
        let key = format!("{guid:08x}");
        catalog.by_guid.get_mut(&key).unwrap().push(0);
        assert_eq!(
            resolve(&catalog, Some(guid), Some(&hash)).unwrap(),
            first.unwrap()
        );
        catalog.by_guid.get_mut(&key).unwrap().push(1);
        assert!(resolve(&catalog, Some(guid), Some(&hash))
            .unwrap_err()
            .contains("Ambiguous"));
        assert!(resolve(&catalog, Some(0), None).is_err());
        assert!(resolve(&catalog, Some(u32::MAX), None).is_err());
    }
}
