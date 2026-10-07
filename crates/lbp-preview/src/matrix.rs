//! Independent static 2×3 UV matrix policy. Evidence/pin/bounds: docs/matrix-preview.md.
use crate::UvTransform;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UvMatrix {
    /// Row-major [a,b,c,d,e,f] from material parameters [0,1,2,3,6,7].
    pub bits: [u32; 6],
}
impl UvMatrix {
    pub fn values(self) -> [f32; 6] {
        self.bits.map(f32::from_bits)
    }
    pub fn validate(self) -> Result<(), String> {
        if self
            .values()
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        {
            return Err("Unsupported: UV matrix exceeds finite preview bounds".into());
        }
        Ok(())
    }
    pub fn sampling(
        self,
        raw: &[[f32; 2]],
        cancelled: impl Fn() -> bool,
    ) -> Result<Vec<[f32; 2]>, String> {
        self.validate()?;
        if raw.len() > 100_000 {
            return Err("UV matrix vertex budget exceeded".into());
        }
        let [a, b, c, d, e, f] = self.values();
        let identity_x = a == 1. && b == 0. && c == 0.;
        let identity_y = d == 0. && e == 1. && f == 0.;
        let mut result = Vec::with_capacity(raw.len());
        for (i, &[u, v]) in raw.iter().enumerate() {
            if i.is_multiple_of(256) && cancelled() {
                return Err("Cancelled".into());
            }
            if !u.is_finite() || !v.is_finite() {
                return Err("Invalid: nonfinite raw UV".into());
            }
            let point = [
                if identity_x { u } else { (a * u + b * v) + c },
                if identity_y { v } else { (d * u + e * v) + f },
            ];
            if point.iter().any(|x| !x.is_finite())
                || (!identity_x && point[0].abs() > 1_000_000.)
                || (!identity_y && point[1].abs() > 1_000_000.)
            {
                return Err("Unsupported: derived matrix UV exceeds finite preview bounds".into());
            }
            result.push(point);
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(result)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingTransform {
    ScaleOffset(UvTransform),
    Matrix(UvMatrix),
}
impl Default for SamplingTransform {
    fn default() -> Self {
        Self::ScaleOffset(UvTransform::default())
    }
}
impl SamplingTransform {
    pub fn subtype(self) -> u32 {
        match self {
            Self::ScaleOffset(_) => 0,
            Self::Matrix(_) => 1,
        }
    }
    pub fn bits(&self) -> &[u32] {
        match self {
            Self::ScaleOffset(t) => &t.bits,
            Self::Matrix(t) => &t.bits,
        }
    }
    pub fn parameter_indices(self) -> &'static [usize] {
        match self {
            Self::ScaleOffset(_) => &[0, 1, 2, 3],
            Self::Matrix(_) => &[0, 1, 2, 3, 6, 7],
        }
    }
    pub fn validate(self) -> Result<(), String> {
        match self {
            Self::ScaleOffset(t) => t.validate(),
            Self::Matrix(t) => t.validate(),
        }
    }
    pub fn sampling(
        self,
        raw: &[[f32; 2]],
        cancelled: impl Fn() -> bool,
    ) -> Result<Vec<[f32; 2]>, String> {
        match self {
            Self::ScaleOffset(t) => t.sampling(raw, cancelled),
            Self::Matrix(t) => t.sampling(raw, cancelled),
        }
    }
    pub fn label(self) -> String {
        match self {
            Self::ScaleOffset(t) => t.label(),
            Self::Matrix(t) => {
                let [a, b, c, d, e, f] = t.values();
                format!("derived matrix UV ({a}u + {b}v + {c}, {d}u + {e}v + {f})")
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asymmetric_cross_terms_rotation_and_identity_preserve_raw_data() {
        let raw = [[0.25, 0.75], [-0., 1.]];
        let matrix = UvMatrix {
            bits: [2., 3., 0.5, -4., 5., -0.25].map(f32::to_bits),
        };
        assert_eq!(
            matrix.sampling(&raw, || false).unwrap(),
            [[3.25, 2.5], [3.5, 4.75]]
        );
        assert_eq!(raw[1][0].to_bits(), (-0f32).to_bits());
        let identity = UvMatrix {
            bits: [1., 0., -0., 0., 1., 0.].map(f32::to_bits),
        };
        assert_eq!(
            identity.sampling(&raw, || false).unwrap()[1][0].to_bits(),
            (-0f32).to_bits()
        );
        let rotation = UvMatrix {
            bits: [0., 1., 0., -1., 0., 1.].map(f32::to_bits),
        };
        assert_eq!(
            rotation.sampling(&raw[..1], || false).unwrap(),
            [[0.75, 0.75]]
        );
    }
    #[test]
    fn matrix_bounds_overflow_and_cancellation_are_explicit() {
        for bad in [f32::NAN, f32::INFINITY, 1_000_001.] {
            let m = UvMatrix {
                bits: [bad, 0., 0., 0., 1., 0.].map(f32::to_bits),
            };
            assert!(m.sampling(&[[0., 0.]], || false).is_err());
        }
        let m = UvMatrix {
            bits: [2., 1., 0., 0., 1., 0.].map(f32::to_bits),
        };
        assert!(m.sampling(&[[f32::MAX, 0.]], || false).is_err());
        assert!(m.sampling(&[[600_000., 0.]], || false).is_err());
        assert!(m.sampling(&[[0., 0.]], || true).is_err());
        assert!(m.sampling(&vec![[0., 0.]; 100_001], || false).is_err());
        assert!(m.sampling(&[[0., f32::NAN]], || false).is_err());
        let count = std::cell::Cell::new(0);
        assert!(m
            .sampling(&vec![[0., 0.]; 257], || {
                count.set(count.get() + 1);
                count.get() > 1
            })
            .is_err());
    }
}
