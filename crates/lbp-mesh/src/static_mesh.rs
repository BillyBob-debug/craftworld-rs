//! D119 bounded STATIC_MESH (`SMHb`, type 44) reader: prebaked level backdrop geometry.
//!
//! Independent specification notes (no code copied or translated), ennuo/toolkit
//! d6a2ef80b5a09999f08f23a803b516096055e5c9, CONFIRMED_FROM_COMMUNITY_CODE:
//! - `types/SerializedResource.java` process: magic "SMH" + 'b', head i32 BE, dependency
//!   table offset i32 BE (head >= 0x109); for STATIC_MESH at head >= 0x189 there is no
//!   branch / compression-flags / compressed-bool header. Instead a `StaticMeshInfo` struct
//!   follows, read with a fresh serializer at that head and compression flags 0 (all
//!   integers fixed big-endian). The payload after it is compressed by default and ends at
//!   the dependency table (`Compressor.decompressData`: the shared chunk format).
//! - `structs/staticmesh/StaticMeshInfo.java` serialize: lightmap, risemap, fallmap
//!   resource descriptors (TEXTURE; flags u32 when head > 0x22e, kind byte, GUID u32 if
//!   bit 2, SHA-1 if bit 1), primitive count i32, node count i32, indexBufferSize i32,
//!   vertexStreamSize i32, arraylist(StaticPrimitive), arraylist(StaticMeshTreeNode), then
//!   one i32 (0x48454c50, "HELP"; the community reader does not check it, this reader
//!   requires it). Both counts must equal the array lengths.
//! - `structs/staticmesh/StaticPrimitive.java`: min v4, max v4, gmat descriptor
//!   (GFX_MATERIAL), vertexStart i32, indexStart i32, numIndices i32, type u8
//!   (`enums/CellGcmPrimitive`: 5 TRIANGLES, 6 TRIANGLE_STRIP).
//!   StaticMeshTreeNode: min v3, firstChild i16, nextSibling i16, max v3, firstPrimitive
//!   i16, numPrimitives i16 (streaming bounds; not used for drawing).
//! - `resources/RStaticMesh.java`: expanded payload = vertexStreamSize bytes of vertices
//!   then indexBufferSize bytes of u16 BE indices. Vertex stride 0x20: position 3 f32,
//!   packed normal u32, UV0 2 x f16, packed tangent u32, UV1 2 x f16, smooth normal u32.
//!   Strips use index 0xFFFF as restart; triangle k of a strip segment uses its k..k+2
//!   indices with the last two swapped for odd k; degenerate triangles are dropped.
//! - `io/exports/MeshExporter.java` GLB.FromMesh / OBJ export: primitive indices are
//!   relative to `vertexStart` (accessor offset, `+ vertexStart` in the OBJ path).
//!
//! CONFIRMED_FROM_LOCAL_DATA (D119): the backdrop meshes named by local levels decode with
//! zero trailing bytes and every primitive's vertices lie inside its stored min/max box
//! (see docs/d119-level-backdrop.md). Applicability: heads 0x191..=0x3f8, ordinary
//! binary only; normals, tangents and the lightmap textures are not interpreted.
use crate::Error;

