# Original levels in the Bevy viewer: research and plan

2026-10-07 America/Chicago (written 2026-10-08 01:xx UTC). Cloud research only: no game
data was read. Repo state read: main `4b73a4f` (after D91-D93). Format evidence is
Toolkit/cwlib at the project pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (upstream
HEAD equals the pin today), read from a local checkout. Labels follow docs/formats.md:
everything below is CONFIRMED_FROM_COMMUNITY_CODE unless marked otherwise. Anything that
needs the game files is listed in "PC checks", not guessed.

## 1. Short answer

A level is one `LVLb` resource holding a recursive graph of Things. Each Thing has an
ordered set of parts (position, render mesh, shape, switch, script...). Parts are
written back to back with **no per-part length**, so a reader cannot skip a part it
does not understand: to get even one mesh out of a level, every part type that occurs
anywhere in that level must be parsed exactly. Toolkit's Thing+part code is about
12,200 Java lines (55 component classes, 54 parts).

Visible geometry comes from two sources:
- `RENDER_MESH` things: a mesh GUID placed by `POS.worldPosition`. The existing mesh,
  material and texture pipeline already renders these as single assets.
- `SHAPE` + `GENERATED_MESH` things (all the cardboard/wood/stone the player builds
  with): the file stores only a 2D polygon, thickness, a bevel GUID and a gfx material.
  The game **generates the 3D mesh at load time**. Toolkit does not do this (its glTF
  `SceneExporter` skips shapes). LBP Asset Studio does, but that code is unlicensed and
  described by its author as ported from the PS3 binary, so it is behaviour reference
  only under AGENTS.md.

Recommended route: get a Toolkit-based level oracle running on the PC first (counts,
part histogram, scene dump), use its dump to bring up a dev-only Bevy "level scene" for
`RENDER_MESH` things early, then build the independent Rust Thing reader part by part
against that oracle, then shape extrusion. Section 5 has the phases; section 6 the
candidate backlog items.

## 2. How levels are stored

### Resource wrapper (already handled)
`LVLb` and `PLNb` share the ordinary binary wrapper the repo already parses
(`lbp-resource::binary::payload`): head revision, branch, compression flags,
dependency table, zlib chunks. D27 census on 01.00: 282 level `.bin` resources
(44.6 MB stored, heads 350..951, 45,946 dependency entries) and 7,342 plans; flags 0/7,
branch 0 and 0x4c44 (LEERDAMMER). LBP2 retail max head is 0x3f8 (`Revisions.LBP2_MAX`),
so subVersion is 0 and all LBP3-only (`subVersion >= ...`) fields are absent.

### RLevel payload (`resources/RLevel.java` serialize)
1. Cross-play Vita hash array only when head > 0x3e6 (strict). Not present on PS3 01.00
   samples (heads <= 951); 01.33 heads reach 0x3f8 so it **may** be present: PC check.
2. `worldThing = reference(Thing)`. This one call recursively reads the whole world.
3. Then playerRecord, tutorial inventory lists, copiedFromSomeoneElse, and a few
   revision-gated tail fields (most gated on subVersion, so absent on LBP2).

### Serializer mechanics a Rust reader needs (`io/serializer/Serializer.java`, `io/streams/MemoryInputStream.java`)
- Compression flags 1/2/4: compressed integers (ULEB, already implemented in the repo),
  compressed vectors, compressed matrices (`m44`: u16 mask, then one f32 per set bit,
  identity defaults for the rest).
- `reference(T)`: i32 tag. 0 = null; a tag already seen returns the earlier object; a
  new tag reads the object **inline, recursively, right there**. One tag table for all
  classes per serializer. Consequence: a Thing's `parent`, `groupHead`, `oldEmitter`
  and every Thing-typed field can pull an entire subtree inline, so recursion depth is
  unbounded by format. Rust needs an explicit depth budget (or an explicit stack) and a
  tag-count budget.
- `resource(...)`: for head > 0x22e and not a descriptor, an i32 flags word; then a
  guid/hash flag byte (0 none, 1 hash, 2 GUID, 3 both), then u32 GUID and/or 20-byte
  SHA1, optional i32 type. The flag meanings swap below 0x191 for class pointers
  (irrelevant for LBP2).
- `thingarray`/`arraylist(..., true)`: count then a reference per element.

### Thing (`structs/things/Thing.java` serialize), LBP2 range (0x341 <= head <= 0x3f8)
Order: 0xAA test marker (head >= 0x2a1) → UID i32 → parent ref → groupHead ref →
oldEmitter ref → createdBy i16, changedBy i16 → planGUID → flags i8 (i16 only on the
Vita branch) → partsRevision s32 → part flags u64 (compressed form, head >= 0x297) →
each present part as `reference(P*)` in `Part` enum order.

