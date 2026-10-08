# D108 Shape geometry research: from PShape to a renderable mesh

2026-10-08 America/Chicago. Research and design only: no product code. This builds on
[level-research.md](level-research.md) (merged at `c4e4998`) and goes deeper on one
thing: turning a `SHAPE` + `GENERATED_MESH` Thing (+ its `RBevel`) into triangles. It
feeds plan items **L1** (scene dump fields), **L5** (SHAPE/Polygon/GENERATED_MESH
parsing), **Lv** (RBevel reader), **Lx** (extrusion v1) and **Ly** (bevel profiles and
side UVs).

Evidence labels follow `docs/formats.md`. Sources, in AGENTS.md order:

- **Toolkit/cwlib (MIT)** at pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`, local
  checkout `references/toolkit`. This is the format spec and the oracle. Files below are
  relative to `lib/cwlib/src/main/java/cwlib/`.
- **LBP Asset Studio** (MindOfBog/lbp_bgm @ `f015b676dc0938ed88b94b6d9b473fd9d25b49e0`,
  `references/lbp_bgm`). No licence file. The extruder
  `src/main/java/bog/lbpas/view3d/managers/assetLoading/ShapeUtil.java` says it was
  "ported from the original PS3 GenerateMesh / BevelFunction implementation" and has
  several `todo`s and guessed constants. Under AGENTS.md this counts only as a
  **behaviour reference**: it may derive from decompiled game code, so nothing may be
  copied, adapted or translated from it. Rules taken from it are marked
  `COMMUNITY_BEHAVIOUR (LBPAS)` and count as STRONG_INFERENCE at best. They are not
  oracle-grade.
- **Public documentation**: searched on 2026-10-08 ("LittleBigPlanet bevel PShape
  polygon generated mesh thickness layers format"). No relevant LBP write-up was found.
  `references/lbp-documentation-wiki` has only `Home.md`. `cwlib-rs` and
  `lbptoolspy-lite` (GPL-3.0, not used) have no generation code.
- **Controlled local experiments** (read-only, user's 01.33 data): pinned Toolkit
  `RBevel`, `RPlan` and `RLevel` deserializers run over every bevel, level and plan in
  `d68-inventory-0133-verified.json`. Outputs and tool sources are under ignored
  `local-analysis/d108-*` (listed in section 5). No executable investigation was done.

Counting unit for local numbers: unique payload SHA1 for resources. For shapes, one
Thing with a SHAPE part, counted per containing resource (a shape that appears in two
levels counts twice).

## 1. Data model

### 1.1 Which Things draw a generated mesh

| # | Rule | Label | Source |
|---|---|---|---|
| T1 | A shape's render inputs are split across two parts: `PShape` (polygon, thickness, bevelSize, colour, physics material) and `PGeneratedMesh` (gfx material, bevel, uvOffset, visibility). The file stores **no generated vertices**. | CONFIRMED_FROM_COMMUNITY_CODE | `structs/things/parts/PShape.java`, `PGeneratedMesh.java` serialize |
| T2 | 410 of 423 level payloads deserialize with Toolkit (13 fail: 6 array errors, 5 "test serialization marker invalid", 2 PShape ClassCast). They hold 753,574 Things and 277,461 SHAPE parts. All 12,905 plans deserialize: 231,911 Things, 31,225 shapes. | CONFIRMED_FROM_LOCAL_DATA | `d108-level-resources.jsonl`, `d108-plan-resources.jsonl` |
| T3 | Level shapes split into: 201,217 SHAPE+GENERATED_MESH without RENDER_MESH (72.5%), 59,432 SHAPE+RENDER_MESH without GENERATED_MESH, 16,804 with neither, and 8 with both. | CONFIRMED_FROM_LOCAL_DATA | `d108-shape-crosstabs.json` |
| T4 | Draw a shape only when it has GENERATED_MESH. With RENDER_MESH alone, the shape is the collision hull of a prebuilt mesh. With neither, it is an invisible collider (e.g. 2,775 SPIKE shapes and 13,988 plain ones). LBPAS draws a shape that has no GENERATED_MESH with alpha 0. | STRONG_INFERENCE | T3 plus COMMUNITY_BEHAVIOUR (LBPAS `core/types/Thing.java` generateShape) |
| T5 | `PGeneratedMesh.visibilityFlags`: PLAY_MODE=1, EDIT_MODE=2. In levels, 201,075 shapes are 3 and 150 are 2 (edit-only). Hide edit-only shapes in a play view. | CONFIRMED_FROM_COMMUNITY_CODE (`enums/VisibilityFlags.java`) / CONFIRMED_FROM_LOCAL_DATA |
| T6 | `noBevel`, `sharded`, `includeSides` and the slide fields are LBP3-only (subVersion gates). They are always at their defaults in LBP2 data (noBevel false, includeSides true on all 201,225). The Vita `uvMode`/`textureScale` fields are branch 0x4431 only. | CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA | `PGeneratedMesh.java` serialize |
| T7 | No shape Thing carries MATERIAL_OVERRIDE or MATERIAL_TWEAK (0 of 277,461). These can be ignored for shapes in v1. | CONFIRMED_FROM_LOCAL_DATA | `d108-shape-summary.json` |

### 1.2 Polygon, loops, winding, holes

| # | Rule | Label | Source |
|---|---|---|---|
| P1 | For head >= 0x341, the layout is: `i32` vertex count, `bool requiresZ`, then the vertices as v3 (if requiresZ) or v2 (z=0), then `intvector loops`. Before 0x341: count, v3 always, loops. | CONFIRMED_FROM_COMMUNITY_CODE | `structs/things/components/shapes/Polygon.java` serialize |
| P2 | `loops` holds **vertex counts per loop**, laid out back to back in the vertex array (default `{4}` for the 4-vertex default square). The counts sum to the vertex count in 277,461 of 277,461 level shapes. | CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA | `Polygon.java` defaults, `PShape(Vector3f[])` ctor; `d108-shape-summary.json` loopsum_ok |
| P3 | Loop 0 is the outer boundary; later loops are holes. The Toolkit comment says "if two loops intersect with each other, it counts as a cut". In levels, 265,986 shapes have 1 loop, 8,116 have 2, and the maximum is 45 (34 distinct loop counts). | CONFIRMED_FROM_COMMUNITY_CODE (comment) / STRONG_INFERENCE (outer-first order, also LBPAS's assumption) / CONFIRMED_FROM_LOCAL_DATA (counts) |
| P4 | **Holes wind opposite to the outer loop**: 22,412 opposite vs 12 same (plans: 1,555 vs 1). | CONFIRMED_FROM_LOCAL_DATA | signed shoelace area per loop, `d108-shape-summary.json` |
| P5 | **Outer winding depends on whether the transform is mirrored**. In local xy, the outer loop is clockwise when `det(worldPosition 3x3) > 0` (217,854) and counter-clockwise when `det < 0` (40,783). So it is clockwise in world space for 93.3% of shapes. The remaining 18,716 (15,806 ccw/pos, 2,910 cw/neg) and 108 zero-area outers show the rule is not guaranteed. **Compute the signed area; never assume a winding.** | CONFIRMED_FROM_LOCAL_DATA | `d108-shape-crosstabs.json` sign_x_det |
| P6 | Polygon `z` does not set the slab depth. requiresZ=false always gives z=0. When requiresZ=true, z is constant per polygon (span 0 in 252,730 of 277,461) and, when non-zero, equals **minus the Thing's world z translation** (e.g. z=200 with tz=-200; z=-100 with tz=+100). Shapes with z=0 and shapes with z=-tz sit at the same tz layer values, and LBPAS ignores vertex z. Treat z as non-geometric and centre the slab on the Thing's origin. | STRONG_INFERENCE (from CONFIRMED_FROM_LOCAL_DATA) | `d108-shape-z.json` |
| P7 | Vertex counts per level shape: 120,859 have 4 or fewer, 49,454 have 5-8, 96,991 have 9-32, 9,826 have 33-128, and 331 have more than 128 (maximum 442). | CONFIRMED_FROM_LOCAL_DATA | |

### 1.3 Thickness, depth and layers

| # | Rule | Label | Source |
|---|---|---|---|
| D1 | PShape fields: `thickness` f32, `massDepth` f32 (head >= 0x227), `bevelSize` f32, `COM` m44 (head <= 0x340 or >= 0x38e). | CONFIRMED_FROM_COMMUNITY_CODE | `PShape.java` serialize |
| D2 | thickness and massDepth come in fixed pairs: 90/1 (114,323), 10/0.2 (97,788), 70/1 (31,865), 190/2, 290/3, 500/5. So `thickness = 100*layers - 10` for thick material and 10 for a thin layer. | CONFIRMED_FROM_LOCAL_DATA | `d108-shape-summary.json` thick_vs_massDepth |
| D3 | **`thickness` is a half-depth: the slab spans z in [-thickness, +thickness] around the Thing origin.** Evidence: root generated shapes with thickness 90 sit at tz in {0, -200, -400}, and thin (10) shapes sit at tz in {+100, -100, -300, -500}. With half-depth, the slabs tile exactly: [-90,90], [-110,-90], [-290,-110], [-310,-290] and so on. With full depth (±45) there would be 110-unit gaps. LBPAS's depth formula also maps the profile from -thickness to +thickness. (Its no-bevel fallback uses ±thickness/2, which is internally inconsistent.) | STRONG_INFERENCE | `d108-shape-z.json`, COMMUNITY_BEHAVIOUR (LBPAS `bevelZFromNormalized`) |
| D4 | The front face is at **+z**, towards the camera. Layers go back towards -z, and the extra thin front layer is at +100. LBPAS gives the front cap normal +z and puts it at the last bevel vertex (z=+1). | STRONG_INFERENCE | D3 data; LBPAS |
| D5 | The Thing's world transform is not always unit scale. Level shapes have 2,700 distinct z-scale values (212,859 are 1.0; 25,012 are 5.564). LBPAS divides thickness by the z-scale so the generated depth stays world-sized. Whether the game does this is unknown. | CONFIRMED_FROM_LOCAL_DATA (scales) / UNKNOWN (behaviour) | `d108-shape-summary.json` scale_z; LBPAS `extrudePolygon` |
| D6 | 147,446 level shapes have a `parent`, but `PPos.worldPosition` is already a world matrix (level-research.md). v1 places each shape by worldPosition only. | CONFIRMED_FROM_COMMUNITY_CODE (PPos) / STRONG_INFERENCE (sufficient) |

### 1.4 Bevel resources (RBevel)

| # | Rule | Label | Source |
|---|---|---|---|
| B1 | RBevel layout, with no revision gates: `i32 4`, 4 gfx-material descriptors, `i32 4`, 4 f32 UV scales, `autoSmoothCutoffAngle` f32, `array<BevelVertex>`, `includeBackface` bool, `smoothWithFront` bool, `relaxStrength` f32, `subDivRadius` f32, `fixedBevelSize` f32, `spongy` bool, soft-physics settings descriptor, `textureRepeats` f32. BevelVertex is `y` f32, `z` f32, `rigidity` f32, `smoothWithPrevious` i8, `gmatSlot` i8, `mappingMode` enum8 (HIDDEN 0, DAVE 1, CYLINDER_01 2, CYLINDER 3, PLANARXZ_01 4, PLANARXZ 5). | CONFIRMED_FROM_COMMUNITY_CODE | `resources/RBevel.java`, `structs/bevel/BevelVertex.java`, `enums/MappingMode.java` |
| B2 | 01.33 has 148 BEVb locations (146 unique SHA1, 144 `.bev` MAP rows), stored at 73-265 B, with heads 306-1014 across 50 cohorts (incl. 10 at 626/0x4c44). **All 146 unique payloads decode with Toolkit RBevel.** Byte-exact consumption was not checked. | CONFIRMED_FROM_LOCAL_DATA | `d108-bevel-census.json`, `d108-bevel-survey.jsonl`, `d108-bevel-summary.json` |
| B3 | Each bevel has 2-15 profile vertices (most often 6). Mapping modes per vertex: DAVE 499, CYLINDER_01 345, CYLINDER 139, PLANARXZ_01 31, HIDDEN 30, PLANARXZ 0. gmatSlot per vertex: 0 (558), 1 (331), 2 (139), 3 (16). UV scales are [1,1,1,1] in all 146. includeBackface is true in 9; spongy in 20. | CONFIRMED_FROM_LOCAL_DATA | `d108-bevel-summary.json` |
| B4 | Profile coordinates: **z runs from about -1 (back) to about +1 (front)**, sometimes overshooting (first z ≈ -1 in most uses; the most-used bevel ends at z=1.5). **y is a radial offset in bevelSize units, with negative meaning inset** (last y is negative in 115,594 of 154,242 bevelled level shapes). | CONFIRMED_FROM_LOCAL_DATA (ranges) / STRONG_INFERENCE (meaning; matches LBPAS) | `d108-bevel-survey.jsonl`, `d108_bev2.py` output |
| B5 | Depth mapping (LBPAS): for \|z\| <= 0.5, depth scales with (2·thickness - bevelSize); in the outer bands it is offset from ±thickness in bevelSize units. This keeps the bevel band a fixed world size whatever the slab thickness. Effective bevel size is `PShape.bevelSize` combined with `RBevel.fixedBevelSize`. LBPAS's active code adds them; its commented-out older code used fixedBevelSize, with -1 meaning "use shape.bevelSize". 7 bevels have fixedBevelSize -1. | COMMUNITY_BEHAVIOUR (LBPAS) / **UNKNOWN which combination is right** |
| B6 | Usage: 23.2% of generated level shapes have **no bevel** (46,732 of 201,225). 134 distinct bevels are used; the top 5 are g99327 (38,442; 2 vertices, a near-straight wall from z=-1 to 1.5), g10790 (26,830), g10783 (10,473), g10813 (9,030) and g10792 (8,896). 143 distinct bevel GUIDs are referenced, all with MAP rows. | CONFIRMED_FROM_LOCAL_DATA | `d108-shape-summary.json`, `d108-bevel-census.json` |
| B7 | What the game draws for a shape with no bevel is unknown. LBPAS substitutes a 2-vertex straight wall (z -1 to 0.5, slot 0, DAVE). | UNKNOWN / COMMUNITY_BEHAVIOUR |

### 1.5 Faces and materials

| # | Rule | Label | Source |
|---|---|---|---|
| M1 | Three different "materials" exist. `PShape.material` is the **physics** RMaterial (ResourceType MATERIAL), never a render material. `PGeneratedMesh.gfxMaterial` is the render gmat. `RBevel.materials[0..3]` are gmat slots referenced by `BevelVertex.gmatSlot`. | CONFIRMED_FROM_COMMUNITY_CODE | `PShape.java`, `PGeneratedMesh.java`, `RBevel.java` |
| M2 | **Slot 0 means the shape's own `gfxMaterial`.** Slots 1-3 are the bevel's extra gmats (trims, edges). Evidence: slot 0 is null in 109 of 146 bevels (81.6% of bevel uses), while vertices using slots 1-3 nearly always have a material (330/331, 136/139, 16/16). LBPAS maps slot 0 to gfxMaterial and other slots to bevel materials. | STRONG_INFERENCE | `d108-bevel-summary.json` vertex_slot_has_material; LBPAS `AsyncModelMan.extrudeShape` |
| M3 | The front face takes the material of the **last** profile vertex's slot, which is 0 in 97.0% of bevelled level shapes. Back-face material comes from the first vertex. Side strips take each profile vertex's slot. | STRONG_INFERENCE (from LBPAS) / CONFIRMED_FROM_LOCAL_DATA (slot frequencies) |
| M4 | A back cap is drawn only when `includeBackface` is set (true for 1,898 of 154,242 bevelled level shapes). LBPAS leaves its back cap commented out. | STRONG_INFERENCE |
| M5 | When a bevel's slot 0 is non-null (37 bevels, e.g. g10813 slot0 = g10803), it is unknown whether it overrides gfxMaterial. | UNKNOWN |
| M6 | Level shapes use 439 distinct gfx materials (plans: 748). How many have a supported diffuse route has not been measured (see Q6). | CONFIRMED_FROM_LOCAL_DATA / UNKNOWN |
| M7 | `PShape.color` (ARGB) plus `brightness` and `colorOpacity` are the likely per-instance colour for THING_COLOR gmats (D86/D88 currently need a supplied colour). Most common values: white (106k), `ff1b1b1b` (33k), `ff4d4d4d` (31k). | STRONG_INFERENCE (needs check) | `d108-shape-crosstabs.json` color_gen |
| M8 | Lethal shapes (FIRE, ELECTRIC, GAS1-6, ...) make up 3.9% of generated shapes. LBPAS drops the bevel for GAS shapes. The game probably adds effects that are not stored in the shape. | COMMUNITY_BEHAVIOUR / UNKNOWN |

### 1.6 UVs

| # | Rule | Label | Source |
|---|---|---|---|
| U1 | `uvOffset` is a v4. In LBP2 data, z and w are almost always 0. x and y fall in [0,1) and look random per shape (23,976 distinct values; 20.2% all-zero). It is likely a randomized texture offset, or the in-game UV tool. | CONFIRMED_FROM_LOCAL_DATA / STRONG_INFERENCE (purpose) | `d108-shape-summary.json` uv |
| U2 | Front-face UVs are planar in the shape's xy, measured **after the Thing's rotation/scale but without translation**, multiplied by a constant, plus uvOffset.xy (UV1 uses .zw), with v flipped. LBPAS uses 0.002 and calls it "random value that seems to work". | COMMUNITY_BEHAVIOUR (LBPAS) / **scale constant UNKNOWN** |
| U3 | Side UVs: u is the cumulative perimeter length along the loop divided by bevelSize. v is the arc length along the bevel profile, normalised so the whole profile spans `textureRepeats`. mappingMode changes the projection: CYLINDER* multiplies u by textureRepeats, PLANARXZ* projects onto xz, and HIDDEN puts the strip at UV 0 (or does not draw it). LBPAS marks this whole block `todo`. | COMMUNITY_BEHAVIOUR (LBPAS, low confidence) / UNKNOWN |
| U4 | Smoothing: an edge is sharp when the angle between adjacent edge normals exceeds `autoSmoothCutoffAngle` (45° in 111 of 146 bevels). `smoothWithPrevious=0` breaks smoothing between profile rings. `relaxStrength`/`subDivRadius`/`spongy` subdivide and relax the outline before extrusion. | COMMUNITY_BEHAVIOUR (LBPAS) / UNKNOWN |

### 1.7 Stored generated-mesh caches

| # | Rule | Label | Source |
|---|---|---|---|
| C1 | **No LBP2 part stores generated vertices.** PGeneratedMesh holds only parameters (T1). Toolkit's only "GENERATED_MESH" packet (`Thing.java` `DATA_GENERATED_MESH = 0x4746584D`) belongs to the custom ALEAR branch and is not LBP2 data. | CONFIRMED_FROM_COMMUNITY_CODE | `PGeneratedMesh.java`; `structs/things/Thing.java` onStartSave/onFinishSave |
| C2 | `STATIC_MESH` (SMHb, type 44) is `PLevelSettings.backdropMesh`: a prebaked **backdrop** mesh with its own Toolkit reader (`resources/RStaticMesh.java`) and glTF export. It is not a shape cache. 01.33 has 27 SMHb resources, referenced by 201 level and 34 plan dependency entries. It is a separate, prebuilt geometry route worth a later item (Lz). | CONFIRMED_FROM_COMMUNITY_CODE / CONFIRMED_FROM_LOCAL_DATA | `PLevelSettings.java`, `d108_cache.py` output |
| C3 | `CACHED_LEVEL_DATA` (CLDb, type 22): 17 resources on 01.33. Toolkit has no reader and nothing references them by dependency. Contents unknown. | UNKNOWN |
| C4 | `RMesh.bevelVertexCount` and `implicitBevelSprings` are softbody/mesh fields, not a shape-mesh cache. | CONFIRMED_FROM_COMMUNITY_CODE | `resources/RMesh.java` |

Conclusion: **shape geometry has to be generated.** Nothing in LBP2 data lets us skip it.

## 2. Proposed minimal first implementation (Lx, "extrusion v1")

Prerequisites: L5 parses SHAPE/Polygon/GENERATED_MESH (or the L1 oracle dump supplies
the same fields). Add these to the L1 dump: polygon vertices (xy), loops, thickness,
bevelSize, colour, gfxMaterial, bevel, uvOffset, visibilityFlags, worldPosition.

Put the generator in a pure, non-UI module (e.g. `lbp-level::shape_mesh` or
`lbp-mesh::extrude`). It outputs an intermediate mesh (positions, normals, UV0, indices,
one material id per face group), which the Bevy layer converts. This follows AGENTS.md
rule 5.

1. **Select**: SHAPE with GENERATED_MESH, visibility bit PLAY_MODE set, gfxMaterial
   non-null. Count skipped shapes per reason (no GENERATED_MESH, edit-only, degenerate,
   triangulation failure, over budget) and show the counts in the panel.
2. **Validate** with checked arithmetic: loops sum to the vertex count, every loop has at
   least 3 vertices, coordinates are finite, and the per-shape vertex and loop budgets
   hold (e.g. 4,096 vertices and 64 loops; observed maximums are 442
   vertices and 45 loops).
3. **Normalise winding**: compute each loop's signed area in local xy and make the outer
   loop CCW and the holes CW for the triangulator. Do not rely on P5. Ignore polygon z
   (P6).
4. **Triangulate the front cap** with holes using ear clipping with hole bridging. This
   needs a decision: an in-repo implementation from the published algorithm, or an
   earcut crate after a licence review (e.g. `earcutr`, ISC; verify before adding).
   Fall back to "skipped: triangulation" and count it, never panic.
5. **Extrude**: front cap at z=+thickness (D3/D4), straight side quads per loop edge from
   -thickness to +thickness with flat per-edge normals, and the back cap (reversed) at
   -thickness. The back cap costs little and hides gaps when the camera orbits; label it.
   Flip triangle order when `det(worldPosition) < 0`.
6. **UV0**: front/back planar UVs = (R·S applied to local xy) × `K` + uvOffset.xy, with v
   flipped. `K` is a named, labelled preview constant (start with 1/500 and calibrate,
   see verification). Side UVs: u = perimeter length × K, v = depth × K.
7. **Material**: all faces use gfxMaterial through the existing diffuse routes. For
   THING_COLOR materials, use PShape.color as the supplied instance colour, behind the
   existing opt-in until M7 is checked.
8. **Place** the mesh with `PPos.worldPosition`. Cache by (polygon hash, thickness,
   gfxMaterial, uvOffset) so repeated pieces share a handle.

**What v1 gets wrong (label these in the app as "shape preview approximation"):**
- No bevel profile. Edges are hard 90° walls rather than rounded or chamfered, and the
  front face is not inset. This is the most visible error, since 77% of shapes have a
  bevel, often 10-30 units wide.
- Side trims that use bevel slots 1-3 (e.g. cardboard edge gmats) show the face
  material instead.
- The front-UV scale K and the uvOffset meaning are unverified, so texture density and
  phase may differ from the game.
- No outline relax or subdivision (`relaxStrength`, `spongy`), and no smoothing normals.
- The depth convention (D3, half-depth) is inferred. If wrong, every slab is twice too
  deep, which would show as layers interpenetrating.
- Lethal effects, glow and editor-only shapes are missing; z-scaled Things may come out
  with the wrong depth (D5).

**Verification:**
- *Parse parity (oracle exists)*: L5 must match Toolkit, per shape, on every field in the
  D108 survey (vertex count, loops, thickness, bevelSize, colour, gfxMaterial, bevel,
  uvOffset, vis, worldPosition). Compare against `d108-*-shapes.jsonl` or the L1 dump.
  Target: all 410 levels and 12,905 plans.
- *Geometry invariants (synthetic, cloud-ok)*: front-cap triangle area equals the
  polygon area (outer minus holes) within epsilon; the closed mesh is edge-manifold;
  outward normals (sides away from the outline, front +z); identical output under
  winding reversal and under mirrored transforms; hole count preserved; malformed
  loops, NaN and budget recipes are refused.
- *Layer tiling check (local data)*: per level, the generated slab z-ranges of root
  shapes should tile without overlap (D3). This is a quantitative test of the half-depth
  inference.
- *No geometry oracle exists.* Toolkit does not generate shapes. LBPAS is unlicensed and
  game-derived, so at most it could be run by the user as a black-box visual reference;
  that is the user's decision, and its code is never read into the product. Visual
  truth comes from user screenshots of chosen story levels (Ly). Calibrate K against a
  textured gmat whose texel size is known relative to Sackboy.
- *Counts*: report newly viewable levels (parse fully and draw at least one shape) and
  shapes drawn vs skipped by reason. Shapes stay non-Complete while bevels are missing.

**Ly (bevel v2)** then adds: RBevel parsing (layout B1, all 146 parse via the oracle), a
profile ring per BevelVertex (outline offset by y·bevelSize along the averaged edge
normal, depth by the B5 mapping), a front cap from the inset outline, slot materials
(M2/M3), the includeBackface cap, smoothing by autoSmoothCutoffAngle and
smoothWithPrevious, and per-mode side UVs. Each step should be checked against
screenshots, because no oracle covers it.

## 3. Open questions, ranked by visual impact

1. **Bevel geometry and size (B5, B7).** How do `bevelSize` and `fixedBevelSize`
   combine? What does the exact depth mapping of profile z look like, how is the outline
   offset at sharp and concave corners (miter vs averaged normal; self-intersection on
   thin pieces), and what is drawn when the bevel is null? This affects every shape
   edge. Resolution: user screenshots of known shapes (a square of each common bevel)
   measured against candidate formulas. Otherwise this needs executable investigation
   of the generator, which is out of scope here and would need a separate user-approved
   item.
2. **Depth convention (D3).** Half-depth is strongly supported by the tiling data, but
   it should be confirmed with one screenshot of a thick shape beside a thin one, or a
   side view.
3. **Front-face UV scale and origin (U1, U2).** Is K world-space, does it include
   translation (world-locked texture), and is uvOffset a random phase? Affects the look
   of every textured face. Resolution: screenshots of two adjacent identical pieces, and
   a piece before and after moving.
4. **Side and bevel UVs and mapping modes (U3).** Arc-length v, perimeter u, how
   CYLINDER/PLANARXZ behave, and whether HIDDEN means "not drawn". Visible on all side
   bands.
5. **Slot-0 precedence (M5)** and back-cap rules (M4). Wrong trims or missing backs on
   37 bevels.
6. **Material coverage**: how many of the 439 level gfx materials have a supported
   diffuse route (join `d108-level-shapes.jsonl` gmats with the selection census). This
   sets how many shapes look right once geometry works.
7. **THING_COLOR from PShape.color/brightness/colorOpacity (M7)**, and what the
   `ff1b1b1b`/`ff4d4d4d` defaults mean.
8. **Transform effects**: does the game divide thickness by the z-scale (D5)? Also
   handling of the 108 zero-area outlines and 12 same-winding holes.
9. **Relax and subdivision (U4)** for spongy and relaxed bevels: smoothness of outlines.
10. **13 levels Toolkit fails on (T2)** and **CLDb contents (C3)**: they affect coverage
    counts, not shape appearance.

## 4. Provenance and licensing

- Toolkit (MIT) was read for field layouts and enum values. No code was copied, adapted
  or translated; a Rust implementation will be written independently from the rules
  above.
- LBP Asset Studio has no licence and its author describes it as ported from the PS3
  binary. It was read only to state behaviour hypotheses, written here in my own words
  and labelled. No formulas are to be lifted into product code without independent
  confirmation (screenshots or user-approved investigation).
- The local survey ran the pinned Toolkit jar against the user's own read-only data.
  The survey programs are research tools under ignored `local-analysis/d108-tools/`
  and are not product code. No game bytes, decoded resources or per-shape dumps are
  committed.

## 5. Local outputs (ignored, `local-analysis/`)

| File | Content |
|---|---|
| `d108-bevel-census.json` | BEVEL type/header cohorts, dependencies, referencing levels/plans (snapshot only) |
| `d108-bevel-manifest.tsv`, `d108-bevel-survey.jsonl`, `d108-bevel-summary.json` | 146 unique bevels decoded by Toolkit RBevel; per-bevel fields; summary |
| `d108-thing-manifest.tsv` | 423 level + 12,905 plan unique payload locations |
| `d108-level-shapes.jsonl` (277,461 lines), `d108-plan-shapes.jsonl` (31,225) | Per-shape facts via Toolkit RLevel/RPlan (no vertex coordinates) |
| `d108-level-resources.jsonl`, `d108-plan-resources.jsonl` | Per-resource parse status and Thing/shape counts |
| `d108-shape-summary.json`, `d108-shape-crosstabs.json`, `d108-shape-z.json` | Aggregates cited above |
| `d108-tools/` | `BevelSurvey.java`, `ShapeSurvey.java`, `d108_*.py` used to produce the above |

Commands (PowerShell, from the main checkout; `$j` = `local-analysis/toolchain/java/jdk-21.0.12.1+1/bin`,
`$jar` = `references/toolkit/lib/cwlib/target/cwlib-0.1.jar`):

```powershell
python -I local-analysis/d108-tools/d108_bevel_census.py local-analysis/d68-inventory-0133-verified.json local-analysis/d108-bevel-census.json
python -I local-analysis/d108-tools/d108_bevel_manifest.py local-analysis/d68-inventory-0133-verified.json local-analysis/d108-bevel-manifest.tsv
python -I local-analysis/d108-tools/d108_thing_manifest.py local-analysis/d68-inventory-0133-verified.json local-analysis/d108-thing-manifest.tsv
& "$j/javac.exe" -cp $jar -d <classes> local-analysis/d108-tools/BevelSurvey.java local-analysis/d108-tools/ShapeSurvey.java
& "$j/java.exe" -Xmx256m -cp "<classes>;$jar" BevelSurvey game-data/a1-disc game-data/a1-update-0133 local-analysis/d108-bevel-manifest.tsv local-analysis/d108-bevel-survey.jsonl
& "$j/java.exe" -Xmx3g -Xss64m -cp "<classes>;$jar" ShapeSurvey game-data/a1-disc game-data/a1-update-0133 local-analysis/d108-thing-manifest.tsv local-analysis/d108-level-shapes.jsonl local-analysis/d108-level-resources.jsonl level
#   same with "plan" -> d108-plan-shapes.jsonl / d108-plan-resources.jsonl
python -I local-analysis/d108-tools/d108_bevel_summary.py local-analysis/d108-bevel-survey.jsonl local-analysis/d108-bevel-summary.json
python -I local-analysis/d108-tools/d108_shape_summary.py local-analysis/d108-shape-summary.json local-analysis/d108-level-shapes.jsonl local-analysis/d108-plan-shapes.jsonl local-analysis/d108-level-resources.jsonl local-analysis/d108-plan-resources.jsonl
python -I local-analysis/d108-tools/d108_cross.py local-analysis/d108-shape-crosstabs.json local-analysis/d108-level-shapes.jsonl
python -I local-analysis/d108-tools/d108_z.py local-analysis/d108-shape-z.json local-analysis/d108-level-shapes.jsonl
python -I local-analysis/d108-tools/d108_bev2.py local-analysis/d108-bevel-survey.jsonl local-analysis/d108-level-shapes.jsonl
```

Results: all runs exit 0. Bevels: 146/146 decoded. Levels: 410 ok, 13 errors.
Plans: 12,905 ok. Runtimes were about 6 s for levels and 4 s for plans.

Not tested: byte-exact consumption by Toolkit for bevels and Things; levels in the
`.bin` aliases outside the snapshot's confirmed set; LBP1 heads; Vita branch; any
rendering. No native window was opened.
