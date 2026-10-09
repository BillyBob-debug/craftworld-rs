//! P2q opt-in surface extras beside the diffuse route: bump (output port 3) and gloss
//! (output port 2) for the LBP-shaped preview lighting. Evidence and limits:
//! docs/p2q-bump-gloss.md. Only two shapes are taken: port 3 wired straight from one
//! static texture sampler, and port 2 wired straight from one static COLOR. Anything
//! else stays unrendered and is named in `skipped`.
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_resource::material::{GraphInspection, GraphMetadata};

/// Output-node port numbers (CONFIRMED_FROM_COMMUNITY_CODE, docs/material-graph.md).
const SPECULAR_PORT: u8 = 2;
const BUMP_PORT: u8 = 3;
/// Upper bounds for untrusted material scalars before they reach a shader.
const MAX_BUMP_LEVEL: f32 = 16.;
const MAX_COSINE_POWER: f32 = 4096.;

pub struct BumpMap {
    pub texture: lbp_texture::Texture,
    pub texture_key: lbp_catalog::attachment::LocationKey,
    /// Sampler coordinates per vertex (raw channel through the sampler's transform).
    pub sampling_uv: Vec<[f32; 2]>,
    pub wrap: crate::TextureWrapping,
    pub uv_channel: usize,
}
#[derive(Default)]
pub struct Surface {
    pub bump: Option<Box<BumpMap>>,
    /// GfxMaterial BumpLevel (finite, clamped to 0..=16).
    pub bump_level: f32,
    /// Static COLOR on the specular port (linear rgb), when wired that way.
    pub specular: Option<[f32; 3]>,
    /// GfxMaterial CosinePower (finite, clamped to 1..=4096).
    pub cosine_power: f32,
    pub alpha_mode_raw: u8,
    /// Wired surface ports this preview does not draw, with the reason.
    pub skipped: Vec<String>,
}
impl Surface {
    pub fn retained_bytes(&self) -> usize {
        self.bump.as_ref().map_or(0, |b| {
            b.texture.rgba.len() + b.sampling_uv.len() * std::mem::size_of::<[f32; 2]>()
        })
    }
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(b) = &self.bump {
            parts.push(format!(
                "bump UV{} × {:.2} ({}×{})",
                b.uv_channel, self.bump_level, b.texture.header.width, b.texture.header.height
            ));
        }
        if let Some(s) = self.specular {
            parts.push(format!(
                "gloss {:.2}/{:.2}/{:.2} ^ {:.0}",
                s[0], s[1], s[2], self.cosine_power
            ));
        }
        parts.extend(self.skipped.iter().cloned());
        parts.push(format!("alpha mode {}", self.alpha_mode_raw));
        format!("P2q surface: {}", parts.join(" • "))
    }
}

