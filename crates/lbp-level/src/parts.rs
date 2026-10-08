//! L5 part readers (subset). Each reader follows the field order of the named Toolkit
//! class at d6a2ef80b5a09999f08f23a803b516096055e5c9 (`structs/things/parts/P*.java`
//! serialize), CONFIRMED_FROM_COMMUNITY_CODE, specialised to the LBP2 band
//! 0x341 <= head <= 0x3f8 with subVersion 0 and no Vita/DOUBLE11 branch fields.
//! Only gates that are open in that band are listed; any other part stops the walk with
//! `Stop::UnsupportedPart` because parts carry no length prefix.
//!
//! - BODY (PBody): posVel v3, angVel f32, frozen i32, editingPlayer reference(Thing).
//! - POS (PPos): thingOfWhichIAmABone reference(Thing), animHash i32, worldPosition m44
//!   (localPosition only below 0x341, so it is absent here).
//! - RENDER_MESH (PRenderMesh): mesh resource, boneThings thingarray, anim resource,
//!   animPos f32, animSpeed f32, animLoop bool, loopStart f32, loopEnd f32, editorColor
//!   i32 (> 0x31a), castShadows i8, RTTEnable bool, visibilityFlags i8 (>= 0x2e3),
//!   poppetRenderScale f32, then parentDistanceFront/Side f32 only below 0x34d.
//! - SHAPE (PShape): Polygon (i32 vertex count, bool requiresZ, v3 or v2 per vertex,
//!   loops intvector), material resource, oldMaterial resource, thickness f32, massDepth
//!   f32, color (v4 up to 0x389, else i32), brightness f32, bevelSize f32, COM m44
//!   (>= 0x38e), behavior i32, colorOff (v4 below 0x38a, else i32), brightnessOff f32,
//!   lethalType (s32 up to 0x345, else u16), soundEnumOverride i32, playerNumberColor
//!   (i32 up to 0x367, else i8), flags (i8 up to 0x345, else i16), ContactCache (array of
//!   {reference(PShape), i8} then bool contactsSorted, bool cacheDirtyButRecomputed),
//!   stickiness/grabbability/grabFilter i8 (>= 0x3bd), colorOpacity/colorOffOpacity i8
//!   (>= 0x3c1), canCollect bool (>= 0x3e2).
//! - GENERATED_MESH (PGeneratedMesh): gfxMaterial resource, bevel resource, uvOffset v4,
//!   planGUID guid, play-mode visibility bool, textureAnimationSpeed/Off f32 (>= 0x305).
//! - GROUP (PGroup): creator NetworkPlayerID (16+1+3+8+8 raw bytes at >= 0x234), plan
//!   resource descriptor (no flags word), emitter reference(Thing), lifetime i32,
//!   aliveFrames i32, flags u8.
//! - REF (PRef): plan resource descriptor (no flags word), oldLifetime i32,
//!   oldAliveFrames i32.
//! - EFFECTOR (PEffector): posVel v3, angVel f32, viscosity f32, density f32, gravity v3,
//!   pushBack bool, swimmable bool, viscosityCheap f32, modScale f32.
//! - SCRIPT_NAME (PScriptName): s32 length then raw bytes.
//! - SPRITE_LIGHT (PSpriteLight): color v4, colorOff v4, multiplier f32, multiplierOff
//!   f32, glowRadius/farDist/sourceSize f32, falloffTexture resource, lookAt
//!   reference(Thing), spotlight bool, (enableFogShadows, enableFog bool below 0x337),
//!   fogAmount f32, onSpeed/offSpeed/flickerProb/flickerAmount f32, behavior i32.
//! - WORLD (PWorld): things arraylist(reference Thing), maxVel f32, maxAVel f32, frame
//!   s32, simFrame s32, frameLevelStarted i32 (>= 0x377), thingUIDCounter i32; the rest
//!   (EditorSelection array onward) is not implemented and stops explicitly.
use crate::serial::{Descriptor, Reader, Result, Tag};
use crate::thing::{thing_array, thing_ref, Things};
use serde::Serialize;

const SUPPORTED: &[&str] = &[
    "BODY",
    "WORLD",
    "RENDER_MESH",
    "POS",
    "GENERATED_MESH",
    "SPRITE_LIGHT",
    "SCRIPT_NAME",
    "SHAPE",
    "EFFECTOR",
    "REF",
    "GROUP",
];

