//! Renderer policy, not original game coordinates/lighting. See docs/bevy-preview.md.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::Indices,
    prelude::*,
    render::render_resource::{
        Extent3d, PrimitiveTopology, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension,
    },
};

/// Bind-pose centre and largest extent; posed frames reuse it so the view stays still.
pub type DisplayFrame = ([f64; 3], f64);
pub fn display_frame(positions: &[[f32; 3]]) -> Option<DisplayFrame> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in positions {
        for i in 0..3 {
            lo[i] = lo[i].min(f64::from(p[i]));
            hi[i] = hi[i].max(f64::from(p[i]));
        }
    }
    let span = (0..3).map(|i| hi[i] - lo[i]).fold(0f64, f64::max);
    (span > 0.).then(|| (std::array::from_fn(|i| (lo[i] + hi[i]) * 0.5), span))
}
/// Display-only centering and uniform normalization in f64 avoids finite f32 overflow.
pub fn to_display(positions: &[[f32; 3]], (centre, span): DisplayFrame) -> Vec<[f32; 3]> {
    positions
        .iter()
        .map(|p| std::array::from_fn(|i| ((f64::from(p[i]) - centre[i]) / span) as f32))
        .collect()
}
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
    if positions.is_empty()
        || positions.len() > lbp_preview::MAX_VERTICES
        || triangles.len() > lbp_preview::MAX_TRIANGLES
    {
        return Err("Unsupported: empty or over-budget geometry".into());
    }
    if positions.iter().flatten().any(|v| !v.is_finite())
        || uv.is_some_and(|v| {
            v.len() != positions.len() || v.iter().flatten().any(|v| !v.is_finite())
        })
    {
        return Err("Invalid: nonfinite position/UV or UV correspondence".into());
    }
    let frame = display_frame(positions).ok_or("Unsupported: zero-size geometry")?;
    let display: Vec<[f32; 3]> = if normalize {
        to_display(positions, frame)
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
/// D121: trust of the generated normals for preview lighting. A referenced vertex whose
/// face normals (nearly) cancel, e.g. a front and back face sharing it, ends with an
/// arbitrary fallback direction. Returns (weak vertices, referenced vertices).
pub fn normal_trust(positions: &[[f32; 3]], indices: &[u32]) -> (usize, usize) {
    let mut sum = vec![Vec3::ZERO; positions.len()];
    let mut magnitude = vec![0f32; positions.len()];
    for t in indices.as_chunks::<3>().0 {
        let at = |i: u32| positions.get(i as usize).copied().map(Vec3::from);
        let (Some(a), Some(b), Some(c)) = (at(t[0]), at(t[1]), at(t[2])) else {
            continue;
        };
        let n = (b - a).cross(c - a);
        for &i in t {
            sum[i as usize] += n;
            magnitude[i as usize] += n.length();
        }
    }
    let referenced = magnitude.iter().filter(|&&m| m > 0.).count();
    let weak = sum
        .iter()
        .zip(&magnitude)
        .filter(|(s, &m)| m > 0. && s.length() <= m * 1e-3)
        .count();
    (weak, referenced)
}
/// P2v: replaces the generated normals of every referenced vertex with the game's decoded
/// ones (unit length; a stored normal that is non-finite or shorter than 0.5, such as a
/// clamped packed one, keeps the generated normal). Returns how many referenced vertices
/// took a game normal, how many are referenced, and the mean |dot| between the game and
/// generated normals over the replaced vertices (a frame/winding diagnostic: near 1 means
/// the stored normals agree with our face geometry).
pub fn apply_game_normals(mesh: &mut Mesh, game: &[[f32; 3]]) -> (usize, usize, f32) {
    use bevy::mesh::VertexAttributeValues;
    let (Some(VertexAttributeValues::Float32x3(generated)), Some(Indices::U32(indices))) =
        (mesh.attribute(Mesh::ATTRIBUTE_NORMAL), mesh.indices())
    else {
        return (0, 0, 0.);
    };
    if game.len() != generated.len() {
        return (0, 0, 0.);
    }
    let mut referenced = vec![false; generated.len()];
    for &i in indices.iter() {
        referenced[i as usize] = true;
    }
    let mut out = generated.clone();
    let (mut used, mut total, mut agree) = (0usize, 0usize, 0f32);
    for (i, &is_ref) in referenced.iter().enumerate() {
        if !is_ref {
            continue;
        }
        total += 1;
        let n = Vec3::from(game[i]);
        if !n.is_finite() || n.length_squared() < 0.25 {
            continue;
        }
        let n = n.normalize();
        agree += n.dot(Vec3::from(generated[i])).abs();
        out[i] = n.to_array();
        used += 1;
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, out);
    (used, total, if used > 0 { agree / used as f32 } else { 0. })
}
/// `normal_trust` over a built mesh: true when every referenced vertex is well defined.
pub fn mesh_normals_trusted(mesh: &Mesh) -> bool {
    use bevy::mesh::VertexAttributeValues;
    let (Some(VertexAttributeValues::Float32x3(p)), Some(Indices::U32(i))) =
        (mesh.attribute(Mesh::ATTRIBUTE_POSITION), mesh.indices())
    else {
        return false;
    };
    let (weak, referenced) = normal_trust(p, i);
    weak == 0 && referenced > 0
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
    image_axes_fmt(width, height, pixels, wrap, true)
}
/// Anisotropy of the trilinear game-texture sampler: the Pod RenderDoc capture shows
/// LINEAR_MIPMAP_LINEAR, LOD bias 0 and MAX_ANISOTROPY 1 (docs/p2u-mipmaps.md).
pub const ANISOTROPY: u16 = 1;
fn srgb_to_linear(v: u8) -> f32 {
    let c = f32::from(v) / 255.;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0., 1.);
    let v = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    };
    (v * 255. + 0.5) as u8
}
/// Mip count down to 1x1 (floor(log2(max side)) + 1).
pub fn mip_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}
/// Full box-filtered mip chain appended after the base level (mip 0 first, then 1..n,
/// tightly packed, the layout `Image` expects for a single-layer 2D texture). `srgb`
/// averages colour in linear light (alpha is averaged as stored); data textures
/// average the stored values. Odd sides clamp the last row/column. Tiny textures (a
/// side of 1) add levels along the other axis only until 1x1.
pub fn with_mips(width: u32, height: u32, base: Vec<u8>, srgb: bool) -> Vec<u8> {
    let levels = mip_count(width, height);
    if levels <= 1 {
        return base;
    }
    let lut: Vec<f32> = (0..=255u8).map(srgb_to_linear).collect();
    let mut out = base;
    let (mut pw, mut ph) = (width as usize, height as usize);
    let mut prev_start = 0usize;
    for _ in 1..levels {
        let (w, h) = ((pw / 2).max(1), (ph / 2).max(1));
        let start = out.len();
        out.reserve(w * h * 4);
        for y in 0..h {
            let y0 = (y * 2).min(ph - 1);
            let y1 = (y * 2 + 1).min(ph - 1);
            for x in 0..w {
                let x0 = (x * 2).min(pw - 1);
                let x1 = (x * 2 + 1).min(pw - 1);
                let at = |xx: usize, yy: usize| prev_start + (yy * pw + xx) * 4;
                let taps = [at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1)];
                for c in 0..4 {
                    let v = if srgb && c < 3 {
                        let sum: f32 = taps.iter().map(|&t| lut[usize::from(out[t + c])]).sum();
                        linear_to_srgb(sum / 4.)
                    } else {
                        let sum: u32 = taps.iter().map(|&t| u32::from(out[t + c])).sum();
                        ((sum + 2) / 4) as u8
                    };
                    out.push(v);
                }
            }
        }
        prev_start = start;
        pw = w;
        ph = h;
    }
    out
}
/// Single-channel box-filtered mip chain (mip 0 first, tightly packed), stored values
/// averaged (data texture).
pub fn with_mips_r8(width: u32, height: u32, base: Vec<u8>) -> Vec<u8> {
    let levels = mip_count(width, height);
    let mut out = base;
    let (mut pw, mut ph) = (width as usize, height as usize);
    let mut prev_start = 0usize;
    for _ in 1..levels {
        let (w, h) = ((pw / 2).max(1), (ph / 2).max(1));
        let start = out.len();
        out.reserve(w * h);
        for y in 0..h {
            let (y0, y1) = ((y * 2).min(ph - 1), (y * 2 + 1).min(ph - 1));
            for x in 0..w {
                let (x0, x1) = ((x * 2).min(pw - 1), (x * 2 + 1).min(pw - 1));
                let at = |xx: usize, yy: usize| u32::from(out[prev_start + yy * pw + xx]);
                let sum = at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1);
                out.push(((sum + 2) / 4) as u8);
            }
        }
        prev_start = start;
        pw = w;
        ph = h;
    }
    out
}
/// P3k: a backdrop lightmap as an R8 texture with a full mip chain, clamped, trilinear.
pub fn lightmap_image(width: u32, height: u32, r8: &[u8]) -> Result<Image, String> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("Unsupported: lightmap extent".into());
    }
    if r8.len() != width as usize * height as usize {
        return Err("Invalid: lightmap pixel count".into());
    }
    let levels = mip_count(width, height);
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        r8.to_vec(),
        TextureFormat::R8Unorm,
        RenderAssetUsages::default(),
    );
    if levels > 1 {
        if let Some(base) = image.data.take() {
            image.data = Some(with_mips_r8(width, height, base));
        }
        image.texture_descriptor.mip_level_count = levels;
    }
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: ANISOTROPY,
        ..default()
    });
    Ok(image)
}
/// P3l: a six-face RGBA cube map (faces +X, -X, +Y, -Y, +Z, -Z, tightly packed `rgba` per
/// face) as a cube texture with a box-filtered mip chain per face, sRGB like the other
/// reflection maps, clamped and trilinear. Faces must be square and equal.
pub fn cube_image(size: u32, faces: &[&[u8]]) -> Result<Image, String> {
    if faces.len() != 6 || size == 0 || size > 1024 {
        return Err("Unsupported: cube map extent".into());
    }
    let face_bytes = size as usize * size as usize * 4;
    if faces.iter().any(|f| f.len() != face_bytes) {
        return Err("Invalid: cube map face size".into());
    }
    let levels = mip_count(size, size);
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        faces.concat(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    if levels > 1 {
        // Layer-major: each face's whole chain, then the next face.
        image.data = Some(
            faces
                .iter()
                .flat_map(|f| with_mips(size, size, f.to_vec(), true))
                .collect(),
        );
        image.texture_descriptor.mip_level_count = levels;
    }
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    Ok(image)
}
fn image_axes_fmt(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    wrap: lbp_preview::TextureWrapping,
    srgb: bool,
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
    let levels = mip_count(width, height);
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::default(),
    );
    // `Image::new` checks the base level only; the packed chain replaces the data after.
    if levels > 1 {
        if let Some(base) = image.data.take() {
            image.data = Some(with_mips(width, height, base, srgb));
        }
        image.texture_descriptor.mip_level_count = levels;
    }
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
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: ANISOTROPY,
        ..default()
    });
    Ok(image)
}
/// Point sampling (the texture asks for it): no mip blending, no anisotropy (wgpu
/// rejects anisotropy with non-linear filters).
pub fn set_nearest(image: &mut Image) {
    if let ImageSampler::Descriptor(s) = &mut image.sampler {
        s.mag_filter = ImageFilterMode::Nearest;
        s.min_filter = ImageFilterMode::Nearest;
        s.mipmap_filter = ImageFilterMode::Nearest;
        s.anisotropy_clamp = 1;
    }
}
/// P2q: a data texture (bump map) sampled without the sRGB decode.
pub fn linear_image(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    wrap: lbp_preview::TextureWrapping,
) -> Result<Image, String> {
    image_axes_fmt(width, height, pixels, wrap, false)
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
    if ranges.len() != bindings.len() || triangles.len() > lbp_preview::MAX_TRIANGLES {
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
    fn normal_trust_flags_cancelling_double_sided_vertices() {
        let quad = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
        // One-sided quad: all four vertices trusted.
        assert_eq!(super::normal_trust(&quad, &[0, 1, 2, 0, 2, 3]), (0, 4));
        // Same vertices also used by the reversed (back) triangles: normals cancel.
        let both = [0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0];
        assert_eq!(super::normal_trust(&quad, &both), (4, 4));
        // A box corner (three perpendicular faces) is trusted.
        let corner = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        assert_eq!(
            super::normal_trust(&corner, &[0, 2, 1, 0, 1, 3, 0, 3, 2]),
            (0, 4)
        );
        // Out-of-range indices are skipped; unreferenced vertices are not counted.
        assert_eq!(super::normal_trust(&quad, &[0, 1, 9]), (0, 0));
        let mesh = super::mesh(&quad, &[[0, 1, 2], [0, 2, 3]], None).unwrap();
        assert!(super::mesh_normals_trusted(&mesh));
        let mesh = super::mesh(&quad, &[[0, 1, 2], [2, 1, 0]], None).unwrap();
        assert!(!super::mesh_normals_trusted(&mesh));
    }
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
    #[test]
    fn game_normals_replace_referenced_vertices_only() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [5., 5., 5.]];
        let mut m = mesh(&positions, &[[0, 1, 2]], None).unwrap();
        let game = [[0., 0., 2.], [0., 0., -1.], [0., 0.1, 0.], [1., 0., 0.]];
        let (used, total, agree) = apply_game_normals(&mut m, &game);
        // Vertex 2's stored normal is too short and keeps the generated one; vertex 3 is unreferenced.
        assert_eq!((used, total), (2, 3));
        assert!((agree - 1.).abs() < 1e-5);
        let Some(VertexAttributeValues::Float32x3(n)) = m.attribute(Mesh::ATTRIBUTE_NORMAL) else {
            panic!()
        };
        assert_eq!(n[0], [0., 0., 1.]);
        assert_eq!(n[1], [0., 0., -1.]);
        assert_eq!(n[2], [0., 0., 1.]);
        assert_eq!(apply_game_normals(&mut m, &game[..2]), (0, 0, 0.));
    }
    #[test]
    fn r8_mip_chain_averages_stored_values() {
        let chain = with_mips_r8(2, 2, vec![0, 100, 200, 255]);
        assert_eq!(chain.len(), 5);
        assert_eq!(chain[4], 139);
        let chain = with_mips_r8(4, 1, vec![10, 20, 30, 40]);
        assert_eq!(chain.len(), 4 + 2 + 1);
        assert!(lightmap_image(2, 2, &[0; 3]).is_err());
        let image = lightmap_image(8, 8, &[7; 64]).unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 4);
        assert_eq!(image.data.as_ref().unwrap().len(), 64 + 16 + 4 + 1);
    }

    #[test]
    fn cube_image_is_six_layers_with_a_chain_per_face() {
        let face = vec![9u8; 4 * 4 * 4];
        let faces = [face.as_slice(); 6];
        let image = cube_image(4, &faces).unwrap();
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
        assert_eq!(image.data.as_ref().unwrap().len(), 6 * (64 + 16 + 4));
        assert_eq!(
            image.texture_view_descriptor.as_ref().unwrap().dimension,
            Some(TextureViewDimension::Cube)
        );
        assert!(cube_image(4, &faces[..5]).is_err());
        assert!(cube_image(4, &[&face[..8]; 6]).is_err());
    }

    #[test]
    fn mip_chain_sizes_and_linear_light_average() {
        assert_eq!(mip_count(1, 1), 1);
        assert_eq!(mip_count(4, 2), 3);
        assert_eq!(mip_count(5, 3), 3);
        assert_eq!(mip_count(512, 512), 10);
        // 2x2 black/white checker: the 1x1 level is linear-light mid grey (188), not 128.
        let px = vec![
            0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
        ];
        let chain = with_mips(2, 2, px.clone(), true);
        assert_eq!(chain.len(), 16 + 4);
        assert!((i32::from(chain[16]) - 188).abs() <= 1, "{}", chain[16]);
        assert_eq!(chain[19], 255);
        // Data textures average the stored values.
        let chain = with_mips(2, 2, px, false);
        assert_eq!(chain[16], 128);
        // Total size over odd, non-square extents matches the declared levels.
        let (w, h) = (5u32, 3u32);
        let chain = with_mips(w, h, vec![200; (w * h * 4) as usize], true);
        let (mut cw, mut ch, mut total) = (w, h, 0u32);
        for _ in 0..mip_count(w, h) {
            total += cw * ch * 4;
            cw = (cw / 2).max(1);
            ch = (ch / 2).max(1);
        }
        assert_eq!(chain.len() as u32, total);
        let i = image(8, 8, vec![255; 8 * 8 * 4], true).unwrap();
        assert_eq!(i.texture_descriptor.mip_level_count, 4);
        let ImageSampler::Descriptor(s) = &i.sampler else {
            panic!()
        };
        assert_eq!(s.mipmap_filter, ImageFilterMode::Linear);
        assert_eq!(s.anisotropy_clamp, ANISOTROPY);
        let mut n = image(8, 8, vec![255; 8 * 8 * 4], true).unwrap();
        set_nearest(&mut n);
        let ImageSampler::Descriptor(s) = &n.sampler else {
            panic!()
        };
        assert_eq!(
            (s.min_filter, s.mipmap_filter, s.anisotropy_clamp),
            (ImageFilterMode::Nearest, ImageFilterMode::Nearest, 1)
        );
        assert_eq!(
            image(1, 1, vec![255; 4], true)
                .unwrap()
                .texture_descriptor
                .mip_level_count,
            1
        );
    }
}
