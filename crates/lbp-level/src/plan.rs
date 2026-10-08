//! D119: RPlan entry point for the shared Thing reader (plans and levels serialize Things
//! the same way). Spec notes from ennuo/toolkit d6a2ef80, CONFIRMED_FROM_COMMUNITY_CODE:
//! `resources/RPlan.java` serialize: (subVersion >= STREAMING_PLAN only, never LBP2) a
//! streaming bool; an i32 plan revision that the community reader ignores (it uses the
//! resource head); `thingData` bytearray (i32 count + bytes); then, from PLAN_DETAILS,
//! the InventoryItemDetails tail (not read here). `RPlan.getThings`: thingData is read
//! with a fresh serializer (new reference table) at the resource revision and compression
//! flags as `array(Thing, reference)`: i32 count, then one Thing reference each.
//! Thing band: the D107 level band (0x341..=0x3f8, branch 0) plus, for plans only,
//! heads 0x335..0x340 with the pre-0x341 Thing header and a short audited part list
//! (`parts::supported_at`); other heads stop with an explicit Unsupported outcome.
use crate::serial::{Limits, Reader, Revision, Stop};
use crate::thing::{thing_array, ThingRecord, Things};
use serde::Serialize;

pub const MAX_INPUT: usize = 16 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 64 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct PlanReport {
    pub revision: Revision,
    pub expanded_bytes: usize,
    /// Plan revision word stored before thingData (ignored by the community reader).
    pub stored_revision: Option<u32>,
    pub thing_data_bytes: Option<usize>,
    /// Top-level array entries (Thing indices; None = null reference).
    pub roots: Vec<Option<usize>>,
    /// None = the whole thingData array was read.
    pub stop: Option<Stop>,
    /// Unread thingData bytes after a complete read (None when stopped).
    pub trailing_thing_bytes: Option<usize>,
    pub things: Vec<ThingRecord>,
}

/// Reads a whole PLNb resource and its Thing array.
pub fn read_plan(
    bytes: &[u8],
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> Result<PlanReport, lbp_resource::binary::Error> {
    let (h, data) =
        lbp_resource::binary::payload(bytes, b"PLNb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    Ok(read_plan_payload(&data, rev, limits, cancel))
}

/// Reads an expanded RPlan payload with a known revision.
pub fn read_plan_payload(
    data: &[u8],
    rev: Revision,
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> PlanReport {
    let mut report = PlanReport {
        revision: rev,
        expanded_bytes: data.len(),
        stored_revision: None,
        thing_data_bytes: None,
        roots: Vec::new(),
        stop: None,
        trailing_thing_bytes: None,
        things: Vec::new(),
    };
    let mut outer = Reader::new(data, rev, limits, cancel);
    let thing_data = (|| {
        if rev.branch_id != 0 {
            return outer.unsupported(format!("branch 0x{:x}", rev.branch_id));
        }
        report.stored_revision = Some(outer.u32()?);
        let n = outer.count(1)?;
        outer.bytes(n)
    })();
    let thing_data = match thing_data {
        Ok(t) => t,
        Err(stop) => {
            report.stop = Some(stop);
            return report;
        }
    };
    report.thing_data_bytes = Some(thing_data.len());
    let mut r = Reader::new(thing_data, rev, limits, cancel);
    let mut things = Things::default();
    let result: Result<Vec<Option<usize>>, Stop> = (|| {
        if rev.head > crate::LBP2_MAX || rev.version() < crate::thing::PRE_BAND_MIN {
            return r.unsupported(format!("Thing layout at head 0x{:x}", rev.head));
        }
        thing_array(&mut r, &mut things)
    })();
    match result {
        Ok(roots) => {
            report.roots = roots;
            report.trailing_thing_bytes = Some(r.remaining());
        }
        Err(stop) => report.stop = Some(stop),
    }
    report.things = things.list;
    report
}
