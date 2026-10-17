//! P2e opt-in runtime texture slot binding (docs/p2e-runtime-texture.md).
//!
//! Some materials sample a texture slot the game fills at runtime (the Pod shell mesh
//! 00002f6c primitive 2 lists slot 0 as an unrendered required slot). With an explicit
//! `MESH:SLOT:TEXTURE` rule, a matching Solid-route group is drawn with that texture
//! instead of its solid colour. Which texture the game binds is not confirmed; the rule
//! is user-supplied. P2g: the material graph's own sampler box for the slot gives the UV
//! channel and transform (CONFIRMED_FROM_LOCAL_DATA per material; UV0 untransformed when
//! it cannot be read). INFERRED display policy: Repeat on both axes, untinted (the
//! replaced solid colour is kept only in the label); the boxes between the sample and
//! the output are listed so the tint can be decided from evidence.
use crate::linked::{Asset, MaterialAsset, SolidAsset};
use lbp_catalog::attachment::LocationKey;
use lbp_resource::material::GraphMetadata;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Rules accepted per viewer run.
pub const MAX_RUNTIME_TEXTURES: usize = 16;
/// Texture slots a material may name (same bound as the unrendered-slot review).
pub const MAX_TEXTURE_SLOTS: usize = 8;

/// One `--runtime-texture MESH:SLOT:TEXTURE` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuntimeTexture {
    pub mesh: u32,
    pub slot: usize,
    pub texture: u32,
}

impl RuntimeTexture {
    /// `MESH:SLOT:TEXTURE`: two eight-digit hexadecimal GUIDs (nonzero) and a slot index
    /// below 8 (decimal or hexadecimal digit; the same value either way).
    pub fn parse(value: &str) -> Result<Self, String> {
        let parts: Vec<&str> = value.trim().split(':').collect();
        let [mesh, slot, texture] = parts[..] else {
            return Err("--runtime-texture expects MESH:SLOT:TEXTURE".into());
        };
        let guid = |s: &str, what: &str| {
            u32::from_str_radix(s, 16)
                .ok()
                .filter(|&g| s.len() == 8 && g != 0 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(|| {
                    format!("--runtime-texture {what} must be eight hexadecimal digits, nonzero")
                })
        };
        let slot = usize::from_str_radix(slot, 16)
            .ok()
            .filter(|&n| slot.len() == 1 && n < MAX_TEXTURE_SLOTS)
            .ok_or("--runtime-texture SLOT must be a single digit 0-7")?;
        Ok(Self {
            mesh: guid(mesh, "MESH")?,
            slot,
            texture: guid(texture, "TEXTURE")?,
        })
    }
    pub fn label(&self) -> String {
        format!(
            "Runtime texture {:08x} on mesh {:08x} slot {}",
            self.texture, self.mesh, self.slot
        )
    }
    /// Matching rule: same mesh GUID, a Solid-route group, and that slot listed as an
    /// unrendered required texture slot. Err is the reason a same-mesh group is left.
    pub fn matches(&self, mesh: u32, binding: &MaterialAsset) -> Result<bool, &'static str> {
        if mesh != self.mesh {
            return Ok(false);
        }
        if !binding.additional_texture_slots().contains(&self.slot) {
            return Ok(false);
        }
        match binding {
            MaterialAsset::Solid(_) => Ok(true),
            MaterialAsset::Texture(_) => {
                Err("group already textured (override applies to Solid routes only)")
            }
        }
    }
}

/// Rejects too many rules and two textures for the same mesh slot.
pub fn validate(rules: &[RuntimeTexture]) -> Result<(), String> {
    if rules.len() > MAX_RUNTIME_TEXTURES {
        return Err(format!(
            "--runtime-texture: at most {MAX_RUNTIME_TEXTURES} rules"
        ));
    }
    for (i, a) in rules.iter().enumerate() {
        if rules[..i]
            .iter()
            .any(|b| b.mesh == a.mesh && b.slot == a.slot && b.texture != a.texture)
        {
            return Err(format!(
                "--runtime-texture: conflicting textures for mesh {:08x} slot {}",
                a.mesh, a.slot
            ));
        }
    }
    Ok(())
}

