//! Independent diagnostic graph layout, no traversal/evaluation/rendering.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! MaterialBox/MaterialWire/MaterialParameterAnimation::serialize,
//! Serializer::arraylist, RGfxMaterial::serialize, BoxType::TEXTURE_SAMPLE.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat
//! Ordinary PS3 heads920–951/branch0 only. Per-rule evidence/conflicts/tests:
//! docs/material-graph.md. Correct unsigned ZigZag; known Toolkit extreme bug
//! is not adopted. Unknown kinds/ports/signed endpoints retained; tails unparsed.
use super::{decode_prefix, MaterialInspection, Reader};
use crate::binary::{check_cancel, Error};
use serde::Serialize;
use sha2::{Digest, Sha256};
pub const MAX_BOXES: usize = 1024;
pub const MAX_WIRES: usize = 4096;
pub const MAX_COMPONENT_VALUES: usize = 4096;
pub const MAX_NAME_BYTES: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct AnimationMetadata {
    pub base_value_bits: [u32; 4],
    pub component_value_count: usize,
    pub component_bytes_sha256: String,
    pub name_hex: String,
    pub components_raw: u8,
    pub derived_key_count: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct BoxMetadata {
    pub type_raw: u32,
    pub declared_parameter_count: usize,
    pub parameters_raw: [u32; 8],
    pub position_bits: [u32; 4],
    pub subtype_raw: u32,
    pub animations: [AnimationMetadata; 2],
    pub texture_slot_raw: Option<u32>,
    pub texture_slot_in_range: Option<bool>,
}
#[derive(Debug, Clone, Serialize)]
pub struct WireMetadata {
    pub box_from: i32,
    pub box_to: i32,
    pub port_from_raw: u8,
    pub port_to_raw: u8,
    pub swizzle_raw: [u8; 5],
    pub from_in_range: bool,
    pub to_in_range: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct GraphMetadata {
    pub box_count: usize,
    pub wire_count: usize,
    pub boxes: Vec<BoxMetadata>,
    pub wires: Vec<WireMetadata>,
    pub graph_start: usize,
    pub parsed_graph_bytes: usize,
    pub unparsed_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct GraphInspection {
    #[serde(flatten)]
    pub material: MaterialInspection,
    pub graph: GraphMetadata,
}
impl Reader<'_> {
    fn bits4(&mut self) -> Result<[u32; 4], Error> {
        Ok([self.word()?, self.word()?, self.word()?, self.word()?])
    }
    fn signed(&mut self) -> Result<i32, Error> {
        let raw = self.integer()?;
        Ok(if self.compact {
            (raw >> 1) as i32 ^ -((raw & 1) as i32)
        } else {
            raw as i32
        })
    }
    pub(super) fn animation(
        &mut self,
        cancel: &impl Fn() -> bool,
    ) -> Result<AnimationMetadata, Error> {
        check_cancel(cancel)?;
        let base_value_bits = self.bits4()?;
        let component_value_count = self.count(MAX_COMPONENT_VALUES)?;
        let bytes = self.take(component_value_count.checked_mul(4).ok_or(Error::Budget)?)?;
        let component_bytes_sha256 = format!("{:x}", Sha256::digest(bytes));
        let name_length = self.count(MAX_NAME_BYTES)?;
        let name_hex = self
            .take(name_length)?
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        let components_raw = self.byte()?;
        let components = (components_raw & 15).count_ones() as usize;
        let derived_key_count = if component_value_count == 0 {
            0
        } else {
            if components == 0 || !component_value_count.is_multiple_of(components) {
                return Err(Error::Invalid("animation component count/mask"));
            }
            component_value_count / components
        };
        Ok(AnimationMetadata {
            base_value_bits,
            component_value_count,
            component_bytes_sha256,
            name_hex,
            components_raw,
            derived_key_count,
        })
    }
    fn material_box(&mut self, cancel: &impl Fn() -> bool) -> Result<BoxMetadata, Error> {
        let type_raw = self.integer()?;
        let declared_parameter_count = self.integer()? as usize;
        if declared_parameter_count != 8 {
            return Err(Error::Unsupported("material parameter count"));
        }
        let mut parameters_raw = [0; 8];
        for value in &mut parameters_raw {
            *value = self.integer()?;
        }
        let position_bits = self.bits4()?;
        let subtype_raw = self.integer()?;
        let animations = [self.animation(cancel)?, self.animation(cancel)?];
        let texture_slot_raw = (type_raw == 1).then_some(parameters_raw[5]);
        Ok(BoxMetadata {
            type_raw,
            declared_parameter_count,
            parameters_raw,
            position_bits,
            subtype_raw,
            animations,
            texture_slot_raw,
            texture_slot_in_range: texture_slot_raw.map(|v| v < 8),
        })
    }
    fn wire(&mut self, box_count: usize) -> Result<WireMetadata, Error> {
        let box_from = self.signed()?;
        let box_to = self.signed()?;
        let port_from_raw = self.byte()?;
        let port_to_raw = self.byte()?;
        let swizzle_raw = self.take(5)?.try_into().unwrap();
        Ok(WireMetadata {
            box_from,
            box_to,
            port_from_raw,
            port_to_raw,
            swizzle_raw,
            from_in_range: box_from >= 0 && (box_from as usize) < box_count,
            to_in_range: box_to >= 0 && (box_to as usize) < box_count,
        })
    }
}
/// Optional diagnostic graph; existing prefix-only inspect/schema is unchanged.
pub fn inspect_graph(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<GraphInspection, Error> {
    decode_graph(bytes, &cancel).map(|(metadata, _)| metadata)
}
pub(super) fn decode_graph(
    bytes: &[u8],
    cancel: &impl Fn() -> bool,
) -> Result<(GraphInspection, Vec<u8>), Error> {
    let (material, data) = decode_prefix(bytes, &cancel)?;
    let graph_start = material.prefix.parsed_prefix_bytes;
    let mut r = Reader {
        data: &data,
        offset: graph_start,
        compact: material.compression_flags & 1 != 0,
    };
    let box_count = r.count(MAX_BOXES)?;
    let mut boxes = Vec::with_capacity(box_count);
    for _ in 0..box_count {
        check_cancel(&cancel)?;
        boxes.push(r.material_box(&cancel)?);
    }
    let wire_count = r.count(MAX_WIRES)?;
    let mut wires = Vec::with_capacity(wire_count);
    for _ in 0..wire_count {
        check_cancel(&cancel)?;
        wires.push(r.wire(box_count)?);
    }
    check_cancel(&cancel)?;
    let graph = GraphMetadata {
        box_count,
        wire_count,
        boxes,
        wires,
        graph_start,
        parsed_graph_bytes: r.offset - graph_start,
        unparsed_bytes: data.len() - r.offset,
    };
    Ok((GraphInspection { material, graph }, data))
}
