//! L4 Thing header and part-flag decode for the LBP2 band (0x341 <= head <= 0x3f8).
//!
//! Specification notes (independent wording), ennuo/toolkit
//! d6a2ef80b5a09999f08f23a803b516096055e5c9, CONFIRMED_FROM_COMMUNITY_CODE:
//! - `structs/things/Thing.java` serialize, read order for head >= 0x341 on branch 0
//!   (other branches are refused before any Thing is read):
//!   u8 test marker 0xAA (head >= 0x2a1) -> UID i32 -> parent reference(Thing) ->
//!   groupHead reference(Thing) -> oldEmitter reference(Thing) (head >= 0x1c7) ->
//!   createdBy i16, changedBy i16 (head >= 0x214) -> planGUID guid (head >= 0x254) ->
//!   flags i8 (i16 only on DOUBLE11 >= 0x62, refused upstream) -> partsRevision s32 ->
//!   part flags u64 (compressed form when head >= 0x297) -> each present part as
//!   reference(P*) in `Part` enum declaration order.
//! - `enums/Part.java` hasPart/serialize and `enums/PartHistory.java`: a part is present
//!   when partsRevision >= its PartHistory value and bit `index` of the flags is set. When
//!   partsRevision >= CREATOR_ANIM (0x32) and subVersion < 0x107 (all LBP2), indices above
//!   0x28 shift up by one and bit 0x29 is the deprecated CREATOR_ANIM. Deprecated parts
//!   (no class) read one i32 that must be 0. Indices 0x36..0x3c never appear at head >=
//!   0x13c, PARTICLE_EMITTER_2 never at head >= 0x18c.
//! - Flag bits that map to no present part are ignored by the community reader; this
//!   reader keeps them in `unmapped_flag_bits` instead of guessing.
use crate::parts::{self, PartData};
use crate::serial::{Obj, Reader, Result, Stop, Tag};
use serde::Serialize;

/// (name, index, PartHistory version, has reader class in Toolkit) in enum order.
pub const PARTS: &[(&str, u32, i32, bool)] = &[
    ("BODY", 0x0, 0x1, true),
    ("JOINT", 0x1, 0x2, true),
    ("WORLD", 0x2, 0x3, true),
    ("RENDER_MESH", 0x3, 0x4, true),
    ("POS", 0x4, 0x5, true),
    ("TRIGGER", 0x5, 0x6, true),
    ("TRIGGER_EFFECTOR", 0x36, 0x7, false),
    ("PAINT", 0x37, 0x8, false),
    ("YELLOWHEAD", 0x6, 0x9, true),
    ("AUDIO_WORLD", 0x7, 0xa, true),
    ("ANIMATION", 0x8, 0xb, true),
    ("GENERATED_MESH", 0x9, 0xc, true),
    ("PARTICLE_CLUMP", 0x38, 0xd, false),
    ("PARTICLE_EMITTER", 0x39, 0xe, false),
    ("CAMERA_ZONE", 0x3a, 0xf, false),
    ("LEVEL_SETTINGS", 0xa, 0x10, true),
    ("SPRITE_LIGHT", 0xb, 0x11, true),
    ("KEYFRAMED_POSITION", 0x3b, 0x12, false),
    ("CAMERA", 0x3c, 0x13, false),
    ("SCRIPT_NAME", 0xc, 0x14, true),
    ("CREATURE", 0xd, 0x15, true),
    ("CHECKPOINT", 0xe, 0x16, true),
    ("STICKERS", 0xf, 0x17, true),
    ("DECORATIONS", 0x10, 0x18, true),
    ("SCRIPT", 0x11, 0x19, true),
    ("SHAPE", 0x12, 0x1a, true),
    ("EFFECTOR", 0x13, 0x1b, true),
    ("EMITTER", 0x14, 0x1c, true),
    ("REF", 0x15, 0x1d, true),
    ("METADATA", 0x16, 0x1e, true),
    ("COSTUME", 0x17, 0x1f, true),
    ("PARTICLE_EMITTER_2", 0x3d, 0x20, false),
    ("CAMERA_TWEAK", 0x18, 0x21, true),
    ("SWITCH", 0x19, 0x22, true),
    ("SWITCH_KEY", 0x1a, 0x23, true),
    ("GAMEPLAY_DATA", 0x1b, 0x24, true),
    ("ENEMY", 0x1c, 0x25, true),
    ("GROUP", 0x1d, 0x26, true),
    ("PHYSICS_TWEAK", 0x1e, 0x27, true),
    ("NPC", 0x1f, 0x28, true),
    ("SWITCH_INPUT", 0x20, 0x29, true),
    ("MICROCHIP", 0x21, 0x2a, true),
    ("MATERIAL_TWEAK", 0x22, 0x2b, true),
    ("MATERIAL_OVERRIDE", 0x23, 0x2c, true),
    ("INSTRUMENT", 0x24, 0x2d, true),
    ("SEQUENCER", 0x25, 0x2e, true),
    ("CONTROLINATOR", 0x26, 0x2f, true),
    ("POPPET_POWERUP", 0x27, 0x30, true),
    ("POCKET_ITEM", 0x28, 0x31, true),
    ("CREATOR_ANIM", 0x3e, 0x32, false),
    ("TRANSITION", 0x29, 0x33, true),
    ("FADER", 0x2a, 0x34, true),
    ("ANIMATION_TWEAK", 0x2b, 0x35, true),
    ("WIND_TWEAK", 0x2c, 0x36, true),
    ("POWER_UP", 0x2d, 0x37, true),
    ("HUD_ELEM", 0x2e, 0x38, true),
    ("TAG_SYNCHRONIZER", 0x2f, 0x39, true),
    ("WORMHOLE", 0x30, 0x3a, true),
    ("QUEST", 0x31, 0x3b, true),
    ("CONNECTOR_HOOK", 0x32, 0x3c, true),
    ("ATMOSPHERIC_TWEAK", 0x33, 0x3d, true),
    ("STREAMING_DATA", 0x34, 0x3e, true),
    ("STREAMING_HINT", 0x35, 0x3f, true),
];

