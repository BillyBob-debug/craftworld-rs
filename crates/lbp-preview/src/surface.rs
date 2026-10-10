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
/// P3h: glow (INFERRED from the decoded shaders: added unlit, oracles/results/2026-10-09-shading-fpdec.md).
pub const GLOW_PORT: u8 = 4;
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
    /// P3d: gloss colour map on port 2 (same layout as `BumpMap`; rgb is the gloss colour).
    pub spec_map: Option<Box<BumpMap>>,
    /// GfxMaterial CosinePower (finite, clamped to 1..=4096).
    pub cosine_power: f32,
    pub alpha_mode_raw: u8,
    /// GfxMaterial AlphaTestLevel (finite, 0..=1; 0 = no cut-out). The decoded shaders write
    /// alpha 0 below it (oracles/results/2026-10-09-shading-fpdec.md); the viewer discards.
    pub alpha_test: f32,
    /// P3h: what output port 4 (glow) evaluates to, loaded by the diffuse route with the glow
    /// graph standing in for port 0 (a solid colour or a baked texture), when wired.
    pub glow: Option<Box<crate::MaterialAsset>>,
    /// Wired surface ports this preview does not draw, with the reason.
    pub skipped: Vec<String>,
}
impl Surface {
    pub fn retained_bytes(&self) -> usize {
        [&self.bump, &self.spec_map]
            .into_iter()
            .flatten()
            .map(|b| b.texture.rgba.len() + b.sampling_uv.len() * std::mem::size_of::<[f32; 2]>())
            .sum::<usize>()
            + match self.glow.as_deref() {
                Some(crate::MaterialAsset::Texture(a)) => {
                    a.texture.rgba.len() + a.sampling_uv.len() * std::mem::size_of::<[f32; 2]>()
                }
                _ => 0,
            }
    }
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if let Some(b) = &self.bump {
            parts.push(format!(
                "bump UV{} × {:.2} ({}×{})",
                b.uv_channel, self.bump_level, b.texture.header.width, b.texture.header.height
            ));
        }
        if let Some(m) = &self.spec_map {
            parts.push(format!(
                "gloss map UV{} ^ {:.0} ({}×{})",
                m.uv_channel, self.cosine_power, m.texture.header.width, m.texture.header.height
            ));
        }
        match self.glow.as_deref() {
            Some(crate::MaterialAsset::Solid(a)) => {
                let [r, g, b, _] = a.selection.color.values();
                parts.push(format!("glow {r:.2}/{g:.2}/{b:.2}"));
            }
            Some(crate::MaterialAsset::Texture(a)) => parts.push(format!(
                "glow map ({}×{})",
                a.texture.header.width, a.texture.header.height
            )),
            None => {}
        }
        if let Some(s) = self.specular {
            parts.push(format!(
                "gloss {:.2}/{:.2}/{:.2} ^ {:.0}",
                s[0], s[1], s[2], self.cosine_power
            ));
        }
        parts.extend(self.skipped.iter().cloned());
        parts.push(format!("alpha mode {}", self.alpha_mode_raw));
        if self.alpha_test > 0. {
            parts.push(format!("alpha test {:.2}", self.alpha_test));
        }
        format!("P2q surface: {}", parts.join(" • "))
    }
}