Part presence (`enums/Part.java` hasPart/fromFlags): bit `index` of the u64 flags, only
if `partsRevision >= PartHistory` value for that part. For partsRevision >=
CREATOR_ANIM and head subVersion < 0x107 (all of LBP2), indices above 0x28 shift by one
and bit 0x29 is the deprecated CREATOR_ANIM. Deprecated parts read an i32 that must be
0. For head > 0x3e2 the maximum partsRevision is STREAMING_HINT (0x3f).

### The world (`parts/PWorld.java`)
PWorld (on the world Thing) holds: backdrop offsets, a material list, the `things`
array (every Thing in the level, as references; most were already read inline through
parent links by then), physics/frame counters, editor selections, `backdrop` and
`backdropNew` Things, lighting/colour-correction factors, and more. PLevelSettings holds
sun/fog/colour presets.

### Parts that matter for drawing
| Part | Fields used for rendering |
|---|---|
| POS | `worldPosition` m44 (and `localPosition`, `thingOfWhichIAmABone`) |
| RENDER_MESH | mesh GUID, `boneThings` (Thing refs), animation GUID + pos/speed, `editorColor`, castShadows, visibilityFlags |
| SHAPE | `Polygon` (vertex count, `requiresZ` flag, v2 or v3 vertices, `loops` int vector: outer loop then holes), physical material (RMaterial, not the gfx one), thickness, colour, brightness, bevelSize, COM matrix |
| GENERATED_MESH | gfxMaterial GUID, bevel GUID (RBevel), uvOffset v4, planGUID, visibilityFlags, texture animation speeds, uvMode, textureScale, noBevel, sharded, includeSides |
| DECORATIONS / STICKERS | decals and attached decoration meshes (later) |
| REF | plan reference: the Thing stands for an instantiated `.plan` (backdrops, emitted objects) |
| GROUP, MATERIAL_OVERRIDE, MATERIAL_TWEAK | grouping and per-instance material changes (later) |

Script instances (PScript) carry their own `InstanceLayout` inline, so parsing them does
not need the RScript resource. That keeps the level reader self-contained.

### Plans (`resources/RPlan.java`)
`thingData` (already located by `plan::inspect` since D29) is `array(Thing)` read with a
fresh serializer at the plan's revision and compression flags. So the Thing reader is
shared between levels and plans, and plans make good smaller test inputs.

### Bevels (`resources/RBevel.java`)
Separate resource type (BEVEL): a list of `BevelVertex` profile points plus
fixedBevelSize, relaxStrength, spongy, textureRepeats, autoSmoothCutoffAngle. Small
reader; not parsed by the repo today.

## 3. What the repo already parses

| Need | State on main 4b73a4f |
|---|---|
| Wrapper, dependencies, zlib, ULEB/BE32 ints | Done (`lbp-resource::binary`) |
| MAP/FARC lookup by GUID/hash, 01.33 overlay | Done (`lbp-archive`, `lbp-catalog`, D69) |
| Meshes (rest geometry, UVs, primitives) | Done for 4,257 of 4,261 mesh GUIDs; bones are skipped, no skinning (`lbp-mesh/src/geometry.rs`) |
| Gfx materials, diffuse routes, textures | Done for most (3,669 all-diffuse meshes; 23,215 of 23,247 texture hashes) |
| Plan prefix + inventory scalars | Done (D29, D33): locates `thingData`, does not read Things |
| Level payload | Header census only (D27). No RLevel or Thing reader |
| Thing, parts, PWorld, references | Nothing |
| RBevel, shape extrusion | Nothing |
| Bevy | One asset at a time (`lbp-bevy-viewer`, Bevy 0.19.1); per-mesh caps 100k positions / 50k triangles; 64 materials and 640 MiB retained per mesh (D66). No scene of many instances, no shared mesh/texture cache across instances |

## 4. Gaps and risks

1. **All-or-nothing parsing per level.** No part lengths. Coverage has to be measured as
   "levels (and plans) that parse to the end with byte-exact consumption", and it stays
   at zero until every part type a level uses is supported. The part histogram (PC
   check 3) decides the order of work.
2. **Size of the Thing layer.** ~12k Java lines incl. PSwitch (498), PCreature (675),
   PWorld (797), PShape (378), PPhysicsTweak (345), PEmitter (327). Most gates are
   head/subVersion conditions; only the LBP2 band (0x341..0x3f8, branch 0, plus
   0x4c44 if the PC census still finds it) needs implementing.
