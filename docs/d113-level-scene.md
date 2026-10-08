# D113 first level scene in the Bevy viewer (L2/L5 subset, with D114 shapes)

2026-10-08 America/Chicago. Level mode for `lbp-bevy-viewer`: reads one level with the
D107 reader (`lbp-level`), places every RENDER_MESH Thing's mesh at its POS world matrix,
adds D114 shape extrusions, frames the result side-on, and shows/prints counts.

## Flags

| Flag | Meaning |
|---|---|
| `--level-guid HEX` | Level by GUID (8 hex digits; must resolve to a magic-confirmed `level` MAP row) |
| `--level-path MAP_PATH` | Level by MAP path, exact or a `/`-suffix, case-insensitive (e.g. `lbp2/th_tutorials/levels/tutorial_23.bin`); several GUIDs matching = error |
| `--level-report` | With a level: build the scene and the display plan, print the counts, exit. No window, no GPU. |

Not combinable with `--synthetic`; `--snapshot` and `--game-dir` required. Existing
`--animation-base` / `--instance-rgba` apply to every material load in the level.
Selecting an asset in the browser leaves level mode. R / Reset / F return to the level's
home view.

## Structure (AGENTS rule 5)

- `lbp_preview::level_scene` (no Bevy): `LevelScene<T> { meshes: Vec<SceneMesh<T>>, items:
  Vec<SceneItem>, counts: SceneCounts }`. `build_scene` (model meshes, generic loader,
  one load per distinct mesh reference, failures cached and counted per Thing),
  `add_shapes` (D114 extrusions merged per gfx material), `load_level_scene` (real data;
  payload `ScenePayload::{Model, Shapes}`), `resolve_level`, `world_matrix_check`.
  `LevelScene::push_item` is the extension point for further mesh sources.
- `linked::load_material` (split out of `load_primitive`, behaviour unchanged) takes a
  `UvInput`: stored mesh UVs or generated UV0, so shape batches use the same diffuse
  route selection as model primitives. `lbp_mesh` now re-exports `UvChannelMetadata`.
- Viewer: `prepare` / `prepare_buckets` / `upload` (the single-mesh path, refactored, same
  budgets); `plan_level` (CPU display plan, also used by `--level-report`),
  `place_items`, `level_home`, `spawn_level`. Model meshes are uploaded once per mesh
  reference and shared by all instances (`adapter::mesh_local` keeps model coordinates).

## What is drawn / skipped

- Model meshes: Things with RENDER_MESH whose mesh descriptor resolves (GUID, else SHA-1),
  with a POS matrix that is finite, affine (last row 0,0,0,1, tol 1e-4) and invertible
  (|det| > 1e-12). Static bind pose, own POS only. Neutral (unsupported material) groups
  draw in the diagnostic grey as in the single-mesh view and are counted.
- Skipped and counted by reason: no/null mesh reference, no POS (JOINT/bone placement not
  implemented), non-finite / projective / non-invertible matrix, mesh unavailable
  (load error text), level budgets. Display stage: prepare errors (e.g. no
  nondegenerate faces) counted as `display-skipped`; TRS decomposition that drops shear
  is counted (0 on both levels).
- Shapes (D114 `extrude_thing`): selection outcomes (`no_generated_mesh`, `edit_only`,
  `no_pos`) are "not selected"; geometry rejections by D114 reason key. Extruded shapes
  are merged per gfx material GUID into world-space batches (identity item matrix,
  chunked at 100,000 vertices / 50,000 triangles, the adapter limits) so each material
  loads once per batch instead of once per shape; the material goes through
  `load_material` with the generated UV0. A material that does not select draws flat
  neutral grey and is counted with its reason. Label shown in panel and log:
  "preview approximation: straight extrusion, no bevel, UV scale uncalibrated".
- Not drawn: DECORATIONS (counted), STICKERS, bones/JOINT placement, animation,
  MATERIAL_OVERRIDE/TWEAK, editor colour, PShape colour, lighting, backdrop plans.

Framing: the whole displayed scene is centred and scaled to unit size (largest axis = 1),
so the existing orbit camera, clip planes and pan limits apply. Home view yaw 0 / pitch 0
(camera on +Z looking along -Z, LBP side-on), distance fitting the X/Y extent for a
1280x800 window at the default 45 degree FOV. Which side is the game's "front" is
UNVERIFIED; the materials are double-sided.

Budgets: per level 1,024 distinct model meshes, 100,000 items, 1,536 MiB aggregate
retained material bytes (each mesh keeps its D66 64-load / 640 MiB limits), 50,000
shapes, 8,000,000 shape output vertices. The level reader runs on a 512 MiB reserved
stack (`LEVEL_STACK_BYTES`), default `Limits` (depth 256).

## Counts (01.33, headless `--level-report`)

Units: Things (tag-table entries); "Things" in display lines are drawn Things; mesh
references are distinct GUIDs. Snapshot `d68-inventory-0133-verified.json`.

| | tutorial_23 (GUID 0001db4f) | main_01_rookie_test |
|---|---:|---:|
| Things total | 91 | 3,247 |
| RENDER_MESH Things | 14 | 455 |
| placed / drawn RENDER_MESH Things | 14 / 14 | 333 / 333 |
| skipped: no POS (joint/bone) | 0 | 122 |
| distinct meshes loaded / failed | 8 / 0 | 43 / 0 |
| drawn with neutral groups | 2 | 130 |
| SHAPE Things / extruded / drawn | 36 / 9 / 9 | 857 / 599 / 599 |
| shapes not selected (no_generated_mesh) | 27 | 258 |
| shapes rejected | 0 | 0 |
| shapes in flat fallback colour | 1 (multiply route unsupported) | 34 (33 multiply route, 1 no gfx GUID) |
| shape material batches | 4 | 19 |
| decorations not drawn | 3 | 51 |
| draw entities | 39 | 684 |
| shear dropped | 0 | 0 |
| reader | complete | complete |

Newly viewable / Complete: these are level-scene counts, not asset counts; +0 newly
viewable and +0 newly Complete assets (no per-asset route changed). Level-level: 2
levels now open in the viewer with every positioned RENDER_MESH instance placed; 0
levels Complete (bones, bevels, decorations missing; visual check pending).

Run time per report about 20-30 s on the PC, mostly catalog load.

## Commands

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check                                   # ok
cargo test -q -p lbp-preview -p lbp-bevy-viewer -p lbp-mesh -p lbp-level   # all ok
cargo clippy -q -p lbp-preview -p lbp-bevy-viewer -p lbp-mesh -p lbp-level --all-targets -- -D warnings   # ok
cargo build -q -p lbp-bevy-viewer -p lbp-viewer           # ok
target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path lbp2/th_tutorials/levels/tutorial_23.bin --level-report
target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path main_01_rookie_test/main_01_rookie_test.bin --level-report
```

Capture (Integrator, second monitor; the window stays hidden until placed):

```powershell
target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path lbp2/th_tutorials/levels/tutorial_23.bin --qa-capture local-analysis/d113-tutorial23.png
```

The QA log prints `Loaded level ...` followed by the same count lines.

Unit tests: `level_scene` synthetic scene (mesh sharing, cached failures, every skip
reason, bounds), matrix checks and `push_item`, shape batching (material grouping, index
rebasing, selection/rejection/fallback counts, label); viewer `place_items` /
`level_home` (normalization, side-on home, display-skip counts).

## Not tested

No native window was opened and no capture taken (Integrator). Visual placement,
orientation/handedness, front-side choice, and whether own-POS placement is right for
skinned props are unverified. Python unittest suite not run (no Python changes). Other
levels not run.
