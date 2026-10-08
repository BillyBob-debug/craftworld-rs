//! Bounded RMesh bone and mirror-table reader; no posing, skinning or animation.
//! CONFIRMED_FROM_COMMUNITY_CODE: Toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9
//! structs/mesh/Bone::serialize, structs/animation/AnimBone::serialize,
//! structs/mesh/MeshShapeVertex/MeshShapeInfo::serialize, MemoryInputStream::m44/s32,
//! RMesh::serialize (bones, mirrorBones, mirrorBoneFlipTypes, mirrorMorphs) and
//! RAnimation::calculateAnimationHash.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib
//! Applicability, conflicts and coverage: docs/mesh-bones.md.
use super::*;

pub const MAX_BONES: u32 = 4096;
const MAX_SHAPE_ROWS: u32 = 4096;

/// One stored bone. Matrices are the 16 stored floats in stream order (column-major,
/// translation at 12..15), identity-filled where a compressed-matrix mask omits them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bone {
    /// Fixed 32-byte name up to the first NUL, lossy ASCII; raw bytes kept in `name_hex`.
    pub name: String,
    pub name_hex: String,
    pub flags: u32,
    pub anim_hash: u32,
    pub parent: i32,
    pub first_child: i32,
    pub next_sibling: i32,
    pub skin_pose: [f32; 16],
    pub skin_pose_mask: u16,
    pub inv_skin_pose: [f32; 16],
    pub inv_skin_pose_mask: u16,
    pub obb_min: [f32; 4],
    pub obb_max: [f32; 4],
    pub shape_vertices: Vec<ShapeVertex>,
    pub shape_infos: Vec<ShapeInfo>,
    pub shape_min_z: f32,
    pub shape_max_z: f32,
    pub bound_box_min: [f32; 4],
    pub bound_box_max: [f32; 4],
    pub bound_sphere: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShapeVertex {
    pub local_position: [f32; 4],
    pub local_normal: [f32; 4],
    pub bone_index: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShapeInfo {
    pub vertex_count: u32,
    /// Raw intbool word; nonzero means point cloud in Toolkit.
    pub point_cloud_raw: u32,
}

/// Bones plus the mirror tables stored directly after them.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub mirror_bones: Vec<i16>,
    /// Raw FlipType bytes; meanings are not interpreted here.
    pub mirror_flip_types: Vec<u8>,
    pub mirror_morphs: Vec<i16>,
}

/// Diagnostic consistency report; a failed check is a finding, not a parse error.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkeletonCheck {
    pub bone_count: usize,
    /// Bones whose parent is -1 (Toolkit treats these as locator roots).
    pub roots: usize,
    pub links_in_range: bool,
    /// Every bone's parent chain reaches -1 within bone_count steps.
    pub acyclic: bool,
    /// firstChild/nextSibling lists enumerate exactly the bones that name each parent.
    pub child_lists_match_parents: bool,
    pub max_depth: usize,
    /// Bones whose stored animHash equals anim_hash(name).
    pub name_hash_matches: usize,
    /// Largest |skin_pose x inv_skin_pose - I| element over all bones; None if any is non-finite.
    pub max_inverse_error: Option<f32>,
}

fn f32_word(r: &mut Reader<'_>) -> Result<f32, Error> {
    Ok(f32::from_bits(r.word()?))
}

fn v4(r: &mut Reader<'_>) -> Result<[f32; 4], Error> {
    let mut v = [0.0; 4];
    for x in &mut v {
        *x = f32_word(r)?;
    }
    Ok(v)
}

/// MemoryInputStream::m44: u16 mask only with USE_COMPRESSED_MATRICES (flag 4).
fn m44(r: &mut Reader<'_>, masked: bool) -> Result<([f32; 16], u16), Error> {
    let mask = if masked {
        u16::from_be_bytes(r.take(2)?.try_into().unwrap())
    } else {
        u16::MAX
    };
    let mut m = IDENTITY;
    for (i, x) in m.iter_mut().enumerate() {
        if mask & (1 << i) != 0 {
            *x = f32_word(r)?;
        }
    }
    Ok((m, mask))
}

