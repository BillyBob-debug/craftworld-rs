//! Independent skin and morph stream reader (runtime plan item Rb); no posing or rendering.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! RMesh::serialize (streams of numVerts*0x10 bytes, morphNames), RMesh::getJoints,
//! getWeights, getNormals, getMorphs, and util/Bytes::unpackNormal24/unpackNormal32.
//! https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java
//! Applicability, conflicts and coverage: docs/mesh-skin.md.
use super::*;

/// Diagnostic summary; serializable, unlike the per-vertex arrays.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkinMetadata {
    pub vertex_count: u32,
    pub stream_count: u32,
    pub morph_count: u32,
    /// Largest joint byte on any vertex whose matching weight is nonzero; None if no weights.
    pub max_weighted_joint: Option<u8>,
    /// Vertices whose first weight byte is 0xFF (Toolkit's single-bone case).
    pub single_bone_vertices: usize,
    /// Vertices whose three stored weight bytes sum above 255, so the derived fourth
    /// weight is negative (kept as decoded; a finding, not an error).
    pub weight_overflow_vertices: usize,
    /// Packed 24-bit normals whose x^2 + y^2 exceeds 1; z is clamped to 0 for these.
    pub clamped_normals: usize,
    pub morph_names: Vec<String>,
}

/// One morph target: per-vertex position offset and unpacked normal.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphTarget {
    pub name: String,
    pub offsets: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

/// CPU skin arrays are deliberately not a serializable asset export.
#[derive(Debug, Clone, PartialEq)]
pub struct SkinStreams {
    pub metadata: SkinMetadata,
    /// Indices into the mesh's bone list (bytes 3, 7, 11, 15 of stream 1).
    pub joints: Vec<[u8; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
    pub smooth_normals: Vec<[f32; 3]>,
    pub morphs: Vec<MorphTarget>,
}

/// Bytes::unpackNormal24: x 11 bits + sign (bit 11), y 10 bits + sign (bit 22),
/// z = sqrt(1 - x^2 - y^2) negated when bit 23 is set. Returns (normal, clamped).
pub fn unpack_normal24(v: u32) -> ([f32; 3], bool) {
    let x = (v & 0x7ff) as f32;
    let x = if v & 0x800 != 0 {
        -(2048.0 - x) / 2047.0
    } else {
        x / 2047.0
    };
    let y = ((v >> 12) & 0x3ff) as f32;
    let y = if (v >> 12) & 0x400 != 0 {
        -(1024.0 - y) / 1023.0
    } else {
        y / 1023.0
    };
    let r = 1.0 - (x * x + y * y);
    let z = r.max(0.0).sqrt();
    ([x, y, if (v >> 23) & 1 != 0 { -z } else { z }], r < 0.0)
}

/// Bytes::unpackNormal32: x 10 bits + sign (bit 10), y 10 bits + sign (bit 21),
/// z 9 bits + sign (bit 31); a set sign maps the field f to -(max - f) / max.
pub fn unpack_normal32(v: u32) -> [f32; 3] {
    let field = |shift: u32, bits: u32, sign: u32| {
        let max = ((1u32 << bits) - 1) as f32;
        let f = ((v >> shift) & ((1 << bits) - 1)) as f32;
        if (v >> sign) & 1 != 0 {
            -(max - f) / max
        } else {
            f / max
        }
    };
    [field(0, 10, 10), field(11, 10, 21), field(22, 9, 31)]
}

fn u24(b: &[u8]) -> u32 {
    u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2])
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_bits(u32::from_be_bytes(b[at..at + 4].try_into().unwrap()))
}