3. **Generated meshes are not in the data.** Shape extrusion must be written
   independently: triangulate the polygon with holes, extrude by thickness, apply the
   bevel profile, generate UVs. Only behaviour references exist (LBP Asset Studio
   `ShapeUtil.java`, unlicensed, author-labelled as ported from the PS3 binary). Visual
   fidelity will be approximate at first and must be reported that way.
4. **Recursion and budgets.** Inline references make depth data-driven. Need depth,
   tag-count, Thing-count and allocation budgets per AGENTS.md rule 7.
5. **Skinned/boned meshes.** `RENDER_MESH.boneThings` place bones via other Things'
   PPos. Our meshes are rest pose only. First pass: use the owning Thing's
   `worldPosition`; multi-bone placement is a later item (PC check 6 sizes it).
6. **Backdrops are plans.** The level's backdrop Thing references a plan via REF; it
   needs plan Thing parsing and instancing.
7. **Scene scale in Bevy.** Hundreds to thousands of instances: need mesh and texture
   caches keyed by GUID/hash, a scene budget instead of per-mesh budgets, and a 2.5D
   camera. Lighting/fog from PLevelSettings is a later fidelity item.
8. **Which levels are "story".** Not established from cloud: PC check 1.

## 5. Phased path to a first level in Bevy

**Phase 0, PC census (no product code).** Run the checks in section 7, including a
Toolkit `RLevel` oracle over every level on 01.33. Output: story level list, part
histogram per level, oracle success rate, max reference depth. This sets the order of
Phase 2 and the counting baseline.

**Phase 1, oracle-fed scene (dev only, fast first picture).** Extend the oracle to dump
a scene JSON under `local-analysis/` per level: for each Thing, UID, worldPosition,
render mesh GUID, gfx material/bevel GUIDs, polygon. Add a Bevy "level scene" mode that
loads that JSON and draws every RENDER_MESH Thing with the existing mesh/material path,
instanced with shared handles. This proves the Bevy side (caching, budgets, camera,
coordinates) and gives the visual target, while the product still never runs Java at
runtime. Shapes appear as flat polygons or are skipped and counted.

**Phase 2, independent Rust Thing reader.** New module (e.g. `lbp-resource::thing`, or
a new `lbp-level` crate if it grows): serializer core (compressed vectors/matrices,
reference tag table with budgets, resource descriptors), Thing header and part flags,
then parts in histogram order. Every step compares against the oracle dump: same UIDs,
part sets, transforms, GUIDs, and exact byte consumption. Plans first (smaller, fewer
part types), then RLevel/PWorld.

**Phase 3, Bevy from Rust.** Swap the Phase 1 JSON feed for the Rust reader. Same
scene code. Counts: levels fully parsed, levels with every RENDER_MESH instance drawn.

**Phase 4, generated meshes.** RBevel reader; polygon triangulation with holes (needs
a crate decision, e.g. an earcut port, or an in-repo implementation); front/back faces
first, then bevel side walls and UVs from GENERATED_MESH uvOffset/textureScale; gfx
material via the existing diffuse routes.

**Phase 5, fidelity.** Backdrop plans via REF; multi-bone placement; DECORATIONS and
STICKERS; MATERIAL_OVERRIDE/TWEAK; PLevelSettings lighting and fog; editorColor tints.

A first recognisable story level (props only) is reachable at the end of Phase 1; a
first level drawn from our own reader at the end of Phase 3; a level that looks like
the game needs Phase 4.

## 6. Candidate backlog items (sized like D-items; Integrator assigns numbers)

