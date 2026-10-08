//! D104 (docs/d104-uv-channel-2.md): texture-sampler UV channel word (parameter 4) to
//! the mesh UV attribute the preview samples.
//!
//! - 0 -> UV0 and 1 -> UV1 (D40).
//! - 2 -> UV0. `CONFIRMED_FROM_LOCAL_DATA`: in all nine 01.33 materials with word 2,
//!   the material's own compiled fragment programs fetch every UV-addressed word-2
//!   sampler from the same interpolant components as word-0 samplers (TEXCOORD0.xy),
//!   while word-1 samplers read TEXCOORD0.zw; none reads TEXCOORD3 (decal UV) for a
//!   fetch. Agrees with `CONFIRMED_FROM_COMMUNITY_CODE` Toolkit pin d6a2ef80
//!   `GfxAssembler.resolve` (TEXTURE_SAMPLE: `zw` only for 1, else `xy`). The third
//!   mesh attribute is not used.
//! - Anything else is unobserved and stays unsupported (`None`).

/// Mesh UV attribute index sampled for a sampler channel word, if supported.
pub fn sampler_uv_channel(raw: u32) -> Option<usize> {
    match raw {
        0 | 2 => Some(0),
        1 => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::sampler_uv_channel;
    #[test]
    fn word_two_samples_uv0_and_unobserved_words_stay_unsupported() {
        assert_eq!(sampler_uv_channel(0), Some(0));
        assert_eq!(sampler_uv_channel(1), Some(1));
        assert_eq!(sampler_uv_channel(2), Some(0));
        for raw in [3, 4, 0x101, u32::MAX] {
            assert_eq!(sampler_uv_channel(raw), None);
        }
    }
}
