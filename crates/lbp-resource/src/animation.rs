//! Independent CPU ANMb scalar/count prefix, no keyframe/locator decoding.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! RAnimation::serialize modern branch, MemoryInputStream::i16/u8/u16/i32/v4.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RAnimation.java
//! Applicability: branch-zero PS3 heads888..951, flags0/7; floats fixed BE32 bits,
//! serialized byte-array length fixed/ULEB32. Optional vectors >907, fat >945.
//! Ordering/gates corroborated on33local prefixes;23actual-reader field matches.
//! Conflict: source fat comment is unproven; signed object fields do not define
//! raw scalar validity. Old layouts/payload meaning/playback remain unsupported.
//! Source URLs, build applicability, limits and tests: docs/animation-prefix.md.
use crate::binary::{check_cancel, Error};
use serde::Serialize;
pub const MAX_INPUT: usize = 2 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 16 * 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct AnimationPrefix {
    pub revision: u32,
    pub frames_raw_u16: u16,
    pub fps_raw_u16: u16,
    pub loop_start_raw_u16: u16,
    pub morph_count_raw_u8: u8,
    /// Bones, rotation/position/scale/morph tracks, locator keys. No validity inferred.
    pub counts: [u16; 6],
    pub position_offset_bits: Option<[u32; 4]>,
    pub position_scale_bits: Option<[u32; 4]>,
    pub fat_raw_u8: Option<u8>,
    pub animation_data_bytes: usize,
    pub animation_data_start: usize,
    pub parsed_prefix_bytes: usize,
    pub unparsed_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct AnimationInspection {
    pub compression_flags: u8,
    pub expanded_bytes: usize,
    pub prefix: AnimationPrefix,
}
struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(count).ok_or(Error::Budget)?;
        let bytes = self
            .data
            .get(self.offset..end)
            .ok_or(Error::Invalid("animation prefix bounds"))?;
        self.offset = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn short(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn vector_bits(&mut self) -> Result<[u32; 4], Error> {
        let mut bits = [0; 4];
        for value in &mut bits {
            *value = u32::from_be_bytes(self.take(4)?.try_into().unwrap());
        }
        Ok(bits)
    }
}
/// Immutable ordinary ANMb only. Returns prefix diagnostics; opaque bytes never escape.
pub fn inspect(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<AnimationInspection, Error> {
    let (header, data) = crate::binary::payload(bytes, b"ANMb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    if !(888..=951).contains(&header.revision)
        || header.branch_id != Some(0)
        || header.branch_revision != Some(0)
    {
        return Err(Error::Unsupported("animation revision/branch"));
    }
    let flags = header
        .compression_flags
        .ok_or(Error::Invalid("flags absent"))?;
    let mut r = Reader {
        data: &data,
        offset: 0,
    };
    let frames = r.short()?;
    let fps = r.short()?;
    let loop_start = r.short()?;
    let morphs = r.byte()?;
    let mut counts = [0; 6];
    for value in counts.iter_mut().take(5) {
        *value = u16::from(r.byte()?);
    }
    counts[5] = r.short()?;
    let (offset_bits, scale_bits) = if header.revision > 907 {
        (Some(r.vector_bits()?), Some(r.vector_bits()?))
    } else {
        (None, None)
    };
    let fat = if header.revision > 945 {
        Some(r.byte()?)
    } else {
        None
    };
    let length = usize::try_from(crate::binary::integer(
        &data,
        &mut r.offset,
        flags & 1 != 0,
    )?)
    .map_err(|_| Error::Budget)?;
    if length > MAX_EXPANDED {
        return Err(Error::Budget);
    }
    let end = r.offset.checked_add(length).ok_or(Error::Budget)?;
    if end > data.len() {
        return Err(Error::Invalid("animation byte array bounds"));
    }
    check_cancel(&cancel)?;
    Ok(AnimationInspection {
        compression_flags: flags,
        expanded_bytes: data.len(),
        prefix: AnimationPrefix {
            revision: header.revision,
            frames_raw_u16: frames,
            fps_raw_u16: fps,
            loop_start_raw_u16: loop_start,
            morph_count_raw_u8: morphs,
            counts,
            position_offset_bits: offset_bits,
            position_scale_bits: scale_bits,
            fat_raw_u8: fat,
            animation_data_bytes: length,
            animation_data_start: r.offset,
            parsed_prefix_bytes: r.offset,
            unparsed_bytes: data.len() - r.offset,
        },
    })
}
