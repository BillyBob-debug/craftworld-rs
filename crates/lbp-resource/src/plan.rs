//! Independent CPU PLNb diagnostics; no Thing or recursive inventory deserialization.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! RPlan::serialize (inner revision, opaque bytearray); MemoryInputStream::i32/bytearray.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RPlan.java
//! Applicability: ordinary PS3 branch0/sub0 heads893..951, flags0/7. Streaming branch
//! unsupported. Raw inner revision is ignored by community resource reader; mismatch
//! is not corrupt. CONFIRMED_FROM_LOCAL_DATA:48modern prefixes/28heads919..951.
//! Full provenance, primitive-oracle limitations and tests: docs/plan-prefix.md.
use crate::binary::{check_cancel, Error};
use serde::Serialize;
use sha2::{Digest, Sha256};
mod inventory;
pub use inventory::{InventoryPrefix, PlanInventoryInspection};
pub const MAX_INPUT: usize = 2 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 16 * 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct PlanPrefix {
    pub revision: u32,
    pub inner_revision_raw_u32: u32,
    pub thing_data_bytes: usize,
    pub thing_data_start: usize,
    pub thing_data_sha256: String,
    pub parsed_prefix_bytes: usize,
    /// Includes opaque Thing bytes and the unparsed inventory tail.
    pub unparsed_bytes: usize,
    pub inventory_tail_start: usize,
    /// Byte extent only: not parsed or validated inventory metadata.
    pub inventory_tail_bytes: usize,
}
#[derive(Debug, Serialize)]
pub struct PlanInspection {
    pub compression_flags: u8,
    pub expanded_bytes: usize,
    pub prefix: PlanPrefix,
}
/// Read-only bounded prefix metadata. Opaque bytes do not escape this API.
pub fn inspect(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<PlanInspection, Error> {
    let (header, data) = crate::binary::payload(bytes, b"PLNb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    prefix(&header, &data, &cancel)
}
/// Optional bounded inventory stage; valid base prefix remains available on deeper error.
/// No public expanded buffer; wrapper inflation and opaque digest computed once.
pub fn inspect_inventory(
    bytes: &[u8],
    cancel: impl Fn() -> bool,
) -> Result<PlanInventoryInspection, Error> {
    let (header, data) = crate::binary::payload(bytes, b"PLNb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let plan = prefix(&header, &data, &cancel)?;
    let inventory = inventory::inspect(&data, &plan, &cancel);
    Ok(PlanInventoryInspection { plan, inventory })
}
fn prefix(
    header: &crate::Header,
    data: &[u8],
    cancel: &impl Fn() -> bool,
) -> Result<PlanInspection, Error> {
    if !(893..=951).contains(&header.revision)
        || header.branch_id != Some(0)
        || header.branch_revision != Some(0)
    {
        return Err(Error::Unsupported("plan revision/branch"));
    }
    let flags = header
        .compression_flags
        .ok_or(Error::Invalid("flags absent"))?;
    let mut offset = 0;
    let inner_revision = crate::binary::integer(data, &mut offset, flags & 1 != 0)?;
    let length = usize::try_from(crate::binary::integer(data, &mut offset, flags & 1 != 0)?)
        .map_err(|_| Error::Budget)?;
    if length > MAX_EXPANDED {
        return Err(Error::Budget);
    }
    let end = offset.checked_add(length).ok_or(Error::Budget)?;
    let opaque = data
        .get(offset..end)
        .ok_or(Error::Invalid("plan byte array bounds"))?;
    let mut hash = Sha256::new();
    for part in opaque.chunks(4096) {
        check_cancel(cancel)?;
        hash.update(part);
    }
    check_cancel(cancel)?;
    Ok(PlanInspection {
        compression_flags: flags,
        expanded_bytes: data.len(),
        prefix: PlanPrefix {
            revision: header.revision,
            inner_revision_raw_u32: inner_revision,
            thing_data_bytes: length,
            thing_data_start: offset,
            thing_data_sha256: format!("{:x}", hash.finalize()),
            parsed_prefix_bytes: offset,
            unparsed_bytes: data.len() - offset,
            inventory_tail_start: end,
            inventory_tail_bytes: data.len() - end,
        },
    })
}
