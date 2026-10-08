//! D84 BLEND (box 16) phase 1: static COLOR and one texture sample, stepped by that
//! sample's alpha. D94 generalises the thresholds: factor = clamp((m − p0)/(p1 − p0), 0, 1)
//! for parameter words p0 < p1 (the community literals 100/15 are p0/p1 = 0.15/0.16).
//! Rule, evidence labels, pin and limits: docs/d94-blend.md, docs/d84-blend-route.md.
/// The threshold pair observed in community code (0.15 / 0.16 as f32 bits).
pub const BLEND_THRESHOLD_BITS: [u32; 2] = [0x3e19_999a, 0x3e23_d70a];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendDiffuseMask {
    pub blend_index: usize,
    pub color: crate::DiffuseTint,
    /// true: COLOR on port 0 (factor 0), sample on port 1 (factor 1).
    pub color_first: bool,
    /// Raw parameter words 0 and 1 (lower/upper threshold); see `blend_thresholds`.
    pub threshold_bits: [u32; 2],
}
/// Parameter words 0/1 as thresholds: both finite, p0 < p1, finite scale 1/(p1 − p0).
pub fn blend_thresholds(bits: [u32; 2]) -> Result<[f64; 2], String> {
    let [p0, p1] = bits.map(f32::from_bits);
    // The compiled form stores 1/(p1 − p0) as an f32 constant, so it must be finite there.
    if !(p0.is_finite() && p1.is_finite() && p0 < p1 && (1. / (p1 - p0)).is_finite()) {
        return Err("Unsupported: BLEND thresholds must be finite with param0 < param1".into());
    }
    Ok([p0, p1].map(f64::from))
}
/// Mask value m to blend factor for thresholds p0 < p1; port 0 at 0, port 1 at 1.
/// Equal to the compiled scale/bias form clamp(m/(p1 − p0) − p0/(p1 − p0), 0, 1).
pub fn blend_factor(mask: f64, [p0, p1]: [f64; 2]) -> f64 {
    ((mask - p0) / (p1 - p0)).clamp(0., 1.)
}
impl BlendDiffuseMask {
    pub fn validate(self) -> Result<(), String> {
        blend_thresholds(self.threshold_bits)?;
        self.color.validate()
    }
    pub fn label(self) -> String {
        let [p0, p1] = self.threshold_bits.map(f32::from_bits);
        format!(
            "BLEND mask alpha of sample, thresholds {p0}/{p1}, color {:?} on port {} • nearest filtering",
            self.color.values(),
            if self.color_first { 0 } else { 1 }
        )
    }
    /// Per-texel bake: the factor is a steep step of the sample's own alpha and does
    /// not commute with filtering, so the result is displayed with nearest sampling.
    /// RGB lerp in linear space (sRGB-decoded sample), alpha linear. Source untouched.
    pub fn pixels(self, rgba: &[u8], cancelled: impl Fn() -> bool) -> Result<Vec<u8>, String> {
        self.validate()?;
        let thresholds = blend_thresholds(self.threshold_bits)?;
        if !rgba.len().is_multiple_of(4) || rgba.len() > 4 * lbp_texture::MAX_PIXELS {
            return Err("Invalid: BLEND pixel extent/budget".into());
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        let colors = self.color.values().map(f64::from);
        let lerp = |sample: f64, channel: usize, f: f64| {
            let (a, b) = if self.color_first {
                (colors[channel], sample)
            } else {
                (sample, colors[channel])
            };
            (b - a) * f + a
        };
        let encode = |linear: f64| {
            let c = if linear <= 0.0031308 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1. / 2.4) - 0.055
            };
            (c * 255.).round().clamp(0., 255.) as u8
        };
        let decoded: [f64; 256] = std::array::from_fn(|v| {
            let x = v as f64 / 255.;
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        });
        // One factor per alpha byte; the table is per alpha byte
        // (256 x 256 x 3 bytes on the heap), like the alpha MIX bake.
        let mut table = Vec::<[u8; 3]>::with_capacity(65536);
        for alpha in 0..256 {
            if alpha % 16 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            let f = blend_factor(alpha as f64 / 255., thresholds);
            for value in decoded {
                table.push(std::array::from_fn(|c| encode(lerp(value, c, f))));
            }
        }
        let mut result = Vec::with_capacity(rgba.len());
        for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if i % 4096 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            let a = usize::from(p[3]);
            let m = f64::from(p[3]) / 255.;
            result.extend_from_slice(&[
                table[a * 256 + p[0] as usize][0],
                table[a * 256 + p[1] as usize][1],
                table[a * 256 + p[2] as usize][2],
                (lerp(m, 3, blend_factor(m, thresholds)) * 255.)
                    .round()
                    .clamp(0., 255.) as u8,
            ]);
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn mask(color: [f32; 4], color_first: bool) -> BlendDiffuseMask {
        BlendDiffuseMask {
            blend_index: 1,
            color: crate::DiffuseTint {
                bits: color.map(f32::to_bits),
            },
            color_first,
            threshold_bits: BLEND_THRESHOLD_BITS,
        }
    }
    const COMMUNITY: [f64; 2] = [0.15000000596046448, 0.1599999964237213];
    fn blend_factor_community(m: f64) -> f64 {
        blend_factor(m, COMMUNITY)
    }
    #[test]
    fn factor_steps_between_thresholds_and_clamps() {
        assert_eq!(BLEND_THRESHOLD_BITS.map(f32::from_bits), [0.15, 0.16]);
        assert_eq!(blend_thresholds(BLEND_THRESHOLD_BITS).unwrap(), COMMUNITY);
        // The general form reproduces the community literals clamp(100·m − 15, 0, 1).
        for i in 0..=1000 {
            let m = f64::from(i) / 1000.;
            let literal = (100. * m - 15.).clamp(0., 1.);
            assert!((blend_factor_community(m) - literal).abs() < 1e-4, "{m}");
        }
        // D94 corpus pairs: 0/1 is the clamped mask itself; 0.67/0.8 and 0.3/0.7 use the
        // scale/bias observed in the materials' compiled constants (7.6923/-5.1538, 2.5/-0.75).
        let unit = blend_thresholds([0, 1f32.to_bits()]).unwrap();
        for m in [0., 0.25, 0.5, 1.] {
            assert_eq!(blend_factor(m, unit), m);
        }
        let high = blend_thresholds([0x3f2b_851e, 0x3f4c_cccc]).unwrap();
        let mid = blend_thresholds([0x3e99_999a, 0x3f33_3333]).unwrap();
        for m in [0.7, 0.75, 0.78] {
            let (scale, bias) = (1. / (high[1] - high[0]), -high[0] / (high[1] - high[0]));
            assert!((scale - 7.6923).abs() < 1e-3 && (bias + 5.1538).abs() < 1e-3);
            assert!((blend_factor(m, high) - (m * scale + bias)).abs() < 1e-9);
            assert!((blend_factor(m, mid) - (2.5 * m - 0.75).clamp(0., 1.)).abs() < 1e-6);
        }
        assert_eq!(blend_factor(0.6, high), 0.);
        assert_eq!(blend_factor(0.81, high), 1.);
        // Invalid pairs: equal, reversed, non-finite, scale overflow.
        for bits in [
            [0x3f00_0000, 0x3f00_0000],
            [0x3f80_0000, 0],
            [f32::NAN.to_bits(), 0x3f80_0000],
            [0, f32::INFINITY.to_bits()],
            [0, 1],
        ] {
            assert_eq!(
                blend_thresholds(bits).unwrap_err(),
                "Unsupported: BLEND thresholds must be finite with param0 < param1",
                "{bits:x?}"
            );
        }
    }
    #[test]
    fn community_pair_steps_between_thresholds_and_clamps() {
        let blend_factor = blend_factor_community;
        for (m, f) in [
            (0., 0.),
            (0.15, 0.),
            (0.16, 1.),
            (1., 1.),
            (-1., 0.),
            (2., 1.),
        ] {
            assert!((blend_factor(m) - f).abs() < 1e-9, "{m}");
        }
        assert!((blend_factor(0.155) - 0.5).abs() < 1e-6);
        // E-b: 8-bit alpha 38/39/40/41 -> 0 / 0.294 / 0.686 / 1.
        let f = [38., 39., 40., 41.].map(|b: f64| blend_factor(b / 255.));
        assert_eq!(f[0], 0.);
        assert!((f[1] - 0.294).abs() < 1e-3 && (f[2] - 0.686).abs() < 1e-3);
        assert_eq!(f[3], 1.);
    }
    #[test]
    fn direction_linear_rgb_alpha_budget_and_cancellation() {
        // Alpha 0 -> factor 0 -> port 0; alpha 255 -> factor 1 -> port 1.
        let raw = [10, 128, 250, 0, 10, 128, 250, 255];
        let color_first = mask([0., 1., 0., 0.5], true);
        assert_eq!(
            color_first.pixels(&raw, || false).unwrap(),
            [0, 255, 0, 128, 10, 128, 250, 255]
        );
        let sample_first = mask([0., 1., 0., 0.5], false);
        assert_eq!(
            sample_first.pixels(&raw, || false).unwrap(),
            [10, 128, 250, 0, 0, 255, 0, 128]
        );
        assert_eq!(raw, [10, 128, 250, 0, 10, 128, 250, 255]);
        // Intermediate steps (alpha bytes 39/40) lerp black and white in linear space and
        // re-encode (E-d: a linear 0.5 would encode to 188, not the byte midpoint 128).
        let stepped = mask([0., 0., 0., 1.], true)
            .pixels(&[255, 255, 255, 39, 255, 255, 255, 40], || false)
            .unwrap();
        let expect = |f: f64| (1.055 * f.powf(1. / 2.4) - 0.055) * 255.;
        assert_eq!(
            f64::from(stepped[0]),
            expect(blend_factor_community(39. / 255.)).round()
        );
        assert_eq!(
            f64::from(stepped[4]),
            expect(blend_factor_community(40. / 255.)).round()
        );
        // D94: thresholds 0/1 make the factor the alpha itself (a plain alpha lerp).
        let mut unit = mask([0., 0., 0., 1.], true);
        unit.threshold_bits = [0, 1f32.to_bits()];
        let lerped = unit
            .pixels(
                &[255, 255, 255, 0, 255, 255, 255, 128, 255, 255, 255, 255],
                || false,
            )
            .unwrap();
        assert_eq!(
            [lerped[0], lerped[4], lerped[8]],
            [0, expect(128. / 255.).round() as u8, 255]
        );
        assert_eq!([lerped[3], lerped[7], lerped[11]], [255, 191, 255]);
        // Determinism.
        assert_eq!(
            mask([0.2, 0.4, 0.6, 1.], true).pixels(&raw, || false),
            mask([0.2, 0.4, 0.6, 1.], true).pixels(&raw, || false)
        );
        assert!(color_first.pixels(&[1, 2, 3], || false).is_err());
        assert!(color_first
            .pixels(&vec![0; 4 * lbp_texture::MAX_PIXELS + 4], || false)
            .is_err());
        assert_eq!(color_first.pixels(&raw, || true).unwrap_err(), "Cancelled");
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            color_first
                .pixels(&vec![128; 4 * 5000], || {
                    let n = calls.get();
                    calls.set(n + 1);
                    n == 18
                })
                .unwrap_err(),
            "Cancelled"
        );
    }
    #[test]
    fn thresholds_and_color_are_validated() {
        let mut m = mask([0., 0., 0., 1.], true);
        assert!(m.validate().is_ok());
        for bits in [
            [0x3e19_999a, 0x3e2e_147b],
            [0, 0x3f80_0000],
            [0x3f2b_851e, 0x3f4c_cccc],
        ] {
            m.threshold_bits = bits;
            assert!(m.validate().is_ok(), "{bits:x?}");
        }
        // The corpus words are one ulp below 0.67 and 0.8; the label shows them exactly.
        assert!(m.label().contains("thresholds 0.66999996/0.79999995"));
        for bits in [[0x3f80_0000, 0], [0x3e19_999a, 0x3e19_999a]] {
            m.threshold_bits = bits;
            assert_eq!(
                m.validate().unwrap_err(),
                "Unsupported: BLEND thresholds must be finite with param0 < param1"
            );
            assert!(m.pixels(&[0; 4], || false).is_err());
        }
        m.threshold_bits = BLEND_THRESHOLD_BITS;
        for bad in [1.5, f32::NAN] {
            m.color.bits[0] = bad.to_bits();
            assert!(m.validate().is_err());
        }
    }
}