const CREATOR_ANIM_HISTORY: i32 = 0x32;
/// Lowest head whose Things this reader accepts (plans only; D119).
pub const PRE_BAND_MIN: u32 = 0x335;

/// Flag bit tested for a part, or None when the part can never appear at this head
/// (LBP2 heads only: subVersion is always 0 here).
fn flag_bit(head: u32, index: u32, history: i32, parts_revision: i32) -> Option<u32> {
    let version = head & 0xffff;
    if version >= 0x13c && (0x36..=0x3c).contains(&index) {
        return None;
    }
    if version >= 0x18c && index == 0x3d {
        return None;
    }
    if parts_revision >= CREATOR_ANIM_HISTORY {
        if index == 0x3e {
            return Some(0x29);
        }
        if index > 0x28 {
            return (parts_revision >= history).then_some(index + 1);
        }
    }
    (parts_revision >= history).then_some(index)
}

/// Present parts in serialization order, plus the flag bits no present part explains.
pub fn present_parts(head: u32, flags: u64, parts_revision: i32) -> (Vec<&'static str>, u64) {
    let mut used = 0u64;
    let mut out = Vec::new();
    for &(name, index, history, _) in PARTS {
        if let Some(bit) = flag_bit(head, index, history, parts_revision) {
            if bit < 64 && flags & (1u64 << bit) != 0 {
                used |= 1u64 << bit;
                out.push(name);
            }
        }
    }
    (out, flags & !used)
}

fn has_class(name: &str) -> bool {
    PARTS.iter().any(|p| p.0 == name && p.3)
}

#[derive(Debug, Clone, Serialize)]
pub struct PartEntry {
    pub part: &'static str,
    /// "parsed", "null" (tag 0), "shared" (back-reference) or "deprecated_zero".
    pub state: &'static str,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ThingRecord {
    /// Index in parse order (registration order of the reference tag).
    pub index: usize,
    pub tag: u32,
    pub start_offset: usize,
    pub end_offset: Option<usize>,
    pub uid: Option<i32>,
    /// Thing indices of referenced Things (None = null reference).
    pub parent: Option<usize>,
    pub group_head: Option<usize>,
    pub old_emitter: Option<usize>,
    pub created_by: i16,
    pub changed_by: i16,
    pub plan_guid: Option<u32>,
    pub flags: i8,
    pub parts_revision: i32,
    pub part_flags: u64,
    pub unmapped_flag_bits: u64,
    /// Parts named by the flags, in serialization order.
    pub parts: Vec<&'static str>,
    /// Parts actually consumed so far, with their state.
    pub read: Vec<PartEntry>,
    pub data: PartData,
    /// True once every present part has been read.
    pub complete: bool,
}

