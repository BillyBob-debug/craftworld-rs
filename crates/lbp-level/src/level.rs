//! RLevel entry point. Spec notes from ennuo/toolkit d6a2ef80, `resources/RLevel.java`
//! serialize (CONFIRMED_FROM_COMMUNITY_CODE): for head > 0x3e6 (strict) an i32 count
//! then that many 20-byte SHA1s (cross-play Vita dependency hashes); then
//! `worldThing = reference(Thing)`; then (head > 0x213) PlayerRecord = array of
//! NetworkPlayerID (36 bytes each at head >= 0x234), intarray playerNumbers, offset i32;
//! then tutorialInventory arraylist(InventoryItem) (> 0x347, see parts::inventory_item)
//! and tutorialInventoryData arraylist(CachedInventoryData) (> 0x35c);
//! tutorialInventoryActive bool (> 0x39b); copiedFromSomeoneElse
//! bool (> 0x3b0). Later fields are DOUBLE11/subVersion-gated and absent here.
use crate::serial::{Limits, Reader, Revision, Stop};
use crate::thing::{thing_ref, ThingRecord, Things};
use serde::Serialize;

pub const MAX_INPUT: usize = 32 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 128 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct LevelReport {
    pub revision: Revision,
    pub expanded_bytes: usize,
    pub cross_play_hashes: Option<usize>,
    /// Bytes consumed when the walk ended (complete or stopped).
    pub consumed_bytes: usize,
    pub max_depth: usize,
    pub tags: usize,
    /// Length of the world `things` array when every element was read.
    pub world_things_walked: Option<usize>,
    /// Detached PShape bodies reached only through ContactCache references.
    pub detached_shapes: usize,
    /// None = the whole RLevel was read (world Thing, PlayerRecord and tail fields).
    pub stop: Option<Stop>,
    /// Unread payload bytes after a complete read (None when stopped). The community
    /// reader does not check this; nonzero would indicate a layout disagreement.
    pub trailing_bytes: Option<usize>,
    pub things: Vec<ThingRecord>,
}

/// Reads a whole LVLb resource (wrapper via `lbp_resource::binary::payload`).
pub fn read(
    bytes: &[u8],
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> Result<LevelReport, lbp_resource::binary::Error> {
    let (h, data) =
        lbp_resource::binary::payload(bytes, b"LVLb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    Ok(read_payload(&data, rev, limits, cancel))
}

/// Reads an expanded RLevel payload with a known revision.
pub fn read_payload(
    data: &[u8],
    rev: Revision,
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> LevelReport {
    let mut r = Reader::new(data, rev, limits, cancel);
    let mut things = Things::default();
    let mut cross = None;
    let result: crate::serial::Result<()> = (|| {
        // Branch 0 only: LEERDAMMER (0x4c44) adds branch gates in some parts (PCreature head
        // piece, PWorld water fields) that are not implemented; all such local levels are
        // below head 0x341 anyway (D107 census).
        if rev.branch_id != 0 {
            return r.unsupported(format!("branch 0x{:x}", rev.branch_id));
        }
        if rev.head > crate::LBP2_MAX || rev.version() < 0x341 {
            return r.unsupported(format!("RLevel at head 0x{:x}", rev.head));
        }
        if rev.version() > 0x3e6 {
            let n = r.count(20)?;
            r.bytes(n * 20)?;
            cross = Some(n);
        }
        if thing_ref(&mut r, &mut things)?.is_none() {
            return r.invalid("null world Thing");
        }
        // PlayerRecord: array(NetworkPlayerID, 36 bytes each), intarray, offset i32.
        let n = r.count(36)?;
        r.bytes(n * 36)?;
        r.int_array()?;
        r.i32()?;
        let v = rev.version();
        if v > 0x347 {
            let n = r.count(4)?;
            for _ in 0..n {
                crate::parts::inventory_item(&mut r)?;
            }
        }
        if v > 0x35c {
            // CachedInventoryData: category u32, location u32, then (> 0x39d)
            // cachedToolType i32 and four i8.
            let n = r.count(2)?;
            for _ in 0..n {
                r.u32()?;
                r.u32()?;
                if v > 0x39d {
                    r.i32()?;
                    r.bytes(4)?;
                }
            }
        }
        if v > 0x39b {
            r.bool()?;
        }
        if v > 0x3b0 {
            r.bool()?;
        }
        Ok(())
    })();
    let stop = result.err();
    let trailing_bytes = stop.is_none().then(|| r.remaining());
    LevelReport {
        trailing_bytes,
        revision: rev,
        expanded_bytes: data.len(),
        cross_play_hashes: cross,
        consumed_bytes: r.pos(),
        max_depth: r.max_depth_seen,
        tags: r.tag_count(),
        world_things_walked: things.world_array_done,
        detached_shapes: r.other_count("SHAPE").saturating_sub(
            things
                .list
                .iter()
                .filter(|t| {
                    t.read
                        .iter()
                        .any(|p| p.part == "SHAPE" && p.state == "parsed")
                })
                .count(),
        ),
        stop,
        things: things.list,
    }
}