/// Recorded on a replacement asset (shown in its label).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeOverride {
    pub rule: RuntimeTexture,
    /// Solid diffuse colour the group had before the override.
    pub replaced_color: [f32; 4],
}

/// Per-rule outcome over primitive groups (non-empty triangle ranges) of distinct
/// loaded meshes (a mesh drawn by several Things counts once).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeOutcome {
    pub applied: usize,
    pub not_applied: BTreeMap<String, usize>,
    /// P2g: how the first matched group was sampled.
    pub sampler: Option<String>,
}

/// P2g: the material graph's sampler for a rule's slot, and the box types on the first
/// wire path from that sample to the output (type 0).
#[derive(Debug, Clone, PartialEq)]
pub struct SlotSampler {
    pub uv_channel: usize,
    pub transform: crate::SamplingTransform,
    pub chain: Vec<u32>,
}

/// Finds the first texture-sample box (type 1) naming `slot` and decodes it with the
/// same static-sampler rule as the diffuse routes. Bounded: graph limits as the slot
/// review, one step per box along the output path.
pub fn slot_sampler(graph: &GraphMetadata, slot: usize) -> Result<SlotSampler, String> {
    let n = graph.boxes.len();
    if n > 1024
        || graph.wires.len() > 4096
        || graph.wires.iter().any(|w| {
            w.box_from < 0 || w.box_to < 0 || w.box_from as usize >= n || w.box_to as usize >= n
        })
    {
        return Err("Unsupported: material graph bounds/endpoints".into());
    }
    let index = graph
        .boxes
        .iter()
        .position(|b| b.type_raw == 1 && b.parameters_raw[5] as usize == slot)
        .ok_or_else(|| format!("no sampler box for slot {slot}"))?;
    let sampler = crate::linked::sampler(graph, index)?;
    let mut seen = vec![false; n];
    seen[index] = true;
    let mut chain = Vec::new();
    let mut at = index;
    while let Some(w) = graph
        .wires
        .iter()
        .find(|w| w.box_from as usize == at && !seen[w.box_to as usize])
    {
        at = w.box_to as usize;
        seen[at] = true;
        chain.push(graph.boxes[at].type_raw);
        if graph.boxes[at].type_raw == 0 {
            break;
        }
    }
    Ok(SlotSampler {
        uv_channel: sampler.uv_channel,
        transform: sampler.uv_transform,
        chain,
    })
}

pub type RuntimeCounts = BTreeMap<RuntimeTexture, RuntimeOutcome>;

/// Status lines; empty when no rule was given (default behaviour unchanged).
pub fn lines(counts: &RuntimeCounts) -> Vec<String> {
    let mut out = Vec::new();
    for (rule, o) in counts {
        out.push(format!(
            "{}: {} groups (opt-in; Repeat, untinted: inferred; game binding unconfirmed)",
            rule.label(),
            o.applied
        ));
        if let Some(sampler) = &o.sampler {
            out.push(format!("  {sampler}"));
        }
        for (reason, n) in &o.not_applied {
            out.push(format!("  not applied {n} groups: {reason}"));
        }
    }
    out
}

/// One UV channel of a mesh and its UV metadata.
pub type MeshUv0 = (Vec<[f32; 2]>, lbp_mesh::UvMetadata);
/// Decoded runtime texture and where it came from.
pub type LoadedTexture = (lbp_texture::Texture, LocationKey);

/// One loaded mesh's groups: triangle ranges, vertex count and per-primitive bindings.
pub(crate) struct Groups<'a> {
    pub ranges: &'a [std::ops::Range<usize>],
    pub vertices: usize,
    pub bindings: &'a mut [Result<Arc<MaterialAsset>, String>],
}

