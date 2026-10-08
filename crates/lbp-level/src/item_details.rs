//! U5: InventoryItemDetails values (the Popit item card: icon, name keys, type bits,
//! location/category, creator, photo and EyeToy data) and the RPlan details tail.
//! UI-independent; built on the L3 serializer core.
//!
//! Independent specification notes (no code copied/translated), written from
//! ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, lib/cwlib/src/main/java/cwlib:
//! `structs/inventory/{InventoryItemDetails,CreationHistory,UserCreatedDetails,
//! InventoryItemPhotoData,PhotoMetadata,PhotoUser,EyetoyData,ColorCorrection}.java`,
//! `structs/profile/{NetworkPlayerID,NetworkOnlineID}.java`, `resources/RPlan.java`,
//! `enums/{InventoryObjectType,GameVersion,ToolType}.java`. CONFIRMED_FROM_COMMUNITY_CODE
//! for the layouts. Two layouts are implemented (head = `Revision::version`):
//! - head > 0x37c: s64 dateAdded (compressed per flag 1), SlotID levelUnlock, guid
//!   highlightSound, i32 colour, i32 type bits, i32 subType, u32 title and description
//!   keys, CreationHistory reference (i32 count + fixed 20-byte names), icon descriptor,
//!   UserCreatedDetails reference (wstr name, wstr description), photo reference, EyeToy
//!   reference, fixed i16 location/category/primary indices, NetworkPlayerID reference,
//!   i8 toolType, i8 flags.
//! - 0x233 <= head <= 0x37c: fixed-width fields (highlight sound, SlotID, three indices,
//!   lastUsed, numUses, a pad word above 0x234, s64 dateAdded, fluffCost, colour, type,
//!   subType, toolType), NetworkPlayerID struct, three bools, i8 flags above 0x334, keys
//!   above 0x2ba else a translation tag, UserCreatedDetails and CreationHistory structs
//!   (wide names), icon descriptor, photo and EyeToy references, a pad byte above 0x358.
//! - 0x197 <= head < 0x233 (early LBP1, U5b): `str` translation tag, fixed i32 location,
//!   category, primary indices and a pad word, fixed type bits, subType and toolType,
//!   icon descriptor, fixed numUses + lastUsed above 0x1c0, fixed highlight sound GUID,
//!   fixed colour, EyeToy reference, photo reference, SlotID (compressed type, fixed
//!   number), copyright bool + NetworkPlayerID struct, UserCreatedDetails above 0x1aa,
//!   CreationHistory (wide names) above 0x1b0, allowEmit above 0x204, fixed s64
//!   dateAdded above 0x221, shareable above 0x222.
//! - head < 0x197 (no plan details; the branches Toolkit keeps for older heads are not
//!   implemented), branches other than 0 and LBP3 are Unsupported.
//!
//! RPlan tail: i32 plan revision (ignored), bytearray thingData (skipped here, D119 reads
//! it), then from head 0x197 the details, then location and category as u32 keys from
//! head 0x2bb, else two `str` tags. Type bits decode with the game-version filter of
//! `InventoryObjectType` (LBP1 at head <= 0x332, else LBP2).
//!
//! The level reader (`parts::inventory_item`) walks the same details layout without
//! keeping values; it is left untouched here (level batch file).
use crate::serial::{Descriptor, Limits, Reader, Result, Revision, Stop, Tag};
use crate::slots::SlotId;
use serde::Serialize;