pub fn supported(name: &str) -> bool {
    SUPPORTED.contains(&name)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PartData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<Pos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_mesh: Option<RenderMesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_mesh: Option<GeneratedMesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_ref: Option<Descriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<World>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pos {
    pub bone_of: Option<usize>,
    pub anim_hash: i32,
    /// Serialized element order (translation in 12..14, see serial.rs).
    pub world: [f32; 16],
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderMesh {
    pub mesh: Option<Descriptor>,
    pub bone_things: Vec<Option<usize>>,
    pub anim: Option<Descriptor>,
    pub anim_pos: f32,
    pub anim_speed: f32,
    pub anim_loop: bool,
    pub editor_color: u32,
    pub cast_shadows: i8,
    pub visibility_flags: i8,
    pub poppet_render_scale: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shape {
    pub vertex_count: usize,
    pub requires_z: bool,
    pub vertices: Vec<[f32; 3]>,
    pub loops: Vec<u32>,
    pub material: Option<Descriptor>,
    pub old_material: Option<Descriptor>,
    pub thickness: f32,
    pub mass_depth: f32,
    /// Raw i32 colour (head > 0x389) or the v4 for older heads.
    pub color: ShapeColor,
    pub brightness: f32,
    pub bevel_size: f32,
    pub com: Option<[f32; 16]>,
    pub behavior: u32,
    pub flags: i16,
    pub contacts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ShapeColor {
    Packed(u32),
    Vector([f32; 4]),
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedMesh {
    pub gfx_material: Option<Descriptor>,
    pub bevel: Option<Descriptor>,
    pub uv_offset: [f32; 4],
    pub plan_guid: Option<u32>,
    pub play_mode_visible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub plan: Option<Descriptor>,
    pub emitter: Option<usize>,
    pub flags: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct World {
    pub things: Vec<Option<usize>>,
    pub thing_uid_counter: Option<i32>,
}

/// `reference(P*)` for a present part of Thing `me`.
pub fn part_ref(r: &mut Reader, things: &mut Things, me: usize, name: &'static str) -> Result<&'static str> {
    let tag = r.tag(name, |r| Ok(r.next_other(name)))?;
    match tag {
        Tag::Null => Ok("null"),
        Tag::Seen(_) => Ok("shared"),
        Tag::New(_) => {
            r.enter()?;
            part_body(r, things, Some(me), name)?;
            r.leave();
            Ok("parsed")
        }
    }
}

fn part_body(r: &mut Reader, things: &mut Things, me: Option<usize>, name: &'static str) -> Result<()> {
    let v = r.rev.version();
    let store = |things: &mut Things, f: &dyn Fn(&mut PartData)| {
        if let Some(i) = me {
            f(&mut things.list[i].data);
        }
    };
    match name {
        "BODY" => {
            r.v3()?;
            r.f32()?;
            r.i32()?;
            thing_ref(r, things)?;
        }
        "POS" => {
            let bone_of = thing_ref(r, things)?;
            let anim_hash = r.i32()?;
            let world = r.m44()?;
            let p = Pos {
                bone_of,
                anim_hash,
                world,
            };
            store(things, &|d| d.pos = Some(p.clone()));
        }
        "RENDER_MESH" => {
            let mesh = r.resource(false)?;
            let bone_things = thing_array(r, things)?;
            let anim = r.resource(false)?;
            let anim_pos = r.f32()?;
            let anim_speed = r.f32()?;
            let anim_loop = r.bool()?;
            r.f32()?;
            r.f32()?;
            let editor_color = r.u32()?;
            let cast_shadows = r.i8()?;
            r.bool()?;
            let visibility_flags = r.i8()?;
            let poppet_render_scale = r.f32()?;
            if v < 0x34d {
                r.f32()?;
                r.f32()?;
            }
            let m = RenderMesh {
                mesh,
                bone_things,
                anim,
                anim_pos,
                anim_speed,
                anim_loop,
                editor_color,
                cast_shadows,
                visibility_flags,
                poppet_render_scale,
            };
            store(things, &|d| d.render_mesh = Some(m.clone()));
        }
        "SHAPE" => {
            let s = shape(r, things)?;
            store(things, &|d| d.shape = Some(s.clone()));
        }
        "GENERATED_MESH" => {
            let g = GeneratedMesh {
                gfx_material: r.resource(false)?,
                bevel: r.resource(false)?,
                uv_offset: r.v4()?,
                plan_guid: r.guid()?,
                play_mode_visible: r.bool()?,
            };
            r.f32()?;
            r.f32()?;
            store(things, &|d| d.generated_mesh = Some(g.clone()));
        }
        "GROUP" => {
            r.bytes(36)?;
            let plan = r.resource(true)?;
            let emitter = thing_ref(r, things)?;
            r.i32()?;
            r.i32()?;
            let flags = r.u8()?;
            let g = Group { plan, emitter, flags };
            store(things, &|d| d.group = Some(g.clone()));
        }
        "REF" => {
            let plan = r.resource(true)?;
            r.i32()?;
            r.i32()?;
            store(things, &|d| d.plan_ref = plan);
        }
        "EFFECTOR" => {
            r.v3()?;
            r.f32()?;
            r.f32()?;
            r.f32()?;
            r.v3()?;
            r.bool()?;
            r.bool()?;
            r.f32()?;
            r.f32()?;
        }
        "SCRIPT_NAME" => {
            let n = r.s32()?;
            let n = usize::try_from(n).or_else(|_| r.invalid("negative script name length"))?;
            r.bytes(n)?;
        }
        "SPRITE_LIGHT" => {
            r.v4()?;
            r.v4()?;
            r.f32()?;
            r.f32()?;
            r.f32()?; // glowRadius
            r.f32()?; // farDist
            r.f32()?; // sourceSize
            r.resource(false)?;
            thing_ref(r, things)?;
            r.bool()?;
            if v < 0x337 {
                r.bool()?;
                r.bool()?;
            }
            for _ in 0..5 {
                r.f32()?;
            }
            r.i32()?;
        }
        "WORLD" => {
            let list = thing_array(r, things)?;
            things.world_array_done = Some(list.len());
            r.f32()?;
            r.f32()?;
            r.s32()?;
            r.s32()?;
            if v >= 0x377 {
                r.i32()?;
            }
            let uid = r.i32()?;
            let w = World {
                things: list,
                thing_uid_counter: Some(uid),
            };
            store(things, &|d| d.world = Some(w.clone()));
            return r.unsupported("PWorld tail from selections (EditorSelection) onward");
        }
        _ => return r.unsupported(format!("part reader {name}")),
    }
    Ok(())
}

fn shape(r: &mut Reader, things: &mut Things) -> Result<Shape> {
    let v = r.rev.version();
    let raw = r.u32()? as usize;
    let requires_z = r.bool()?;
    let size = if requires_z { 12 } else { 8 };
    match raw.checked_mul(size) {
        Some(b) if b <= r.remaining() && raw <= r.limits.max_array => {}
        _ => return r.invalid("polygon vertex count exceeds remaining bytes"),
    }
    let mut vertices = Vec::with_capacity(raw);
    for _ in 0..raw {
        vertices.push(if requires_z {
            r.v3()?
        } else {
            let [x, y] = r.v2()?;
            [x, y, 0.0]
        });
    }
    let loops = r.int_vector()?;
    let material = r.resource(false)?;
    let old_material = r.resource(false)?;
    let thickness = r.f32()?;
    let mass_depth = r.f32()?;
    let color = if v <= 0x389 {
        ShapeColor::Vector(r.v4()?)
    } else {
        ShapeColor::Packed(r.u32()?)
    };
    let brightness = r.f32()?;
    let bevel_size = r.f32()?;
    let com = if v >= 0x38e { Some(r.m44()?) } else { None };
    let behavior = r.u32()?;
    if v < 0x38a {
        r.v4()?;
    } else {
        r.u32()?;
    }
    r.f32()?;
    if v <= 0x345 {
        r.s32()?;
    } else {
        r.u16()?;
    }
    r.i32()?;
    if v <= 0x367 {
        r.i32()?;
    } else {
        r.i8()?;
    }
    let flags = if v <= 0x345 { i16::from(r.i8()?) } else { r.i16()? };
    // ContactCache: array(Contact) of {reference(PShape), i8 flags}, then two bools.
    let contacts = r.count(2)?;
    for _ in 0..contacts {
        let tag = r.tag("SHAPE", |r| Ok(r.next_other("SHAPE")))?;
        if let Tag::New(_) = tag {
            // A shape reached only through a contact: read inline, not owned by a Thing.
            r.enter()?;
            part_body(r, things, None, "SHAPE")?;
            r.leave();
        }
        r.i8()?;
    }
    r.bool()?;
    r.bool()?;
    if v >= 0x3bd {
        r.bytes(3)?;
    }
    if v >= 0x3c1 {
        r.bytes(2)?;
    }
    if v >= 0x3e2 {
        r.bool()?;
    }
    Ok(Shape {
        vertex_count: raw,
        requires_z,
        vertices,
        loops,
        material,
        old_material,
        thickness,
        mass_depth,
        color,
        brightness,
        bevel_size,
        com,
        behavior,
        flags,
        contacts,
    })
}
