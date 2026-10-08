//! Independent bounded MSH prefix, geometry and UV readers; no rendering/asset export.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! RMesh::serialize/getVertices, MemoryInputStream::i32/str/floatarray/bytearray,
//! SerializedResource::process and Compressor::decompressData.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib
//! Applicability, conflicts and synthetic/local oracle evidence: docs/mesh-prefix.md.
//! inspect returns prefix metadata; geometry and UV arrays require separate bounded APIs.
pub use lbp_resource::binary::Error;
mod uv;
use serde::Serialize;
use sha2::{Digest, Sha256};
pub use uv::{decode_uvs, UvCoordinates, UvMetadata};
pub const MAX_INPUT: usize = 2 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 16 * 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct MeshPrefix {
    pub revision: u32,
    pub compression_flags: u8,
    pub counts: [u32; 7],
    pub morph_names_hex: Vec<String>,
    pub min_uv_bits: Vec<u32>,
    pub max_uv_bits: Vec<u32>,
    pub area_scale_bits: Option<u32>,
    pub stream_offsets: Vec<u32>,
    pub position_bounds: Option<[[f32; 3]; 2]>,
    pub position_bounds_bits: Option<[[u32; 3]; 2]>,
    pub position_bytes_sha256: String,
    pub attribute_bytes: usize,
    pub attribute_sha256: String,
    pub index_bytes: usize,
    pub index_sha256: String,
    pub expanded_bytes: usize,
    pub parsed_prefix_bytes: usize,
    pub unparsed_bytes: usize,
}
fn checked(cancel: &impl Fn() -> bool) -> Result<(), Error> {
    if cancel() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
struct Reader<'a> {
    b: &'a [u8],
    p: usize,
    compact: bool,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.p.checked_add(n).ok_or(Error::Budget)?;
        let v = self
            .b
            .get(self.p..end)
            .ok_or(Error::Invalid("prefix bounds"))?;
        self.p = end;
        Ok(v)
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn integer(&mut self) -> Result<u32, Error> {
        if !self.compact {
            return self.word();
        }
        let mut n = 0u32;
        for i in 0..5 {
            let b = self.take(1)?[0];
            if i == 4 && b & 0xf0 != 0 {
                return Err(Error::Invalid("uleb32 overflow"));
            }
            n |= u32::from(b & 127) << (7 * i);
            if b & 128 == 0 {
                return Ok(n);
            }
        }
        Err(Error::Invalid("unterminated uleb32"))
    }
    fn count(&mut self, max: u32) -> Result<u32, Error> {
        let n = self.integer()?;
        if n > max {
            Err(Error::Budget)
        } else {
            Ok(n)
        }
    }
    fn floats(&mut self) -> Result<Vec<u32>, Error> {
        let n = self.count(16)?;
        (0..n).map(|_| self.word()).collect()
    }
    fn bytearray(&mut self) -> Result<&'a [u8], Error> {
        let n = self.count(MAX_EXPANDED as u32)?;
        self.take(n as usize)
    }
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
/// Strictly bounded prefix diagnostic; original owner supplies immutable bytes.
/// Magic preflight for source owners before allocating or reading an entire candidate.
pub fn preflight(b: &[u8]) -> Result<(), Error> {
    if b.len() < 4 {
        return Err(Error::Invalid("magic"));
    }
    if lbp_resource::is_protected_prefix(b) {
        return Err(Error::Blocked);
    }
    if &b[..4] != b"MSHb" {
        return Err(Error::Unsupported("ordinary binary MSH only"));
    }
    Ok(())
}
fn payload(b: &[u8], cancel: &impl Fn() -> bool) -> Result<(lbp_resource::Header, Vec<u8>), Error> {
    lbp_resource::binary::payload(b, b"MSHb", MAX_INPUT, MAX_EXPANDED, cancel)
}
/// Strictly bounded prefix diagnostic; original owner supplies immutable bytes.
pub fn inspect(b: &[u8], cancel: impl Fn() -> bool) -> Result<MeshPrefix, Error> {
    let (h, data) = payload(b, &cancel)?;
    prefix(&h, &data, &cancel)
}
fn prefix(
    h: &lbp_resource::Header,
    data: &[u8],
    cancel: &impl Fn() -> bool,
) -> Result<MeshPrefix, Error> {
    let flags = h.compression_flags.ok_or(Error::Invalid("flags absent"))?;
    let mut r = Reader {
        b: data,
        p: 0,
        compact: flags & 1 != 0,
    };
    let mut counts = [0; 7];
    for (i, n) in counts.iter_mut().enumerate() {
        *n = r.count(if i < 4 { 1_000_000 } else { 34 })?;
    }
    if counts[0] > 100_000 || counts[4] > 34 || counts[5] > 8 || counts[6] > 32 {
        return Err(Error::Budget);
    }
    if counts[4] == 0 {
        return Err(Error::Unsupported("no geometry stream"));
    }
    let names = r
        .take(32 * 16)?
        .as_chunks::<16>()
        .0
        .iter()
        .map(|v| v.iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    let (min_uv, max_uv, area) = if h.revision >= 0x239 {
        (r.floats()?, r.floats()?, Some(r.word()?))
    } else {
        (vec![], vec![], None)
    };
    let offsets = (0..counts[4] + 2)
        .map(|_| r.integer())
        .collect::<Result<Vec<_>, _>>()?;
    let stream_size = (counts[0] as usize).checked_mul(16).ok_or(Error::Budget)?;
    let total = stream_size
        .checked_mul(counts[4] as usize)
        .ok_or(Error::Budget)?;
    if total > MAX_EXPANDED {
        return Err(Error::Budget);
    }
    // Preserve source offsets rather than requiring Toolkit's write-side recipe:
    // its reader skips these fields and instead reads fixed numVerts*16 streams.
    let streams = r.take(total)?;
    let mut bounds = None;
    let mut digest = Sha256::new();
    for (i, v) in streams[..stream_size]
        .as_chunks::<16>()
        .0
        .iter()
        .enumerate()
    {
        if i % 256 == 0 {
            checked(cancel)?;
        }
        digest.update(&v[..12]);
        let p: [f32; 3] = std::array::from_fn(|j| {
            f32::from_bits(u32::from_be_bytes(v[j * 4..j * 4 + 4].try_into().unwrap()))
        });
        if p.iter().any(|v| !v.is_finite()) {
            return Err(Error::Unsupported("non-finite geometry"));
        }
        let b = bounds.get_or_insert([p, p]);
        for j in 0..3 {
            b[0][j] = b[0][j].min(p[j]);
            b[1][j] = b[1][j].max(p[j]);
        }
    }
    let attrs = r.bytearray()?;
    let indices = r.bytearray()?;
    checked(cancel)?;
    Ok(MeshPrefix {
        revision: h.revision,
        compression_flags: flags,
        counts,
        morph_names_hex: names,
        min_uv_bits: min_uv,
        max_uv_bits: max_uv,
        area_scale_bits: area,
        stream_offsets: offsets,
        position_bounds: bounds,
        position_bounds_bits: bounds.map(|b| b.map(|v| v.map(f32::to_bits))),
        position_bytes_sha256: format!("{:x}", digest.finalize()),
        attribute_bytes: attrs.len(),
        attribute_sha256: hash(attrs),
        index_bytes: indices.len(),
        index_sha256: hash(indices),
        expanded_bytes: data.len(),
        parsed_prefix_bytes: r.p,
        unparsed_bytes: data.len() - r.p,
    })
}

mod geometry;
mod skeleton;
mod skin;
pub use geometry::{decode_geometry, Geometry, GeometryMetadata, Primitive};
pub use skeleton::{anim_hash, Bone, ShapeInfo, ShapeVertex, Skeleton, SkeletonCheck, MAX_BONES};
pub use skin::{
    decode_skin, unpack_normal24, unpack_normal32, MorphTarget, SkinMetadata, SkinStreams,
};
