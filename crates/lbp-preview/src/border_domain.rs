//! Guarded nearest-only interior sampling of source BORDER axes.
//! No border color is inferred. Evidence/bounds: docs/interior-border-preview.md.
pub const INTERIOR_MARGIN: f32 = 1. / 4096.;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderDomain {
    pub source_wrap_raw: [u8; 2],
    pub min_bits: [u32; 2],
    pub max_bits: [u32; 2],
}
/// Proposed display addressing. Caller must confirm the complete sampling UV domain
/// before publishing any asset carrying source BORDER metadata.
pub(crate) fn addressing(
    raw: [Option<u8>; 2],
    alpha_mix: bool,
) -> Result<crate::TextureWrapping, String> {
    if !raw.contains(&Some(4)) {
        return crate::TextureWrapping::from_raw(raw[0], raw[1]);
    }
    if !alpha_mix {
        return Err(
            "Unsupported: BORDER requires nearest same-sample alpha MIX and validated interior UVs"
                .into(),
        );
    }
    let mapped = raw.map(|v| if v == Some(4) { Some(3) } else { v });
    crate::TextureWrapping::from_raw(mapped[0], mapped[1])
}
impl BorderDomain {
    pub(crate) fn confirm(
        raw: [Option<u8>; 2],
        uv: &[[f32; 2]],
        cancelled: impl Fn() -> bool,
    ) -> Result<Option<Self>, String> {
        if !raw.contains(&Some(4)) {
            return Ok(None);
        }
        addressing(raw, true)?;
        if uv.is_empty() || uv.len() > crate::MAX_VERTICES {
            return Err("Unsupported: BORDER UV correspondence/budget".into());
        }
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        for (i, p) in uv.iter().enumerate() {
            if i % 256 == 0 && cancelled() {
                return Err("Cancelled".into());
            }
            for axis in 0..2 {
                if !p[axis].is_finite() {
                    return Err("Invalid: nonfinite BORDER sampling UV".into());
                }
                if raw[axis] == Some(4)
                    && !(INTERIOR_MARGIN..=1. - INTERIOR_MARGIN).contains(&p[axis])
                {
                    return Err("Unsupported: BORDER sampling UV can reach border; color/edge behavior unresolved".into());
                }
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        if cancelled() {
            return Err("Cancelled".into());
        }
        Ok(Some(Self {
            source_wrap_raw: [raw[0].unwrap(), raw[1].unwrap()],
            min_bits: min.map(f32::to_bits),
            max_bits: max.map(f32::to_bits),
        }))
    }
    pub fn label(self) -> String {
        format!("source wrap {:?}: BORDER stays inside validated UV range {:?}..{:?} • nearest; border color unused",self.source_wrap_raw,self.min_bits.map(f32::from_bits),self.max_bits.map(f32::from_bits))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interior_bounds_axes_correspondence_and_cancellation() {
        let raw = [Some(4), Some(4)];
        let uv = [[0.001, 0.999], [0.7, 0.2]];
        let proof = BorderDomain::confirm(raw, &uv, || false).unwrap().unwrap();
        assert_eq!(proof.source_wrap_raw, [4, 4]);
        assert_eq!(proof.min_bits, [0.001f32.to_bits(), 0.2f32.to_bits()]);
        assert_eq!(addressing(raw, true).unwrap().raw(), [3, 3]);
        assert!(addressing(raw, false).is_err());
        assert_eq!(addressing([Some(4), Some(2)], true).unwrap().raw(), [3, 2]);
        assert!(addressing([Some(4), Some(6)], true).is_err());
        assert!(addressing([Some(4), None], true).is_err());
        for v in [
            0.,
            1.,
            -1.,
            f32::NAN,
            f32::INFINITY,
            INTERIOR_MARGIN / 2.,
            1. - INTERIOR_MARGIN / 2.,
        ] {
            assert!(BorderDomain::confirm(raw, &[[v, 0.5]], || false).is_err());
        }
        assert!(BorderDomain::confirm([Some(4), Some(1)], &[[0.5, -10.]], || false).is_ok());
        assert!(BorderDomain::confirm([Some(1), Some(4)], &[[-10., 0.5]], || false).is_ok());
        assert!(BorderDomain::confirm(raw, &[], || false).is_err());
        assert_eq!(
            BorderDomain::confirm(raw, &uv, || true).unwrap_err(),
            "Cancelled"
        );
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            BorderDomain::confirm(raw, &vec![[0.5; 2]; 300], || {
                let n = calls.get();
                calls.set(n + 1);
                n == 1
            })
            .unwrap_err(),
            "Cancelled"
        );
        assert_eq!(uv, [[0.001, 0.999], [0.7, 0.2]]);
    }
}
