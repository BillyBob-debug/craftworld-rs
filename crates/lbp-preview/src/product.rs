//! Independent bounded nearest-grid diffuse product. Pin/coverage: docs/product-preview.md.
//! Optional static COLOR mix before encoding: docs/static-mix-preview.md.
use crate::SamplingTransform;
pub(crate) fn scales(transform: SamplingTransform) -> Result<[u32; 2], String> {
    let SamplingTransform::ScaleOffset(t) = transform else {
        return Err("Unsupported: texture product matrix transform".into());
    };
    let [x, y, u, v] = t.values();
    if !x.is_finite()
        || !y.is_finite()
        || x.fract() != 0.
        || y.fract() != 0.
        || !(1. ..=16.).contains(&x)
        || !(1. ..=16.).contains(&y)
        || u != 0.
        || v != 0.
    {
        return Err(
            "Unsupported: texture product needs integer scales 1..16 and zero offset".into(),
        );
    }
    Ok([x as u32, y as u32])
}
fn grid(a: u32, b: u32) -> Result<u32, String> {
    let (mut x, mut y) = (a, b);
    while y != 0 {
        (x, y) = (y, x % y);
    }
    a.checked_div(x)
        .and_then(|v| v.checked_mul(b))
        .filter(|v| *v <= 4096)
        .ok_or("Unsupported: texture product grid budget".into())
}
pub(crate) fn pixels(
    a: &lbp_texture::Texture,
    b: &lbp_texture::Texture,
    ta: SamplingTransform,
    tb: SamplingTransform,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    bake(
        [a.header.width, a.header.height],
        &a.rgba,
        scales(ta)?,
        [b.header.width, b.header.height],
        &b.rgba,
        scales(tb)?,
        cancelled,
    )
}
pub(crate) fn pixels_blended(
    a: &lbp_texture::Texture,
    b: &lbp_texture::Texture,
    ta: SamplingTransform,
    tb: SamplingTransform,
    blend: crate::DiffuseBlend,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    blend.validate()?;
    bake_policy(
        [
            [a.header.width, a.header.height],
            [b.header.width, b.header.height],
        ],
        [&a.rgba, &b.rgba],
        [scales(ta)?, scales(tb)?],
        Some(blend),
        cancelled,
    )
}
fn bake(
    da: [u32; 2],
    a: &[u8],
    sa: [u32; 2],
    db: [u32; 2],
    b: &[u8],
    sb: [u32; 2],
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    bake_policy([da, db], [a, b], [sa, sb], None, cancelled)
}
pub(crate) fn extent(
    dimensions: [[u32; 2]; 2],
    sources: [&[u8]; 2],
    scales: [[u32; 2]; 2],
) -> Result<(u32, u32, u64), String> {
    let [da, db] = dimensions;
    let [sa, sb] = scales;
    for (d, p) in dimensions.iter().zip(sources) {
        if d.contains(&0)
            || d.iter().any(|n| *n > 4096)
            || u64::from(d[0]) * u64::from(d[1]) > lbp_texture::MAX_PIXELS as u64
            || u64::from(d[0]) * u64::from(d[1]) * 4 != p.len() as u64
        {
            return Err("Invalid: product source extent".into());
        }
    }
    if sa.iter().chain(&sb).any(|s| !(1..=16).contains(s)) {
        return Err("Unsupported: product scale budget".into());
    }
    let width = grid(da[0] * sa[0], db[0] * sb[0])?;
    let height = grid(da[1] * sa[1], db[1] * sb[1])?;
    let count = u64::from(width) * u64::from(height);
    if count > lbp_texture::MAX_PIXELS as u64 {
        return Err("Unsupported: texture product pixel budget".into());
    }
    Ok((width, height, count))
}
/// D83: `(port1 - port0) * w + port0` on the nearest grid, `w` = factor sample alpha.
/// `a` is the port-0 sample, `b` port 1. RGB in linear space, alpha linear,
/// matching the D58 policy (docs/alpha-mix-preview.md, docs/d83-mix-shapes.md).
pub(crate) fn pixels_alpha_mixed(
    a: &lbp_texture::Texture,
    b: &lbp_texture::Texture,
    ta: SamplingTransform,
    tb: SamplingTransform,
    mix: crate::TextureAlphaMix,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    bake_alpha_mix(
        [
            [a.header.width, a.header.height],
            [b.header.width, b.header.height],
        ],
        [&a.rgba, &b.rgba],
        [scales(ta)?, scales(tb)?],
        mix.factor_port,
        cancelled,
    )
}
fn bake_alpha_mix(
    dimensions: [[u32; 2]; 2],
    sources: [&[u8]; 2],
    scales: [[u32; 2]; 2],
    factor_port: u8,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    if factor_port > 1 {
        return Err("Unsupported: alpha MIX factor port".into());
    }
    let (width, height, count) = extent(dimensions, sources, scales)?;
    if cancelled() {
        return Err("Cancelled".into());
    }
    let [da, db] = dimensions;
    let [a, b] = sources;
    let [sa, sb] = scales;
    let linear: [f64; 256] = std::array::from_fn(|i| {
        let c = i as f64 / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let encode = |c: f64| {
        let c = if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1. / 2.4) - 0.055
        };
        (c * 255.).round().clamp(0., 255.) as u8
    };
    let mut output = Vec::with_capacity(count as usize * 4);
    for y in 0..height {
        for x in 0..width {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            let index = |d: [u32; 2], s: [u32; 2]| {
                let px = ((u64::from(x) * u64::from(d[0]) * u64::from(s[0]) / u64::from(width))
                    % u64::from(d[0])) as usize;
                let py = ((u64::from(y) * u64::from(d[1]) * u64::from(s[1]) / u64::from(height))
                    % u64::from(d[1])) as usize;
                (py * d[0] as usize + px) * 4
            };
            let (i, j) = (index(da, sa), index(db, sb));
            let w = f64::from(if factor_port == 0 { a[i + 3] } else { b[j + 3] }) / 255.;
            for c in 0..3 {
                let (l, r) = (linear[a[i + c] as usize], linear[b[j + c] as usize]);
                output.push(encode((r - l) * w + l));
            }
            let (l, r) = (f64::from(a[i + 3]) / 255., f64::from(b[j + 3]) / 255.);
            output.push((((r - l) * w + l) * 255.).round().clamp(0., 255.) as u8);
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((width, height, output))
}
/// D97: two-texture BLEND on the nearest grid; `a` is the primary sample, `b` the
/// second. Per texel: factor from the mask texel (red sRGB-decoded or alpha linear),
/// then `TextureBlendMask::texel` (docs/d97-blend-mask.md).
pub(crate) fn pixels_blend_masked(
    a: &lbp_texture::Texture,
    b: &lbp_texture::Texture,
    ta: SamplingTransform,
    tb: SamplingTransform,
    blend: crate::TextureBlendMask,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    bake_blend_mask(
        [
            [a.header.width, a.header.height],
            [b.header.width, b.header.height],
        ],
        [&a.rgba, &b.rgba],
        [scales(ta)?, scales(tb)?],
        blend,
        cancelled,
    )
}
fn bake_blend_mask(
    dimensions: [[u32; 2]; 2],
    sources: [&[u8]; 2],
    scales: [[u32; 2]; 2],
    blend: crate::TextureBlendMask,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    blend.validate()?;
    let thresholds = crate::blend_thresholds(blend.threshold_bits)?;
    let (width, height, count) = extent(dimensions, sources, scales)?;
    if cancelled() {
        return Err("Cancelled".into());
    }
    let [da, db] = dimensions;
    let [a, b] = sources;
    let [sa, sb] = scales;
    let linear: [f64; 256] = std::array::from_fn(|v| crate::blend_mask::srgb_decode(v as u8));
    let mut output = Vec::with_capacity(count as usize * 4);
    for y in 0..height {
        for x in 0..width {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            let index = |d: [u32; 2], s: [u32; 2]| {
                let px = ((u64::from(x) * u64::from(d[0]) * u64::from(s[0]) / u64::from(width))
                    % u64::from(d[0])) as usize;
                let py = ((u64::from(y) * u64::from(d[1]) * u64::from(s[1]) / u64::from(height))
                    % u64::from(d[1])) as usize;
                (py * d[0] as usize + px) * 4
            };
            let (i, j) = (index(da, sa), index(db, sb));
            blend.texel(
                [&a[i..i + 4], &b[j..j + 4]],
                thresholds,
                &linear,
                &mut output,
            );
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((width, height, output))
}
fn bake_policy(
    dimensions: [[u32; 2]; 2],
    sources: [&[u8]; 2],
    scales: [[u32; 2]; 2],
    blend: Option<crate::DiffuseBlend>,
    cancelled: impl Fn() -> bool,
) -> Result<(u32, u32, Vec<u8>), String> {
    if let Some(blend) = blend {
        blend.validate()?;
    }
    let [da, db] = dimensions;
    let [a, b] = sources;
    let [sa, sb] = scales;
    let (width, height, count) = extent(dimensions, sources, scales)?;
    if cancelled() {
        return Err("Cancelled".into());
    }
    // RGB multiply occurs in linear space; encoded table avoids per-pixel pow calls.
    let linear: [f64; 256] = std::array::from_fn(|i| {
        let c = i as f64 / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let channels = if blend.is_some() { 3 } else { 1 };
    let mut table = vec![0u8; 65536 * channels];
    for x in 0..256 {
        if x % 16 == 0 && cancelled() {
            return Err("Cancelled".into());
        }
        for y in 0..256 {
            for channel in 0..channels {
                let product = linear[x] * linear[y];
                let c = blend.map_or(product, |blend| blend.channel(product, channel));
                let c = if c <= 0.0031308 {
                    12.92 * c
                } else {
                    1.055 * c.powf(1. / 2.4) - 0.055
                };
                table[channel * 65536 + x * 256 + y] = (c * 255.).round().clamp(0., 255.) as u8;
            }
        }
    }
    let mut output = Vec::with_capacity(count as usize * 4);
    for y in 0..height {
        for x in 0..width {
            if output.len().is_multiple_of(16384) && cancelled() {
                return Err("Cancelled".into());
            }
            let index = |d: [u32; 2], s: [u32; 2]| {
                let px = ((u64::from(x) * u64::from(d[0]) * u64::from(s[0]) / u64::from(width))
                    % u64::from(d[0])) as usize;
                let py = ((u64::from(y) * u64::from(d[1]) * u64::from(s[1]) / u64::from(height))
                    % u64::from(d[1])) as usize;
                (py * d[0] as usize + px) * 4
            };
            let (i, j) = (index(da, sa), index(db, sb));
            for c in 0..3 {
                output.push(
                    table[if blend.is_some() { c * 65536 } else { 0 }
                        + a[i + c] as usize * 256
                        + b[j + c] as usize],
                );
            }
            output.push(if let Some(blend) = blend {
                let product = f64::from(a[i + 3]) * f64::from(b[j + 3]) / (255. * 255.);
                (blend.channel(product, 3) * 255.).round().clamp(0., 255.) as u8
            } else {
                ((u32::from(a[i + 3]) * u32::from(b[j + 3]) + 127) / 255) as u8
            });
        }
    }
    if cancelled() {
        return Err("Cancelled".into());
    }
    Ok((width, height, output))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_mix_linear_rgb_alpha_endpoints_direction_and_cancel() {
        let a = [0, 0, 0, 128];
        let b = [0, 0, 0, 128];
        let mut blend = crate::DiffuseBlend {
            mix_index: 0,
            color: crate::DiffuseTint {
                bits: [1., 0., 0.25, 1.].map(f32::to_bits),
            },
            weight_bits: 0.5f32.to_bits(),
            color_first: true,
        };
        let pixels = |blend| {
            bake_policy([[1, 1]; 2], [&a, &b], [[1, 1]; 2], Some(blend), || false)
                .unwrap()
                .2
        };
        assert_eq!(pixels(blend), [188, 0, 99, 160]);
        blend.weight_bits = 0f32.to_bits();
        assert_eq!(pixels(blend), [255, 0, 137, 255]);
        blend.weight_bits = 1f32.to_bits();
        assert_eq!(pixels(blend), [0, 0, 0, 64]);
        blend.color_first = false;
        assert_eq!(pixels(blend), [255, 0, 137, 255]);
        assert_eq!(a, [0, 0, 0, 128]);
        assert_eq!(b, a);
        assert!(bake_policy([[1, 1]; 2], [&a, &b], [[1, 1]; 2], Some(blend), || true).is_err());
        let calls = std::cell::Cell::new(0);
        assert!(bake_policy(
            [[4096, 2], [1, 1]],
            [&vec![255; 4096 * 2 * 4], &b],
            [[1, 1]; 2],
            Some(blend),
            || {
                calls.set(calls.get() + 1);
                calls.get() > 18
            }
        )
        .is_err());
    }
    #[test]
    fn independent_repeat_scales_common_grid_and_linear_product() {
        let a = [255, 128, 0, 128, 0, 255, 255, 255];
        let b = [255, 255, 255, 128, 255, 0, 255, 255, 255, 255, 255, 255];
        let (w, h, p) = bake([2, 1], &a, [2, 1], [3, 1], &b, [1, 1], || false).unwrap();
        assert_eq!((w, h), (12, 1));
        assert_eq!(&p[..4], &[255, 128, 0, 64]);
        assert_eq!(&p[16..20], &[0, 0, 255, 255]);
        assert_eq!(&p[32..36], &[255, 128, 0, 128]);
        assert_eq!(a[1], 128);
        let (w, h, column) = bake([1, 2], &a, [1, 2], [1, 3], &b, [1, 1], || false).unwrap();
        assert_eq!((w, h), (1, 12));
        assert_eq!(column, p);
        let (_, _, p) = bake([1, 1], &[128; 4], [1, 1], [1, 1], &[128; 4], [1, 1], || {
            false
        })
        .unwrap();
        assert_eq!(p, [61, 61, 61, 64]); // Linear RGB differs from encoded-byte multiplication.
    }
    #[test]
    fn extent_lcm_pixel_limits_transform_and_cancellation() {
        assert!(bake([0, 1], &[], [1, 1], [1, 1], &[255; 4], [1, 1], || false).is_err());
        assert!(grid(4096, 4095).is_err());
        assert!(bake(
            [4096, 1],
            &vec![255; 4096 * 4],
            [1, 1],
            [1, 4096],
            &vec![255; 4096 * 4],
            [1, 1],
            || false
        )
        .is_err());
        assert!(
            bake([1, 1], &[255; 4], [1, 1], [1, 1], &[255; 4], [1, 1], || {
                true
            })
            .is_err()
        );
        let calls = std::cell::Cell::new(0);
        assert!(
            bake([1, 1], &[255; 4], [1, 1], [1, 1], &[255; 4], [1, 1], || {
                calls.set(calls.get() + 1);
                calls.get() > 3
            })
            .is_err()
        );
        for scale in [0., -1., 0.5, 17., f32::NAN] {
            assert!(scales(SamplingTransform::ScaleOffset(crate::UvTransform {
                bits: [scale, 1., 0., 0.].map(f32::to_bits)
            }))
            .is_err());
            calls.set(0);
            assert!(bake(
                [4096, 2],
                &vec![255; 4096 * 2 * 4],
                [1, 1],
                [1, 1],
                &[255; 4],
                [1, 1],
                || {
                    calls.set(calls.get() + 1);
                    calls.get() > 18
                }
            )
            .is_err());
        }
    }
    #[test]
    fn two_texture_blend_grid_mask_and_cancel() {
        // Port 0 COLOR green, port 1 texture 1 (1x1 red at scale 2), mask = red of
        // texture 2 (2x1: red 0, red 255), thresholds 0/1.
        let red = [255, 0, 0, 255];
        let mask = [0, 7, 7, 7, 255, 7, 7, 7];
        let blend = crate::TextureBlendMask {
            blend_index: 1,
            ports: [
                crate::BlendPort::Color(crate::DiffuseTint {
                    bits: [0., 1., 0., 1.].map(f32::to_bits),
                }),
                crate::BlendPort::Texture(0),
            ],
            mask_texture: 1,
            mask_red: true,
            threshold_bits: [0, 1f32.to_bits()],
        };
        let run = |blend, cancel: &dyn Fn() -> bool| {
            bake_blend_mask(
                [[1, 1], [2, 1]],
                [&red, &mask],
                [[2, 1], [1, 1]],
                blend,
                cancel,
            )
        };
        let (width, height, pixels) = run(blend, &|| false).unwrap();
        assert_eq!((width, height), (2, 1));
        assert_eq!(pixels, [0, 255, 0, 255, 255, 0, 0, 255]);
        assert_eq!(run(blend, &|| true).unwrap_err(), "Cancelled");
        let mut bad = blend;
        bad.threshold_bits = [0, 0];
        assert!(run(bad, &|| false).is_err());
        assert!(
            bake_blend_mask([[1, 1]; 2], [&red, &red], [[17, 1], [1, 1]], blend, || {
                false
            })
            .is_err()
        );
    }
    #[test]
    fn two_texture_alpha_mix_grid_endpoints_factor_port_and_cancel() {
        // Port 0: opaque red 1x1 at scale 2; port 1: transparent white, half-alpha blue.
        let red = [255, 0, 0, 255];
        let top = [255, 255, 255, 0, 0, 0, 255, 128];
        let run = |port: u8, cancel: &dyn Fn() -> bool| {
            bake_alpha_mix(
                [[1, 1], [2, 1]],
                [&red, &top],
                [[2, 1], [1, 1]],
                port,
                cancel,
            )
        };
        let (width, height, pixels) = run(1, &|| false).unwrap();
        assert_eq!((width, height), (2, 1));
        assert_eq!(pixels, [255, 0, 0, 255, 187, 0, 188, 191]);
        // Factor from port 0 (opaque): result is port 1 everywhere.
        assert_eq!(run(0, &|| false).unwrap().2, top);
        assert!(run(2, &|| false).is_err());
        assert_eq!(run(1, &|| true).unwrap_err(), "Cancelled");
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            run(1, &|| {
                calls.set(calls.get() + 1);
                calls.get() > 1
            })
            .unwrap_err(),
            "Cancelled"
        );
        assert!(
            bake_alpha_mix([[1, 1], [2, 1]], [&red, &top[..4]], [[1, 1]; 2], 1, || {
                false
            })
            .is_err()
        );
        assert!(bake_alpha_mix([[1, 1]; 2], [&red, &red], [[17, 1], [1, 1]], 1, || false).is_err());
    }
}
