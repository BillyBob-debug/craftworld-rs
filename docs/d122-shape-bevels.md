# D122 Shape bevels: RBevel reader and bevelled extrusion (plan items Lv, Ly)

2026-10-08 America/Chicago. Builds on [d108-shape-geometry.md](d108-shape-geometry.md)
(research) and [d114-shape-extrude.md](d114-shape-extrude.md) (straight extrusion).
Product code: `crates/lbp-level/src/bevel.rs` (reader), `crates/lbp-level/src/shape_mesh.rs`
(`extrude_bevelled`, `extrude_thing_bevelled`), opt-in hook in
`crates/lbp-preview/src/level_scene.rs` and the Bevy viewer flag `--shape-bevels`.

**Label shown with every bevelled shape** (`shape_mesh::BEVEL_LABEL`): "preview
approximation: bevel geometry: inferred, unverified; bevel UVs approximate, UV scale
uncalibrated". Nothing here is game-accurate until checked against screenshots. Shapes stay
non-Complete. The straight extrusion (`extrude`, D114) is unchanged and is still the default.

## 1. RBevel reader (Lv)

Source: Toolkit/cwlib (MIT) at `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
`resources/RBevel.java` serialize, `structs/bevel/BevelVertex.java`, `enums/MappingMode.java`,
`io/serializer/Serializer.java` `resource(value, type)` and `io/streams/MemoryInputStream.java`
(`guid`, `bool`, `enum8`). Read for field layout only; the Rust reader is written from the notes
below, no code copied or translated.

| # | Rule | Label |
|---|---|---|
| V1 | Layout without revision gates: i32 4, 4 gfx-material descriptors, i32 4, 4 f32 UV scales, f32 autoSmoothCutoffAngle, arraylist BevelVertex (f32 y, f32 z, f32 rigidity, i8 smoothWithPrevious, i8 gmatSlot, u8 mappingMode), bool includeBackface, bool smoothWithFront, f32 relaxStrength, f32 subDivRadius, f32 fixedBevelSize, bool spongy, descriptor softPhysicsSettings, f32 textureRepeats. | CONFIRMED_FROM_COMMUNITY_CODE; CONFIRMED_FROM_LOCAL_DATA (parity below) |
| V2 | Descriptors: i32 flags word only for head > 0x22e; kind byte GUID = 2 / HASH = 1 from head 0x191, swapped below 0x191 (local bevels exist at head 0x132). GUID 0 and an all-zero SHA-1 count as absent; neither present = None. | CONFIRMED_FROM_COMMUNITY_CODE; local heads 306..1014 parse |
| V3 | Strictness conflicts: the two "4" counts must be 4 (Toolkit ignores them; Unsupported here); mappingMode values outside 0..5 are kept raw (Toolkit yields null); trailing bytes are reported, not rejected. | Implementation choice |
| V4 | Budgets: 64 KiB stored, 256 KiB expanded, 256 profile vertices (observed max 15), counts checked against remaining bytes before allocation; DOUBLE11 and heads above LBP2_MAX refused. | Implementation choice |

**Oracle parity (01.33).** New authored oracle `scripts/BevelOracle.java` (+
`run-bevel-oracle.ps1`, manifest helper `scripts/bevel_report.py`) runs the pinned Toolkit
reader and projects every field with floats as raw IEEE bits; `crates/lbp-level/examples/bevel_parity.rs`
compares. Unit: unique bevel payload (SHA-1).

```powershell
python -I <wt>/scripts/bevel_report.py manifest local-analysis/d68-inventory-0133-verified.json local-analysis/d122-bevel-manifest.json
<wt>/scripts/run-bevel-oracle.ps1 -Output local-analysis/d122-bevel-oracle-0133.json -SampleManifest local-analysis/d122-bevel-manifest.json -ProjectRoot <main checkout>
<wt>/target/release/examples/bevel_parity.exe local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d122-bevel-oracle-0133.json local-analysis/d122-bevel-parity-0133.json
```

Results: manifest 146 unique payloads from 148 bevel locations; oracle self-test passed,
146 `parsed_exact` (Toolkit consumes every payload byte; this closes the D108 "byte-exact not
checked" gap); Rust parity **146 / 146 match**, 0 field mismatches, 1,044 profile vertices
(same total as D108 B3). Outputs (ignored): `d122-bevel-manifest.json`,
`d122-bevel-oracle-0133.json`, `d122-bevel-parity-0133.json`.

## 2. Bevelled extrusion (Ly)

Entry points: `shape_mesh::extrude_bevelled(&Shape, &GeneratedMesh, &[f32; 16], Option<&Bevel>,
&BevelOptions)` and `extrude_thing_bevelled(&ThingRecord, bevel_of, &BevelOptions)`. Output is
the same `ShapeMesh` (world space, CCW outside, mirrored transforms flipped) with new fields:
`groups` (face range + bevel slot + material), `bevel: BevelOutcome`, `slot_material_fallbacks`,
`hidden_strips`; `ShapeMesh::material_parts()` splits by material (a single-material mesh comes
back whole, so straight shapes batch exactly as in D114).

Evidence for the profile reading (local data, 146 bevels, from the D122 parity output and
`d108-bevel-survey.jsonl`):
- |z| values cluster at 1.0 (and 0.5): 375 of 1,044 vertices have |z| in [0.95, 1.05] and 135
  in [0.45, 0.55] (a 0.1-wide bin), against 119 for all of [0, 0.45).
- The most used rounded bevel (g10790) has (y, z) = (0, 0.5), (-0.167, 0.847), (-0.5, 1.0):
  within 0.02 of a circle of radius 0.5 centred at (-0.5, 0.5), i.e. a quarter round **only if
  y and z share one scale in the band 0.5..1**. g10792 is a symmetric 45 degree chamfer of
  0.5 by 0.5 at both ends ((-0.51, -1), (0, -0.5), (0, 0.5), (-0.51, 1)).
- PShape.bevelSize varies per shape for the same bevel (e.g. g99327 is used with 20, 10 and 5);
  fixedBevelSize is per bevel (58 of 146 are 10, 7 are -1). Only 34,382 of 154,493 bevelled
  level shapes in the D108 Toolkit survey have bevelSize = fixedBevelSize.

| # | Rule | Label |
|---|---|---|
| R1 | Profile vertices run back (z about -1) to front; consecutive duplicates dropped; fewer than 2 left, non-finite, or \|y\|/\|z\| > 100 falls back to straight walls (`profile_too_short` / `profile_non_finite` / `profile_oversized`). | Implementation choice |
| R2 | Bevel size b: default `BevelSizeRule::ShapeBevelSize` = PShape.bevelSize, fixedBevelSize when the shape value is not positive. Options `FixedBevelSize` and `Sum` exist for screenshot comparison. | **Inferred, unverified** (combination UNKNOWN, D108 B5) |
| R3 | Ring k = every loop's outline offset by y_k * b along the vertex miter (outward = away from material: outer loops grow, holes shrink for y > 0), miter capped at 4x. Depth: for \|z\| >= 0.5, sign(z) (t - (1 - \|z\|) s); for \|z\| < 0.5, z * 2 (t - s/2), with s = min(b, 2t) and t = PShape.thickness (half depth, D114 X4). So z = +-1 sits on the slab faces, the outer bands keep the profile isotropic, the middle band stretches; continuous at 0.5. | STRONG_INFERENCE from local data (above) for isotropy and the 0.5 break; **overall inferred, unverified**. Matches the shape LBPAS uses (COMMUNITY_BEHAVIOUR, hypothesis only, nothing copied) |
| R4 | Every ring must be a valid polygon with the input's loop orientation (same checks as the input: no self/loop intersection, holes inside, nonzero area). On failure the size is halved, up to 4 times (`BevelOptions::max_shrink_steps`), then the shape falls back to straight walls with the failing reason. | Preview choice (game behaviour on thin pieces UNKNOWN) |
| R5 | Faces: one flat-shaded quad per outline edge per profile step (ring k to k+1); front cap = triangulation of the last ring (+z normal), back cap = reversed triangulation of the first ring. The back cap is always drawn (closed surface), whatever includeBackface says. | Preview choice; includeBackface meaning UNKNOWN (D108 M4) |
| R6 | Materials: strip k (ring k to k+1) uses vertex k's gmatSlot ("leaving" vertex); front cap uses the last vertex's slot, back cap the first's. Slot 0 = GENERATED_MESH.gfxMaterial (bevel slot 0 ignored even when set, D108 M5); slots 1-3 = RBevel.materials[slot]; a null slot material or a slot outside 0..3 falls back to gfxMaterial (counted). | STRONG_INFERENCE for slot 0 / front cap (D108 M2/M3); **strip-to-vertex mapping inferred, unverified** |
| R7 | UVs: caps planar as D114 (world 3x3 x UV_SCALE + uvOffset.xy, v flipped); sides u = cumulative world outline length x UV_SCALE (same per ring), v = remaining profile arc length (local, scaled by the world z axis) x UV_SCALE, v = 0 at the last ring. mappingMode, textureRepeats and the RBevel UV scales are not applied. HIDDEN strips are drawn and counted. | **Preview approximation** (D108 U3 unknown) |
| R8 | Not modelled: autoSmoothCutoffAngle / smoothWithPrevious (flat normals only), relaxStrength, subDivRadius, spongy, rigidity, softPhysicsSettings, z-scale compensation (D108 D5). | Not implemented |

No bevel (GENERATED_MESH.bevel null): straight walls, outcome `no_bevel` (what the game draws
is UNKNOWN, D108 B7). Bevel named but not loaded: straight walls, `bevel_unavailable`.

## 3. Hook and flag

`level_scene::LevelSceneOptions { shape_bevels }` (default false),
`load_level_scene_with_options(..)` (`load_level_scene` delegates with defaults),
`add_shapes_with(scene, things, Option<&BevelTable>, ..)` (`add_shapes` = `None`). With the
option on, every distinct bevel descriptor in the level is resolved and read once (budget 1,024),
each shape is extruded with its bevel and split by material; a Thing is counted once in
`shapes_placed` and in `SceneItem::shapes` even when it spans several batches. Counts:
`shape_bevel_outcomes`, `bevels_loaded`, `bevels_failed`, shown in the level panel lines with
the label. Viewer: `lbp-bevy-viewer ... --level-path X --shape-bevels` (requires a level; also
works with `--level-report`). With the flag off the scene, batches and panel lines are as before.
The viewer's "Drawn in viewer" line now classifies items by mesh source (a shape batch can
carry 0 newly counted Things), so the model/shape Thing split stays right with bevels on.

`--level-report --shape-bevels` on tutorial_23.bin (no window): 9 shapes extruded and placed in
5 material batches (4 without bevels), 5 bevel resources loaded, 0 failed; 8 bevelled, 1
bevelled after shrinking.

## 4. Census (01.33, `shape_census --bevels`)

`shape_census SNAPSHOT GAME-DIRS NEW-JSON --bevels` keeps the D114 straight totals and tiling
check and adds a separate bevelled run. Output `local-analysis/d122-shape-census-0133.json`.
Unit: Thing with SHAPE data per containing level content hash; scope: 236 completely read
levels of 398 level hashes (142 out of band, 20 wrapper errors), as D114.

| | all 236 levels | tutorial_23.bin |
|---|---|---|
| straight extrusion OK / rejected (unchanged from D114) | 121,167 / 119 | 9 / 0 |
| bevelled at requested size | 84,485 | 8 |
| bevelled after 1 / 2 / 3 / 4 halvings | 3,503 / 1,578 / 582 / 226 (5,889) | 1 / 0 / 0 / 0 |
| **bevelled total** | **90,374** | **9** |
| bevel named, fell back to straight | 301 (217 self_intersecting, 66 loops_intersect, 18 ring_flipped) | 0 |
| no bevel (straight walls) | 30,492 | 0 |
| bevel unavailable | 0 | 0 |
| rejected (same reasons as D114) | 119 | 0 |
| shapes with more than one material | 23,602 | 6 |
| slot material fallbacks (faces groups) | 10 | 0 |
| HIDDEN-mode strips drawn | 2,044 | 0 |
| triangles / vertices | 9,630,704 / 17,780,866 | 1,620 / 3,066 |

96 distinct bevel descriptors are referenced by these levels; all 96 parse. Asset counts: +0
newly viewable, +0 newly Complete (shapes are not catalogue assets and remain non-Complete).

## 5. Tests (synthetic recipes only)

`crates/lbp-level/tests/shape_bevel.rs` (12): reader with flags 0 and 7, head 0x132 swapped
descriptor bits without flags word and a trailing byte, zero GUID / zero hash, every truncation,
material count 3, oversized vertex count, head above LBP2, cancellation, wrong magic; depth
mapping ends, continuity and monotonicity on thin slabs; chamfered square (closed, normals,
no self-intersection, exact swept volume, slot materials, groups tile the index buffer, two
compacted material parts, inset front cap); round profile on a concave L and on a square with
a hole; mirrored + scaled transform (volume = |det| x local); thin strip shrinks (and falls back
with 0 shrink steps); missing / short / NaN / zero-size bevels fall back; size rules; slot
fallbacks; straight extrusion = one unchanged part. `level_scene` test: bevelled shape split
into two material batches, Things counted once, option off gives the D114 scene.
Self-intersection is checked in the tests by an edge-through-triangle test over all
non-adjacent triangle pairs (coplanar overlaps not detected); the product only checks rings.

## 6. Not done / not tested

- No visual check: no window opened, no capture (the Integrator renders). No comparison with
  game screenshots, so R2, R3 (scale and depth), R6 (strip slot) and R7 are unverified.
- Rings are checked per ring only; strips of different loops could still cross between rings in
  rare shapes. Outward (y > 0) offsets of nearby outline parts fall back rather than merge.
- Smoothing, relax/subdivision, mapping modes, textureRepeats, includeBackface, spongy,
  z-scale compensation (R8); plans; levels the reader stops on.