/// Where the static ports of the single output node come from (no I/O).
#[derive(Debug, Default, PartialEq)]
pub struct SurfaceRoutes {
    pub bump: Option<crate::linked::SamplerSelection>,
    pub specular: Option<[f32; 3]>,
    /// P3d: port 2 wired straight from one sampler (a gloss colour map).
    pub spec_map: Option<crate::linked::SamplerSelection>,
    pub skipped: Vec<String>,
}
/// P3h: a copy of `graph` whose output node reads `port` as its port 0, so the diffuse route
/// can evaluate the glow graph. `Ok(None)` when `port` is unwired; one wire only.
pub fn rewire_output(graph: &GraphMetadata, port: u8) -> Result<Option<GraphMetadata>, String> {
    let outputs: Vec<usize> = (0..graph.boxes.len())
        .filter(|&i| graph.boxes[i].type_raw == 0)
        .collect();
    let [output] = outputs[..] else {
        return Err("Unsupported: surface needs one output node".into());
    };
    let into = |p: u8| {
        graph
            .wires
            .iter()
            .enumerate()
            .filter(move |(_, w)| w.box_to == output as i32 && w.port_to_raw == p)
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    let wired = into(port);
    let [wire] = wired[..] else {
        return if wired.is_empty() {
            Ok(None)
        } else {
            Err(format!("several port {port} wires"))
        };
    };
    let mut derived = graph.clone();
    let diffuse = into(0);
    derived.wires[wire].port_to_raw = 0;
    let mut index = 0;
    derived.wires.retain(|_| {
        let keep = !diffuse.contains(&index);
        index += 1;
        keep
    });
    derived.wire_count = derived.wires.len();
    Ok(Some(derived))
}
/// P3h: whether the compiled program has the glow add. A colour box (type 3) feeding port 4 is
/// bound to the fragment program as constant `ta<k>` (k = its box index, CONFIRMED over 8,043
/// gmats, oracles 2026-10-10-glow-gate); programs that never name it (e.g. the frozen cube
/// 00005407) have no glow add. Raw scan of the shader blob for `\0ta<k>\0` (never the bare
/// prefix: `ta1` is inside `ta10`); named in any variant counts. Texture, mix and other source
/// kinds are not decided here and pass.
pub fn glow_in_program(graph: &GraphMetadata, blob: &[u8]) -> bool {
    let Some(output) = graph.boxes.iter().position(|b| b.type_raw == 0) else {
        return true;
    };
    let source = graph
        .wires
        .iter()
        .find(|w| w.box_to == output as i32 && w.port_to_raw == GLOW_PORT)
        .map(|w| w.box_from);
    let Some(k) = source.and_then(|f| usize::try_from(f).ok()) else {
        return true;
    };
    if graph.boxes.get(k).map(|b| b.type_raw) != Some(3) {
        return true;
    }
    let name = format!("\0ta{k}\0").into_bytes();
    blob.windows(name.len()).any(|w| w == name.as_slice())
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
            None => match endpoint(w).filter(|&i| boxes[i].type_raw == 1) {
                Some(i) => match crate::linked::sampler(graph, i) {
                    Ok(s) => routes.spec_map = Some(s),
                    Err(e) => routes.skipped.push(format!("gloss map not drawn ({e})")),
                },
                None => routes
                    .skipped
                    .push("gloss not drawn (port 2 is not one static COLOR or one sampler)".into()),
            },
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
        alpha_test: scalar(scalars.alpha_test_bits, 0., 1., 0.),
        specular: routes.specular,
        spec_map: None,
        glow: None,
        skipped: routes.skipped,
        bump: None,
    };
    if let Some(sampler) = routes.bump.filter(|_| surface.bump_level > 0.) {
        match load_bump(source, catalog, graph, sampler, vertices, uv, control) {
            Ok(b) => surface.bump = Some(Box::new(b)),
            Err(e) => surface.skipped.push(format!("bump not drawn ({e})")),
        }
    }
    if let Some(sampler) = routes.spec_map {
        match load_bump(source, catalog, graph, sampler, vertices, uv, control) {
            Ok(b) => surface.spec_map = Some(Box::new(b)),
            Err(e) => surface.skipped.push(format!("gloss map not drawn ({e})")),
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
    fn p3h_glow_graph_replaces_the_diffuse_wire_on_a_copy() {
        // output 0; sampler 1 -> port 0 (diffuse); sampler 2 -> port 4 (glow)
        let g = graph(
            vec![
                node(0, [0; 8]),
                node(1, sampler_params(0)),
                node(1, sampler_params(1)),
            ],
            vec![wire(1, 0, 0), wire(2, 0, GLOW_PORT)],
        );
        let d = rewire_output(&g, GLOW_PORT).unwrap().unwrap();
        assert_eq!(d.wires.len(), 1);
        assert_eq!((d.wires[0].box_from, d.wires[0].port_to_raw), (2, 0));
        assert_eq!(d.wire_count, 1);
        // The original is untouched; an unwired port is None; two wires are refused.
        assert_eq!(g.wires.len(), 2);
        assert!(rewire_output(&g, BUMP_PORT).unwrap().is_none());
        let two = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(0))],
            vec![wire(1, 0, GLOW_PORT), wire(1, 0, GLOW_PORT)],
        );
        assert!(rewire_output(&two, GLOW_PORT).is_err());
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

        // A swizzled bump wire is named, not drawn; a sampler on the gloss port is a gloss map.
        let mut swizzled = full.clone();
        swizzled.wires[0].swizzle_raw = [b'a', 0, 0, 0, 0];
        swizzled.wires[1].box_from = 1;
        let routes = select_routes(&swizzled).unwrap();
        assert!(routes.bump.is_none() && routes.specular.is_none());
        assert_eq!(routes.spec_map.unwrap().slot, 2);
        assert_eq!(routes.skipped.len(), 1);

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

    #[test]
    fn p3h_glow_needs_the_program_to_name_the_colour_box() {
        // output <- port 4 <- colour box 1; the blob names ta1 or only ta10
        let g = graph(
            vec![node(0, [0; 8]), node(3, [0; 8])],
            vec![wire(1, 0, GLOW_PORT)],
        );
        assert!(glow_in_program(&g, b"CGB\0ta1\0ta1\0\0ta1\0x"));
        assert!(!glow_in_program(&g, b"\0ta10\0\0ta2\0"));
        assert!(!glow_in_program(&g, b""));
        // texture sources are not decided here
        let t = graph(
            vec![node(0, [0; 8]), node(1, sampler_params(0))],
            vec![wire(1, 0, GLOW_PORT)],
        );
        assert!(glow_in_program(&t, b""));
    }
}