/// Reads stream 1 (skinning) and streams 2.. (morphs). A mesh with fewer than two
/// streams is Unsupported("no skinning stream"); missing morph streams are Invalid.
pub fn decode_skin(bytes: &[u8], cancel: impl Fn() -> bool) -> Result<SkinStreams, Error> {
    let (header, data) = payload(bytes, &cancel)?;
    let prefix = prefix(&header, &data, &cancel)?;
    let [vertices, _, _, _, streams, _, morphs] = prefix.counts;
    if streams < 2 {
        return Err(Error::Unsupported("no skinning stream"));
    }
    if morphs > streams - 2 {
        return Err(Error::Invalid("morph count exceeds morph streams"));
    }
    let mut r = Reader {
        b: &data,
        p: 0,
        compact: prefix.compression_flags & 1 != 0,
    };
    for _ in 0..7 {
        r.integer()?;
    }
    r.take(32 * 16)?;
    if header.revision >= 0x239 {
        r.floats()?;
        r.floats()?;
        r.word()?;
    }
    for _ in 0..streams + 2 {
        r.integer()?;
    }
    // prefix() already bounded vertices * 16 * streams by MAX_EXPANDED.
    let size = vertices as usize * 16;
    let all = r.take(size * streams as usize)?;
    let stream = |i: usize| &all[i * size..(i + 1) * size];
    let n = vertices as usize;
    let (mut joints, mut weights) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let mut normal_sets: [Vec<[f32; 3]>; 3] = std::array::from_fn(|_| Vec::with_capacity(n));
    let (mut single, mut overflow, mut clamped) = (0, 0, 0);
    let mut max_joint: Option<u8> = None;
    for (i, v) in stream(1).as_chunks::<16>().0.iter().enumerate() {
        if i % 1024 == 0 {
            checked(&cancel)?;
        }
        let j = [v[3], v[7], v[11], v[15]];
        let w = if v[2] == 0xff {
            single += 1;
            [1.0, 0.0, 0.0, 0.0]
        } else {
            let (a, b, c) = (f32::from(v[2]), f32::from(v[1]), f32::from(v[0]));
            let d = 255.0 - a - b - c;
            overflow += usize::from(d < 0.0);
            [a / 255.0, b / 255.0, c / 255.0, d / 255.0]
        };
        for k in 0..4 {
            if w[k] != 0.0 {
                max_joint = max_joint.max(Some(j[k]));
            }
        }
        joints.push(j);
        weights.push(w);
        // getNormals 0x4, getSmoothNormals 0x8, getTangents 0xC (set order: normal,
        // tangent, smooth normal).
        for (set, at) in normal_sets.iter_mut().zip([4, 12, 8]) {
            let (normal, c) = unpack_normal24(u24(&v[at..at + 3]));
            clamped += usize::from(c);
            set.push(normal);
        }
    }
    let names: Vec<String> = prefix
        .morph_names_hex
        .iter()
        .take(morphs as usize)
        .map(|hex| {
            let raw: Vec<u8> = (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0))
                .collect();
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            raw[..end]
                .iter()
                .map(|&b| if b.is_ascii() { b as char } else { '\u{fffd}' })
                .collect()
        })
        .collect();
    let mut targets = Vec::with_capacity(morphs as usize);
    for (m, name) in names.iter().enumerate() {
        checked(&cancel)?;
        let (mut offsets, mut normals) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for v in stream(m + 2).as_chunks::<16>().0 {
            offsets.push([f32_at(v, 0), f32_at(v, 4), f32_at(v, 8)]);
            normals.push(unpack_normal32(u32::from_be_bytes(
                v[12..16].try_into().unwrap(),
            )));
        }
        targets.push(MorphTarget {
            name: name.clone(),
            offsets,
            normals,
        });
    }
    let [normals, tangents, smooth_normals] = normal_sets;
    Ok(SkinStreams {
        metadata: SkinMetadata {
            vertex_count: vertices,
            stream_count: streams,
            morph_count: morphs,
            max_weighted_joint: max_joint,
            single_bone_vertices: single,
            weight_overflow_vertices: overflow,
            clamped_normals: clamped,
            morph_names: names,
        },
        joints,
        weights,
        normals,
        tangents,
        smooth_normals,
        morphs: targets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal24_axes_signs_and_clamp() {
        let (n, c) = unpack_normal24(0);
        assert_eq!((n, c), ([0.0, 0.0, 1.0], false));
        let (n, _) = unpack_normal24(1 << 23);
        assert_eq!(n, [0.0, 0.0, -1.0]);
        let (n, _) = unpack_normal24(0x7ff);
        assert_eq!(n[0], 1.0);
        assert!(n[2].abs() < 1e-6);
        // Sign bit with field 0 is -1, the most negative value.
        let (n, _) = unpack_normal24(0x800);
        assert_eq!(n[0], -2048.0 / 2047.0);
        let (n, _) = unpack_normal24(0x3ff << 12);
        assert_eq!(n[1], 1.0);
        let (n, _) = unpack_normal24(0x400 << 12);
        assert_eq!(n[1], -1024.0 / 1023.0);
        // x = y = 1 is outside the unit disc: z clamps to 0 and is reported.
        let (n, c) = unpack_normal24(0x7ff | 0x3ff << 12);
        assert!(c && n[2] == 0.0);
    }

    #[test]
    fn normal32_fields() {
        assert_eq!(unpack_normal32(0), [0.0, 0.0, 0.0]);
        assert_eq!(unpack_normal32(0x3ff), [1.0, 0.0, 0.0]);
        assert_eq!(unpack_normal32(1 << 10), [-1.0, 0.0, 0.0]);
        assert_eq!(unpack_normal32(0x3ff << 11), [0.0, 1.0, 0.0]);
        assert_eq!(unpack_normal32(1 << 21), [0.0, -1.0, 0.0]);
        assert_eq!(unpack_normal32(0x1ff << 22), [0.0, 0.0, 1.0]);
        assert_eq!(unpack_normal32(1 << 31), [0.0, 0.0, -1.0]);
    }
}