pub const MAX_INPUT: usize = 32 * 1024 * 1024;
pub const MAX_EXPANDED: usize = 64 * 1024 * 1024;
pub const MAX_PRIMITIVES: usize = 65_536;
pub const MAX_NODES: usize = 65_536;
pub const VERTEX_STRIDE: usize = 0x20;
const HELP: u32 = 0x4845_4c50;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StaticDescriptor {
    pub flags: u32,
    pub guid: Option<u32>,
    pub sha1: Option<[u8; 20]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticPrimitive {
    pub min: [f32; 4],
    pub max: [f32; 4],
    pub material: Option<StaticDescriptor>,
    pub vertex_start: u32,
    pub index_start: u32,
    pub index_count: u32,
    /// CellGcmPrimitive value (5 triangles, 6 triangle strip).
    pub kind: u8,
}

#[derive(Debug, Clone)]
pub struct StaticMesh {
    pub revision: u32,
    pub lightmaps: [Option<StaticDescriptor>; 3],
    pub primitives: Vec<StaticPrimitive>,
    pub node_count: usize,
    pub positions: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub uv1: Vec<[f32; 2]>,
    /// Raw u16 index buffer.
    pub indices: Vec<u16>,
    pub expanded_bytes: usize,
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.p.checked_add(n).ok_or(Error::Budget)?;
        let v = self
            .b
            .get(self.p..end)
            .ok_or(Error::Invalid("static mesh header bounds"))?;
        self.p = end;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i16(&mut self) -> Result<i16, Error> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn count(&mut self, max: usize, min_size: usize) -> Result<usize, Error> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Error::Budget);
        }
        if n.saturating_mul(min_size) > self.b.len().saturating_sub(self.p) {
            return Err(Error::Invalid("static mesh count exceeds remaining bytes"));
        }
        Ok(n)
    }
    fn descriptor(&mut self, revision: u32) -> Result<Option<StaticDescriptor>, Error> {
        let flags = if revision > 0x22e { self.u32()? } else { 0 };
        let kind = self.take(1)?[0];
        if kind & !3 != 0 {
            return Err(Error::Unsupported("static mesh descriptor kind"));
        }
        if kind == 0 {
            return Ok(None);
        }
        let guid = if kind & 2 != 0 {
            Some(self.u32()?)
        } else {
            None
        };
        let sha1 = if kind & 1 != 0 {
            Some(self.take(20)?.try_into().unwrap())
        } else {
            None
        };
        Ok(Some(StaticDescriptor { flags, guid, sha1 }))
    }
}

/// IEEE 754 binary16 to f32.
pub fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0f32 } else { 1.0 };
    let exp = (h >> 10) & 0x1f;
    let frac = f32::from(h & 0x3ff);
    match exp {
        0 => sign * frac * (2f32).powi(-24),
        0x1f if frac == 0. => sign * f32::INFINITY,
        0x1f => f32::NAN,
        e => sign * (1. + frac / 1024.) * (2f32).powi(i32::from(e) - 15),
    }
}

/// Reads a whole `SMHb` resource. Positions/UVs are decoded for every vertex; indices are
/// kept raw (see `primitive_triangles`).
pub fn read(b: &[u8], cancel: impl Fn() -> bool) -> Result<StaticMesh, Error> {
    lbp_resource::binary::check_cancel(&cancel)?;
    if b.len() > MAX_INPUT {
        return Err(Error::Budget);
    }
    if b.len() < 12 {
        return Err(Error::Invalid("static mesh header"));
    }
    if lbp_resource::is_protected_prefix(&b[..4]) {
        return Err(Error::Blocked);
    }
    if &b[..4] != b"SMHb" {
        return Err(Error::Unsupported("static mesh magic"));
    }
    let revision = u32::from_be_bytes(b[4..8].try_into().unwrap());
    if !(0x191..=lbp_resource::LBP2_MAX_REVISION).contains(&revision) {
        return Err(Error::Unsupported("static mesh revision"));
    }
    let dep = u32::from_be_bytes(b[8..12].try_into().unwrap()) as usize;
    if dep > b.len() || dep < 12 {
        return Err(Error::Invalid("dependency table offset"));
    }
    let mut c = Cursor {
        b: &b[..dep],
        p: 12,
    };
    let lightmaps = [
        c.descriptor(revision)?,
        c.descriptor(revision)?,
        c.descriptor(revision)?,
    ];
    let primitive_count = c.u32()? as usize;
    let node_count = c.u32()? as usize;
    let index_bytes = c.u32()? as usize;
    let vertex_bytes = c.u32()? as usize;
    // Smallest primitive: 32 bytes of bounds, flags + kind byte, 3 i32, type byte.
    let n = c.count(MAX_PRIMITIVES, 46)?;
    let mut primitives = Vec::with_capacity(n);
    for _ in 0..n {
        let mut min = [0.; 4];
        let mut max = [0.; 4];
        for v in &mut min {
            *v = c.f32()?;
        }
        for v in &mut max {
            *v = c.f32()?;
        }
        let material = c.descriptor(revision)?;
        primitives.push(StaticPrimitive {
            min,
            max,
            material,
            vertex_start: c.u32()?,
            index_start: c.u32()?,
            index_count: c.u32()?,
            kind: c.take(1)?[0],
        });
    }
    let nodes = c.count(MAX_NODES, 32)?;
    for _ in 0..nodes {
        c.take(12)?;
        c.i16()?;
        c.i16()?;
        c.take(12)?;
        c.i16()?;
        c.i16()?;
    }
    if primitive_count != n || node_count != nodes {
        return Err(Error::Invalid("static mesh primitive/node count mismatch"));
    }
    if c.u32()? != HELP {
        return Err(Error::Invalid("static mesh info end marker"));
    }
    lbp_resource::binary::check_cancel(&cancel)?;
    let data = lbp_resource::binary::expand(&b[c.p..dep], MAX_EXPANDED, &cancel)?;
    let total = vertex_bytes.checked_add(index_bytes).ok_or(Error::Budget)?;
    if data.len() != total {
        return Err(Error::Invalid(
            "static mesh stream sizes do not match payload",
        ));
    }
    if !vertex_bytes.is_multiple_of(VERTEX_STRIDE) || !index_bytes.is_multiple_of(2) {
        return Err(Error::Invalid("static mesh stream alignment"));
    }
    let vertices = vertex_bytes / VERTEX_STRIDE;
    let mut positions = Vec::with_capacity(vertices);
    let mut uv0 = Vec::with_capacity(vertices);
    let mut uv1 = Vec::with_capacity(vertices);
    let word = |s: &[u8], o: usize| u32::from_be_bytes(s[o..o + 4].try_into().unwrap());
    let half = |s: &[u8], o: usize| half_to_f32(u16::from_be_bytes([s[o], s[o + 1]]));
    for (i, v) in data[..vertex_bytes]
        .as_chunks::<VERTEX_STRIDE>()
        .0
        .iter()
        .enumerate()
    {
        if i % 65_536 == 0 {
            lbp_resource::binary::check_cancel(&cancel)?;
        }
        positions.push([0, 4, 8].map(|o| f32::from_bits(word(v, o))));
        uv0.push([half(v, 16), half(v, 18)]);
        uv1.push([half(v, 24), half(v, 26)]);
    }
    let indices = data[vertex_bytes..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u16::from_be_bytes([p[0], p[1]]))
        .collect();
    Ok(StaticMesh {
        revision,
        lightmaps,
        primitives,
        node_count: nodes,
        positions,
        uv0,
        uv1,
        indices,
        expanded_bytes: data.len(),
    })
}