#[derive(Debug, Default)]
pub struct Things {
    pub list: Vec<ThingRecord>,
    /// Length of the PWorld `things` array once every element has been read.
    pub world_array_done: Option<usize>,
    /// InstanceLayout bodies by per-class sequence number.
    pub layouts: Vec<Vec<parts::LayoutField>>,
    /// PShape / PRenderMesh bodies by per-class sequence number, so a later
    /// back-reference (Thing part, ContactCache, Decoration) resolves to the same data.
    pub shapes: std::collections::HashMap<usize, parts::Shape>,
    pub render_meshes: std::collections::HashMap<usize, parts::RenderMesh>,
}

fn thing_index(t: Tag) -> Option<usize> {
    match t {
        Tag::Seen(Obj::Thing(i)) => Some(i),
        _ => None,
    }
}

/// Reads `reference(Thing)`. Returns the Thing index or None for null.
pub fn thing_ref(r: &mut Reader, things: &mut Things) -> Result<Option<usize>> {
    let start = r.pos();
    let next = things.list.len();
    let tag = r.tag("Thing", |r| {
        if next >= r.limits.max_things {
            return r.budget("things");
        }
        Ok(Obj::Thing(next))
    })?;
    match tag {
        Tag::Null => Ok(None),
        Tag::Seen(_) => Ok(thing_index(tag)),
        Tag::New(t) => {
            things.list.push(ThingRecord {
                index: next,
                tag: t,
                start_offset: start,
                ..Default::default()
            });
            r.enter()?;
            thing_body(r, things, next)?;
            r.leave();
            Ok(Some(next))
        }
    }
}

/// Reads `thingarray`/`arraylist(Thing, reference)`: i32 count then a reference each.
pub fn thing_array(r: &mut Reader, things: &mut Things) -> Result<Vec<Option<usize>>> {
    let n = r.count(1)?;
    let mut out = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        out.push(thing_ref(r, things)?);
    }
    Ok(out)
}

fn thing_body(r: &mut Reader, things: &mut Things, me: usize) -> Result<()> {
    r.check_cancel()?;
    let v = r.rev.version();
    // D119: plans at 0x335..0x340 (four local backdrop plans) use the pre-0x341 header
    // and a short audited part list (`parts::supported_at`). RLevel refuses heads below
    // 0x341 before any Thing is read, so levels keep the D107 band.
    if !(PRE_BAND_MIN..=crate::LBP2_MAX).contains(&v) || r.rev.head > 0xffff {
        return r.unsupported(format!("Thing layout at head 0x{:x}", r.rev.head));
    }
    if r.u8()? != 0xaa {
        return r.invalid("Thing test marker is not 0xAA");
    }
    things.list[me].uid = Some(r.i32()?);
    let parent = thing_ref(r, things)?;
    things.list[me].parent = parent;
    let group = thing_ref(r, things)?;
    things.list[me].group_head = group;
    let emitter = thing_ref(r, things)?;
    things.list[me].old_emitter = emitter;
    let created_by = r.i16()?;
    let changed_by = r.i16()?;
    let (plan_guid, flags) = if v < 0x341 {
        // Toolkit Thing.serialize below THING_FLAGS (0x341): isStamping bool (> 0x21a),
        // planGUID (>= 0x254), hidden bool (>= 0x2f2); no flags byte.
        r.bool()?;
        let plan = r.guid()?;
        r.bool()?;
        (plan, 0)
    } else {
        (r.guid()?, r.i8()?)
    };
    let parts_revision = r.s32()?;
    let part_flags = r.u64()?;
    let (present, unmapped) = present_parts(r.rev.head, part_flags, parts_revision);
    {
        let t = &mut things.list[me];
        t.created_by = created_by;
        t.changed_by = changed_by;
        t.plan_guid = plan_guid;
        t.flags = flags;
        t.parts_revision = parts_revision;
        t.part_flags = part_flags;
        t.unmapped_flag_bits = unmapped;
        t.parts = present.clone();
    }
    for name in present {
        if !has_class(name) {
            // Deprecated part: one i32 that must be zero.
            if r.i32()? != 0 {
                return r.invalid(format!("deprecated part {name} is not zero"));
            }
            things.list[me].read.push(PartEntry {
                part: name,
                state: "deprecated_zero",
            });
            continue;
        }
        if !parts::supported_at(name, v) {
            return Err(Stop::UnsupportedPart {
                part: name,
                head: r.rev.head,
                parts_revision,
                offset: r.pos(),
            });
        }
        let state = parts::part_ref(r, things, me, name)?;
        things.list[me].read.push(PartEntry { part: name, state });
    }
    let t = &mut things.list[me];
    t.complete = true;
    t.end_offset = Some(r.pos());
    Ok(())
}
