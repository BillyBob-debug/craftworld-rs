//! Static COLOR/MULTIPLY MIX projection. Evidence: docs/static-mix-preview.md.
//! Single-sample constant MIX baking: docs/single-mix-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffuseBlend {
    pub mix_index: usize,
    pub color: crate::DiffuseTint,
    pub weight_bits: u32,
    pub color_first: bool,
}
impl DiffuseBlend {
    pub fn validate(self) -> Result<(), String> {
        self.color.validate()?;
        let weight = f32::from_bits(self.weight_bits);
        if !weight.is_finite() || !(0. ..=1.).contains(&weight) {
            return Err("Unsupported: static MIX weight outside finite 0..1".into());
        }
        Ok(())
    }
    pub(crate) fn channel(self, product: f64, channel: usize) -> f64 {
        let color = f64::from(self.color.values()[channel]);
        let weight = f64::from(f32::from_bits(self.weight_bits));
        let (left, right) = if self.color_first {
            (color, product)
        } else {
            (product, color)
        };
        (right - left) * weight + left
    }
    pub fn label(self) -> String {
        format!(
            "static MIX color {:?}, weight {}, color on port {}",
            self.color.values(),
            f32::from_bits(self.weight_bits),
            if self.color_first { 0 } else { 1 }
        )
    }
    /// A constant linear mix commutes with linear filtering before quantization.
    /// Original source pixels remain intact; output is bounded like tint baking.
    pub fn pixels(
        self,
        rgba: &[u8],
        cancelled: impl Fn() -> bool,
    ) -> Result<Option<Vec<u8>>, String> {
        self.validate()?;
        if !rgba.len().is_multiple_of(4) || rgba.len() > 4 * lbp_texture::MAX_PIXELS {
            return Err("Invalid: single MIX pixel extent/budget".into());
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        let weight = f32::from_bits(self.weight_bits);
        if (self.color_first && weight == 1.) || (!self.color_first && weight == 0.) {
            return Ok(None);
        }
        let tables: [[u8; 256]; 4] = std::array::from_fn(|channel| {
            std::array::from_fn(|value| {
                let encoded = value as f64 / 255.;
                let linear = if channel == 3 || encoded <= 0.04045 {
                    if channel == 3 {
                        encoded
                    } else {
                        encoded / 12.92
                    }
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                };
                let mixed = self.channel(linear, channel);
                let encoded = if channel == 3 {
                    mixed
                } else if mixed <= 0.0031308 {
                    mixed * 12.92
                } else {
                    1.055 * mixed.powf(1. / 2.4) - 0.055
                };
                (encoded * 255.).round().clamp(0., 255.) as u8
            })
        });
        let mut result = Vec::with_capacity(rgba.len());
        for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if index % 4096 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            result.extend((0..4).map(|c| tables[c][pixel[c] as usize]));
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(Some(result))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_mix_linear_rgb_alpha_endpoints_and_cancellation() {
        let original = [255, 128, 0, 64];
        let mut blend = DiffuseBlend {
            mix_index: 3,
            color: crate::DiffuseTint {
                bits: [0., 0., 1., 1.].map(f32::to_bits),
            },
            weight_bits: 0.5f32.to_bits(),
            color_first: true,
        };
        assert_eq!(
            blend.pixels(&original, || false).unwrap().unwrap(),
            [188, 92, 188, 160]
        );
        blend.weight_bits = 1f32.to_bits();
        assert!(blend.pixels(&original, || false).unwrap().is_none());
        blend.weight_bits = 0f32.to_bits();
        assert_eq!(
            blend.pixels(&original, || false).unwrap().unwrap(),
            [0, 0, 255, 255]
        );
        blend.color_first = false;
        assert!(blend.pixels(&original, || false).unwrap().is_none());
        blend.weight_bits = 0.25f32.to_bits();
        let reversed = blend.pixels(&original, || false).unwrap().unwrap();
        blend.color_first = true;
        let forward = blend.pixels(&original, || false).unwrap().unwrap();
        assert_ne!(forward, reversed);
        assert_eq!(original, [255, 128, 0, 64]);
        assert!(blend.pixels(&[1, 2, 3], || false).is_err());
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            blend
                .pixels(&vec![128; 4 * 5000], || {
                    let n = calls.get();
                    calls.set(n + 1);
                    n == 2
                })
                .unwrap_err(),
            "Cancelled"
        );
        assert_eq!(blend.pixels(&original, || true).unwrap_err(), "Cancelled");
    }
    #[test]
    fn mix_preserves_direction_endpoints_raw_bits_and_rejects_invalid_weights() {
        let mut blend = DiffuseBlend {
            mix_index: 0,
            color: crate::DiffuseTint {
                bits: [0.25, 0.5, 1., 0.75].map(f32::to_bits),
            },
            weight_bits: 0.25f32.to_bits(),
            color_first: true,
        };
        assert_eq!(blend.channel(1., 0), 0.4375);
        blend.color_first = false;
        assert_eq!(blend.channel(1., 0), 0.8125);
        for (weight, expected) in [(0f32, 1.), (1f32, 0.25)] {
            blend.weight_bits = weight.to_bits();
            assert_eq!(blend.channel(1., 0), expected);
        }
        for invalid in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            blend.weight_bits = invalid.to_bits();
            assert!(blend.validate().is_err());
        }
        blend.weight_bits = (-0f32).to_bits();
        assert!(blend.validate().is_ok());
        assert_eq!(blend.weight_bits, 0x80000000);
    }
}
