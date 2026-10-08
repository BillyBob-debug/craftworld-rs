# Mesh bones and mirror tables (runtime plan item Ra)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. First step of
`docs/runtime-research.md` phase R1 (animated Sackboy in the viewer). Independent Rust;
no community source translated; no rendering, posing or skinning change.

## What changed

- `lbp_mesh::skeleton` (new): `read_skeleton` replaces `geometry::skip_bone` and the
  three mirror-table skips. `Geometry.skeleton` now carries every stored bone: name
  (32-byte field to the first NUL, raw bytes kept as hex), flags, animHash,
  parent/firstChild/nextSibling, `skinPoseMatrix` and `invSkinPoseMatrix` (16 stored
  floats, identity-filled where a mask omits entries, plus the raw u16 mask), OBB,
  per-bone shape vertices/infos, shape z range, bound box and sphere; then
  `mirrorBones` (i16), `mirrorBoneFlipTypes` (raw bytes) and `mirrorMorphs` (i16, max 32).
  Byte consumption is unchanged, so `parsed_geometry_bytes`, triangles and every
  existing census number stay the same.
- `lbp_mesh::anim_hash(name)`: the bone-name hash rule, used to self-check stored hashes.
- `Skeleton::check()` → `SkeletonCheck`: diagnostic only (a failed check is a finding,
  not a parse error): link ranges, parent cycles, firstChild/nextSibling lists agree with
  parents, roots (parent -1, Toolkit's locator roots), max depth, bones whose stored
  animHash equals `anim_hash(name)`, and max |skin x inverse - I|.
- `lbp-preview` example `corpus_skeleton_census` for the PC check below.

## Rules and provenance

All CONFIRMED_FROM_COMMUNITY_CODE, Toolkit
[d6a2ef80](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib):

| Rule | Source |
|---|---|
| Bone field order: str(32), i32 flags, AnimBone, m44, m44, v4 obbMin/Max, array(MeshShapeVertex), array(MeshShapeInfo), f32 minZ/maxZ, v4 x3 | `structs/mesh/Bone.java` serialize |
| AnimBone: i32 animHash, s32 parent/firstChild/nextSibling | `structs/animation/AnimBone.java` serialize |
| s32 with compressed integers = zigzag ULEB, else BE i32 | `io/streams/MemoryInputStream.java` s32 |
| m44: u16 mask only when USE_COMPRESSED_MATRICES (flag 4); bit i set = float i stored; others identity; float order is JOML `set(float[])` (column-major) | MemoryInputStream m44, `enums/CompressionFlags.java` |
| MeshShapeVertex {v4, v4, i32}; MeshShapeInfo {i32, intbool} | `structs/mesh/MeshShapeVertex.java`, `MeshShapeInfo.java` |
| bones, shortarray mirrorBones, enumarray(u8) mirrorBoneFlipTypes, shortarray mirrorMorphs, enum8 primitiveType | `resources/RMesh.java` serialize |
| Hash: if name contains " R " remove all, offset 3; else if " L " remove all, offset 2; h = h*0x1003F + UTF-16 unit (64-bit wrap); (h*4 mod 2^32) + offset | `resources/RAnimation.java` calculateAnimationHash |

Applicability: same as D2 mesh geometry (revision >= 567; local LBP2 meshes).
Conflict fixed: `skip_bone` read the matrix mask when compressed *integers* (flag 1)
were set; the rule is flag 4. Local meshes use flags 0 or 7 (D27/D81 censuses), where
both agree; flags with only one of the two bits would now parse correctly.
Budgets: 4096 bones (unchanged), 4096 shape rows per bone, 32 mirror morphs; truncation,
budget overflow and cancellation return errors.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0):
- `cargo fmt --all --check`: clean.
- `cargo test -q --locked <CI crates>`: all pass (4 new `skeleton` unit tests; the
  existing geometry tail test also asserts the bone and mask now decoded).
- `cargo clippy -q --locked <CI crates> --all-targets -- -D warnings`: clean.
- `cargo check -q --locked -p lbp-viewer --all-targets`: clean.

Not tested here: `lbp-bevy-viewer` check (left to CI), any game data.

## PC check (Integrator)

```powershell
cargo run -q --release -p lbp-preview --example corpus_skeleton_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/NEW-skeleton-census.json
```

Expected: decoded + failed equals the D81/D105 corpus mesh run (4,257 viewable implies
no new failures); links out of range, parent cycles and child-list mismatches 0;
name-hash mismatched bones 0 (any nonzero is a finding to record, not a reader bug by
itself); inverse outliers listed for review. The shared-signature list should show the
Sackboy skeleton (43 bones, `Dummy01`, `Bip01 ...` names per runtime-evidence/sackboy.md)
grouping the body and costume pieces; this answers runtime checks S1 and part of S7.

Counts: +0 newly viewable, +0 newly Complete (no rendering change; unit distinct
catalog GUID, baseline D105 corpus 4,257 / 3,700 / 0).

## Next

Rb: skin streams (joint bytes 3/7/11/15 of stream 1, weights) and morph streams; then Rc
ANMb keyframes and Rd the Bevy skinned preview.
