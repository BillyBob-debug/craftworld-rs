//! Independent PS3 GMT metadata prefix; shader/graph bytes are never returned.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! RGfxMaterial::serialize/getBlobOffsetCount/getShaderCount, Serializer::resource,
//! MemoryInputStream::i32/f32/enumarray, Revisions::GFXMATERIAL_ALPHA_MODE.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java
//! Branch-zero heads920–951 only. Layout, conflicts and coverage: docs/material-prefix.md.
//! Optional graph/tail diagnostics are separate APIs; shader behavior remains unknown.
use crate::binary::{check_cancel, Error};
use serde::Serialize;
mod graph;
mod tail;
pub use graph::{
    inspect_graph, AnimationMetadata, BoxMetadata, GraphInspection, GraphMetadata, WireMetadata,
    MAX_BOXES, MAX_WIRES,
};
pub use tail::{
    inspect_tail, TailInspection, TailMetadata, MAX_PARAMETER_ANIMATIONS, MAX_TAIL_BYTES,
};
pub const MAX_INPUT: usize = 1024 * 1024;
pub const MAX_EXPANDED: usize = 8 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct Scalars {
    pub flags: u32,
    pub alpha_test_bits: u32,
    pub alpha_layer_raw: u8,
    pub alpha_mode_raw: u8,
    pub shadow_cast_raw: u8,
    pub bump_level_bits: u32,
    pub cosine_power_bits: u32,
    pub reflection_blur_bits: u32,
    pub refractive_index_bits: u32,
}
#[derive(Debug, Serialize)]
pub struct TextureDescriptor {
    pub flags: u32,
    pub kind: u8,
    pub guid: Option<u32>,
    pub sha1: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct MaterialPrefix {
    pub revision: u32,
    pub scalars: Scalars,
    pub shader_offset_count: usize,
    pub shader_count: usize,
    pub shader_end_offsets: Vec<u32>,
    pub blob_bytes: usize,
    pub blob_start: usize,
    pub uninterpreted_after_shader_bytes: usize,
    pub textures: Vec<TextureDescriptor>,
    pub wrap_s_raw: Vec<u8>,
    pub wrap_t_raw: Vec<u8>,
    pub parsed_prefix_bytes: usize,
    pub unparsed_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct MaterialInspection {
    pub compression_flags: u8,
    pub expanded_bytes: usize,
    pub prefix: MaterialPrefix,
}
struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
    compact: bool,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(count).ok_or(Error::Budget)?;
        let bytes = self
            .data
            .get(self.offset..end)
            .ok_or(Error::Invalid("material prefix bounds"))?;
        self.offset = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn integer(&mut self) -> Result<u32, Error> {
        crate::binary::integer(self.data, &mut self.offset, self.compact)
    }
    fn count(&mut self, maximum: usize) -> Result<usize, Error> {
        let count = usize::try_from(self.integer()?).map_err(|_| Error::Budget)?;
        if count > maximum {
            Err(Error::Budget)
        } else {
            Ok(count)
        }
    }
    fn descriptor(&mut self) -> Result<TextureDescriptor, Error> {
        let flags = self.integer()?;
        let kind = self.byte()?;
        if kind & !3 != 0 {
            return Err(Error::Unsupported("texture descriptor kind"));
        }
        let guid = if kind & 2 != 0 {
            Some(self.integer()?)
        } else {
            None
        };
        let sha1 = if kind & 1 != 0 {
            Some(self.take(20)?.iter().map(|v| format!("{v:02x}")).collect())
        } else {
            None
        };
        Ok(TextureDescriptor {
            flags,
            kind,
            guid,
            sha1,
        })
    }
    fn wraps(&mut self) -> Result<Vec<u8>, Error> {
        let count = self.count(8)?;
        Ok(self.take(count)?.to_vec())
    }
}
/// Read immutable clear GMTb, returning diagnostic metadata only.
pub fn inspect(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<MaterialInspection, Error> {
    decode_prefix(bytes, &cancel).map(|(metadata, _)| metadata)
}
fn decode_prefix(
    bytes: &[u8],
    cancel: &impl Fn() -> bool,
) -> Result<(MaterialInspection, Vec<u8>), Error> {
    let (header, data) = crate::binary::payload(bytes, b"GMTb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    if !(920..=951).contains(&header.revision)
        || header.branch_id != Some(0)
        || header.branch_revision != Some(0)
    {
        return Err(Error::Unsupported("material revision/branch"));
    }
    let compression_flags = header
        .compression_flags
        .ok_or(Error::Invalid("flags absent"))?;
    let mut r = Reader {
        data: &data,
        offset: 0,
        compact: compression_flags & 1 != 0,
    };
    let scalars = Scalars {
        flags: r.integer()?,
        alpha_test_bits: r.word()?,
        alpha_layer_raw: r.byte()?,
        alpha_mode_raw: r.byte()?,
        shadow_cast_raw: r.byte()?,
        bump_level_bits: r.word()?,
        cosine_power_bits: r.word()?,
        reflection_blur_bits: r.word()?,
        refractive_index_bits: r.word()?,
    };
    // For branch-zero PS3 heads in this contract, the reader's !Vita condition
    // selects ten offsets regardless of material flags. These are ends, not starts.
    let shader_end_offsets = (0..10)
        .map(|_| r.integer())
        .collect::<Result<Vec<_>, _>>()?;
    let blob_bytes = r.count(MAX_EXPANDED)?;
    let blob_start = r.offset;
    let mut previous = 0;
    for &end in &shader_end_offsets {
        if end < previous || u64::from(end) > blob_bytes as u64 {
            return Err(Error::Invalid("shader interval outside blob"));
        }
        previous = end;
    }
    r.take(blob_bytes)?; // Bounds-check and skip; opaque shader bytes never escape.
    check_cancel(&cancel)?;
    let textures = (0..8)
        .map(|_| r.descriptor())
        .collect::<Result<Vec<_>, _>>()?;
    let wrap_s_raw = r.wraps()?;
    let wrap_t_raw = r.wraps()?;
    check_cancel(&cancel)?;
    let metadata = MaterialInspection {
        compression_flags,
        expanded_bytes: data.len(),
        prefix: MaterialPrefix {
            revision: header.revision,
            scalars,
            shader_offset_count: 10,
            shader_count: 10,
            shader_end_offsets,
            blob_bytes,
            blob_start,
            uninterpreted_after_shader_bytes: blob_bytes - previous as usize,
            textures,
            wrap_s_raw,
            wrap_t_raw,
            parsed_prefix_bytes: r.offset,
            unparsed_bytes: data.len() - r.offset,
        },
    };
    Ok((metadata, data))
}