/// MemoryInputStream::s32: zigzag ULEB with compressed integers, else BE i32.
fn s32(r: &mut Reader<'_>) -> Result<i32, Error> {
    let v = r.integer()?;
    Ok(if r.compact {
        (v >> 1) as i32 ^ -((v & 1) as i32)
    } else {
        v as i32
    })
}

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn bone(r: &mut Reader<'_>, masked: bool, cancel: &impl Fn() -> bool) -> Result<Bone, Error> {
    checked(cancel)?;
    let raw = r.take(32)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    let name = raw[..end]
        .iter()
        .map(|&b| if b.is_ascii() { b as char } else { '\u{fffd}' })
        .collect();
    let flags = r.integer()?;
    let anim_hash = r.integer()?;
    let (parent, first_child, next_sibling) = (s32(r)?, s32(r)?, s32(r)?);
    let (skin_pose, skin_pose_mask) = m44(r, masked)?;
    let (inv_skin_pose, inv_skin_pose_mask) = m44(r, masked)?;
    let (obb_min, obb_max) = (v4(r)?, v4(r)?);
    let n = r.count(MAX_SHAPE_ROWS)?;
    let mut shape_vertices = Vec::with_capacity(n as usize);
    for _ in 0..n {
        shape_vertices.push(ShapeVertex {
            local_position: v4(r)?,
            local_normal: v4(r)?,
            bone_index: r.integer()?,
        });
    }
    let n = r.count(MAX_SHAPE_ROWS)?;
    let mut shape_infos = Vec::with_capacity(n as usize);
    for _ in 0..n {
        shape_infos.push(ShapeInfo {
            vertex_count: r.integer()?,
            point_cloud_raw: r.integer()?,
        });
    }
    Ok(Bone {
        name,
        name_hex: raw.iter().map(|b| format!("{b:02x}")).collect(),
        flags,
        anim_hash,
        parent,
        first_child,
        next_sibling,
        skin_pose,
        skin_pose_mask,
        inv_skin_pose,
        inv_skin_pose_mask,
        obb_min,
        obb_max,
        shape_vertices,
        shape_infos,
        shape_min_z: f32_word(r)?,
        shape_max_z: f32_word(r)?,
        bound_box_min: v4(r)?,
        bound_box_max: v4(r)?,
        bound_sphere: v4(r)?,
    })
}

fn shorts(r: &mut Reader<'_>, max: u32) -> Result<Vec<i16>, Error> {
    let n = r.count(max)?;
    Ok(r.take(n as usize * 2)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| i16::from_be_bytes(*v))
        .collect())
}

/// Reads `array(Bone)` and the three mirror tables, leaving the reader at primitiveType.
pub(crate) fn read_skeleton(
    r: &mut Reader<'_>,
    masked_matrices: bool,
    cancel: &impl Fn() -> bool,
) -> Result<Skeleton, Error> {
    let n = r.count(MAX_BONES)?;
    let mut bones = Vec::with_capacity(n as usize);
    for _ in 0..n {
        bones.push(bone(r, masked_matrices, cancel)?);
    }
    let mirror_bones = shorts(r, MAX_BONES)?;
    let n = r.count(MAX_BONES)?;
    let mirror_flip_types = r.take(n as usize)?.to_vec();
    let mirror_morphs = shorts(r, 32)?;
    Ok(Skeleton {
        bones,
        mirror_bones,
        mirror_flip_types,
        mirror_morphs,
    })
}

/// Bone-name hash as described by RAnimation::calculateAnimationHash: the first of
/// " R " (offset 3) or else " L " (offset 2) is checked, every occurrence of it removed,
/// then h = h * 0x1003F + code unit (wrapping 64-bit), result (h * 4 mod 2^32) + offset.
pub fn anim_hash(name: &str) -> u32 {
    let (name, offset) = if name.contains(" R ") {
        (name.replace(" R ", ""), 3)
    } else if name.contains(" L ") {
        (name.replace(" L ", ""), 2)
    } else {
        (name.to_owned(), 0)
    };
    let h = name.encode_utf16().fold(0u64, |h, c| {
        h.wrapping_mul(0x1003f).wrapping_add(u64::from(c))
    });
    (h.wrapping_mul(4) as u32).wrapping_add(offset)
}

fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    // Column-major: element (row, col) at col * 4 + row.
    std::array::from_fn(|i| {
        let (col, row) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum()
    })
}