/// Where the static ports of the single output node come from (no I/O).
#[derive(Debug, Default, PartialEq)]
pub struct SurfaceRoutes {
    pub bump: Option<crate::linked::SamplerSelection>,
    pub specular: Option<[f32; 3]>,
    pub skipped: Vec<String>,
}
fn is_static(node: &lbp_resource::material::BoxMetadata) -> bool {
    node.subtype_raw == 0
        && node
            .animations
            .iter()
            .all(|a| a.component_value_count == 0 && a.name_hex.bytes().all(|v| v == b'0'))
}
pub fn select_routes(graph: &GraphMetadata) -> Result<SurfaceRoutes, String> {
    let boxes = &graph.boxes;
    let wires = &graph.wires;
    if boxes.len() > 1024 || wires.len() > 4096 {
        return Err("Unsupported: surface graph budget".into());
    }
    let outputs: Vec<usize> = (0..boxes.len())
        .filter(|&i| boxes[i].type_raw == 0)
        .collect();
    let [output] = outputs[..] else {
        return Err("Unsupported: surface needs one output node".into());
    };
    let mut routes = SurfaceRoutes::default();
    let port = |p: u8| {
        wires
            .iter()
            .filter(|w| w.box_to == output as i32 && w.port_to_raw == p)
            .collect::<Vec<_>>()
    };
    let endpoint = |w: &lbp_resource::material::WireMetadata| {
        usize::try_from(w.box_from)
            .ok()
            .filter(|&i| i < boxes.len() && w.port_from_raw == 0 && w.swizzle_raw == [0; 5])
    };
    match port(BUMP_PORT)[..] {
        [] => {}
        [w] => match endpoint(w).filter(|&i| boxes[i].type_raw == 1) {
            Some(i) => match crate::linked::sampler(graph, i) {
                Ok(s) => routes.bump = Some(s),
                Err(e) => routes.skipped.push(format!("bump not drawn ({e})")),
            },
            None => routes
                .skipped
                .push("bump not drawn (port 3 is not one unswizzled sampler)".into()),
        },
        _ => routes
            .skipped
            .push("bump not drawn (several port 3 wires)".into()),
    }
    match port(SPECULAR_PORT)[..] {
        [] => {}
        [w] => match endpoint(w).filter(|&i| {
            boxes[i].type_raw == 3
                && is_static(&boxes[i])
                && !wires.iter().any(|w| w.box_to == i as i32)
        }) {
            Some(i) => {
                let tint = crate::DiffuseTint {
                    bits: boxes[i].parameters_raw[..4].try_into().unwrap(),
                };
                tint.validate()?;
                let [r, g, b, _] = tint.values();
                routes.specular = Some([r, g, b]);
            }
            None => routes
                .skipped
                .push("gloss not drawn (port 2 is not one static COLOR)".into()),
        },
        _ => routes
            .skipped
            .push("gloss not drawn (several port 2 wires)".into()),
    }
    Ok(routes)
}
fn scalar(bits: u32, lo: f32, hi: f32, fallback: f32) -> f32 {
    let v = f32::from_bits(bits);
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        fallback
    }
}
/// Builds the surface extras for one material from its inspected graph. `uv` gives the
/// vertex coordinates (the same input the diffuse route used).
pub(crate) fn load(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    vertices: usize,
    uv: crate::linked::UvInput,
    control: &LoadControl,
) -> Result<Surface, String> {
    control.check()?;
    let scalars = &graph.material.prefix.scalars;
    let routes = select_routes(&graph.graph)?;
    let mut surface = Surface {
        bump_level: scalar(scalars.bump_level_bits, 0., MAX_BUMP_LEVEL, 0.),
        // CosinePower 22 is the decoded shaders' default (pod-fidelity README, D196).
        cosine_power: scalar(scalars.cosine_power_bits, 1., MAX_COSINE_POWER, 22.),
        alpha_mode_raw: scalars.alpha_mode_raw,
        specular: routes.specular,
        skipped: routes.skipped,
        bump: None,
    };
    if let Some(sampler) = routes.bump.filter(|_| surface.bump_level > 0.) {
        match load_bump(source, catalog, graph, sampler, vertices, uv, control) {
            Ok(b) => surface.bump = Some(Box::new(b)),
            Err(e) => surface.skipped.push(format!("bump not drawn ({e})")),
        }
    }
    Ok(surface)
}
fn load_bump(
    source: &AttachedSource,
    catalog: &Catalog,
    graph: &GraphInspection,
    sampler: crate::linked::SamplerSelection,
    vertices: usize,
    uv: crate::linked::UvInput,
    control: &LoadControl,
) -> Result<BumpMap, String> {
    let prefix = &graph.material.prefix;
    let descriptor = prefix
        .textures
        .get(sampler.slot)
        .ok_or("Unsupported: bump texture slot")?;
    if descriptor.kind == 0 || descriptor.flags != 0 {
        return Err("Unsupported: absent bump texture or descriptor flags".into());
    }
    let wrap = crate::TextureWrapping::from_raw(
        prefix.wrap_s_raw.get(sampler.slot).copied(),
        prefix.wrap_t_raw.get(sampler.slot).copied(),
    )?;
    let coordinates = uv.decode(control)?;
    let raw = crate::linked::selected_uv(&coordinates, sampler.uv_channel, vertices)?;
    let sampling_uv = sampler
        .uv_transform
        .sampling(&raw, || control.check().is_err())?;
    let texture_key = crate::resolve(catalog, descriptor.guid, descriptor.sha1.as_deref())?;
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
    Ok(BumpMap {
        texture,
        texture_key,
        sampling_uv,
        wrap,
        uv_channel: sampler.uv_channel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};

    fn node(type_raw: u32, parameters_raw: [u32; 8]) -> BoxMetadata {
        BoxMetadata {
            type_raw,
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
    fn wire(from: i32, to: i32, port_to_raw: u8) -> WireMetadata {
        WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        }
    }
    fn sampler_params(slot: u32) -> [u32; 8] {
        [1f32.to_bits(), 1f32.to_bits(), 0, 0, 0, slot, 0, 0]
    }

    #[test]
    fn p2q_routes_take_one_bump_sampler_and_one_static_gloss_color() {
        let gloss = [0.6f32, 0.6, 0.6, 1.].map(f32::to_bits);
        let mut color = [0; 8];
        color[..4].copy_from_slice(&gloss);
        let full = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(2)), node(3, color)],
            vec![wire(1, 0, BUMP_PORT), wire(2, 0, SPECULAR_PORT)],
        );
        let routes = select_routes(&full).unwrap();
        assert_eq!(routes.bump.unwrap().slot, 2);
        assert_eq!(routes.specular, Some([0.6, 0.6, 0.6]));
        assert!(routes.skipped.is_empty());

        // A swizzled bump wire and a sampler on the gloss port are named, not drawn.
        let mut swizzled = full.clone();
        swizzled.wires[0].swizzle_raw = [b'a', 0, 0, 0, 0];
        swizzled.wires[1].box_from = 1;
        let routes = select_routes(&swizzled).unwrap();
        assert!(routes.bump.is_none() && routes.specular.is_none());
        assert_eq!(routes.skipped.len(), 2);

        // No surface wires: nothing drawn, nothing skipped.
        let plain = graph(full.boxes, vec![wire(1, 0, 0)]);
        assert_eq!(select_routes(&plain).unwrap(), SurfaceRoutes::default());
    }

    #[test]
    fn p2q_untrusted_scalars_are_clamped_or_defaulted() {
        assert_eq!(scalar(f32::NAN.to_bits(), 1., 4096., 22.), 22.);
        assert_eq!(scalar(1e9f32.to_bits(), 1., 4096., 22.), 4096.);
        assert_eq!(scalar(0.6f32.to_bits(), 0., 16., 0.), 0.6);
        assert_eq!(scalar((-1f32).to_bits(), 0., 16., 0.), 0.);
    }
}
