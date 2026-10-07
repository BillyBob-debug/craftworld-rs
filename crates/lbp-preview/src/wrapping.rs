//! Independent per-axis addressing. Sources/pin/applicability: docs/axis-wrap-preview.md.
//! MIRROR mappings and measured limitations: docs/mirror-preview.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureAddress {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
}
impl TextureAddress {
    pub fn raw(self) -> u8 {
        match self {
            Self::Repeat => 1,
            Self::MirroredRepeat => 2,
            Self::ClampToEdge => 3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Repeat => "Repeat",
            Self::MirroredRepeat => "Mirrored repeat",
            Self::ClampToEdge => "Clamp to edge",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureWrapping {
    pub u: TextureAddress,
    pub v: TextureAddress,
}
impl TextureWrapping {
    pub fn uniform(repeat: bool) -> Self {
        let axis = if repeat {
            TextureAddress::Repeat
        } else {
            TextureAddress::ClampToEdge
        };
        Self { u: axis, v: axis }
    }
    pub fn from_raw(u: Option<u8>, v: Option<u8>) -> Result<Self, String> {
        let axis = |raw| match raw {
            Some(1) => Ok(TextureAddress::Repeat),
            Some(2) => Ok(TextureAddress::MirroredRepeat),
            Some(3) => Ok(TextureAddress::ClampToEdge),
            _ => Err(
                "Unsupported: texture axis requires explicit Repeat, Mirror or ClampToEdge"
                    .to_owned(),
            ),
        };
        Ok(Self {
            u: axis(u)?,
            v: axis(v)?,
        })
    }
    pub fn raw(self) -> [u8; 2] {
        [self.u.raw(), self.v.raw()]
    }
    /// Uniform-only frontends must reject None rather than replace either axis.
    pub fn uniform_axis(self) -> Option<TextureAddress> {
        (self.u == self.v).then_some(self.u)
    }
    /// A boolean cannot represent mirror; retain explicit rejection for old callers.
    pub fn uniform_repeat(self) -> Option<bool> {
        match self.uniform_axis() {
            Some(TextureAddress::Repeat) => Some(true),
            Some(TextureAddress::ClampToEdge) => Some(false),
            _ => None,
        }
    }
    pub fn label(self) -> String {
        format!("U {} / V {}", self.u.label(), self.v.label())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_axes_preserve_raw_modes_and_reject_unknown_or_absent_axes() {
        for u in [1, 2, 3] {
            for v in [1, 2, 3] {
                let wrap = TextureWrapping::from_raw(Some(u), Some(v)).unwrap();
                assert_eq!(wrap.raw(), [u, v]);
                assert_eq!(
                    wrap.uniform_repeat(),
                    if u == v && u != 2 { Some(u == 1) } else { None }
                );
                assert_eq!(wrap.uniform_axis().is_some(), u == v);
            }
        }
        for mode in [0, 4, 5, 6, 7, 8, 255] {
            assert!(TextureWrapping::from_raw(Some(mode), Some(1)).is_err());
            assert!(TextureWrapping::from_raw(Some(3), Some(mode)).is_err());
        }
        assert!(TextureWrapping::from_raw(None, Some(1)).is_err());
        assert!(TextureWrapping::from_raw(Some(3), None).is_err());
    }
}
