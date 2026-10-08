//! U4: bounded readers for the front-end data resources the Pod, earth/moon and Popit
//! screens show: SLOT_LIST (`SLTb`, Slot), PINS (`PINb`, Pin), PALETTE (`PALb`) and
//! PACKS (`PCKb`, Pack). UI-independent; built on the L3 serializer core.
//!
//! Independent specification notes (no code copied/translated), written from
//! ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib:
//! `resources/RSlotList.java`, `structs/slot/{Slot,SlotID,Label,Collectabubble,Pack}.java`,
//! `structs/profile/Pin.java`, `resources/{RPins,RPalette,RPacks}.java`,
//! `structs/profile/NetworkOnlineID.java`, `enums/{SlotType,LevelType,ContentsType}.java`.
//! CONFIRMED_FROM_COMMUNITY_CODE for the layout; not yet compared with game files
//! (docs/u4-slot-pin-palette.md has the PC checks). Rules as implemented:
//! - Arrays of structs: count (i32, compressed per flag 1) then each struct inline.
//! - enum32: i32 (not zigzag); unknown values are kept raw and counted (Toolkit maps them
//!   to null).
//! - `str`: s32 byte length then bytes (Java default charset, here UTF-8 lossy), NULs
//!   removed. `wstr`: s32 unit count then UTF-16BE units (lossy), NULs removed.
//! - Descriptors marked "descriptor" in the notes have no flags word; others have one
//!   above head 0x22e (serial.rs).
//! - Only branch 0 with no LBP3 sub-revision is read: Vita (DOUBLE11) and LBP3 add Slot
//!   and Pack fields that are not implemented here, and are refused as Unsupported.
use crate::serial::{Descriptor, Limits, Reader, Result, Revision, Stop};
use serde::Serialize;

/// Toolkit `Revisions.PRODUCTION_BUILD`: RSlotList gains `fromProductionBuild`.
const PRODUCTION_BUILD: u32 = 0x3b6;
/// Toolkit `Revisions.PALETTE_CONVERTED_PLANS`.
const PALETTE_CONVERTED_PLANS: u32 = 0x323;

pub const MAX_INPUT: usize = 64 << 20;
const MAX_EXPANDED: usize = 256 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SlotId {
    /// `SlotType` raw value (0 DEVELOPER, 1 USER_CREATED_ON_SERVER, 2 USER_CREATED_STORED_LOCAL,
    /// 3 LOCAL_GROUP, 4 DEVELOPER_GROUP, 5 POD, 6 FAKE, 7 REMOTE_MOON, 8 DLC_LEVEL,
    /// 9 DLC_PACK, 10 PLAYLIST, 11..17 adventure kinds).
    pub slot_type: u32,
    pub number: u32,
}

