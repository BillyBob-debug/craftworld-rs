//! Fixed COLOR/sample MIX weighted by the same sample's alpha.
//! Sources, nearest-only policy and limits: docs/alpha-mix-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaDiffuseMix {
    pub mix_index: usize,
    pub color: crate::DiffuseTint,
    pub color_first: bool,
    /// Preserved metadata; the wired factor overrides this value.
    pub unused_weight_bits: u32,
}
/// Two-texture MIX (port 0 / port 1 samples) whose port-2 factor is the alpha of
/// one of those same samples. Baked on the nearest product grid: docs/d83-mix-shapes.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureAlphaMix {
    pub mix_index: usize,
    /// MIX input port (0 or 1) whose sample also feeds port 2 with swizzle `a`.
    pub factor_port: u8,
    /// Preserved metadata; the wired factor overrides this value.
    pub unused_weight_bits: u32,
}
impl TextureAlphaMix {
    pub fn label(self) -> String {
        format!(
            "two-texture MIX weighted by port {} sample alpha • nearest grid",
            self.factor_port
        )
    }
}
impl AlphaDiffuseMix {
    pub fn label(self) -> String {
        format!(
            "sample-alpha MIX color {:?}, color on port {} • nearest filtering",
            self.color.values(),
            if self.color_first { 0 } else { 1 }
        )
    }
    pub fn pixels(self, rgba: &[u8], cancelled: impl Fn() -> bool) -> Result<Vec<u8>, String> {
        self.color.validate()?;
        if !rgba.len().is_multiple_of(4) || rgba.len() > 4 * lbp_texture::MAX_PIXELS {
            return Err("Invalid: alpha MIX pixel extent/budget".into());
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        let colors = self.color.values().map(f64::from);
        let linear: [f64; 256] = std::array::from_fn(|v| {
            let x = v as f64 / 255.;
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        });
        let mix = |sample: f64, channel: usize, w: f64| {
            let (a, b) = if self.color_first {
                (colors[channel], sample)
            } else {
                (sample, colors[channel])
            };
            (b - a) * w + a
        };
        // Heap LUT: 65536 three-byte entries, plus a 2048-byte transfer table.
        let mut table = Vec::<[u8; 3]>::with_capacity(65536);
        for alpha in 0..256 {
            if alpha % 16 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            for value in linear {
                table.push(std::array::from_fn(|c| {
                    let mixed = mix(value, c, alpha as f64 / 255.);
                    let encoded = if mixed <= 0.0031308 {
                        mixed * 12.92
                    } else {
                        1.055 * mixed.powf(1. / 2.4) - 0.055
                    };
                    (encoded * 255.).round().clamp(0., 255.) as u8
                }));
            }
        }
        let mut result = Vec::with_capacity(rgba.len());
        for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if i % 4096 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            let w = f64::from(p[3]) / 255.;
            result.extend_from_slice(&[
                table[p[3] as usize * 256 + p[0] as usize][0],
                table[p[3] as usize * 256 + p[1] as usize][1],
                table[p[3] as usize * 256 + p[2] as usize][2],
                (mix(w, 3, w) * 255.).round().clamp(0., 255.) as u8,
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
    #[test]
    fn alpha_mix_endpoints_linear_rgb_alpha_direction_and_cancel() {
        let mut mix = AlphaDiffuseMix {
            mix_index: 3,
            color: crate::DiffuseTint {
                bits: [0., 0., 0., 1.].map(f32::to_bits),
            },
            color_first: true,
            unused_weight_bits: f32::NAN.to_bits(),
        };
        let raw = [255, 255, 255, 128, 128, 23, 201, 255, 255, 255, 255, 0];
        let result = mix.pixels(&raw, || false).unwrap();
        assert_eq!(
            result,
            [188, 188, 188, 191, 128, 23, 201, 255, 0, 0, 0, 255]
        );
        assert_eq!(
            raw,
            [255, 255, 255, 128, 128, 23, 201, 255, 255, 255, 255, 0]
        );
        mix.color_first = false;
        assert_ne!(mix.pixels(&raw, || false).unwrap(), result);
        assert!(mix.pixels(&[1, 2, 3], || false).is_err());
        assert_eq!(mix.pixels(&raw, || true).unwrap_err(), "Cancelled");
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            mix.pixels(&raw, || {
                let n = calls.get();
                calls.set(n + 1);
                n == 2
            })
            .unwrap_err(),
            "Cancelled"
        );
        calls.set(0);
        assert_eq!(
            mix.pixels(&vec![128; 4 * 5000], || {
                let n = calls.get();
                calls.set(n + 1);
                n == 18
            })
            .unwrap_err(),
            "Cancelled"
        );
        mix.color.bits[0] = f32::INFINITY.to_bits();
        assert!(mix.pixels(&raw, || false).is_err());
    }
}
