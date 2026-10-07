//! Independent optional diagnostic tail, no sound/animation evaluation or key export.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! RGfxMaterial::serialize, Revisions::GFXMATERIAL_SOUND_ENUM/PARAMETER_ANIMATIONS,
//! Serializer::array, MaterialParameterAnimation::serialize (shared graph helper).
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java
//! Ordinary branch-zero PS3 heads920–951; thresholds347/674 precede this scope.
//! Per-rule local evidence, limits, conflicts and tests: docs/material-tail.md.
//! Raw unknown sound/name/mask bits and residual counts retained; null/empty ambiguous.
use super::{graph::decode_graph, AnimationMetadata, GraphInspection, Reader};
use crate::binary::{check_cancel, Error};
use serde::Serialize;

pub const MAX_PARAMETER_ANIMATIONS: usize = 128;
pub const MAX_TAIL_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct TailMetadata {
    pub sound_enum_raw: u32,
    pub parameter_animation_count: usize,
    pub parameter_animations: Vec<AnimationMetadata>,
    pub tail_start: usize,
    pub parsed_tail_bytes: usize,
    pub unparsed_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct TailInspection {
    #[serde(flatten)]
    pub inspection: GraphInspection,
    pub tail: TailMetadata,
}

/// Optional complete metadata walk; prefix/graph-only APIs and schemas unchanged.
/// One bounded expansion; original bytes and animation key arrays never returned.
pub fn inspect_tail(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<TailInspection, Error> {
    let (inspection, data) = decode_graph(bytes, &cancel)?;
    check_cancel(&cancel)?;
    let tail_start = inspection
        .graph
        .graph_start
        .checked_add(inspection.graph.parsed_graph_bytes)
        .ok_or(Error::Budget)?;
    let remaining = data
        .len()
        .checked_sub(tail_start)
        .ok_or(Error::Invalid("material tail bounds"))?;
    if remaining > MAX_TAIL_BYTES {
        return Err(Error::Budget);
    }
    let mut r = Reader {
        data: &data,
        offset: tail_start,
        compact: inspection.material.compression_flags & 1 != 0,
    };
    let sound_enum_raw = r.integer()?;
    let parameter_animation_count = r.count(MAX_PARAMETER_ANIMATIONS)?;
    let mut parameter_animations = Vec::with_capacity(parameter_animation_count);
    for _ in 0..parameter_animation_count {
        check_cancel(&cancel)?;
        parameter_animations.push(r.animation(&cancel)?);
    }
    check_cancel(&cancel)?;
    let tail = TailMetadata {
        sound_enum_raw,
        parameter_animation_count,
        parameter_animations,
        tail_start,
        parsed_tail_bytes: r.offset - tail_start,
        unparsed_bytes: data.len() - r.offset,
    };
    Ok(TailInspection { inspection, tail })
}
