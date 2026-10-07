//! Independently expressed static sampler rule and authored diagnostic bounds.
//! Source/pin/evidence/applicability/conflicts: docs/affine-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UvTransform {
    /// Scale X/Y then offset X/Y, preserving original material IEEE-754 bits.
    pub bits: [u32; 4],
}
impl Default for UvTransform {
    fn default() -> Self {
        Self {
            bits: [1f32.to_bits(), 1f32.to_bits(), 0, 0],
        }
    }
}
impl UvTransform {
    pub fn values(self) -> [f32; 4] {
        self.bits.map(f32::from_bits)
    }
    pub fn is_identity(self) -> bool {
        let [sx, sy, ox, oy] = self.values();
        sx == 1. && sy == 1. && ox == 0. && oy == 0.
    }
    pub fn validate(self) -> Result<(), String> {
        if self
            .values()
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        {
            return Err("Unsupported: diffuse UV transform exceeds finite preview bounds".into());
        }
        Ok(())
    }
    pub fn label(self) -> String {
        if self.is_identity() {
            return "raw coordinates (identity)".into();
        }
        let [sx, sy, ox, oy] = self.values();
        format!("derived UV × ({sx}, {sy}) + ({ox}, {oy})")
    }
    pub fn sampling(
        self,
        raw: &[[f32; 2]],
        cancelled: impl Fn() -> bool,
    ) -> Result<Vec<[f32; 2]>, String> {
        self.validate()?;
        if raw.len() > 100_000 {
            return Err("UV sampling vertex budget exceeded".into());
        }
        let [sx, sy, ox, oy] = self.values();
        let mut derived = Vec::with_capacity(raw.len());
        for (i, uv) in raw.iter().enumerate() {
            if i % 256 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            if uv.iter().any(|v| !v.is_finite()) {
                return Err("Invalid: nonfinite raw UV".into());
            }
            // Preserve identity-axis bits (including signed zero). Separate multiply
            // and add, with no fused-operation/rounding claim about the game GPU.
            let point = [
                if sx == 1. && ox == 0. {
                    uv[0]
                } else {
                    uv[0] * sx + ox
                },
                if sy == 1. && oy == 0. {
                    uv[1]
                } else {
                    uv[1] * sy + oy
                },
            ];
            if point.iter().any(|v| !v.is_finite())
                || (!self.is_identity() && point.iter().any(|v| v.abs() > 1_000_000.))
            {
                return Err("Unsupported: derived UV exceeds finite preview bounds".into());
            }
            derived.push(point);
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(derived)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_order_negative_zero_and_raw_preservation() {
        let raw = [[0.25, 0.75], [-0., 1.]];
        let before = raw.map(|p| p.map(f32::to_bits));
        let t = UvTransform {
            bits: [2., -3., 0.5, 0.25].map(f32::to_bits),
        };
        assert_eq!(
            t.sampling(&raw, || false).unwrap(),
            vec![[1., -2.], [0.5, -2.75]]
        );
        assert_eq!(raw.map(|p| p.map(f32::to_bits)), before);
        assert_eq!(
            UvTransform::default().sampling(&raw, || false).unwrap()[1][0].to_bits(),
            (-0f32).to_bits()
        );
        let zero = UvTransform {
            bits: [0., 0., 0.25, -0.5].map(f32::to_bits),
        };
        assert_eq!(
            zero.sampling(&raw, || false).unwrap(),
            vec![[0.25, -0.5]; 2]
        );
    }
    #[test]
    fn finite_bounds_overflow_cancellation_and_length_guards() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1_000_001.] {
            let mut t = UvTransform::default();
            t.bits[0] = value.to_bits();
            assert!(t.sampling(&[[0., 0.]], || false).is_err());
        }
        let t = UvTransform {
            bits: [2., 1., 0., 0.].map(f32::to_bits),
        };
        assert!(t.sampling(&[[f32::MAX, 0.]], || false).is_err());
        assert!(t.sampling(&[[600_000., 0.]], || false).is_err());
        assert!(t.sampling(&[[0., 0.]], || true).is_err());
        assert!(t.sampling(&vec![[0., 0.]; 100_001], || false).is_err());
        assert!(t.sampling(&[[0., f32::NAN]], || false).is_err());
    }
}
