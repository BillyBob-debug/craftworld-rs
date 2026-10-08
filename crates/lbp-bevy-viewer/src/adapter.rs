//! Renderer policy, not original game coordinates/lighting. See docs/bevy-preview.md.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::Indices,
    prelude::*,
    render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};

pub fn mesh(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    uv: Option<&[[f32; 2]]>,
) -> Result<Mesh, String> {
    build_mesh(positions, triangles, uv, true)
}
/// D113 level scenes: source model coordinates are kept (no centering/normalization) so
/// each instance's world matrix places the mesh; the scene is framed as a whole instead.
pub fn mesh_local(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    uv: Option<&[[f32; 2]]>,
) -> Result<Mesh, String> {
    build_mesh(positions, triangles, uv, false)
}
fn build_mesh(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    uv: Option<&[[f32; 2]]>,
    normalize: bool,
) -> Result<Mesh, String> {
    if positions.is_empty() || positions.len() > 100_000 || triangles.len() > 50_000 {
        return Err("Unsupported: empty or over-budget geometry".into());
    }
    if positions.iter().flatten().any(|v| !v.is_finite())
        || uv.is_some_and(|v| {
            v.len() != positions.len() || v.iter().flatten().any(|v| !v.is_finite())
        })
    {
        return Err("Invalid: nonfinite position/UV or UV correspondence".into());
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in positions {
        for i in 0..3 {
            lo[i] = lo[i].min(f64::from(p[i]));
            hi[i] = hi[i].max(f64::from(p[i]));
        }
    }
    let span = (0..3).map(|i| hi[i] - lo[i]).fold(0f64, f64::max);
    if span <= 0. {
        return Err("Unsupported: zero-size geometry".into());
    }
    // Display-only centering and uniform normalization in f64 avoids finite f32 overflow.
    let display: Vec<[f32; 3]> = if normalize {
        positions
            .iter()
            .map(|p| {
                std::array::from_fn(|i| ((f64::from(p[i]) - (lo[i] + hi[i]) * 0.5) / span) as f32)
            })
            .collect()
    } else {
        positions.to_vec()
    };
    let mut indices = Vec::with_capacity(triangles.len() * 3);
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for t in triangles {
        if t.iter().any(|&i| i as usize >= positions.len()) {
            return Err("Invalid: triangle index".into());
        }
        let a = Vec3::from(display[t[0] as usize]);
        let b = Vec3::from(display[t[1] as usize]);
        let c = Vec3::from(display[t[2] as usize]);
        let normal = (b - a).cross(c - a);
        if normal.length_squared() > 1e-20 {
            indices.extend_from_slice(t);
            for &index in t {
                normals[index as usize] += normal;
            }
        }
    }
    if indices.is_empty() {
        return Err("Unsupported: no nondegenerate faces".into());
    }
    let normals: Vec<[f32; 3]> = normals
        .into_iter()
        .map(|n| n.try_normalize().unwrap_or(Vec3::Y).to_array())
        .collect();
    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, display)
    // Bounded generated diagnostic normals satisfy the unlit pipeline, never original attributes.
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        uv.map_or_else(|| vec![[0.; 2]; positions.len()], <[_]>::to_vec),
    )
    .with_inserted_indices(Indices::U32(indices)))
}
/// Separate GPU sampling attribute, preserving the primary coordinates and indices.
pub fn secondary_uv(mesh: &mut Mesh, uv: &[[f32; 2]]) -> Result<(), String> {
    if uv.len() != mesh.count_vertices() || uv.iter().flatten().any(|v| !v.is_finite()) {
        return Err("Invalid: secondary UV correspondence".into());
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv.to_vec());
    Ok(())
}
pub fn image(width: u32, height: u32, pixels: Vec<u8>, repeat: bool) -> Result<Image, String> {
    image_axes(
        width,
        height,
        pixels,
        lbp_preview::TextureWrapping::uniform(repeat),
    )
}
/// Original U/V addressing retained independently; no coordinate folding.
pub fn image_axes(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    wrap: lbp_preview::TextureWrapping,
) -> Result<Image, String> {
    let count = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or("Image size overflow")?;
    if width == 0 || height == 0 || count * 4 != pixels.len() as u64 {
        return Err("Invalid: texture extent/budget".into());
    }
    // GPU upload keeps the 4 Mi pixel cap (at most 16 MiB RGBA per image): D95's 4096x4096
    // B8 masks decode on the CPU (64 MiB RGBA) but are refused here, explicitly.
    if width > 4096 || height > 4096 || count > lbp_texture::MAX_PIXELS as u64 {
        return Err("Unsupported: GPU texture budget (4 Mi pixels)".into());
    }
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let address = |axis| match axis {
        lbp_preview::TextureAddress::Repeat => ImageAddressMode::Repeat,
        lbp_preview::TextureAddress::MirroredRepeat => ImageAddressMode::MirrorRepeat,
        // Raw 5 legacy CLAMP: edge-clamped display approximation (docs/d82-wrap-modes.md).
        lbp_preview::TextureAddress::ClampToEdge | lbp_preview::TextureAddress::Clamp => {
            ImageAddressMode::ClampToEdge
        }
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: address(wrap.u),
        address_mode_v: address(wrap.v),
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    Ok(image)
}
/// Distinct material draws per mesh: lbp-preview's 64-load cap plus one neutral bucket.
/// GPU image memory stays bounded by lbp-preview's 640 MiB retained-bytes budget.
pub const MAX_DRAWS: usize = 65;
/// Validate the complete primitive partition before grouping draws by shared binding.
/// Preserve triangle order, overlapping source ownership, and one neutral bucket.
pub fn buckets<T>(
    triangles: &[[u32; 3]],
    ranges: &[std::ops::Range<usize>],
    bindings: &[Result<std::sync::Arc<T>, String>],
) -> Result<Vec<Bucket<T>>, String> {
    if ranges.len() != bindings.len() || triangles.len() > 50_000 {
        return Err("Invalid: primitive binding correspondence".into());
    }
    let mut end = 0;
    let mut out: Vec<Bucket<T>> = Vec::new();
    for (range, binding) in ranges.iter().zip(bindings) {
        if range.start != end || range.end < range.start || range.end > triangles.len() {
            return Err("Invalid: primitive triangle partition".into());
        }
        end = range.end;
        let asset = binding.as_ref().ok().cloned();
        let index = out
            .iter()
            .position(|(existing, _)| match (existing, &asset) {
                (None, None) => true,
                (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
                _ => false,
            })
            .unwrap_or_else(|| {
                out.push((asset, Vec::new()));
                out.len() - 1
            });
        // D66: one draw per distinct material (preview cap 64) plus one neutral bucket.
        if out.len() > MAX_DRAWS {
            return Err("Unsupported: material draw budget".into());
        }
        out[index].1.extend_from_slice(&triangles[range.clone()]);
    }
    if end != triangles.len() {
        return Err("Invalid: unowned triangles".into());
    }
    Ok(out)
}
pub type Bucket<T> = (Option<std::sync::Arc<T>>, Vec<[u32; 3]>);
#[cfg(test)]
mod group_tests {
    use super::*;
    #[test]
    fn draw_budget_allows_64_materials_and_one_neutral_bucket() {
        let triangles = vec![[0, 1, 2]; 66];
        let ranges: Vec<_> = (0..66).map(|i| i..i + 1).collect();
        let mut bindings: Vec<Result<std::sync::Arc<usize>, String>> =
            (0..64).map(|i| Ok(std::sync::Arc::new(i))).collect();
        bindings.push(Err("neutral".into()));
        bindings.push(Err("neutral again".into()));
        assert_eq!(
            buckets(&triangles, &ranges, &bindings).unwrap().len(),
            MAX_DRAWS
        );
        bindings[65] = Ok(std::sync::Arc::new(64));
        assert!(buckets(&triangles, &ranges, &bindings).is_err());
    }
    #[test]
    fn mixed_bindings_preserve_ownership_and_shared_full_mesh_alignment() {
        let a = std::sync::Arc::new(1);
        let b = std::sync::Arc::new(2);
        let triangles = [[0, 1, 2], [2, 1, 3], [0, 1, 2], [3, 4, 5]];
        let groups = buckets(
            &triangles,
            &[0..1, 1..2, 2..3, 3..4, 4..4],
            &[
                Ok(a.clone()),
                Err("complex graph".into()),
                Ok(a),
                Ok(b),
                Err("empty".into()),
            ],
        )
        .unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].1, [triangles[0], triangles[2]]);
        assert_eq!(groups[1].1, [triangles[1]]);
        assert!(groups[1].0.is_none());
        assert_eq!(groups[2].1, [triangles[3]]);
        let positions = [
            [-3., 0., 0.],
            [-2., 0., 0.],
            [-3., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [3., 1., 0.],
        ];
        let m0 = mesh(&positions, &groups[0].1, None).unwrap();
        let m1 = mesh(&positions, &groups[2].1, None).unwrap();
        assert_eq!(
            format!("{:?}", m0.attribute(Mesh::ATTRIBUTE_POSITION)),
            format!("{:?}", m1.attribute(Mesh::ATTRIBUTE_POSITION))
        );
        assert!(buckets(
            &triangles,
            std::slice::from_ref(&(0..4)),
            &[Ok(std::sync::Arc::new(0))]
        )
        .is_ok());
        assert!(buckets(
            &triangles,
            std::slice::from_ref(&(0..3)),
            &[Ok(std::sync::Arc::new(0))]
        )
        .is_err());
        assert!(buckets::<i32>(&triangles, std::slice::from_ref(&(0..4)), &[]).is_err());
        assert!(buckets(
            &triangles,
            std::slice::from_ref(&(1..4)),
            &[Ok(std::sync::Arc::new(0))]
        )
        .is_err());
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn secondary_sampling_preserves_both_arrays_and_index_ownership() {
        let primary = [[0., 0.], [1., 0.], [0., 1.]];
        let secondary = [[1., -0.], [0., 1.], [1., 1.]];
        let mut mesh = super::mesh(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            &[[2, 0, 1]],
            Some(&primary),
        )
        .unwrap();
        super::secondary_uv(&mut mesh, &secondary).unwrap();
        let values = |attribute| match mesh.attribute(attribute).unwrap() {
            bevy::mesh::VertexAttributeValues::Float32x2(v) => {
                v.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>()
            }
            _ => panic!("wrong sampling attribute"),
        };
        assert_eq!(
            values(bevy::mesh::Mesh::ATTRIBUTE_UV_0),
            primary.map(|p| p.map(f32::to_bits))
        );
        assert_eq!(
            values(bevy::mesh::Mesh::ATTRIBUTE_UV_1),
            secondary.map(|p| p.map(f32::to_bits))
        );
        assert_eq!(
            mesh.indices(),
            Some(&bevy::mesh::Indices::U32(vec![2, 0, 1]))
        );
        assert!(super::secondary_uv(&mut mesh, &secondary[..2]).is_err());
        assert!(super::secondary_uv(&mut mesh, &[[f32::NAN, 0.]; 3]).is_err());
    }
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    #[test]
    fn correspondence_extreme_bounds_and_degenerate_filter() {
        let positions = [[-f32::MAX, 0., 0.], [f32::MAX, 0., 0.], [0., f32::MAX, 0.]];
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let m = mesh(&positions, &[[0, 1, 2], [0, 0, 1]], Some(&uv)).unwrap();
        assert!(matches!(m.indices(), Some(Indices::U32(v)) if v == &[0,1,2]));
        assert!(
            matches!(m.attribute(Mesh::ATTRIBUTE_UV_0), Some(VertexAttributeValues::Float32x2(v)) if v == &uv)
        );
        assert!(
            matches!(m.attribute(Mesh::ATTRIBUTE_POSITION), Some(VertexAttributeValues::Float32x3(v)) if v.iter().flatten().all(|x| x.is_finite()))
        );
        assert_eq!(positions[0][0], -f32::MAX);
        assert!(mesh(&positions, &[[0, 1, 3]], None).is_err());
        assert!(mesh(&[[0.; 3]; 3], &[[0, 1, 2]], None).is_err());
        assert!(mesh(&positions, &[[0, 1, 2]], Some(&[[0.; 2]])).is_err());
    }
    #[test]
    fn sampler_extent_and_srgb_are_explicit() {
        for repeat in [false, true] {
            let i = image(2, 2, vec![255; 16], repeat).unwrap();
            assert_eq!(i.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
            let ImageSampler::Descriptor(s) = i.sampler else {
                panic!()
            };
            assert_eq!(
                s.address_mode_u,
                if repeat {
                    ImageAddressMode::Repeat
                } else {
                    ImageAddressMode::ClampToEdge
                }
            );
            assert_eq!(s.address_mode_v, s.address_mode_u);
        }
        assert!(image(u32::MAX, 2, vec![], true).is_err());
        assert!(image(2, 2, vec![0; 15], true).is_err());
        // D95: a CPU-decoded 4096x4096 B8 mask is refused at upload, not uploaded.
        assert_eq!(
            image(4096, 4096, vec![0; lbp_texture::MAX_RGBA], true).unwrap_err(),
            "Unsupported: GPU texture budget (4 Mi pixels)"
        );
        for raw in [
            [1, 3],
            [3, 1],
            [2, 2],
            [1, 2],
            [2, 1],
            [2, 3],
            [3, 2],
            [5, 1],
            [2, 5],
            [5, 5],
        ] {
            let wrap = lbp_preview::TextureWrapping::from_raw(Some(raw[0]), Some(raw[1])).unwrap();
            let i = image_axes(2, 2, vec![255; 16], wrap).unwrap();
            let ImageSampler::Descriptor(s) = i.sampler else {
                panic!()
            };
            let expected = |raw| match raw {
                1 => ImageAddressMode::Repeat,
                2 => ImageAddressMode::MirrorRepeat,
                _ => ImageAddressMode::ClampToEdge,
            };
            assert_eq!(s.address_mode_u, expected(raw[0]));
            assert_eq!(s.address_mode_v, expected(raw[1]));
            assert_eq!(s.mag_filter, ImageFilterMode::Linear);
            assert_eq!(s.min_filter, ImageFilterMode::Linear);
        }
    }
}
