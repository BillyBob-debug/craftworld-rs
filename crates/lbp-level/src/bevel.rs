//! D122 (plan item Lv): bounded RBevel (BEVb) reader.
//!
//! Independent specification notes (no code copied or translated), written from
//! ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9, `resources/RBevel.java`
//! serialize, `structs/bevel/BevelVertex.java` serialize, `enums/MappingMode.java` and
//! `io/serializer/Serializer.java` `resource(value, type)`. Evidence label:
//! CONFIRMED_FROM_COMMUNITY_CODE for the layout, CONFIRMED_FROM_LOCAL_DATA for the D122
//! oracle parity on every 01.33 bevel (docs/d122-shape-bevels.md). Layout has no
//! revision gates:
//!
//! - i32 material count (Toolkit writes 4 and ignores the value when reading; this reader
//!   requires 4: strictness conflict, reported as Unsupported), then 4 resource
//!   descriptors (gfx materials, slots 0-3);
//! - i32 UV-scale count (also 4), then 4 f32 UV scales;
//! - f32 autoSmoothCutoffAngle;
//! - arraylist BevelVertex: i32 count, then per vertex f32 y, f32 z, f32 rigidity,
//!   i8 smoothWithPrevious, i8 gmatSlot, u8 mappingMode (HIDDEN 0, DAVE 1, CYLINDER_01 2,
//!   CYLINDER 3, PLANARXZ_01 4, PLANARXZ 5; Toolkit maps other values to null, kept raw
//!   here);
//! - bool includeBackface, bool smoothWithFront, f32 relaxStrength, f32 subDivRadius,
//!   f32 fixedBevelSize, bool spongy, resource descriptor softPhysicsSettings,
//!   f32 textureRepeats.
//!
//! Resource descriptors (`Serializer.resource` with isDescriptor=false, class pointer):
//! for head > 0x22e an i32 flags word; then an i8 kind: bit GUID and bit HASH, where
//! GUID = 2 and HASH = 1 from head 0x191, swapped (GUID = 1, HASH = 2) below 0x191
//! (local bevels exist at head 0x132). A zero GUID counts as absent and an all-zero SHA1
//! as invalid; a descriptor with neither is None (Toolkit `isValid`).
//!
//! Integers follow the compression flags (`serial`); f32 are always plain BE32. Every
//! count is bounded against the remaining bytes before allocation. Trailing bytes after
//! textureRepeats are reported, not rejected (Toolkit does not check them).
use crate::serial::{Descriptor, Limits, Reader, Revision, Stop};
use serde::Serialize;

pub const MAX_INPUT: usize = 64 * 1024;
pub const MAX_EXPANDED: usize = 256 * 1024;
/// Profile vertices per bevel (observed maximum on 01.33: 15, D108 B3).
pub const MAX_BEVEL_VERTICES: usize = 256;
pub const SLOTS: usize = 4;

/// BevelVertex.mappingMode values (Toolkit `MappingMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MappingMode {
    Hidden,
    Dave,
    Cylinder01,
    Cylinder,
    Planarxz01,
    Planarxz,
}

impl MappingMode {
    pub fn from_raw(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Hidden,
            1 => Self::Dave,
            2 => Self::Cylinder01,
            3 => Self::Cylinder,
            4 => Self::Planarxz01,
            5 => Self::Planarxz,
            _ => return None,
        })
    }
    /// Toolkit enum constant name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Hidden => "HIDDEN",
            Self::Dave => "DAVE",
            Self::Cylinder01 => "CYLINDER_01",
            Self::Cylinder => "CYLINDER",
            Self::Planarxz01 => "PLANARXZ_01",
            Self::Planarxz => "PLANARXZ",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BevelVertex {
    /// Radial offset of the outline in bevel-size units (sign convention: docs/d122).
    pub y: f32,
    /// Normalised depth, about -1 (back) to +1 (front).
    pub z: f32,
    pub rigidity: f32,
    pub smooth_with_previous: i8,
    pub gmat_slot: i8,
    pub mapping_mode_raw: u8,
}

impl BevelVertex {
    pub fn mapping_mode(&self) -> Option<MappingMode> {
        MappingMode::from_raw(self.mapping_mode_raw)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bevel {
    pub revision: Revision,
    pub materials: [Option<Descriptor>; SLOTS],
    pub uv_scales: [f32; SLOTS],
    pub auto_smooth_cutoff_angle: f32,
    pub vertices: Vec<BevelVertex>,
    pub include_backface: bool,
    pub smooth_with_front: bool,
    pub relax_strength: f32,
    pub sub_div_radius: f32,
    pub fixed_bevel_size: f32,
    pub spongy: bool,
    pub soft_physics_settings: Option<Descriptor>,
    pub texture_repeats: f32,
    pub consumed_bytes: usize,
    pub trailing_bytes: usize,
}

#[derive(Debug)]
pub enum BevelError {
    Wrapper(lbp_resource::binary::Error),
    Payload(Stop),
}

impl std::fmt::Display for BevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wrapper(e) => write!(f, "bevel wrapper: {e}"),
            Self::Payload(s) => write!(f, "bevel payload: {}", s.label()),
        }
    }
}

