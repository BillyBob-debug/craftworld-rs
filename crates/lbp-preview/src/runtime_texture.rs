//! P2e opt-in runtime texture slot binding (docs/p2e-runtime-texture.md).
//!
//! Some materials sample a texture slot the game fills at runtime (the Pod shell mesh
//! 00002f6c primitive 2 lists slot 0 as an unrendered required slot). With an explicit
//! `MESH:SLOT:TEXTURE` rule, a matching Solid-route group is drawn with that texture
//! instead of its solid colour. Which texture the game binds is not confirmed; the rule
//! is user-supplied. INFERRED display policy: mesh UV0 untransformed, Repeat on both
//! axes, untinted (the replaced solid colour is kept only in the label).
use crate::linked::{Asset, MaterialAsset, SolidAsset};
use lbp_catalog::attachment::LocationKey;
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
}

pub type RuntimeCounts = BTreeMap<RuntimeTexture, RuntimeOutcome>;

/// Status lines; empty when no rule was given (default behaviour unchanged).
pub fn lines(counts: &RuntimeCounts) -> Vec<String> {
    let mut out = Vec::new();
    for (rule, o) in counts {
        out.push(format!(
            "{}: {} groups (opt-in; UV0, Repeat, untinted: inferred; game binding unconfirmed)",
            rule.label(),
            o.applied
        ));
        for (reason, n) in &o.not_applied {
            out.push(format!("  not applied {n} groups: {reason}"));
        }
    }
    out
}

/// A mesh's UV0 coordinates and its UV metadata.
pub type MeshUv0 = (Vec<[f32; 2]>, lbp_mesh::UvMetadata);
/// Decoded runtime texture and where it came from.
pub type LoadedTexture = (lbp_texture::Texture, LocationKey);

/// One loaded mesh's groups: triangle ranges, vertex count and per-primitive bindings.
pub(crate) struct Groups<'a> {
    pub ranges: &'a [std::ops::Range<usize>],
    pub vertices: usize,
    pub bindings: &'a mut [Result<Arc<MaterialAsset>, String>],
}

/// Applies matching rules to one loaded mesh. `uv0` decodes the mesh's UV0 on demand
/// (called at most once); `texture` loads a texture GUID (called at most once per
/// rule). Failures leave the groups Solid and are counted with their reason.
pub(crate) fn apply(
    mesh: u32,
    grouped: Groups<'_>,
    rules: &[RuntimeTexture],
    counts: &mut RuntimeCounts,
    mut uv0: impl FnMut() -> Result<MeshUv0, String>,
    mut texture: impl FnMut(u32) -> Result<LoadedTexture, String>,
) {
    let Groups {
        ranges,
        vertices,
        bindings,
    } = grouped;
    let mut coords: Option<Result<MeshUv0, String>> = None;
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
            let uv = coords.get_or_insert_with(&mut uv0);
            let tex = loaded.get_or_insert_with(|| {
                texture(rule.texture)
                    .map(Arc::new)
                    .map_err(|e| format!("texture {:08x}: {e}", rule.texture))
            });
            let built = match (uv, tex) {
                (Ok((uv, meta)), Ok(tex)) => {
                    if uv.len() != vertices {
                        Err("Invalid: UV0/vertex correspondence".to_string())
                    } else {
                        Ok(replacement(solid, rule, uv, meta, tex))
                    }
                }
                (Err(e), _) => Err(format!("mesh UV0: {e}")),
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
    uv: &[[f32; 2]],
    meta: &lbp_mesh::UvMetadata,
    tex: &LoadedTexture,
) -> MaterialAsset {
    let selection = crate::DiffuseSelection {
        slot: rule.slot,
        uv_channel: 0,
        ..Default::default()
    };
    MaterialAsset::Texture(Box::new(Asset {
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
        product: None,
        tinted_rgba: None,
        sampling_uv: uv.to_vec(),
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

    fn solid(slots: Vec<usize>) -> Arc<MaterialAsset> {
        Arc::new(MaterialAsset::Solid(Box::new(SolidAsset {
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
            || {
                uv_calls += 1;
                Ok((vec![[0.25, 0.75]; 3], meta()))
            },
            |g| {
                tex_calls += 1;
                assert_eq!(g, 0x27fb);
                Ok(texture())
            },
        );
        assert_eq!((uv_calls, tex_calls), (1, 1));
        assert_eq!(counts[&rule].applied, 2);
        let a = bindings[0].as_ref().unwrap();
        let b = bindings[2].as_ref().unwrap();
        assert!(Arc::ptr_eq(a, b));
        let t = a.texture().unwrap();
        assert_eq!(t.sampling_uv, vec![[0.25, 0.75]; 3]);
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
        assert!(lines(&counts)[0].contains("00002f6c slot 0: 2 groups"));
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
            || Ok((vec![[0.; 2]; 3], meta())),
            |_| Err("Unresolved: GUID".into()),
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
            || Ok((vec![[0.; 2]; 2], meta())),
            |_| Ok(texture()),
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
            || panic!("no UV read"),
            |_| panic!("no texture read"),
        );
        assert!(counts.is_empty());
    }
}
