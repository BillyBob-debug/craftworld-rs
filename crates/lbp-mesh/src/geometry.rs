//! D2 bounded topology traversal: pinned Toolkit Primitive/Bone/AnimBone/shape serializers,
//! Serializer::resource, MemoryInputStream::m44 and RMesh::getTriangles.
//! Exact sources/applicability/conflicts/coverage: docs/mesh-geometry.md.
use super::*;
#[derive(Debug, Serialize)]
pub struct Primitive {
    pub material_flags: u32,
    pub material_kind: u8,
    pub material_guid: Option<u32>,
    pub material_hash: Option<String>,
    pub alternatives_flags: u32,
    pub alternatives_kind: u8,
    pub alternatives_guid: Option<u32>,
    pub alternatives_hash: Option<String>,
    pub min_vertex_raw: u32,
    pub max_vertex_raw: u32,
    pub first_index: u32,
    pub index_count: u32,
    pub region: u32,
}
#[derive(Debug, Serialize)]
pub struct GeometryMetadata {
    pub prefix: MeshPrefix,
    pub primitives: Vec<Primitive>,
    pub bone_count: u32,
    pub primitive_type: u8,
    pub triangle_count: usize,
    pub triangle_indices_sha256: String,
    pub parsed_geometry_bytes: usize,
    pub unparsed_after_geometry: usize,
}
/// CPU geometry only, intentionally not serializable as diagnostic asset output.
#[derive(Debug)]
pub struct Geometry {
    pub metadata: GeometryMetadata,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    /// One contiguous triangle span per metadata primitive, including empty spans.
    /// Overlapping source index ranges retain separate ownership, never deduplicate.
    pub primitive_triangles: Vec<std::ops::Range<usize>>,
}
fn descriptor(
    r: &mut Reader<'_>,
    revision: u32,
) -> Result<(u32, u8, Option<u32>, Option<String>), Error> {
    let flags = if revision > 0x22e { r.integer()? } else { 0 };
    let kind = r.take(1)?[0];
    if kind & !3 != 0 {
        return Err(Error::Unsupported("descriptor kind"));
    }
    let guid = if kind & 2 != 0 {
        Some(r.integer()?)
    } else {
        None
    };
    let hash = if kind & 1 != 0 {
        Some(r.take(20)?.iter().map(|b| format!("{b:02x}")).collect())
    } else {
        None
    };
    Ok((flags, kind, guid, hash))
}
fn skip_bone(r: &mut Reader<'_>, cancel: &impl Fn() -> bool) -> Result<(), Error> {
    checked(cancel)?;
    r.take(32)?;
    for _ in 0..5 {
        r.integer()?;
    } // flags, animation hash, parent/child/sibling (ULEB/zigzag consume same bytes)
    for _ in 0..2 {
        let mask = if r.compact {
            u16::from_be_bytes(r.take(2)?.try_into().unwrap())
        } else {
            u16::MAX
        };
        r.take(mask.count_ones() as usize * 4)?;
    }
    r.take(32)?; // OBB min/max
    let shapes = r.count(4096)?;
    for _ in 0..shapes {
        r.take(32)?;
        r.integer()?;
    }
    let infos = r.count(4096)?;
    for _ in 0..infos {
        r.integer()?;
        r.integer()?;
    }
    r.take(8 + 48)?;
    Ok(()) // Z range, bounds and sphere
}
/// Stored rest geometry, no bone/morph transforms, no material shading.
pub fn decode_geometry(b: &[u8], cancel: impl Fn() -> bool) -> Result<Geometry, Error> {
    let (h, data) = payload(b, &cancel)?;
    if h.revision < 567 {
        return Err(Error::Unsupported(
            "mesh revision below verified local range",
        ));
    }
    let prefix = prefix(&h, &data, &cancel)?;
    let mut r = Reader {
        b: &data,
        p: 0,
        compact: prefix.compression_flags & 1 != 0,
    };
    for _ in 0..7 {
        r.integer()?;
    }
    r.take(512)?;
    if h.revision >= 0x239 {
        r.floats()?;
        r.floats()?;
        r.word()?;
    }
    for _ in 0..prefix.counts[4] + 2 {
        r.integer()?;
    }
    let positions_bytes = r.take(prefix.counts[0] as usize * 16)?;
    let positions = positions_bytes
        .as_chunks::<16>()
        .0
        .iter()
        .map(|v| {
            std::array::from_fn(|j| {
                f32::from_bits(u32::from_be_bytes(v[j * 4..j * 4 + 4].try_into().unwrap()))
            })
        })
        .collect::<Vec<_>>();
    r.p = prefix.parsed_prefix_bytes - prefix.index_bytes;
    let raw_indices = r.take(prefix.index_bytes)?;
    if prefix.index_bytes % 2 != 0 || prefix.counts[1] as usize > prefix.index_bytes / 2 {
        return Err(Error::Invalid("index buffer/count"));
    }
    let primitive_count = r.count(4096)?;
    let mut primitives = Vec::with_capacity(primitive_count as usize);
    for _ in 0..primitive_count {
        checked(&cancel)?;
        let material = descriptor(&mut r, h.revision)?;
        let alternatives = if h.revision >= 0x179 {
            descriptor(&mut r, h.revision)?
        } else {
            (0, 0, None, None)
        };
        let p = Primitive {
            material_flags: material.0,
            material_kind: material.1,
            material_guid: material.2,
            material_hash: material.3,
            alternatives_flags: alternatives.0,
            alternatives_kind: alternatives.1,
            alternatives_guid: alternatives.2,
            alternatives_hash: alternatives.3,
            min_vertex_raw: r.integer()?,
            max_vertex_raw: r.integer()?,
            first_index: r.integer()?,
            index_count: r.integer()?,
            region: r.integer()?,
        };
        if p.first_index
            .checked_add(p.index_count)
            .is_none_or(|end| end > prefix.counts[1])
        {
            return Err(Error::Invalid("primitive index range"));
        }
        primitives.push(p);
    }
    let bones = r.count(4096)?;
    for _ in 0..bones {
        skip_bone(&mut r, &cancel)?;
    }
    let n = r.count(4096)?;
    r.take(n as usize * 2)?;
    let n = r.count(4096)?;
    r.take(n as usize)?;
    let n = r.count(32)?;
    r.take(n as usize * 2)?;
    let topology = r.take(1)?[0];
    if !matches!(topology, 5 | 6) {
        return Err(Error::Unsupported("primitive topology"));
    }
    let mut triangles = Vec::new();
    let mut primitive_triangles = Vec::with_capacity(primitives.len());
    let mut digest = Sha256::new();
    for p in &primitives {
        let start = p.first_index as usize * 2;
        let end = start + p.index_count as usize * 2;
        let indices = raw_indices[start..end]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| u16::from_be_bytes(*v) as u32)
            .collect::<Vec<_>>();
        let result = triangulate(&indices, topology, positions.len(), &cancel)?;
        if triangles.len() + result.len() > 1_000_000 {
            return Err(Error::Budget);
        }
        for t in &result {
            for i in t {
                digest.update(i.to_be_bytes());
            }
        }
        let first = triangles.len();
        triangles.extend(result);
        primitive_triangles.push(first..triangles.len());
    }
    checked(&cancel)?;
    let metadata = GeometryMetadata {
        prefix,
        primitives,
        bone_count: bones,
        primitive_type: topology,
        triangle_count: triangles.len(),
        triangle_indices_sha256: format!("{:x}", digest.finalize()),
        parsed_geometry_bytes: r.p,
        unparsed_after_geometry: data.len() - r.p,
    };
    Ok(Geometry {
        metadata,
        positions,
        triangles,
        primitive_triangles,
    })
}
// Triangle-strip parity advances even for degenerates, and resets on fixed u16 restart.
// No culling, winding transform or original coordinate system is inferred.
fn triangulate(
    indices: &[u32],
    topology: u8,
    vertices: usize,
    cancel: &impl Fn() -> bool,
) -> Result<Vec<[u32; 3]>, Error> {
    let mut out = Vec::new();
    let mut history = [0u32; 2];
    let mut n = 0usize;
    if topology == 5 && !indices.len().is_multiple_of(3) {
        return Err(Error::Invalid("triangle-list length"));
    }
    for (i, &v) in indices.iter().enumerate() {
        if i % 256 == 0 {
            checked(cancel)?;
        }
        if topology == 6 && v == 65535 {
            n = 0;
            continue;
        }
        if v as usize >= vertices {
            return Err(Error::Invalid("vertex index bounds"));
        }
        if topology == 5 {
            if i % 3 == 2 {
                out.push([indices[i - 2], indices[i - 1], v]);
            }
        } else {
            if n >= 2 {
                out.push(if n.is_multiple_of(2) {
                    [history[0], history[1], v]
                } else {
                    [history[0], v, history[1]]
                });
            }
            history = [history[1], v];
            n += 1;
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_kinds_preserve_raw_ids_and_reject_unknown_bits() {
        for kind in 0u8..4 {
            let mut b = vec![0, kind];
            if kind & 2 != 0 {
                b.push(42);
            }
            if kind & 1 != 0 {
                b.extend_from_slice(&[7; 20]);
            }
            let mut r = Reader {
                b: &b,
                p: 0,
                compact: true,
            };
            let d = descriptor(&mut r, 950).unwrap();
            assert_eq!(d.1, kind);
            assert_eq!(d.2, if kind & 2 != 0 { Some(42) } else { None });
            assert_eq!(d.3.is_some(), kind & 1 != 0);
            assert_eq!(r.p, b.len());
        }
        assert!(descriptor(
            &mut Reader {
                b: &[0, 4],
                p: 0,
                compact: true
            },
            950
        )
        .is_err());
    }
    #[test]
    fn list_strip_restart_degenerates_and_bounds() {
        assert_eq!(
            triangulate(&[0, 1, 2, 3, 65535, 2, 3, 4], 6, 5, &|| false).unwrap(),
            [[0, 1, 2], [1, 3, 2], [2, 3, 4]]
        );
        assert_eq!(
            triangulate(&[0, 0, 1, 2], 6, 3, &|| false).unwrap(),
            [[0, 0, 1], [0, 2, 1]]
        );
        assert_eq!(
            triangulate(&[65535, 0, 1], 6, 3, &|| false).unwrap().len(),
            0
        );
        assert!(triangulate(&[0, 1], 5, 3, &|| false).is_err());
        assert!(triangulate(&[0, 1, 4], 5, 3, &|| false).is_err());
    }
}