/// Applies matching rules to one loaded mesh. `uv` decodes one UV channel on demand
/// (at most once per channel); `texture` loads a texture GUID (at most once per rule);
/// `sampler` reads a material's sampler for a slot. Failures leave the groups Solid
/// and are counted with their reason.
pub(crate) fn apply(
    mesh: u32,
    grouped: Groups<'_>,
    rules: &[RuntimeTexture],
    counts: &mut RuntimeCounts,
    mut uv: impl FnMut(usize) -> Result<MeshUv0, String>,
    mut texture: impl FnMut(u32) -> Result<LoadedTexture, String>,
    mut sampler: impl FnMut(&LocationKey, usize) -> Result<SlotSampler, String>,
) {
    let Groups {
        ranges,
        vertices,
        bindings,
    } = grouped;
    let mut coords: BTreeMap<usize, Result<MeshUv0, String>> = BTreeMap::new();
    for rule in rules.iter().filter(|r| r.mesh == mesh) {
        let outcome = counts.entry(*rule).or_default();
        let mut loaded: Option<Result<Arc<LoadedTexture>, String>> = None;
        // One replacement per distinct shared binding keeps groups bucketed together.
        let mut replaced: Vec<(Arc<MaterialAsset>, Arc<MaterialAsset>)> = Vec::new();
        for (index, binding) in bindings.iter_mut().enumerate() {
            if ranges.get(index).is_none_or(|r| r.is_empty()) {
                continue;
            }
            let Ok(asset) = binding else { continue };
            match rule.matches(mesh, asset) {
                Ok(false) => continue,
                Err(reason) => {
                    *outcome.not_applied.entry(reason.into()).or_default() += 1;
                    continue;
                }
                Ok(true) => {}
            }
            if let Some((_, new)) = replaced.iter().find(|(old, _)| Arc::ptr_eq(old, asset)) {
                *binding = Ok(new.clone());
                outcome.applied += 1;
                continue;
            }
            let MaterialAsset::Solid(solid) = asset.as_ref() else {
                continue;
            };
            let read = sampler(&solid.material_key, rule.slot);
            let (channel, transform) = match &read {
                Ok(s) => (s.uv_channel, s.transform),
                Err(_) => (0, crate::SamplingTransform::default()),
            };
            outcome.sampler.get_or_insert_with(|| match &read {
                Ok(s) => format!(
                    "material sampler UV{} {}; sample-to-output box types {:?}",
                    s.uv_channel,
                    s.transform.label(),
                    s.chain
                ),
                Err(e) => format!("material sampler not read ({e}); UV0 untransformed"),
            });
            let coords = coords.entry(channel).or_insert_with(|| uv(channel));
            let tex = loaded.get_or_insert_with(|| {
                texture(rule.texture)
                    .map(Arc::new)
                    .map_err(|e| format!("texture {:08x}: {e}", rule.texture))
            });
            let built = match (coords, tex) {
                (Ok((uv, meta)), Ok(tex)) => {
                    if uv.len() != vertices {
                        Err(format!("Invalid: UV{channel}/vertex correspondence"))
                    } else {
                        transform.sampling(uv, || false).map(|sampling| {
                            replacement(
                                solid,
                                rule,
                                (uv, sampling, meta),
                                (channel, transform),
                                tex,
                            )
                        })
                    }
                }
                (Err(e), _) => Err(format!("mesh UV{channel}: {e}")),
                (_, Err(e)) => Err(e.clone()),
            };
            match built {
                Ok(new) => {
                    let new = Arc::new(new);
                    replaced.push((asset.clone(), new.clone()));
                    *binding = Ok(new);
                    outcome.applied += 1;
                }
                Err(reason) => *outcome.not_applied.entry(reason).or_default() += 1,
            }
        }
    }
}