impl StaticMesh {
    /// Triangles of primitive `i` as absolute vertex indices (vertexStart applied),
    /// degenerate triangles dropped. Errors for out-of-range spans or vertices and for
    /// primitive types other than triangles / triangle strips.
    pub fn primitive_triangles(&self, i: usize) -> Result<Vec<[u32; 3]>, Error> {
        let p = self
            .primitives
            .get(i)
            .ok_or(Error::Invalid("primitive index"))?;
        let start = p.index_start as usize;
        let end = start
            .checked_add(p.index_count as usize)
            .ok_or(Error::Budget)?;
        let span = self
            .indices
            .get(start..end)
            .ok_or(Error::Invalid("primitive index span"))?;
        let base = p.vertex_start as usize;
        let abs = |k: u16| -> Result<u32, Error> {
            let v = base.checked_add(usize::from(k)).ok_or(Error::Budget)?;
            if v >= self.positions.len() {
                return Err(Error::Invalid("primitive vertex out of range"));
            }
            Ok(v as u32)
        };
        let mut out = Vec::new();
        let mut push = |a: u16, b: u16, c: u16| -> Result<(), Error> {
            if a != b && b != c && a != c {
                out.push([abs(a)?, abs(b)?, abs(c)?]);
            }
            Ok(())
        };
        match p.kind {
            5 => {
                if span.len() % 3 != 0 {
                    return Err(Error::Invalid("triangle list length"));
                }
                for t in span.as_chunks::<3>().0 {
                    if t.contains(&0xffff) {
                        return Err(Error::Invalid("restart index in triangle list"));
                    }
                    push(t[0], t[1], t[2])?;
                }
            }
            6 => {
                for strip in span.split(|&k| k == 0xffff) {
                    for (k, w) in strip.windows(3).enumerate() {
                        if k % 2 == 0 {
                            push(w[0], w[1], w[2])?;
                        } else {
                            push(w[0], w[2], w[1])?;
                        }
                    }
                }
            }
            _ => return Err(Error::Unsupported("static mesh primitive type")),
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn half(v: f32) -> u16 {
        // Exact for the small test values used here (0, 0.5, 1, 2).
        match v {
            0.0 => 0,
            0.5 => 0x3800,
            1.0 => 0x3c00,
            2.0 => 0x4000,
            _ => panic!("test value"),
        }
    }

    /// Synthetic SMHb: one strip primitive over 4 vertices, stored uncompressed (chunk
    /// count 0).
    fn sample(kind: u8, indices: &[u16], marker: u32) -> Vec<u8> {
        let mut info = Vec::new();
        for _ in 0..3 {
            info.extend(0u32.to_be_bytes()); // flags
            info.push(0); // null descriptor
        }
        info.extend(1u32.to_be_bytes());
        info.extend(1u32.to_be_bytes());
        info.extend(((indices.len() * 2) as u32).to_be_bytes());
        info.extend(((4 * VERTEX_STRIDE) as u32).to_be_bytes());
        info.extend(1u32.to_be_bytes());
        for v in [0f32, 0., 0., 1., 1., 1., 0., 1.] {
            info.extend(v.to_be_bytes());
        }
        info.extend(0u32.to_be_bytes());
        info.push(2);
        info.extend(0x1234u32.to_be_bytes());
        info.extend(0u32.to_be_bytes()); // vertexStart
        info.extend(0u32.to_be_bytes()); // indexStart
        info.extend((indices.len() as u32).to_be_bytes());
        info.push(kind);
        info.extend(1u32.to_be_bytes());
        info.extend([0; 32]); // one tree node
        info.extend(marker.to_be_bytes());
        let mut payload = vec![0, 1, 0, 0];
        for (x, y) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.)] {
            for v in [x, y, 0f32] {
                payload.extend(v.to_be_bytes());
            }
            payload.extend([0; 4]);
            payload.extend(half(x).to_be_bytes());
            payload.extend(half(y).to_be_bytes());
            payload.extend([0; 4]);
            payload.extend(half(0.5).to_be_bytes());
            payload.extend(half(2.).to_be_bytes());
            payload.extend([0; 4]);
        }
        for i in indices {
            payload.extend(i.to_be_bytes());
        }
        let mut b = b"SMHb".to_vec();
        b.extend(0x3e5u32.to_be_bytes());
        let dep = 12 + info.len() + payload.len();
        b.extend((dep as u32).to_be_bytes());
        b.extend(info);
        b.extend(payload);
        b.extend(0u32.to_be_bytes()); // empty dependency table
        b
    }