/// Toolkit `Revisions.PLAN_DETAILS`.
const PLAN_DETAILS: u32 = 0x197;
/// Toolkit `Revisions.LAMS_KEYS`.
const LAMS_KEYS: u32 = 0x2bb;
pub const MAX_INPUT: usize = 16 * 1024 * 1024;
const MAX_EXPANDED: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UserCreated {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Photo {
    pub icon: Option<Descriptor>,
    pub sticker: Option<Descriptor>,
    pub photo: Option<Descriptor>,
    pub level: SlotId,
    pub level_name: String,
    pub users: usize,
    pub painting: Option<Descriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Eyetoy {
    pub frame: Option<Descriptor>,
    pub alpha_mask: Option<Descriptor>,
    pub outline: Option<Descriptor>,
}

/// A reference field: absent (null tag), read here, or a back-reference to an object
/// read earlier in the same resource.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Ref<T> {
    Null,
    Value(T),
    Shared,
}

impl<T> Ref<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Ref::Value(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemDetails {
    /// s64 as stored (decoded from ULEB/zigzag or fixed); not interpreted as a date.
    pub date_added: i64,
    pub level_unlock: SlotId,
    pub highlight_sound: Option<u32>,
    pub colour: u32,
    pub type_bits: u32,
    pub sub_type: u32,
    pub title_key: Option<u32>,
    pub description_key: Option<u32>,
    pub translation_tag: Option<String>,
    pub creation_history: Ref<Vec<String>>,
    pub icon: Option<Descriptor>,
    pub user_created: Ref<UserCreated>,
    pub photo: Ref<Photo>,
    pub eyetoy: Ref<Eyetoy>,
    pub location_index: i32,
    pub category_index: i32,
    pub primary_index: i32,
    /// NetworkOnlineID name of the creator (empty for game content).
    pub creator: Ref<String>,
    pub tool_type: u8,
    pub flags: u8,
    /// Older layout only.
    pub last_used: Option<u32>,
    pub num_uses: Option<u32>,
    pub fluff_cost: Option<u32>,
    pub allow_emit: Option<bool>,
    pub shareable: Option<bool>,
    pub copyright: Option<bool>,
}

/// RPlan location/category after the details: keys, or tags before LAMS keys.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Placement {
    Keys { location: u32, category: u32 },
    Tags { location: String, category: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanDetails {
    pub stored_revision: u32,
    pub thing_data_bytes: usize,
    /// None below head 0x197 (no details in the plan).
    pub details: Option<ItemDetails>,
    pub placement: Option<Placement>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanDetailsReport {
    pub revision: Revision,
    pub result: std::result::Result<PlanDetails, Stop>,
    pub trailing_bytes: usize,
}

/// Reads the details tail of a whole PLNb resource.
pub fn read_plan_details(
    bytes: &[u8],
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> std::result::Result<PlanDetailsReport, lbp_resource::binary::Error> {
    let (h, data) =
        lbp_resource::binary::payload(bytes, b"PLNb", MAX_INPUT, MAX_EXPANDED, &cancel)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    Ok(read_plan_details_payload(&data, rev, limits, cancel))
}

/// Reads the details tail of an expanded RPlan payload at a known revision.
pub fn read_plan_details_payload(
    data: &[u8],
    rev: Revision,
    limits: Limits,
    cancel: &dyn Fn() -> bool,
) -> PlanDetailsReport {
    let mut r = Reader::new(data, rev, limits, cancel);
    let result = (|| {
        check_band(&r)?;
        let stored_revision = r.u32()?;
        let n = size(&mut r)?;
        r.bytes(n)?;
        let v = rev.version();
        if v < PLAN_DETAILS {
            return Ok(PlanDetails {
                stored_revision,
                thing_data_bytes: n,
                details: None,
                placement: None,
            });
        }
        let details = item_details(&mut r)?;
        let placement = if v >= LAMS_KEYS {
            Placement::Keys {
                location: r.u32()?,
                category: r.u32()?,
            }
        } else {
            Placement::Tags {
                location: str8(&mut r)?,
                category: str8(&mut r)?,
            }
        };
        Ok(PlanDetails {
            stored_revision,
            thing_data_bytes: n,
            details: Some(details),
            placement: Some(placement),
        })
    })();
    let trailing_bytes = if result.is_ok() { r.remaining() } else { 0 };
    PlanDetailsReport {
        revision: rev,
        result,
        trailing_bytes,
    }
}

fn check_band(r: &Reader) -> Result<()> {
    let rev = r.rev;
    if rev.branch_id != 0 || rev.head >> 16 != 0 || rev.head > crate::LBP2_MAX {
        return r.unsupported(format!(
            "details branch 0x{:x} / head 0x{:x}",
            rev.branch_id, rev.head
        ));
    }
    Ok(())
}

/// Reads one InventoryItemDetails struct at the reader's revision.
pub fn item_details(r: &mut Reader) -> Result<ItemDetails> {
    check_band(r)?;
    let v = r.rev.version();
    if v > 0x37c {
        modern(r)
    } else if v >= 0x233 {
        middle(r)
    } else if v >= PLAN_DETAILS {
        early(r)
    } else {
        r.unsupported(format!("InventoryItemDetails layout at head 0x{v:x}"))
    }
}

fn modern(r: &mut Reader) -> Result<ItemDetails> {
    let date_added = s64(r)?;
    let level_unlock = SlotId {
        slot_type: r.u32()?,
        number: r.u32()?,
    };
    let highlight_sound = r.guid()?;
    let colour = r.u32()?;
    let type_bits = r.u32()?;
    let sub_type = r.u32()?;
    let title_key = Some(r.u32()?);
    let description_key = Some(r.u32()?);
    let creation_history = reference(r, "CreationHistory", |r| {
        let n = r.count(20)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(fixed_str(r, 20)?);
        }
        Ok(v)
    })?;
    let icon = r.resource(true)?;
    let user_created = reference(r, "UserCreatedDetails", user_created)?;
    let photo = reference(r, "InventoryItemPhotoData", photo)?;
    let eyetoy = reference(r, "EyetoyData", eyetoy)?;
    let location_index = i32::from(r.i16()?);
    let category_index = i32::from(r.i16()?);
    let primary_index = i32::from(r.i16()?);
    let creator = reference(r, "NetworkPlayerID", network_player_id)?;
    let tool_type = r.u8()?;
    let flags = r.u8()?;
    Ok(ItemDetails {
        date_added,
        level_unlock,
        highlight_sound,
        colour,
        type_bits,
        sub_type,
        title_key,
        description_key,
        translation_tag: None,
        creation_history,
        icon,
        user_created,
        photo,
        eyetoy,
        location_index,
        category_index,
        primary_index,
        creator,
        tool_type,
        flags,
        last_used: None,
        num_uses: None,
        fluff_cost: None,
        allow_emit: None,
        shareable: None,
        copyright: None,
    })
}

fn middle(r: &mut Reader) -> Result<ItemDetails> {
    let v = r.rev.version();
    let highlight_sound = Some(r.fixed_u32()?).filter(|g| *g != 0);
    let level_unlock = SlotId {
        slot_type: r.fixed_u32()?,
        number: r.fixed_u32()?,
    };
    let location_index = r.fixed_u32()? as i32;
    let category_index = r.fixed_u32()? as i32;
    let primary_index = r.fixed_u32()? as i32;
    let last_used = Some(r.fixed_u32()?);
    let num_uses = Some(r.fixed_u32()?);
    if v > 0x234 {
        r.fixed_u32()?;
    }
    let date_added = r.fixed_u64()? as i64;
    let fluff_cost = Some(r.fixed_u32()?);
    let colour = r.fixed_u32()?;
    let type_bits = r.fixed_u32()?;
    let sub_type = r.fixed_u32()?;
    let tool_type = r.fixed_u32()? as u8;
    let creator = Ref::Value(network_player_id(r)?);
    let allow_emit = Some(r.bool()?);
    let shareable = Some(r.bool()?);
    let copyright = Some(r.bool()?);
    let flags = if v > 0x334 { r.u8()? } else { 0 };
    let (title_key, description_key, translation_tag) = if v > 0x2ba {
        (Some(r.u32()?), Some(r.u32()?), None)
    } else {
        (None, None, Some(str8(r)?))
    };
    let user_created = Ref::Value(self::user_created(r)?);
    let n = r.count(1)?;
    let mut history = Vec::with_capacity(n);
    for _ in 0..n {
        history.push(wstr(r)?);
    }
    let icon = r.resource(true)?;
    let photo = reference(r, "InventoryItemPhotoData", photo)?;
    let eyetoy = reference(r, "EyetoyData", eyetoy)?;
    if v > 0x358 {
        r.u8()?;
    }
    Ok(ItemDetails {
        date_added,
        level_unlock,
        highlight_sound,
        colour,
        type_bits,
        sub_type,
        title_key,
        description_key,
        translation_tag,
        creation_history: Ref::Value(history),
        icon,
        user_created,
        photo,
        eyetoy,
        location_index,
        category_index,
        primary_index,
        creator,
        tool_type,
        flags,
        last_used,
        num_uses,
        fluff_cost,
        allow_emit,
        shareable,
        copyright,
    })
}

fn early(r: &mut Reader) -> Result<ItemDetails> {
    let v = r.rev.version();
    let translation_tag = Some(str8(r)?);
    let location_index = r.fixed_u32()? as i32;
    let category_index = r.fixed_u32()? as i32;
    let primary_index = r.fixed_u32()? as i32;
    r.fixed_u32()?; // pad
    let type_bits = r.fixed_u32()?;
    let sub_type = r.fixed_u32()?;
    let tool_type = r.fixed_u32()? as u8;
    let icon = r.resource(true)?;
    let (num_uses, last_used) = if v > 0x1c0 {
        (Some(r.fixed_u32()?), Some(r.fixed_u32()?))
    } else {
        (None, None)
    };
    let highlight_sound = Some(r.fixed_u32()?).filter(|g| *g != 0);
    let colour = r.fixed_u32()?;
    let eyetoy = reference(r, "EyetoyData", eyetoy)?;
    let photo = reference(r, "InventoryItemPhotoData", photo)?;
    let level_unlock = SlotId {
        slot_type: r.u32()?,
        number: r.fixed_u32()?,
    };
    let copyright = Some(r.bool()?);
    let creator = Ref::Value(network_player_id(r)?);
    let user_created = if v > 0x1aa {
        Ref::Value(self::user_created(r)?)
    } else {
        Ref::Null
    };
    let creation_history = if v > 0x1b0 {
        let n = r.count(1)?;
        let mut history = Vec::with_capacity(n);
        for _ in 0..n {
            history.push(wstr(r)?);
        }
        Ref::Value(history)
    } else {
        Ref::Null
    };
    let allow_emit = if v > 0x204 { Some(r.bool()?) } else { None };
    let date_added = if v > 0x221 { r.fixed_u64()? as i64 } else { 0 };
    let shareable = if v > 0x222 { Some(r.bool()?) } else { None };
    Ok(ItemDetails {
        date_added,
        level_unlock,
        highlight_sound,
        colour,
        type_bits,
        sub_type,
        title_key: None,
        description_key: None,
        translation_tag,
        creation_history,
        icon,
        user_created,
        photo,
        eyetoy,
        location_index,
        category_index,
        primary_index,
        creator,
        tool_type,
        flags: 0,
        last_used,
        num_uses,
        fluff_cost: None,
        allow_emit,
        shareable,
        copyright,
    })
}

fn reference<T>(
    r: &mut Reader,
    class: &'static str,
    body: impl FnOnce(&mut Reader) -> Result<T>,
) -> Result<Ref<T>> {
    match r.tag(class, |r| Ok(r.next_other(class)))? {
        Tag::Null => Ok(Ref::Null),
        Tag::Seen(_) => Ok(Ref::Shared),
        Tag::New(_) => {
            r.enter()?;
            let v = body(r);
            r.leave();
            Ok(Ref::Value(v?))
        }
    }
}

fn user_created(r: &mut Reader) -> Result<UserCreated> {
    Ok(UserCreated {
        name: wstr(r)?,
        description: wstr(r)?,
    })
}

fn photo(r: &mut Reader) -> Result<Photo> {
    let icon = r.resource(true)?;
    let sticker = r.resource(true)?;
    let photo = r.resource(true)?;
    let level = SlotId {
        slot_type: r.u32()?,
        number: r.u32()?,
    };
    let level_name = wstr(r)?;
    r.sha1()?;
    s64(r)?;
    let users = r.count(1)?;
    for _ in 0..users {
        r.bytes(20)?;
        wstr(r)?;
        r.v4()?;
    }
    let painting = if r.rev.version() > 0x3c7 {
        r.resource(true)?
    } else {
        None
    };
    Ok(Photo {
        icon,
        sticker,
        photo,
        level,
        level_name,
        users,
        painting,
    })
}

fn eyetoy(r: &mut Reader) -> Result<Eyetoy> {
    // EyetoyData is empty below head 0x15e; never reached in the implemented band.
    let frame = r.resource(false)?;
    let alpha_mask = r.resource(false)?;
    r.m44()?;
    r.skip_f32(6)?; // ColorCorrection
    let outline = if r.rev.version() > 0x39f {
        r.resource(false)?
    } else {
        None
    };
    Ok(Eyetoy {
        frame,
        alpha_mask,
        outline,
    })
}

/// NetworkPlayerID: NetworkOnlineID (16 name bytes, i8, 3 bytes; length-prefixed below
/// 0x234) then 8 + 8 bytes (also length-prefixed below 0x234). Returns the name.
fn network_player_id(r: &mut Reader) -> Result<String> {
    let prefixed = r.rev.version() < 0x234;
    let run = |r: &mut Reader, n: usize| -> Result<Vec<u8>> {
        if prefixed && r.u32()? as usize != n {
            return r.invalid("NetworkPlayerID run length");
        }
        Ok(r.bytes(n)?.to_vec())
    };
    let name = run(r, 16)?;
    r.i8()?;
    run(r, 3)?;
    run(r, 8)?;
    run(r, 8)?;
    let end = name.iter().position(|b| *b == 0).unwrap_or(16);
    Ok(String::from_utf8_lossy(&name[..end]).into_owned())
}

fn s64(r: &mut Reader) -> Result<i64> {
    let v = r.u64()?;
    Ok(if r.rev.compression & crate::serial::FLAG_INTEGERS != 0 {
        ((v >> 1) as i64) ^ -((v & 1) as i64)
    } else {
        v as i64
    })
}

fn size(r: &mut Reader) -> Result<usize> {
    let n = r.i32()?;
    usize::try_from(n).or_else(|_| r.invalid("negative size"))
}

fn str8(r: &mut Reader) -> Result<String> {
    let n = r.s32()?;
    let n = usize::try_from(n).or_else(|_| r.invalid("negative string length"))?;
    Ok(String::from_utf8_lossy(r.bytes(n)?).replace('\0', ""))
}

fn fixed_str(r: &mut Reader, n: usize) -> Result<String> {
    Ok(String::from_utf8_lossy(r.bytes(n)?).replace('\0', ""))
}

fn wstr(r: &mut Reader) -> Result<String> {
    let n = r.s32()?;
    let n = usize::try_from(n).or_else(|_| r.invalid("negative string length"))?;
    let bytes = n.checked_mul(2).ok_or(Stop::Budget {
        what: "string length",
        offset: r.pos(),
    })?;
    let units: Vec<u16> = r
        .bytes(bytes)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_be_bytes(*b))
        .collect();
    Ok(String::from_utf16_lossy(&units).replace('\0', ""))
}

/// `InventoryObjectType` names set in `bits`, with Toolkit's game-version filter
/// (LBP1 at head <= 0x332, LBP2 above; bits mean different things per game).
pub fn type_names(bits: u32, head: u32) -> Vec<&'static str> {
    const ALL: u8 = 7;
    const LBP1: u8 = 1;
    const LBP2: u8 = 2;
    const L23: u8 = 6;
    const TABLE: [(&str, u32, u8); 46] = [
        ("primitive_material", 0, ALL),
        ("readymade", 1, ALL),
        ("decoration", 2, ALL),
        ("sticker", 3, ALL),
        ("costume", 4, ALL),
        ("costume_material", 5, ALL),
        ("joint", 6, ALL),
        ("user_object", 7, ALL),
        ("background", 8, ALL),
        ("gameplay_kit", 9, ALL),
        ("user_sticker", 10, ALL),
        ("primitive_shape", 11, ALL),
        ("sequencer", 12, L23),
        ("danger", 13, ALL),
        ("eyetoy", 14, ALL),
        ("gadget", 15, ALL),
        ("tool", 16, ALL),
        ("sackbot_mesh", 17, 4),
        ("creatures_characters", 18, 4),
        ("player_colour", 19, ALL),
        ("user_costume", 20, ALL),
        ("music", 21, ALL),
        ("sound", 22, ALL),
        ("photobooth", 23, ALL),
        ("user_planet", 24, L23),
        ("level_key", 25, L23),
        ("emitted_item", 26, L23),
        ("gun_item", 27, L23),
        ("npc_costume", 28, L23),
        ("instrument", 29, L23),
        ("user_pod", 30, L23),
        ("costume_tweaker_tool", 31, 4),
        ("paint", 12, LBP1),
        ("flood_fill", 17, LBP1),
        ("sticker_tool", 18, LBP1),
        ("costume_tool", 24, LBP1),
        ("plan_tool", 25, LBP1),
        ("photo_tool", 26, LBP1),
        ("picture_tools", 27, LBP1),
        ("com_photo_tools", 28, LBP1),
        ("com_object_tools", 29, LBP1),
        ("user_pod_lbp1", 30, LBP1),
        ("pod_tool_lbp1", 31, LBP1),
        ("edit_mode_tool", 17, LBP2),
        ("pod_tool_lbp2", 18, LBP2),
        ("earth_tool", 31, LBP2),
    ];
    let game = if head & 0xffff <= 0x332 { LBP1 } else { LBP2 };
    TABLE
        .iter()
        .filter(|(_, bit, games)| games & game != 0 && bits & (1 << bit) != 0)
        .map(|(n, _, _)| *n)
        .collect()
}