fn replacement(
    solid: &SolidAsset,
    rule: &RuntimeTexture,
    (uv, sampling, meta): (&[[f32; 2]], Vec<[f32; 2]>, &lbp_mesh::UvMetadata),
    (uv_channel, uv_transform): (usize, crate::SamplingTransform),
    tex: &LoadedTexture,
) -> MaterialAsset {
    let selection = crate::DiffuseSelection {
        slot: rule.slot,
        uv_channel,
        uv_transform,
        ..Default::default()
    };
    MaterialAsset::Texture(Box::new(Asset {
        // P2q: the material's own bump/gloss extras stay with the replaced diffuse.
        surface: solid.surface.clone(),
        instance_projection: solid.instance_projection.clone(),
        additional_texture_slots: solid
            .additional_texture_slots
            .iter()
            .copied()
            .filter(|&s| s != rule.slot)
            .collect(),
        border_domain: None,
        fourth: None,
        third: None,
        dual: None,
        fragment: None,
        overlay_alpha: false,
        product: None,
        tinted_rgba: None,
        sampling_uv: sampling,
        uv: uv.to_vec(),
        uv_metadata: lbp_mesh::UvMetadata {
            vertex_count: meta.vertex_count,
            channel_count: meta.channel_count,
            channels: meta
                .channels
                .iter()
                .map(|c| lbp_mesh::UvChannelMetadata {
                    channel: c.channel,
                    uv_bits_sha256: c.uv_bits_sha256.clone(),
                })
                .collect(),
        },
        texture: lbp_texture::Texture {
            header: tex.0.header.clone(),
            expanded_bytes: tex.0.expanded_bytes,
            top_mip_block_bytes: tex.0.top_mip_block_bytes,
            unparsed_after_top_mip: tex.0.unparsed_after_top_mip,
            rgba: tex.0.rgba.clone(),
        },
        material_key: solid.material_key.clone(),
        texture_key: tex.1.clone(),
        wrap: crate::TextureWrapping::uniform(true),
        selection,
        runtime_texture: Some(RuntimeOverride {
            rule: *rule,
            replaced_color: solid.selection.color.values(),
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_hex_guids_and_slot_and_rejects_malformed() {
        assert_eq!(
            RuntimeTexture::parse("00002f6c:0:000027fb").unwrap(),
            RuntimeTexture {
                mesh: 0x2f6c,
                slot: 0,
                texture: 0x27fb
            }
        );
        assert_eq!(
            RuntimeTexture::parse("00002F6C:7:00004277").unwrap().slot,
            7
        );
        for bad in [
            "",
            "00002f6c:0",
            "00002f6c:0:000027fb:1",
            "2f6c:0:000027fb",
            "00002f6c:8:000027fb",
            "00002f6c:-1:000027fb",
            "00002f6c:00:000027fb",
            "00002f6c:0:0000zzzz",
            "00000000:0:000027fb",
            "00002f6c:0:00000000",
            "+0002f6c:0:000027fb",
        ] {
            assert!(RuntimeTexture::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn validate_bounds_and_conflicts() {
        let r = |t| RuntimeTexture {
            mesh: 1,
            slot: 0,
            texture: t,
        };
        assert!(validate(&[r(2), r(2)]).is_ok());
        assert!(validate(&[r(2), r(3)]).unwrap_err().contains("conflicting"));
        assert!(validate(&vec![r(2); MAX_RUNTIME_TEXTURES + 1]).is_err());
    }

    /// D345 (`crate::tint_solid`, here for the solid helper): the rgb is multiplied, the
    /// alpha kept; a textured material is refused.
    #[test]
    fn d345_tint_solid_multiplies_rgb() {
        let mut m = Arc::try_unwrap(solid(vec![])).ok().unwrap();
        let tint = crate::level_scene::editor_color_tint(0x80ce_b424);
        crate::tint_solid(&mut m, tint).unwrap();
        let MaterialAsset::Solid(s) = &m else {
            unreachable!()
        };
        let [r, g, b, a] = s.selection.color.values();
        let close = |x: f32, y: f32| (x - y).abs() < 1e-6;
        assert!(close(r, 0.118 * 206. / 255.) && close(g, 0.118 * 180. / 255.));
        assert!(close(b, 0.118 * 36. / 255.) && a == 1.);
    }

    fn solid(slots: Vec<usize>) -> Arc<MaterialAsset> {
        Arc::new(MaterialAsset::Solid(Box::new(SolidAsset {
            surface: None,
            instance_projection: None,
            additional_texture_slots: slots,
            material_key: LocationKey {
                path: "authored".into(),
                offset: 0,
                size: 0,
            },
            selection: crate::SolidSelection {
                output_index: 0,
                color_index: 1,
                color: crate::DiffuseTint {
                    bits: [
                        0.118f32.to_bits(),
                        0.118f32.to_bits(),
                        0.118f32.to_bits(),
                        1f32.to_bits(),
                    ],
                },
                ignored_nodes: 0,
                ignored_wires: 0,
                unwired_diffuse: false,
                output_swizzle: [0; 5],
                product_of: None,
                product_tree: None,
            },
        })))
    }

    fn texture() -> LoadedTexture {
        (
            lbp_texture::Texture {
                header: lbp_texture::Header {
                    pixel_format: lbp_texture::PixelFormat::BGRA8,
                    width: 1,
                    height: 1,
                    mips: 1,
                    wrapper: "authored",
                    flags: None,
                    remap: None,
                    format_raw: None,
                    no_srgb: false,
                    dds: None,
                },
                expanded_bytes: 4,
                top_mip_block_bytes: 4,
                unparsed_after_top_mip: 0,
                rgba: vec![1, 2, 3, 255],
            },
            LocationKey {
                path: "authored-texture".into(),
                offset: 0,
                size: 4,
            },
        )
    }

    fn meta() -> lbp_mesh::UvMetadata {
        lbp_mesh::UvMetadata {
            vertex_count: 3,
            channel_count: 1,
            channels: vec![],
        }
    }

    fn ranges(n: usize) -> Vec<std::ops::Range<usize>> {
        (0..n).map(|i| i..i + 1).collect()
    }

    fn groups<'a>(
        ranges: &'a [std::ops::Range<usize>],
        bindings: &'a mut [Result<Arc<MaterialAsset>, String>],
    ) -> Groups<'a> {
        Groups {
            ranges,
            vertices: 3,
            bindings,
        }
    }

    #[test]
    fn matching_rule_needs_mesh_solid_route_and_listed_slot() {
        let rule = RuntimeTexture::parse("00002f6c:0:000027fb").unwrap();
        assert_eq!(rule.matches(0x2f6c, &solid(vec![0])), Ok(true));
        assert_eq!(rule.matches(0x2f6d, &solid(vec![0])), Ok(false));
        assert_eq!(rule.matches(0x2f6c, &solid(vec![1])), Ok(false));
        assert_eq!(rule.matches(0x2f6c, &solid(vec![])), Ok(false));
    }

    #[test]
    fn apply_replaces_shared_solid_once_and_counts_groups() {
        let rule = RuntimeTexture::parse("00002f6c:0:000027fb").unwrap();
        let shared = solid(vec![0, 3]);
        let mut bindings = vec![
            Ok(shared.clone()),
            Ok(solid(vec![1])),
            Ok(shared.clone()),
            Err("Unsupported".into()),
            Ok(shared),
        ];
        let mut r = ranges(5);
        r[4] = 4..4; // empty group: not counted, not replaced
        let grouped = &mut bindings;
        let mut counts = RuntimeCounts::new();
        let (mut uv_calls, mut tex_calls) = (0, 0);
        apply(
            0x2f6c,
            groups(&r, grouped),
            &[rule],
            &mut counts,
            |channel| {
                uv_calls += 1;
                assert_eq!(channel, 1);
                Ok((vec![[0.25, 0.75]; 3], meta()))
            },
            |g| {
                tex_calls += 1;
                assert_eq!(g, 0x27fb);
                Ok(texture())
            },
            |_, slot| {
                assert_eq!(slot, 0);
                Ok(SlotSampler {
                    uv_channel: 1,
                    transform: crate::SamplingTransform::ScaleOffset(crate::UvTransform {
                        bits: [2f32.to_bits(), 2f32.to_bits(), 0, 0],
                    }),
                    chain: vec![3, 0],
                })
            },
        );
        assert_eq!((uv_calls, tex_calls), (1, 1));
        assert_eq!(counts[&rule].applied, 2);
        let a = bindings[0].as_ref().unwrap();
        let b = bindings[2].as_ref().unwrap();
        assert!(Arc::ptr_eq(a, b));
        let t = a.texture().unwrap();
        assert_eq!(t.uv, vec![[0.25, 0.75]; 3]);
        assert_eq!(t.sampling_uv, vec![[0.5, 1.5]; 3]);
        assert_eq!(t.selection.uv_channel, 1);
        assert_eq!(t.additional_texture_slots, vec![3]);
        assert_eq!(t.wrap, crate::TextureWrapping::uniform(true));
        assert!(a.label().starts_with("Runtime texture 000027fb"));
        assert!(matches!(
            bindings[1].as_deref(),
            Ok(MaterialAsset::Solid(_))
        ));
        assert!(matches!(
            bindings[4].as_deref(),
            Ok(MaterialAsset::Solid(_))
        ));
        let out = lines(&counts);
        assert!(out[0].contains("00002f6c slot 0: 2 groups"));
        assert!(
            out[1].contains("UV1") && out[1].contains("[3, 0]"),
            "{out:?}"
        );
    }

    #[test]
    fn apply_failures_keep_solid_and_record_reason() {
        let rule = RuntimeTexture::parse("00002f6c:0:000027fb").unwrap();
        let mut bindings = vec![Ok(solid(vec![0]))];
        let r = ranges(1);
        let mut counts = RuntimeCounts::new();
        apply(
            0x2f6c,
            groups(&r, &mut bindings),
            &[rule],
            &mut counts,
            |_| Ok((vec![[0.; 2]; 3], meta())),
            |_| Err("Unresolved: GUID".into()),
            |_, _| Err("no sampler box for slot 0".into()),
        );
        assert!(matches!(
            bindings[0].as_deref(),
            Ok(MaterialAsset::Solid(_))
        ));
        assert_eq!(counts[&rule].applied, 0);
        assert_eq!(
            counts[&rule].not_applied["texture 000027fb: Unresolved: GUID"],
            1
        );
        // Vertex/UV count mismatch is refused, not truncated.
        let mut counts = RuntimeCounts::new();
        apply(
            0x2f6c,
            groups(&r, &mut bindings),
            &[rule],
            &mut counts,
            |channel| {
                assert_eq!(channel, 0);
                Ok((vec![[0.; 2]; 2], meta()))
            },
            |_| Ok(texture()),
            |_, _| Err("no sampler box for slot 0".into()),
        );
        assert_eq!(counts[&rule].applied, 0);
        assert_eq!(
            counts[&rule].not_applied["Invalid: UV0/vertex correspondence"],
            1
        );
        // Another mesh: no calls, no entry.
        let mut counts = RuntimeCounts::new();
        apply(
            0x1234,
            groups(&r, &mut bindings),
            &[rule],
            &mut counts,
            |_| panic!("no UV read"),
            |_| panic!("no texture read"),
            |_, _| panic!("no material read"),
        );
        assert!(counts.is_empty());
    }

    #[test]
    fn slot_sampler_reads_the_slots_sample_box_and_its_output_path() {
        use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};
        let node = |kind: u32, p: [u32; 8]| BoxMetadata {
            type_raw: kind,
            subtype_raw: 0,
            declared_parameter_count: 8,
            parameters_raw: p,
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
        };
        let edge = |from: i32, to: i32| WireMetadata {
            box_from: from,
            box_to: to,
            port_from_raw: 0,
            port_to_raw: 0,
            swizzle_raw: [0; 5],
            from_in_range: true,
            to_in_range: true,
        };
        let one = 1f32.to_bits();
        let half = 0.5f32.to_bits();
        let boxes = vec![
            node(0, [0; 8]),
            node(1, [one, one, 0, 0, 0, 1, 0, 0]),
            node(1, [half, one, 0, 0, 1, 0, 0, 0]),
            node(3, [0; 8]),
        ];
        let wires = vec![edge(2, 3), edge(3, 0), edge(1, 0)];
        let mut graph = GraphMetadata {
            box_count: boxes.len(),
            wire_count: wires.len(),
            boxes,
            wires,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
        };
        let s = slot_sampler(&graph, 0).unwrap();
        assert_eq!((s.uv_channel, s.chain.clone()), (1, vec![3, 0]));
        assert!(
            s.transform.label().contains("0.5"),
            "{}",
            s.transform.label()
        );
        assert_eq!(slot_sampler(&graph, 1).unwrap().chain, vec![0]);
        assert!(slot_sampler(&graph, 2).unwrap_err().contains("no sampler"));
        graph.wires.insert(0, edge(3, 3)); // a loop to a seen box is skipped
        assert_eq!(slot_sampler(&graph, 0).unwrap().chain, vec![3, 0]);
        graph.wires.push(edge(9, 0));
        assert!(slot_sampler(&graph, 0).is_err());
    }
}
