//! UI-independent extension hints and bounded A2 header/dependency inspection.
//!
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit ResourceType.java at d6a2ef80b5a09999f08f23a803b516096055e5c9.
//! Extension is a STRONG_INFERENCE about any individual file, never a confirmed type.
use serde::Serialize;
pub mod animated_texture;
pub mod animation;
pub mod binary;
pub mod font;
pub mod material;
mod metadata;
pub mod plan;
pub mod translation;
pub use metadata::{
    inspect, is_protected_prefix, Dependency, Header, Inspection, InspectionLimits, ResourceType,
    DOUBLE11_BRANCH_ID, DOUBLE11_HEADS, DOUBLE11_MAX_BRANCH_REVISION, LBP2_MAX_REVISION,
};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct TypeHint {
    pub name: &'static str,
    pub basis: &'static str,
}

pub fn type_hint(path: &str) -> Option<TypeHint> {
    let (_, extension) = path.rsplit_once('.')?;
    let name = match extension.to_ascii_lowercase().as_str() {
        "tex" => "texture",
        "mol" => "mesh",
        "gmat" => "graphics_material",
        "mat" => "physics_material",
        "anim" => "animation",
        "bin" => "level_candidate",
        "plan" => "plan",
        "smh" => "static_mesh",
        "trans" => "translation",
        "ff" => "script",
        "fpo" => "pixel_shader",
        "vpo" => "vertex_shader",
        "sbu" => "spu_program",
        "fsb" => "audio_bank_candidate",
        "mp3" => "audio_candidate",
        "bik" => "video_candidate",
        "slt" => "slot_list",
        "pck" => "packs",
        "raw" => "raw_bytes",
        _ => return None,
    };
    Some(TypeHint {
        name,
        basis: "extension_inference",
    })
}
