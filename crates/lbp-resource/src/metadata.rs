//! Format rules: CONFIRMED_FROM_COMMUNITY_CODE, corroborated on BCUS98245 01.00.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9
//! cwlib/types/SerializedResource.java::process/processDependencies,
//! cwlib/enums/{ResourceType,SerializationType,Branch,Revisions}.java.
//! Independent layout implementation, MIT knowledge notice in THIRD-PARTY-NOTICES.
//! Scope: LBP1/2 heads <=0x3f8 (Revisions.LBP2_MAX; header layout unchanged after 0x297),
//! locally observed through 1016 with update 01.33. Ordinary binary branches 0/LD, plus
//! DOUBLE11 0x4431 header/dependencies (D77, Vita heads only); SMH common header
//! and dependency table only. No payload decompression, extra ALSR interpretation,
//! texture/static-mesh payload parsing or encryption. Tests use synthetic recipes;
//! local metadata compared with unmodified pinned Toolkit (see docs/scanner.md).
use lbp_core::{ByteReader, ContentHash, Error, ErrorKind, Guid, Result};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};

/// Last LBP2 head (Toolkit `Revisions.LBP2_MAX`); BCUS98245 01.33 tops out here.
pub const LBP2_MAX_REVISION: u32 = 0x3f8;
/// DOUBLE11 (Vita) branch, D77. CONFIRMED_FROM_COMMUNITY_CODE: Toolkit `Branch.DOUBLE11`
/// ID 0x4431; `Revision.isVita` heads 0x3c1..=0x3e2; `Revisions.D1_MAX` = 0x87.
/// `SerializedResource.process` reads the same branch/flags/compressed fields for every
/// branch at head >= 0x297. Header and dependency table only (docs/d77-branch-4431.md).
pub const DOUBLE11_BRANCH_ID: u16 = 0x4431;
pub const DOUBLE11_HEADS: std::ops::RangeInclusive<u32> = 0x3c1..=0x3e2;
pub const DOUBLE11_MAX_BRANCH_REVISION: u16 = 0x87;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ResourceType {
    pub id: u32,
    pub name: &'static str,
}
// IDs with no three-byte wrapper are intentionally not guessed from payload bytes.
const TYPES: &[(&str, u32, &str)] = &[
    ("TEX", 1, "texture"),
    ("GTF", 1, "texture"),
    ("MSH", 2, "mesh"),
    ("ANM", 5, "animation"),
    ("GSB", 6, "guid_substitution"),
    ("GMT", 7, "graphics_material"),
    ("LVL", 9, "level"),
    ("FSH", 11, "script"),
    ("CHA", 12, "character_settings"),
    ("SSP", 14, "soft_physics_settings"),
    ("FNT", 15, "fontface"),
    ("MAT", 16, "physics_material"),
    ("DLC", 17, "downloadable_content"),
    ("JNT", 19, "joint"),
    ("CON", 20, "game_constants"),
    ("POP", 21, "poppet_settings"),
    ("CLD", 22, "cached_level_data"),
    ("PRF", 23, "synced_profile"),
    ("BEV", 24, "bevel"),
    ("GAM", 25, "game"),
    ("NWS", 26, "network_settings"),
    ("PCK", 27, "packs"),
    ("BPR", 28, "big_profile"),
    ("SLT", 29, "slot_list"),
    ("ADC", 31, "adventure_create_profile"),
    ("IPR", 32, "local_profile"),
    ("LMT", 33, "limits_settings"),
    ("TUT", 34, "tutorials"),
    ("GLT", 35, "guid_list"),
    ("AUM", 36, "audio_materials"),
    ("SSF", 37, "fluid_settings"),
    ("PLN", 38, "plan"),
    ("TXL", 39, "texture_list"),
    ("MUS", 40, "music_settings"),
    ("MIX", 41, "mixer_settings"),
    ("REP", 42, "replay_config"),
    ("PAL", 43, "palette"),
    ("SMH", 44, "static_mesh"),
    ("ATX", 45, "animated_texture"),
    ("VOP", 46, "voip_recording"),
    ("PIN", 47, "pins"),
    ("INS", 48, "instrument"),
    ("OUT", 50, "outfit_list"),
    ("PBR", 51, "paint_brush"),
    ("REC", 52, "thing_recording"),
    ("PTG", 53, "painting"),
    ("QST", 54, "quest"),
    ("ABK", 55, "animation_bank"),
    ("AST", 56, "animation_set"),
    ("SMP", 57, "skeleton_map"),
    ("SRG", 58, "skeleton_registry"),
    ("SAS", 59, "skeleton_animation_styles"),
    ("CHK", 61, "streaming_chunk"),
    ("ADS", 62, "adventure_shared_data"),
    ("ADP", 63, "adventure_play_profile"),
    ("AMP", 64, "animation_map"),
    ("CCD", 65, "cached_costume_data"),
    ("DLA", 66, "data_labels"),
    ("ADM", 67, "adventure_maps"),
    ("BST", 128, "toolkit_custom_bone_set"),
];
fn from_magic(magic: &[u8]) -> Option<ResourceType> {
    TYPES
        .iter()
        // Canonical uppercase signatures avoid interpreting plain "game" as GAMe.
        // Toolkit accepts lowercase too; that permissive behavior is not adopted.
        .find(|(m, _, _)| magic == m.as_bytes())
        .map(|(_, id, name)| ResourceType { id: *id, name })
}
fn from_id(id: u32) -> Option<ResourceType> {
    if let Some((_, _, name)) = TYPES.iter().find(|(_, i, _)| *i == id) {
        return Some(ResourceType { id, name });
    }
    let name = match id {
        3 => "pixel_shader",
        4 => "vertex_shader",
        8 => "spu_program",
        10 => "filename",
        13 => "file_of_bytes",
        18 => "editor_settings",
        30 => "translation",
        49 => "sample",
        60 => "crossplay_vita",
        _ => return None,
    };
    Some(ResourceType { id, name })
}
#[derive(Debug, Clone, Copy)]
pub struct InspectionLimits {
    pub max_metadata_bytes: usize,
    pub max_dependencies: usize,
}
impl Default for InspectionLimits {
    fn default() -> Self {
        Self {
            max_metadata_bytes: 1024 * 1024,
            max_dependencies: 65536,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Header {
    pub revision: u32,
    pub branch_id: Option<u16>,
    pub branch_revision: Option<u16>,
    pub compression_flags: Option<u8>,
    pub compressed: Option<bool>,
    pub dependency_offset: Option<u32>,
    pub payload_offset: Option<u32>,
    pub scope: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct Dependency {
    pub index: usize,
    pub flags: u8,
    pub guid: Option<Guid>,
    pub sha1: Option<ContentHash>,
    pub type_id: u32,
    pub known_type: Option<ResourceType>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Inspection {
    pub status: &'static str,
    pub reason: Option<String>,
    pub magic_hex: String,
    pub confirmed_type: Option<ResourceType>,
    pub serialization: &'static str,
    pub header: Option<Header>,
    pub dependency_status: &'static str,
    pub dependency_count_raw: Option<u32>,
    pub dependencies: Vec<Dependency>,
    pub metadata_bytes_read: usize,
}
struct Reader<'a, R> {
    source: &'a mut R,
    base: u64,
    size: u64,
    budget: usize,
    read: usize,
}
impl<R: Read + Seek> Reader<'_, R> {
    fn bytes(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        if offset
            .checked_add(size as u64)
            .is_none_or(|end| end > self.size)
        {
            return Err(Error::malformed("metadata range outside resource"));
        }
        let total = self
            .read
            .checked_add(size)
            .ok_or_else(|| Error::limit("metadata budget overflow"))?;
        if total > self.budget {
            return Err(Error::limit("resource metadata byte budget exceeded"));
        }
        let absolute = self
            .base
            .checked_add(offset)
            .ok_or_else(|| Error::malformed("resource offset overflow"))?;
        absolute
            .checked_add(size as u64)
            .ok_or_else(|| Error::malformed("absolute metadata range overflow"))?;
        self.source.seek(SeekFrom::Start(absolute))?;
        let mut bytes = vec![0; size];
        self.source.read_exact(&mut bytes)?;
        self.read = total;
        Ok(bytes)
    }
}
/// Inspect exactly a resource range in a seekable input. All reads stay in that
/// range, with a separate metadata budget. Per-resource failures become diagnostics.
pub fn inspect<R: Read + Seek>(
    source: &mut R,
    base: u64,
    size: u64,
    limits: InspectionLimits,
) -> Inspection {
    let mut out = Inspection {
        status: "unknown_magic",
        reason: None,
        magic_hex: String::new(),
        confirmed_type: None,
        serialization: "unknown",
        header: None,
        dependency_status: "not_inspected",
        dependency_count_raw: None,
        dependencies: Vec::new(),
        metadata_bytes_read: 0,
    };
    let mut reader = Reader {
        source,
        base,
        size,
        budget: limits.max_metadata_bytes,
        read: 0,
    };
    if let Err(e) = parse(&mut reader, limits, &mut out) {
        out.status = match e.kind {
            ErrorKind::Io => "io_error",
            ErrorKind::Malformed => "malformed",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::LimitExceeded => "limit_exceeded",
        };
        out.reason = Some(e.message);
        out.dependencies.clear(); // never expose a partial table as a complete one
        out.dependency_status = "not_parsed";
    }
    out.metadata_bytes_read = reader.read;
    out
}
fn parse<R: Read + Seek>(
    reader: &mut Reader<R>,
    limits: InspectionLimits,
    out: &mut Inspection,
) -> Result<()> {
    let prefix = reader.bytes(0, reader.size.min(4) as usize)?;
    out.magic_hex = prefix.iter().map(|b| format!("{b:02x}")).collect();
    if prefix.len() < 4 {
        return Err(Error::malformed("resource shorter than four-byte magic"));
    }
    out.confirmed_type = from_magic(&prefix[..3]);
    if is_protected_prefix(&prefix) {
        out.status = "blocked_encrypted";
        out.serialization = "protected";
        out.reason = Some("protected marker; no further reads or decryption".into());
        out.confirmed_type = from_magic(&prefix[..3]);
        return Ok(());
    }
    out.confirmed_type = from_magic(&prefix[..3]);
    let Some(kind) = out.confirmed_type else {
        return Ok(());
    };
    out.serialization = match prefix[3] {
        b'b' => "binary",
        b't' => "text",
        b' ' => "compressed_texture",
        b's' => "gtf_swizzled",
        b'S' => "gxt_swizzled",
        _ => "unknown",
    };
    if out.serialization == "unknown" {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "unknown serialization marker",
        ));
    }
    if prefix[3] != b'b' {
        out.status = "identified_unparsed";
        out.dependency_status = "not_applicable_to_wrapper";
        return Ok(());
    }
    let revision = ByteReader::new(&reader.bytes(4, 4)?).be_u32()?;
    let mut header = Header {
        revision,
        branch_id: None,
        branch_revision: None,
        compression_flags: None,
        compressed: None,
        dependency_offset: None,
        payload_offset: None,
        scope: "ordinary_binary",
    };
    if revision > LBP2_MAX_REVISION || kind.id == 128 {
        out.header = Some(header);
        return Err(Error::new(
            ErrorKind::Unsupported,
            "revision/custom resource outside tested LBP1/2 header scope",
        ));
    }
    // FNTb (01.33 icon_patch_font.fnt, head 985): the word at +8 does not lead to a
    // plausible dependency table (count 4292876038), so fonts are not ordinary
    // serialized resources here. CONFIRMED_FROM_LOCAL_DATA, one sample.
    if kind.id == 15 {
        out.header = Some(header);
        return Err(Error::new(
            ErrorKind::Unsupported,
            "font face layout is not an ordinary resource header",
        ));
    }
    let offset = if revision >= 0x109 {
        Some(ByteReader::new(&reader.bytes(8, 4)?).be_u32()?)
    } else {
        None
    };
    header.dependency_offset = offset;
    let mut position = if offset.is_some() { 12 } else { 8 };
    if kind.id == 44 && revision >= 0x189 {
        header.scope = "static_mesh_common_header_only";
    } else {
        header.branch_id = Some(0);
        header.branch_revision = Some(0);
        let mut compressed = revision >= 0x132;
        let mut flags = 0;
        if revision >= 0x189 {
            if revision >= 0x271 {
                let bytes = reader.bytes(position, 4)?;
                position += 4;
                header.branch_id = Some(u16::from_be_bytes(bytes[..2].try_into().unwrap()));
                header.branch_revision = Some(u16::from_be_bytes(bytes[2..].try_into().unwrap()));
            }
            if revision >= 0x297
                || (revision == 0x272
                    && header.branch_id == Some(0x4c44)
                    && header.branch_revision >= Some(2))
            {
                flags = reader.bytes(position, 1)?[0];
                position += 1;
            }
            let boolean = reader.bytes(position, 1)?[0];
            position += 1;
            if boolean > 1 {
                return Err(Error::malformed("compression boolean is not 0 or 1"));
            }
            compressed = boolean != 0;
        }
        header.compression_flags = Some(flags);
        header.compressed = Some(compressed);
        header.payload_offset = Some(position as u32);
        if header.branch_id == Some(DOUBLE11_BRANCH_ID) {
            let in_head = DOUBLE11_HEADS.contains(&revision);
            let in_branch = header.branch_revision <= Some(DOUBLE11_MAX_BRANCH_REVISION);
            if !(in_head && in_branch) {
                out.header = Some(header);
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    if in_head {
                        "branch 0x4431 revision above Toolkit D1_MAX"
                    } else {
                        "branch 0x4431 head outside Toolkit Vita range 0x3c1..=0x3e2"
                    },
                ));
            }
            // Header layout matches ordinary heads >= 0x297; payloads stay branch-0 only.
            header.scope = "double11_branch_header";
        } else if header.branch_id != Some(0)
            && !(revision == 0x272 && header.branch_id == Some(0x4c44))
        {
            out.header = Some(header);
            return Err(Error::new(
                ErrorKind::Unsupported,
                "branch outside tested LBP1/2 scope",
            ));
        }
    }
    out.header = Some(header);
    let Some(offset) = offset else {
        out.status = "metadata_parsed";
        out.dependency_status = "table_absent_in_revision";
        return Ok(());
    };
    if u64::from(offset) < position {
        return Err(Error::malformed("dependency table overlaps header"));
    }
    let count = ByteReader::new(&reader.bytes(offset as u64, 4)?).be_u32()?;
    out.dependency_count_raw = Some(count);
    if count as usize > limits.max_dependencies {
        return Err(Error::limit("dependency entry budget exceeded"));
    }
    let start = u64::from(offset) + 4;
    let remaining = reader.size - start;
    if u64::from(count) * 5 > remaining {
        return Err(Error::malformed(
            "dependency count exceeds minimum table capacity",
        ));
    }
    let length = remaining.min(u64::from(count) * 29) as usize;
    let bytes = reader.bytes(start, length)?;
    let mut table = ByteReader::new(&bytes);
    let mut dependencies = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let flags = table.take(1)?[0];
        if flags & !3 != 0 {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "unknown dependency flag bits",
            ));
        }
        let guid = if flags & 2 != 0 {
            Some(Guid(table.be_u32()?))
        } else {
            None
        };
        let sha1 = if flags & 1 != 0 {
            Some(table.hash()?)
        } else {
            None
        };
        let type_id = table.be_u32()?;
        dependencies.push(Dependency {
            index,
            flags,
            guid,
            sha1,
            type_id,
            known_type: from_id(type_id),
        });
    }
    out.dependencies = dependencies;
    out.dependency_status = "parsed";
    out.status = if kind.id == 44 && revision >= 0x189 {
        "metadata_partial_static_mesh"
    } else {
        "metadata_parsed"
    };
    Ok(())
}

/// Existing A2 boundary rule, shared with read-only B2 byte inspection.
/// Known encrypted wrappers or protected container signatures stop after the prefix.
/// Accepts longer preflight buffers too; only the first four bytes are classified.
pub fn is_protected_prefix(prefix: &[u8]) -> bool {
    let Some(prefix) = prefix.get(..4) else {
        return false;
    };
    prefix == b"NPD\0"
        || prefix == b"SCE\0"
        || prefix == [0x7f, b'P', b'K', b'G']
        || (from_magic(&prefix[..3]).is_some() && prefix[3] == b'e')
}
