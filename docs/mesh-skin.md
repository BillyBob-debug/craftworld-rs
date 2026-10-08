# Mesh skin and morph streams (runtime plan item Rb)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Second step of
`docs/runtime-research.md` phase R1, after D110 (bones). Independent Rust; no community
source translated; no rendering, posing or skinning change.

## What changed

- `lbp_mesh::decode_skin(bytes)` → `SkinStreams`: per vertex, four joint bytes and four
  weights from stream 1, plus its three packed 24-bit normals (normal, tangent, smooth
  normal); one `MorphTarget` per morph stream (name, position offsets, unpacked normals).
  `SkinMetadata` (serializable) carries counts and findings: largest weighted joint,
  single-bone vertices, weight-byte overflow, clamped normals, morph names.
- `unpack_normal24` / `unpack_normal32` exposed for tests and later use.
- `lbp-preview` example `corpus_skin_census` for the PC check below.

## Rules and provenance

All CONFIRMED_FROM_COMMUNITY_CODE, Toolkit
[d6a2ef80](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RMesh.java):

| Rule | Source |
|---|---|
| Header counts order numVerts, numIndices, numEdgeIndices, numTris, streamCount, attributeCount, morphCount; 32 morph names of 16 bytes; streams are `numVerts * 0x10` bytes each after streamCount + 2 offset words (offsets skipped) | `RMesh.serialize` |
| Stream 1 = skinning. Joints: bytes 3, 7, 11, 15 | `RMesh.getJoints` |
| Weights: (b2, b1, b0, 255 - b2 - b1 - b0) / 255; if b2 == 0xFF then (1, 0, 0, 0) | `RMesh.getWeights` |
| Normal at byte 4, smooth normal at 8, tangent at 12, each a BE u24 (conflict: the stream constant's name lists tangent before smooth normal; the getters' offsets are followed) | `getNormals`, `getSmoothNormals`, `getTangents` |
| u24: x = bits 0-10 with sign bit 11 (negative: -(2048 - x) / 2047); y = bits 12-21 with sign bit 22 (negative: -(1024 - y) / 1023); z = sqrt(1 - x² - y²), negated if bit 23 | `util/Bytes.unpackNormal24` |
| Morph stream m (m < morphCount) = stream m + 2; per vertex BE f32 x3 offset + u32 normal | `RMesh.getMorphs` |
| u32: x bits 0-9 sign 10, y bits 11-20 sign 21, z bits 22-30 sign 31; negative: -(max - f) / max with max 1023/1023/511 | `util/Bytes.unpackNormal32` |

Our choices where Toolkit is silent (labelled, not format facts):
- `sqrt` of a negative radicand (Java gives NaN) is clamped to z = 0 and counted.
- A negative fourth weight (stored bytes summing above 255) is kept as decoded and counted;
  the renderer step (Rd) decides how to normalise.
- `morphCount > streamCount - 2` is Invalid; `streamCount < 2` is Unsupported("no skinning
  stream"). Morph names are the 16-byte fields to the first NUL, lossy ASCII.
- Stream 0's last four bytes (Toolkit comments call them deprecated / softbody) are not read.

Budgets: inherited from `prefix` (100,000 vertices, 34 streams, 32 morphs, streams within
16 MiB expanded); cancellation every 1,024 vertices and per morph.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0): fmt clean; CI-crate tests pass (2 unit tests
for the normal unpackers, 3 integration recipes in `crates/lbp-mesh/tests/skin.rs`); Clippy
-D warnings clean on the CI crates. Not tested here: lbp-bevy-viewer (CI), game data.

## PC check (Integrator)

```powershell
cargo run -q --release -p lbp-preview --example corpus_skin_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/NEW-skin-census.json
```

Expected: decoded + failed = 4,245 mesh hashes (failures should be "no skinning stream" or
the 2 known D110 failures). Weighted joints past the bone list: 0 (a nonzero count is a
finding). Weight overflow and clamped normals: report the counts. The morph-name sets
should include the Sackboy face set (runtime check S2: expected about 29 names on the
43-bone skeleton meshes; D110 found 2,047 meshes on that skeleton).

Counts: +0 newly viewable, +0 newly Complete (no rendering change; baseline D110
4,257 / 3,700 / 0, unit distinct catalog GUID).

## Next

Rc: ANMb keyframe decode and CPU sampler; Rd: Bevy skinned preview.