impl SlotId {
    pub fn type_name(&self) -> Option<&'static str> {
        const NAMES: [&str; 18] = [
            "DEVELOPER",
            "USER_CREATED_ON_SERVER",
            "USER_CREATED_STORED_LOCAL",
            "LOCAL_GROUP",
            "DEVELOPER_GROUP",
            "POD",
            "FAKE",
            "REMOTE_MOON",
            "DLC_LEVEL",
            "DLC_PACK",
            "PLAYLIST",
            "DEVELOPER_ADVENTURE",
            "DEVELOPER_ADVENTURE_PLANET",
            "DEVELOPER_ADVENTURE_AREA",
            "USER_ADVENTURE_PLANET_PUBLISHED",
            "ADVENTURE_PLANET_LOCAL",
            "ADVENTURE_LEVEL_LOCAL",
            "ADVENTURE_AREA_LEVEL",
        ];
        NAMES.get(self.slot_type as usize).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Label {
    /// LAMS key of the label text.
    pub key: u32,
    pub order: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Collectabubble {
    pub plan: Option<Descriptor>,
    pub count: i32,
}

/// One level badge (story level, group, Pod, DLC pack page, ...). Fields absent at the
/// file's revision are `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Slot {
    pub id: SlotId,
    pub root: Option<Descriptor>,
    pub icon: Option<Descriptor>,
    pub location: [f32; 4],
    /// NetworkOnlineID: 16 name bytes (NUL padded), terminator byte, 3 padding bytes.
    pub author_id: String,
    pub author_name: Option<String>,
    /// Name translation tag (key = LAMS hash of the tag, U1).
    pub translation_tag: Option<String>,
    pub name: String,
    pub description: String,
    pub primary_link_level: SlotId,
    pub group: Option<SlotId>,
    pub initially_locked: bool,
    pub shareable: Option<bool>,
    pub background_guid: Option<u32>,
    pub planet_decorations: Option<Descriptor>,
    /// `LevelType` raw value (0 MAIN_PATH .. 8 LBP2_DLC_HUB).
    pub developer_level_type: Option<u32>,
    pub game_progression_state: Option<u32>,
    pub labels: Vec<Label>,
    pub collectabubbles_required: Vec<Collectabubble>,
    pub collectabubbles_contained: Vec<Collectabubble>,
    pub is_sub_level: Option<bool>,
    pub min_players: Option<i8>,
    pub max_players: Option<i8>,
    pub move_recommended: Option<bool>,
    pub cross_compatible: Option<bool>,
    pub show_on_planet: Option<bool>,
    pub lives_override: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotList {
    pub slots: Vec<Slot>,
    pub from_production_build: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Pin {
    pub id: u32,
    pub progress_type: u32,
    pub category: u32,
    pub title_key: u32,
    pub description_key: u32,
    pub icon: Option<Descriptor>,
    pub initial_progress: u32,
    pub target: u32,
    pub trophy_to_unlock: i8,
    pub behaviour_flags: i16,
    pub trophy_to_unlock_lbp1: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Palette {
    pub plans: Vec<Option<Descriptor>>,
    pub location: u32,
    pub description: u32,
    pub converted_plans: Option<Vec<Option<Descriptor>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pack {
    /// `ContentsType` raw value (0 GROUP, 1 PLANS, 2 LEVEL, 3 COSTUME, 5 ADVENTURE).
    pub contents_type: u32,
    pub mesh: Option<Descriptor>,
    pub slot: Slot,
    pub content_id: String,
    pub timestamp: i64,
}

/// Counted oddities that do not stop the read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Checks {
    pub unknown_slot_types: usize,
    pub unknown_level_types: usize,
    pub unknown_contents_types: usize,
    pub invalid_utf16: usize,
    /// Payload bytes after the resource body (Toolkit ignores them).
    pub trailing_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiData {
    SlotList(SlotList),
    Pins {
        pins: Vec<Pin>,
    },
    Palette(Palette),
    /// File order; Toolkit sorts by unsigned timestamp after reading.
    Packs {
        packs: Vec<Pack>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UiDataReport {
    pub revision: Revision,
    pub result: std::result::Result<UiData, Stop>,
    pub checks: Checks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UiKind {
    SlotList,
    Pins,
    Palette,
    Packs,
}

impl UiKind {
    pub fn magic(self) -> &'static [u8; 4] {
        match self {
            UiKind::SlotList => b"SLTb",
            UiKind::Pins => b"PINb",
            UiKind::Palette => b"PALb",
            UiKind::Packs => b"PCKb",
        }
    }
    pub fn from_magic(magic: &[u8]) -> Option<Self> {
        [Self::SlotList, Self::Pins, Self::Palette, Self::Packs]
            .into_iter()
            .find(|k| k.magic().as_slice() == magic)
    }
}

/// Reads a whole resource file whose magic is one of the four U4 kinds.
pub fn read(
    bytes: &[u8],
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> std::result::Result<UiDataReport, lbp_resource::binary::Error> {
    let kind = bytes.get(..4).and_then(UiKind::from_magic).ok_or(
        lbp_resource::binary::Error::Unsupported("not a U4 resource magic"),
    )?;
    let (h, data) =
        lbp_resource::binary::payload(bytes, kind.magic(), MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    Ok(read_payload(kind, &data, rev, limits, cancel))
}

/// Reads an expanded payload of `kind` at a known revision.
pub fn read_payload(
    kind: UiKind,
    data: &[u8],
    rev: Revision,
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> UiDataReport {
    let mut r = Reader::new(data, rev, limits, cancel);
    let mut checks = Checks::default();
    let result = (|| {
        if rev.branch_id != 0 || rev.head >> 16 != 0 {
            return r.unsupported(format!(
                "branch 0x{:x} / head 0x{:x} (Vita and LBP3 fields not implemented)",
                rev.branch_id, rev.head
            ));
        }
        if rev.head > crate::LBP2_MAX || rev.version() < 0x191 {
            return r.unsupported(format!("head 0x{:x}", rev.head));
        }
        let c = &mut checks;
        Ok(match kind {
            UiKind::SlotList => {
                let n = r.count(1)?;
                let mut slots = Vec::with_capacity(n);
                for _ in 0..n {
                    r.check_cancel()?;
                    slots.push(slot(&mut r, c)?);
                }
                let from_production_build = if rev.version() >= PRODUCTION_BUILD {
                    Some(r.bool()?)
                } else {
                    None
                };
                UiData::SlotList(SlotList {
                    slots,
                    from_production_build,
                })
            }
            UiKind::Pins => {
                let n = r.count(1)?;
                let mut pins = Vec::with_capacity(n);
                for _ in 0..n {
                    pins.push(pin(&mut r)?);
                }
                UiData::Pins { pins }
            }
            UiKind::Palette => {
                let plans = descriptors(&mut r, false)?;
                let location = r.u32()?;
                let description = r.u32()?;
                let converted_plans = if rev.version() >= PALETTE_CONVERTED_PLANS {
                    Some(descriptors(&mut r, true)?)
                } else {
                    None
                };
                UiData::Palette(Palette {
                    plans,
                    location,
                    description,
                    converted_plans,
                })
            }
            UiKind::Packs => {
                let n = r.count(1)?;
                let mut packs = Vec::with_capacity(n);
                for _ in 0..n {
                    r.check_cancel()?;
                    let contents_type = r.u32()?;
                    if !matches!(contents_type, 0..=3 | 5) {
                        c.unknown_contents_types += 1;
                    }
                    let mesh = r.resource(true)?;
                    let slot = slot(&mut r, c)?;
                    let content_id = str8(&mut r)?;
                    let timestamp = s64(&mut r)?;
                    packs.push(Pack {
                        contents_type,
                        mesh,
                        slot,
                        content_id,
                        timestamp,
                    });
                }
                UiData::Packs { packs }
            }
        })
    })();
    checks.trailing_bytes = if result.is_ok() { r.remaining() } else { 0 };
    UiDataReport {
        revision: rev,
        result,
        checks,
    }
}

fn descriptors(r: &mut Reader, descriptor: bool) -> Result<Vec<Option<Descriptor>>> {
    let n = r.count(1)?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(r.resource(descriptor)?);
    }
    Ok(v)
}

/// Toolkit `s64()`: fixed BE64, or zigzag ULEB when integers are compressed.
fn s64(r: &mut Reader) -> Result<i64> {
    let v = r.u64()?;
    Ok(if r.rev.compression & crate::serial::FLAG_INTEGERS != 0 {
        ((v >> 1) as i64) ^ -((v & 1) as i64)
    } else {
        v as i64
    })
}

fn length(r: &mut Reader) -> Result<usize> {
    let n = r.s32()?;
    usize::try_from(n).or_else(|_| r.invalid("negative string length"))
}

fn str8(r: &mut Reader) -> Result<String> {
    let n = length(r)?;
    Ok(String::from_utf8_lossy(r.bytes(n)?).replace('\0', ""))
}

fn wstr(r: &mut Reader, c: &mut Checks) -> Result<String> {
    let n = length(r)?;
    let bytes = n.checked_mul(2).ok_or(Stop::Budget {
        what: "string length",
        offset: r.pos(),
    })?;
    let units: Vec<u16> = r
        .bytes(bytes)?
        .chunks_exact(2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect();
    let s = String::from_utf16(&units).unwrap_or_else(|_| {
        c.invalid_utf16 += 1;
        String::from_utf16_lossy(&units)
    });
    Ok(s.replace('\0', ""))
}

fn slot_id(r: &mut Reader, c: &mut Checks) -> Result<SlotId> {
    let id = SlotId {
        slot_type: r.u32()?,
        number: r.u32()?,
    };
    if id.type_name().is_none() {
        c.unknown_slot_types += 1;
    }
    Ok(id)
}

fn network_online_id(r: &mut Reader) -> Result<String> {
    let prefixed = r.rev.version() < 0x234;
    if prefixed && r.u32()? != 16 {
        return r.invalid("NetworkOnlineID data length");
    }
    let data = r.bytes(16)?;
    r.i8()?;
    if prefixed && r.u32()? != 3 {
        return r.invalid("NetworkOnlineID dummy length");
    }
    r.bytes(3)?;
    let end = data.iter().position(|b| *b == 0).unwrap_or(16);
    Ok(String::from_utf8_lossy(&data[..end]).into_owned())
}

fn collectabubbles(r: &mut Reader) -> Result<Vec<Collectabubble>> {
    let n = r.count(2)?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(Collectabubble {
            plan: r.resource(true)?,
            count: r.i32()?,
        });
    }
    Ok(v)
}

fn slot(r: &mut Reader, c: &mut Checks) -> Result<Slot> {
    let v = r.rev.version();
    let id = slot_id(r, c)?;
    // ADVENTURE (adventure descriptor) is an LBP3 sub-revision gate; refused above.
    let root = r.resource(true)?;
    let icon = r.resource(true)?;
    let location = r.v4()?;
    let author_id = network_online_id(r)?;
    let author_name = if v >= 0x13b { Some(wstr(r, c)?) } else { None };
    let translation_tag = if v >= 0x183 { Some(str8(r)?) } else { None };
    let name = wstr(r, c)?;
    let description = wstr(r, c)?;
    let primary_link_level = slot_id(r, c)?;
    let group = if v >= 0x134 {
        Some(slot_id(r, c)?)
    } else {
        None
    };
    let initially_locked = r.bool()?;
    let (shareable, background_guid) = if v > 0x237 {
        (Some(r.bool()?), r.guid()?)
    } else {
        (None, None)
    };
    let planet_decorations = if v >= 0x333 { r.resource(true)? } else { None };
    if v < 0x188 {
        r.u8()?;
    }
    let developer_level_type = if v > 0x1de {
        let t = r.u32()?;
        if t > 8 {
            c.unknown_level_types += 1;
        }
        Some(t)
    } else {
        r.bool()?; // SideMission
        None
    };
    if v > 0x1ad && v < 0x1b9 {
        r.u8()?;
    }
    let game_progression_state = if v > 0x1b8 && v < 0x36c {
        Some(r.u32()?)
    } else {
        None
    };
    let mut slot = Slot {
        id,
        root,
        icon,
        location,
        author_id,
        author_name,
        translation_tag,
        name,
        description,
        primary_link_level,
        group,
        initially_locked,
        shareable,
        background_guid,
        planet_decorations,
        developer_level_type,
        game_progression_state,
        labels: Vec::new(),
        collectabubbles_required: Vec::new(),
        collectabubbles_contained: Vec::new(),
        is_sub_level: None,
        min_players: None,
        max_players: None,
        move_recommended: None,
        cross_compatible: None,
        show_on_planet: None,
        lives_override: None,
    };
    if v <= 0x2c3 {
        return Ok(slot);
    }
    if v >= 0x33c {
        let n = r.count(2)?;
        for _ in 0..n {
            slot.labels.push(Label {
                key: r.u32()?,
                order: r.i32()?,
            });
        }
    }
    if v >= 0x2ea {
        slot.collectabubbles_required = collectabubbles(r)?;
    }
    if v >= 0x2f4 {
        slot.collectabubbles_contained = collectabubbles(r)?;
    }
    if v >= 0x352 {
        slot.is_sub_level = Some(r.bool()?);
    }
    if v < 0x3d0 {
        return Ok(slot);
    }
    slot.min_players = Some(r.i8()?);
    slot.max_players = Some(r.i8()?);
    // SLOT_ENFORCE_MINMAX is an LBP3 sub-revision gate; refused above.
    slot.move_recommended = Some(r.bool()?);
    if v >= 0x3e9 {
        slot.cross_compatible = Some(r.bool()?);
    }
    if v >= 0x3d1 {
        slot.show_on_planet = Some(r.bool()?);
    }
    if v >= 0x3d2 {
        slot.lives_override = Some(r.i8()?);
    }
    Ok(slot)
}

fn pin(r: &mut Reader) -> Result<Pin> {
    Ok(Pin {
        id: r.u32()?,
        progress_type: r.u32()?,
        category: r.u32()?,
        title_key: r.u32()?,
        description_key: r.u32()?,
        icon: r.resource(true)?,
        initial_progress: r.u32()?,
        target: r.u32()?,
        trophy_to_unlock: r.i8()?,
        behaviour_flags: r.i16()?,
        trophy_to_unlock_lbp1: if r.rev.version() >= 0x3f7 {
            Some(r.i8()?)
        } else {
            None
        },
    })
}