impl Skeleton {
    pub fn check(&self) -> SkeletonCheck {
        let n = self.bones.len();
        let valid = |i: i32| i == -1 || (i >= 0 && (i as usize) < n);
        let links_in_range = self
            .bones
            .iter()
            .all(|b| valid(b.parent) && valid(b.first_child) && valid(b.next_sibling));
        let mut acyclic = links_in_range;
        let mut max_depth = 0;
        if links_in_range {
            for start in 0..n {
                let (mut i, mut depth) = (self.bones[start].parent, 0usize);
                while i != -1 && depth <= n {
                    depth += 1;
                    i = self.bones[i as usize].parent;
                }
                if depth > n {
                    acyclic = false;
                    break;
                }
                max_depth = max_depth.max(depth);
            }
        }
        let child_lists_match_parents = acyclic && {
            let mut seen = vec![0usize; n];
            let mut ok = true;
            for (p, b) in self.bones.iter().enumerate() {
                let (mut c, mut steps) = (b.first_child, 0);
                while c != -1 && ok {
                    steps += 1;
                    let child = &self.bones[c as usize];
                    ok = steps <= n && child.parent == p as i32;
                    seen[c as usize] += 1;
                    c = child.next_sibling;
                }
            }
            ok && self
                .bones
                .iter()
                .zip(&seen)
                .all(|(b, &s)| s == usize::from(b.parent != -1))
        };
        let mut max_inverse_error = Some(0.0f32);
        for b in &self.bones {
            let p = mul(&b.skin_pose, &b.inv_skin_pose);
            for (i, v) in p.iter().enumerate() {
                let e = (v - IDENTITY[i]).abs();
                max_inverse_error = match max_inverse_error {
                    Some(m) if e.is_finite() => Some(m.max(e)),
                    _ => None,
                };
            }
        }
        SkeletonCheck {
            bone_count: n,
            roots: self.bones.iter().filter(|b| b.parent == -1).count(),
            links_in_range,
            acyclic,
            child_lists_match_parents,
            max_depth,
            name_hash_matches: self
                .bones
                .iter()
                .filter(|b| anim_hash(&b.name) == b.anim_hash)
                .count(),
            max_inverse_error: if n == 0 { None } else { max_inverse_error },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uleb(out: &mut Vec<u8>, mut v: u32) {
        loop {
            let b = (v & 127) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 128);
        }
    }
    fn zz(out: &mut Vec<u8>, v: i32) {
        uleb(out, ((v << 1) ^ (v >> 31)) as u32);
    }
    fn f(out: &mut Vec<u8>, v: f32) {
        out.extend_from_slice(&v.to_be_bytes());
    }
    /// Synthetic compact bone: translation-only skin pose stored with a mask.
    fn bone_bytes(out: &mut Vec<u8>, name: &str, links: [i32; 3], t: [f32; 3], shapes: bool) {
        let mut raw = [0u8; 32];
        raw[..name.len()].copy_from_slice(name.as_bytes());
        out.extend_from_slice(&raw);
        uleb(out, 0);
        uleb(out, anim_hash(name));
        for l in links {
            zz(out, l);
        }
        for sign in [1.0f32, -1.0] {
            out.extend_from_slice(&0x7000u16.to_be_bytes()); // entries 12, 13, 14
            for v in t {
                f(out, v * sign);
            }
        }
        for _ in 0..8 {
            f(out, 0.5);
        }
        uleb(out, u32::from(shapes));
        if shapes {
            for _ in 0..8 {
                f(out, 1.0);
            }
            uleb(out, 7);
        }
        uleb(out, u32::from(shapes));
        if shapes {
            uleb(out, 1);
            uleb(out, 0);
        }
        for _ in 0..14 {
            f(out, 2.0);
        }
    }
    fn skeleton_bytes() -> Vec<u8> {
        let mut b = vec![];
        uleb(&mut b, 3);
        bone_bytes(&mut b, "Bip01", [-1, 1, -1], [0.0, 1.0, 0.0], false);
        bone_bytes(&mut b, "Bip01 L Thigh", [0, -1, 2], [1.0, 0.0, 0.0], true);
        bone_bytes(
            &mut b,
            "Bip01 R Thigh",
            [0, -1, -1],
            [-1.0, 0.0, 0.0],
            false,
        );
        uleb(&mut b, 3);
        b.extend_from_slice(&[0, 0, 0, 2, 0, 1]);
        uleb(&mut b, 3);
        b.extend_from_slice(&[1, 2, 3]);
        uleb(&mut b, 1);
        b.extend_from_slice(&[0xff, 0xff]);
        b.push(6); // primitiveType, left unread
        b
    }

    #[test]
    fn anim_hash_side_offsets_and_wrapping() {
        // Independent recomputation of the documented rule on short names.
        assert_eq!(anim_hash(""), 0);
        assert_eq!(anim_hash("A"), 65 * 4);
        assert_eq!(anim_hash("Bip01 R Hand"), anim_hash("Bip01Hand") + 3);
        assert_eq!(anim_hash("Bip01 L Hand"), anim_hash("Bip01Hand") + 2);
        assert_eq!(
            anim_hash("Bip01 R Hand") & !3,
            anim_hash("Bip01 L Hand") & !3
        );
        // " R " wins when both are present; " L " is then kept in the hashed text.
        assert_eq!(anim_hash("a R b L c") & 3, 3);
        assert_ne!(anim_hash("a R b L c"), anim_hash("ab c") + 3);
        // Long names wrap in 64 bits without panicking.
        let _ = anim_hash(&"z".repeat(64));
    }

    #[test]
    fn reads_bones_masks_shapes_and_mirror_tables() {
        let b = skeleton_bytes();
        let mut r = Reader {
            b: &b,
            p: 0,
            compact: true,
        };
        let s = read_skeleton(&mut r, true, &|| false).unwrap();
        assert_eq!(r.p, b.len() - 1);
        assert_eq!(s.bones.len(), 3);
        let thigh = &s.bones[1];
        assert_eq!(thigh.name, "Bip01 L Thigh");
        assert_eq!(thigh.anim_hash, anim_hash("Bip01 L Thigh"));
        assert_eq!(
            (thigh.parent, thigh.first_child, thigh.next_sibling),
            (0, -1, 2)
        );
        assert_eq!(thigh.skin_pose_mask, 0x7000);
        assert_eq!(&thigh.skin_pose[12..16], &[1.0, 0.0, 0.0, 1.0]);
        assert_eq!(thigh.skin_pose[0], 1.0);
        assert_eq!(thigh.shape_vertices[0].bone_index, 7);
        assert_eq!(thigh.shape_infos[0].vertex_count, 1);
        assert_eq!(thigh.bound_sphere, [2.0; 4]);
        assert_eq!(s.mirror_bones, [0, 2, 1]);
        assert_eq!(s.mirror_flip_types, [1, 2, 3]);
        assert_eq!(s.mirror_morphs, [-1]);
        let c = s.check();
        assert!(c.links_in_range && c.acyclic && c.child_lists_match_parents);
        assert_eq!((c.roots, c.max_depth, c.name_hash_matches), (1, 1, 3));
        assert_eq!(c.max_inverse_error, Some(0.0));
    }

    #[test]
    fn uncompressed_streams_use_plain_words_and_full_matrices() {
        let mut b = vec![0, 0, 0, 1];
        b.extend_from_slice(&[0; 32]);
        for v in [5u32, 9, u32::MAX, u32::MAX, u32::MAX] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        for _ in 0..2 {
            for (i, v) in IDENTITY.iter().enumerate() {
                f(&mut b, if i == 12 { 0.0 } else { *v });
            }
        }
        b.extend_from_slice(&[0; 32]);
        b.extend_from_slice(&[0; 8]); // empty shape arrays
        b.extend_from_slice(&[0; 8 + 48]);
        b.extend_from_slice(&[0; 12]); // empty mirror tables
        let mut r = Reader {
            b: &b,
            p: 0,
            compact: false,
        };
        let s = read_skeleton(&mut r, false, &|| false).unwrap();
        assert_eq!(r.p, b.len());
        let bone = &s.bones[0];
        assert_eq!((bone.flags, bone.anim_hash, bone.parent), (5, 9, -1));
        assert_eq!(bone.skin_pose_mask, u16::MAX);
        assert_eq!(bone.name, "");
    }

    #[test]
    fn checks_report_bad_links_cycles_and_budgets() {
        let mut s = {
            let b = skeleton_bytes();
            read_skeleton(
                &mut Reader {
                    b: &b,
                    p: 0,
                    compact: true,
                },
                true,
                &|| false,
            )
            .unwrap()
        };
        s.bones[0].parent = 2;
        let c = s.check();
        assert!(c.links_in_range && !c.acyclic && !c.child_lists_match_parents);
        s.bones[0].parent = 9;
        assert!(!s.check().links_in_range);
        s.bones[0].parent = -1;
        s.bones[2].next_sibling = 1;
        assert!(!s.check().child_lists_match_parents);
        s.bones[0].skin_pose[0] = f32::NAN;
        assert_eq!(s.check().max_inverse_error, None);
        // Counts above the budget and truncated input are errors, not partial skeletons.
        let mut b = vec![];
        uleb(&mut b, MAX_BONES + 1);
        assert!(matches!(
            read_skeleton(
                &mut Reader {
                    b: &b,
                    p: 0,
                    compact: true
                },
                true,
                &|| false
            ),
            Err(Error::Budget)
        ));
        let b = skeleton_bytes();
        assert!(read_skeleton(
            &mut Reader {
                b: &b[..b.len() / 2],
                p: 0,
                compact: true
            },
            true,
            &|| false
        )
        .is_err());
        assert!(matches!(
            read_skeleton(
                &mut Reader {
                    b: &b,
                    p: 0,
                    compact: true
                },
                true,
                &|| true
            ),
            Err(Error::Cancelled)
        ));
    }
}
