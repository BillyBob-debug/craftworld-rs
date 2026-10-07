//! Static COLOR × texture diffuse projection. Provenance/bounds: docs/tinted-preview.md.
//! sRGB transfer specification: https://wikis.khronos.org/opengl/GLAPI/glTexImage2D
//! Linear tint and 8-bit baking are authored preview policy, not game-GPU parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffuseTint {
    pub bits: [u32; 4],
}
impl Default for DiffuseTint {
    fn default() -> Self {
        Self {
            bits: [1f32.to_bits(); 4],
        }
    }
}
impl DiffuseTint {
    pub fn values(self) -> [f32; 4] {
        self.bits.map(f32::from_bits)
    }
    pub fn validate(self) -> Result<(), String> {
        if self
            .values()
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(
                "Unsupported: static diffuse color outside finite 0..1 preview bounds".into(),
            );
        }
        Ok(())
    }
    pub fn label(self) -> String {
        if self == Self::default() {
            return "identity color".into();
        }
        let [r, g, b, a] = self.values();
        format!("static linear tint ({r}, {g}, {b}, {a})")
    }
    /// Preserve original decoded pixels. Identity needs no second pixel allocation.
    /// Constant multiplication commutes with linear filtering before 8-bit quantization.
    pub fn pixels(
        self,
        rgba: &[u8],
        cancelled: impl Fn() -> bool,
    ) -> Result<Option<Vec<u8>>, String> {
        self.validate()?;
        if !rgba.len().is_multiple_of(4) || rgba.len() > 4 * lbp_texture::MAX_PIXELS {
            return Err("Invalid: diffuse tint pixel extent/budget".into());
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        if self == Self::default() {
            return Ok(None);
        }
        let tint = self.values();
        // Tables bound nonlinear work to 768 conversions, even for a 4M-pixel image.
        let tables: [[u8; 256]; 3] = std::array::from_fn(|channel| {
            std::array::from_fn(|value| {
                if tint[channel] == 1. {
                    return value as u8;
                }
                let encoded = value as f64 / 255.;
                let linear = if encoded <= 0.04045 {
                    encoded / 12.92
                } else {
                    ((encoded + 0.055) / 1.055).powf(2.4)
                };
                let product = linear * f64::from(tint[channel]);
                let encoded = if product <= 0.0031308 {
                    product * 12.92
                } else {
                    1.055 * product.powf(1. / 2.4) - 0.055
                };
                (encoded * 255.).round().clamp(0., 255.) as u8
            })
        });
        let mut result = Vec::with_capacity(rgba.len());
        for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
            if index % 4096 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            result.extend_from_slice(&[
                tables[0][pixel[0] as usize],
                tables[1][pixel[1] as usize],
                tables[2][pixel[2] as usize],
                (f64::from(pixel[3]) * f64::from(tint[3])).round() as u8,
            ]);
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
    fn linear_tint_preserves_original_pixels_and_identity_and_linear_alpha() {
        let pixels = [255, 128, 10, 200];
        assert!(DiffuseTint::default()
            .pixels(&pixels, || false)
            .unwrap()
            .is_none());
        let tint = DiffuseTint {
            bits: [0.5, 1., 0., 0.5].map(f32::to_bits),
        };
        assert_eq!(
            tint.pixels(&pixels, || false).unwrap().unwrap(),
            [188, 128, 0, 100]
        );
        assert_eq!(pixels, [255, 128, 10, 200]);
        // Low sRGB values use the linear segment, not a gamma approximation.
        let half = DiffuseTint {
            bits: [0.5; 4].map(f32::to_bits),
        };
        assert_eq!(
            half.pixels(&[10, 4, 2, 255], || false).unwrap().unwrap(),
            [5, 2, 1, 128]
        );
        assert!(half.pixels(&pixels, || true).is_err());
        let checks = std::cell::Cell::new(0);
        assert!(half
            .pixels(&vec![255; 4097 * 4], || {
                checks.set(checks.get() + 1);
                checks.get() > 2
            })
            .is_err());
        assert!(half.pixels(&[0; 3], || false).is_err());
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.01] {
            assert!(DiffuseTint {
                bits: [value.to_bits(); 4]
            }
            .pixels(&pixels, || false)
            .is_err());
        }
    }
}
