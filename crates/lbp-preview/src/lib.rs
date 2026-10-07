//! Shared bounded CPU preview preparation. No GUI, GPU, decryption or executable tools.
//! Supported format/predicate evidence remains docs/textured-mesh.md.
mod affine;
mod alpha_mix;
pub use alpha_mix::AlphaDiffuseMix;
mod animation_base;
pub use animation_base::AnimationBaseSnapshot;
mod blend;
mod blend_mask;
pub use blend_mask::{blend_factor, BlendDiffuseMask, BLEND_THRESHOLD_BITS};
mod border_domain;
pub use blend::DiffuseBlend;
pub use border_domain::{BorderDomain, INTERIOR_MARGIN};
mod matrix;
mod product;
pub use matrix::{SamplingTransform, UvMatrix};
mod instance;
pub use instance::{project_instance_color, InstanceProjection, PreviewOptions};
mod linked;
mod solid;
mod tint;
mod wrapping;
pub use affine::UvTransform;
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    Catalog, LoadControl,
};
pub use linked::{
    load, resolve, select_diffuse, select_diffuse_with_animation_base, Asset, DiffuseSelection,
    MaterialAsset, RelativeSampling, SolidAsset,
};
pub use solid::{select_solid_diffuse, SolidSelection};
pub use tint::DiffuseTint;
pub use wrapping::{TextureAddress, TextureWrapping};
pub struct LoadedMesh {
    pub geometry: lbp_mesh::Geometry,
    pub linked: Option<Result<MaterialAsset, String>>,
}
pub fn load_mesh(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    textured: bool,
    control: &LoadControl,
) -> Result<LoadedMesh, String> {
    let (geometry, data) = read_mesh(source, key, control)?;
    let linked = textured.then(|| load(source, catalog, &geometry, &data, control));
    control.check()?;
    source.check_stable()?;
    Ok(LoadedMesh { geometry, linked })
}
fn read_mesh(
    source: &AttachedSource,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<(lbp_mesh::Geometry, Vec<u8>), String> {
    control.check()?;
    let mut data = source
        .read_hex_checked(key, 0, key.size.min(32) as usize, control)?
        .bytes;
    lbp_mesh::preflight(&data).map_err(|e| e.to_string())?;
    if key.size > lbp_mesh::MAX_INPUT as u64 {
        return Err("Mesh budget exceeded".into());
    }
    data.reserve(key.size as usize - data.len());
    while data.len() < key.size as usize {
        control.check()?;
        let count = (key.size as usize - data.len()).min(4096);
        let page = source
            .read_hex_checked(key, data.len() as u64, count, control)?
            .bytes;
        if page.len() != count {
            return Err("Short mesh page".into());
        }
        data.extend(page);
    }
    let geometry =
        lbp_mesh::decode_geometry(&data, || control.check().is_err()).map_err(|e| e.to_string())?;
    if geometry.triangles.len() > 50_000 {
        return Err("Wireframe preview budget: more than 50,000 triangles; inspect metadata with the diagnostic tool".into());
    }
    Ok((geometry, data))
}

/// Per-primitive outcomes share cached assets; no GPU handles or serialized asset data.
pub struct GroupedMesh {
    pub geometry: lbp_mesh::Geometry,
    pub bindings: Vec<Result<std::sync::Arc<MaterialAsset>, String>>,
}
/// D66: at most 64 distinct material loads per mesh, and the retained display bytes
/// of successful loads (decoded + derived pixels and coordinate arrays) may total at
/// most 640 MiB, the same aggregate the earlier eight-material count implied
/// (512 MiB raw + 128 MiB derived). A load that would exceed it fails explicitly
/// for its groups and is not retained. Each load keeps its own per-material read,
/// decompression and pixel limits, so worst-case stored reads and expansion scale
/// with the 64-load cap (docs/material-budget-preview.md). Derived graph metadata
/// clones are bounded by 1024 boxes / 4096 wires per material (no key arrays
/// copied). The parser's 4096 primitive cap is retained; failures stay explicit.
pub fn load_mesh_groups(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
) -> Result<GroupedMesh, String> {
    load_mesh_groups_with_animation_base(source, catalog, key, control, false)
}
/// Opt-in diagnostic snapshot of stored local sampler animation base values.
pub fn load_mesh_groups_with_animation_base(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
    animation_base: bool,
) -> Result<GroupedMesh, String> {
    load_mesh_groups_with_options(
        source,
        catalog,
        key,
        control,
        PreviewOptions {
            animation_base,
            ..Default::default()
        },
    )
}
/// Explicit diagnostic inputs; defaults retain static source-only behavior.
pub fn load_mesh_groups_with_options(
    source: &AttachedSource,
    catalog: &Catalog,
    key: &LocationKey,
    control: &LoadControl,
    options: PreviewOptions,
) -> Result<GroupedMesh, String> {
    if let Some(color) = options.instance_color {
        color.validate()?;
    }
    let (geometry, data) = read_mesh(source, key, control)?;
    let mut cache: std::collections::HashMap<
        LocationKey,
        Result<std::sync::Arc<MaterialAsset>, String>,
    > = std::collections::HashMap::new();
    let mut retained = 0usize;
    let mut bindings = Vec::with_capacity(geometry.metadata.primitives.len());
    for primitive in &geometry.metadata.primitives {
        control.check()?;
        let result = if primitive.material_flags != 0
            || primitive.alternatives_kind != 0
            || primitive.alternatives_flags != 0
        {
            Err("Unsupported: material flags or alternative materials".into())
        } else {
            resolve(
                catalog,
                primitive.material_guid,
                primitive.material_hash.as_deref(),
            )
            .and_then(|material_key| {
                cached_binding(
                    &mut cache,
                    &mut retained,
                    material_key,
                    MaterialAsset::retained_bytes,
                    || {
                        linked::load_primitive(
                            source, catalog, &geometry, &data, primitive, control, options,
                        )
                    },
                )
            })
        };
        bindings.push(result);
    }
    control.check()?;
    source.check_stable()?;
    Ok(GroupedMesh { geometry, bindings })
}

const MAX_MATERIAL_LOADS: usize = 64;
const MAX_RETAINED_BYTES: usize = 640 * 1024 * 1024;

fn cached_binding<T>(
    cache: &mut std::collections::HashMap<LocationKey, Result<std::sync::Arc<T>, String>>,
    retained: &mut usize,
    key: LocationKey,
    size: impl Fn(&T) -> usize,
    load: impl FnOnce() -> Result<T, String>,
) -> Result<std::sync::Arc<T>, String> {
    if let Some(result) = cache.get(&key) {
        return result.clone();
    }
    if cache.len() >= MAX_MATERIAL_LOADS {
        return Err("Unsupported: 64 distinct material preview budget".into());
    }
    let result = load().and_then(|value| {
        let total = retained
            .checked_add(size(&value))
            .filter(|&t| t <= MAX_RETAINED_BYTES)
            .ok_or("Unsupported: mesh preview memory budget (640 MiB retained)")?;
        *retained = total;
        Ok(std::sync::Arc::new(value))
    });
    cache.insert(key, result.clone());
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_bindings_share_success_failure_and_budget_without_new_reads() {
        let mut cache = std::collections::HashMap::new();
        let mut retained = 0;
        let size = |_: &usize| 1;
        let key = |n| LocationKey {
            path: format!("authored-{n}"),
            offset: 0,
            size: 1,
        };
        let first = cached_binding(&mut cache, &mut retained, key(0), size, || Ok(42)).unwrap();
        let repeat = cached_binding(&mut cache, &mut retained, key(0), size, || {
            panic!("duplicate read")
        })
        .unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &repeat));
        assert_eq!(
            cached_binding(&mut cache, &mut retained, key(1), size, || Err(
                "Unsupported graph".into()
            ))
            .unwrap_err(),
            "Unsupported graph"
        );
        assert_eq!(
            cached_binding(&mut cache, &mut retained, key(1), size, || panic!(
                "duplicate failed read"
            ))
            .unwrap_err(),
            "Unsupported graph"
        );
        for n in 2..MAX_MATERIAL_LOADS {
            cached_binding(&mut cache, &mut retained, key(n), size, || Ok(n)).unwrap();
        }
        assert_eq!(retained, MAX_MATERIAL_LOADS - 1); // failures retain nothing
        assert!(cached_binding(
            &mut cache,
            &mut retained,
            key(MAX_MATERIAL_LOADS),
            size,
            || panic!("count budget must precede read")
        )
        .unwrap_err()
        .contains("budget"));
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), size, || panic!(
                "cached after budget"
            ))
            .is_ok()
        );
        // Aggregate retained bytes: an over-budget load fails, is cached as failed,
        // and leaves the total unchanged; later small loads still fit.
        let mut cache = std::collections::HashMap::new();
        let mut retained = MAX_RETAINED_BYTES - 10;
        let big = |v: &usize| *v;
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), big, || Ok(11))
                .unwrap_err()
                .contains("memory budget")
        );
        assert_eq!(retained, MAX_RETAINED_BYTES - 10);
        assert!(
            cached_binding(&mut cache, &mut retained, key(0), big, || panic!("cached")).is_err()
        );
        assert!(cached_binding(&mut cache, &mut retained, key(1), big, || Ok(10)).is_ok());
        assert_eq!(retained, MAX_RETAINED_BYTES);
    }
}