impl std::error::Error for BevelError {}

/// Reads a whole BEVb resource (wrapper via `lbp_resource::binary::payload`).
pub fn read(bytes: &[u8], cancel: &dyn Fn() -> bool) -> Result<Bevel, BevelError> {
    let (h, data) = lbp_resource::binary::payload(bytes, b"BEVb", MAX_INPUT, MAX_EXPANDED, &cancel)
        .map_err(BevelError::Wrapper)?;
    let rev = Revision {
        head: h.revision,
        branch_id: h.branch_id.unwrap_or(0),
        branch_revision: h.branch_revision.unwrap_or(0),
        compression: h.compression_flags.unwrap_or(0),
    };
    read_payload(&data, rev, cancel).map_err(BevelError::Payload)
}

fn descriptor(r: &mut Reader<'_>) -> crate::serial::Result<Option<Descriptor>> {
    let v = r.rev.version();
    let (guid_bit, hash_bit) = if v < 0x191 { (1u8, 2u8) } else { (2u8, 1u8) };
    let flags = if v > 0x22e { Some(r.u32()?) } else { None };
    let kind_byte = r.u8()?;
    if kind_byte == 0 {
        return Ok(None);
    }
    let guid = if kind_byte & guid_bit != 0 {
        r.guid()?
    } else {
        None
    };
    let sha1 = if kind_byte & hash_bit != 0 {
        Some(r.sha1()?).filter(|h| h.iter().any(|&b| b != 0))
    } else {
        None
    };
    if guid.is_none() && sha1.is_none() {
        return Ok(None);
    }
    Ok(Some(Descriptor {
        flags,
        kind_byte,
        guid,
        sha1,
    }))
}

/// Reads an expanded RBevel payload with a known revision.
pub fn read_payload(data: &[u8], rev: Revision, cancel: &dyn Fn() -> bool) -> Result<Bevel, Stop> {
    let mut r = Reader::new(data, rev, Limits::default(), cancel);
    if rev.head > crate::LBP2_MAX {
        return r.unsupported(format!("RBevel at head 0x{:x}", rev.head));
    }
    if rev.branch_id == lbp_resource::DOUBLE11_BRANCH_ID {
        return r.unsupported("DOUBLE11 branch");
    }
    r.check_cancel()?;
    if r.i32()? != SLOTS as i32 {
        return r.unsupported("bevel material count other than 4");
    }
    let mut materials = [None; SLOTS];
    for m in &mut materials {
        *m = descriptor(&mut r)?;
    }
    if r.i32()? != SLOTS as i32 {
        return r.unsupported("bevel UV scale count other than 4");
    }
    let mut uv_scales = [0f32; SLOTS];
    for s in &mut uv_scales {
        *s = r.f32()?;
    }
    let auto_smooth_cutoff_angle = r.f32()?;
    // 3 f32 + 3 bytes per vertex.
    let n = r.count(15)?;
    if n > MAX_BEVEL_VERTICES {
        return r.budget("bevel vertex count");
    }
    let mut vertices = Vec::with_capacity(n);
    for _ in 0..n {
        vertices.push(BevelVertex {
            y: r.f32()?,
            z: r.f32()?,
            rigidity: r.f32()?,
            smooth_with_previous: r.i8()?,
            gmat_slot: r.i8()?,
            mapping_mode_raw: r.u8()?,
        });
    }
    let include_backface = r.bool()?;
    let smooth_with_front = r.bool()?;
    let relax_strength = r.f32()?;
    let sub_div_radius = r.f32()?;
    let fixed_bevel_size = r.f32()?;
    let spongy = r.bool()?;
    let soft_physics_settings = descriptor(&mut r)?;
    let texture_repeats = r.f32()?;
    Ok(Bevel {
        revision: rev,
        materials,
        uv_scales,
        auto_smooth_cutoff_angle,
        vertices,
        include_backface,
        smooth_with_front,
        relax_strength,
        sub_div_radius,
        fixed_bevel_size,
        spongy,
        soft_physics_settings,
        texture_repeats,
        consumed_bytes: r.pos(),
        trailing_bytes: r.remaining(),
    })
}