    #[test]
    fn strip_with_restart_and_list_decode() {
        let m = read(&sample(6, &[0, 1, 2, 3, 0xffff, 0, 1, 2], HELP), || false).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.uv0[3], [1., 1.]);
        assert_eq!(m.uv1[0], [0.5, 2.]);
        assert_eq!(
            m.primitives[0].material.as_ref().unwrap().guid,
            Some(0x1234)
        );
        assert_eq!(
            m.primitive_triangles(0).unwrap(),
            vec![[0, 1, 2], [1, 3, 2], [0, 1, 2]]
        );
        let m = read(&sample(5, &[0, 1, 2, 2, 2, 3], HELP), || false).unwrap();
        assert_eq!(m.primitive_triangles(0).unwrap(), vec![[0, 1, 2]]);
    }

    #[test]
    fn malformed_inputs_are_explicit() {
        assert_eq!(
            read(&sample(6, &[0, 1, 2], 0), || false).unwrap_err(),
            Error::Invalid("static mesh info end marker")
        );
        let m = read(&sample(6, &[0, 1, 9], HELP), || false).unwrap();
        assert!(m.primitive_triangles(0).is_err());
        let m = read(&sample(4, &[0, 1, 2], HELP), || false).unwrap();
        assert_eq!(
            m.primitive_triangles(0).unwrap_err(),
            Error::Unsupported("static mesh primitive type")
        );
        let mut b = sample(6, &[0, 1, 2], HELP);
        b[3] = b'x';
        assert!(read(&b, || false).is_err());
        assert_eq!(
            read(&sample(6, &[0, 1, 2], HELP), || true).unwrap_err(),
            Error::Cancelled
        );
        assert_eq!(half_to_f32(0xbc00), -1.);
        assert_eq!(half_to_f32(0x0001), 2f32.powi(-24));
    }
}