| # | Item | Where | Done when |
|---|---|---|---|
| L0 | Story level census + Toolkit RLevel/RPlan oracle (`scripts/LevelOracle.java`, `run-level-oracle.ps1`, `level_report.py`): parse success, Thing count, part histogram, max ref depth, compression flags, head/branch | PC | Report with counts in `local-analysis/`, summary in a milestone doc |
| L1 | Oracle scene dump (per-Thing UID, POS, RENDER_MESH, SHAPE polygon, GENERATED_MESH GUIDs) for N story levels | PC | Deterministic JSON, repeat run byte-identical |
| L2 | Bevy level-scene mode from the L1 JSON: shared mesh/material/texture caches, scene budget, 2.5D camera, RENDER_MESH instances only | cloud-ok (synthetic scene JSON) + PC QA | First story level visible; counts of drawn/skipped instances in the panel |
| L3 | Rust serializer core: compressed v3/v4/m44, reference tag table with depth/tag budgets, resource descriptor, thing/array helpers; synthetic tests only | cloud-ok | Synthetic recipes pass; fmt/clippy clean |
| L4 | Thing header + part flags decode for LBP2 heads (marker, UID, parent/groupHead/oldEmitter, planGUID, flags, partsRevision, u64 flags, CREATOR_ANIM shift, deprecated-part zero check) | cloud-ok | Synthetic tests; part list matches oracle for sampled things |
| L5 | Render parts: POS, RENDER_MESH, SHAPE + Polygon, GENERATED_MESH, GROUP, REF, BODY | cloud-ok + PC parity | Plans using only these parts parse byte-exact; count reported |
| L6..Ln | Remaining parts in histogram order, about 4-6 small parts or 1 big part (PSwitch, PCreature, PWorld, PScript+InstanceLayout) per item | cloud-ok + PC parity | Plans/levels fully parsed rises each item |
| Lw | RLevel wrapper + PWorld + PLevelSettings | cloud-ok + PC parity | First story level parses to the end, matches oracle |
| Lb | Bevy scene from the Rust reader (replaces L2 feed) | PC QA | Same picture as L2 without Java |
| Lv | RBevel reader | cloud-ok + PC oracle | All bevel resources on 01.33 parse |
| Lx | Shape extrusion v1: triangulated front/back faces + straight sides, gfx material, uvOffset | cloud-ok (synthetic polygons) + PC QA | Shapes visible in story levels; fidelity marked approximate |
| Ly | Bevel profile sides and UVs | PC QA with user screenshots | Side-by-side check on chosen levels |
| Lz | Backdrop plans, multi-bone placement, decorations/stickers, level lighting | split later | — |

Counting unit for these milestones: distinct level resources (by hash) on the 01.33
overlay, with "story" as a named subset once L0 defines it. "Viewable" = parses fully
and at least one Thing is drawn; "Complete" keeps the AGENTS.md meaning (all textures
and correct geometry, manually verified), which realistically starts after Lx/Ly.

## 7. PC checks for the Integrator (cannot be answered from cloud)

1. Which level resources are story levels: MAP paths of every `LVLb` on the 01.33
   overlay grouped by directory prefix, and whether a slot list/packs resource names
   them. Report counts per prefix.
2. Head, branch and compression-flag histogram of those levels on 01.33 (D27 was 01.00).
   Does any have head > 0x3e6 (cross-play hash array present)?
3. Toolkit `RLevel` deserialize over all levels: success/failure, Thing count, part
   histogram (by Part name, count of Things and count of levels using it), max inline
   reference depth.
4. Same for the plans that story levels reference through REF (backdrops first).
5. Share of Things with RENDER_MESH vs SHAPE+GENERATED_MESH per story level, and how
   many RENDER_MESH GUIDs are already in the 4,257 viewable meshes / 3,669 all-diffuse.
6. `boneThings` length distribution for RENDER_MESH Things; bone counts of those meshes.
7. Count of BEVEL resources and their head/branch; gfx materials used by GENERATED_MESH
   and how many already have a supported diffuse route.
8. Any PStreamingData/PStreamingHint use in LBP2 story levels (expected none).

## 8. Provenance and licensing notes

- Toolkit/cwlib (MIT) at the pin is the format specification and oracle, as for
  D27-D33. Independent Rust implementation, oracle comparison, no translation.
- LBP Asset Studio (MindOfBog/lbp_bgm @ f015b676, no licence found) has a working
  shape extruder (`view3d/managers/assetLoading/ShapeUtil.java`, earcut4j-based) whose
  header says it was ported from the PS3 GenerateMesh/BevelFunction code. Treat it as
  behaviour evidence only (what the output should look like), never as source.
- A triangulation dependency is a licensing decision for the user (permissive crates
  exist); an in-repo ear-clipping implementation avoids that.
- No existing open project renders full LBP levels including shapes from licensed code
  (GitHub topic and community-projects.md sweep); LBP Asset Studio is the closest.

Sources read: Toolkit `RLevel.java`, `RPlan.java`, `RBevel.java`, `Thing.java`,
`Part.java`, `PartHistory.java`, `Revisions.java`, `Serializer.java`,
`MemoryInputStream.java`, parts `PPos`, `PRenderMesh`, `PShape`, `PGeneratedMesh`,
`PWorld`, `PRef`, `PScript`/`ScriptInstance`, `PLevelSettings`, `Polygon`,
`cwlib-gl/exporter/SceneExporter.java`; lbp_bgm `ShapeUtil.java` (header and
signatures only); repo docs `level-plan-recon.md`, `plan-parser.md`,
`plan-inventory-parser.md`, `community-projects.md`, `STATUS-CURRENT.md`, `backlog.md`.
