//! Independent modern inventory scalar prefix, stops before CreationHistory.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! InventoryItemDetails::serialize head>0x37c, SlotID::serialize, MemoryInputStream integers.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/inventory/InventoryItemDetails.java
//! Local48samples heads919..951/branch0/flags7 match primitive/SlotID external oracle.
//! Applicability893..951/branch0/flags0or7 enforced by parent plan prefix.
//! Conflict: pinned s64 uses arithmetic right shift on high-bit compact integers.
//! Preserve unsigned encoded bits, no signed-date/timestamp normalization or enum loss.
//! Full sources, coverage and build applicability: docs/plan-inventory-prefix.md.
use super::PlanInspection;
use crate::binary::{check_cancel, integer, Error};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct InventoryPrefix {
    /// Decoded unsigned wire integer, exact 16 hexadecimal digits. Not a timestamp.
    pub date_added_encoded_u64_hex: String,
    pub slot_type_raw_u32: u32,
    pub slot_number_raw_u32: u32,
    pub highlight_sound_guid_raw_u32: u32,
    pub colour_raw_u32: u32,
    pub type_flags_raw_u32: u32,
    pub sub_type_raw_u32: u32,
    pub title_key_raw_u32: u32,
    pub description_key_raw_u32: u32,
    pub inventory_prefix_start: usize,
    pub parsed_inventory_prefix_bytes: usize,
    pub inventory_prefix_end: usize,
    pub unparsed_after_inventory_prefix: usize,
}
#[derive(Debug)]
pub struct PlanInventoryInspection {
    pub plan: PlanInspection,
    /// Deeper failure preserves the valid base stage, not fabricated inventory fields.
    /// Callers must still honor their cancellation token before publishing results.
    pub inventory: Result<InventoryPrefix, Error>,
}
fn encoded_u64(data: &[u8], offset: &mut usize, compact: bool) -> Result<u64, Error> {
    if !compact {
        let end = offset.checked_add(8).ok_or(Error::Budget)?;
        let bytes = data
            .get(*offset..end)
            .ok_or(Error::Invalid("inventory date bounds"))?;
        *offset = end;
        return Ok(u64::from_be_bytes(bytes.try_into().unwrap()));
    }
    let mut value = 0;
    for i in 0..10 {
        let byte = *data
            .get(*offset)
            .ok_or(Error::Invalid("inventory date bounds"))?;
        *offset = offset.checked_add(1).ok_or(Error::Budget)?;
        if i == 9 && byte > 1 {
            return Err(Error::Invalid("uleb64 overflow"));
        }
        value |= u64::from(byte & 127) << (i * 7);
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(Error::Invalid("unterminated uleb64"))
}
pub(super) fn inspect(
    data: &[u8],
    plan: &PlanInspection,
    cancel: &impl Fn() -> bool,
) -> Result<InventoryPrefix, Error> {
    check_cancel(cancel)?;
    let start = plan.prefix.inventory_tail_start;
    let mut offset = start;
    let compact = plan.compression_flags & 1 != 0;
    let encoded = encoded_u64(data, &mut offset, compact)?;
    let mut fields = [0; 8];
    for field in &mut fields {
        check_cancel(cancel)?;
        *field = integer(data, &mut offset, compact)?;
    }
    check_cancel(cancel)?;
    Ok(InventoryPrefix {
        date_added_encoded_u64_hex: format!("{encoded:016x}"),
        slot_type_raw_u32: fields[0],
        slot_number_raw_u32: fields[1],
        highlight_sound_guid_raw_u32: fields[2],
        colour_raw_u32: fields[3],
        type_flags_raw_u32: fields[4],
        sub_type_raw_u32: fields[5],
        title_key_raw_u32: fields[6],
        description_key_raw_u32: fields[7],
        inventory_prefix_start: start,
        parsed_inventory_prefix_bytes: offset - start,
        inventory_prefix_end: offset,
        unparsed_after_inventory_prefix: data.len() - offset,
    })
}
