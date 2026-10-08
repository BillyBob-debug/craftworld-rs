# D153: undrawn level shapes classified

Date: 2026-10-08 (America/Chicago). Inputs: 01.33 snapshot `local-analysis/d133-inventory-0133-edat.json`,
`--game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"`.

Question: `--level-report` drew 599/857 (Rookie Test), 589/1,061 (Grab and Swing),
171/318 (Hedge Hopping), 9/36 (tutorial_23) SHAPE Things. Why is each other shape not
drawn, and should any of them be visible in the game?

## Classification (unit: SHAPE Things in the level's Thing list)

Probe: `crates/lbp-preview/examples/shape_visibility_probe.rs` (read-only, counts only).
"Parent tree" = Things with the same top `parent` ancestor.

| Reason | Rookie Test | Grab and Swing | Hedge Hopping | tutorial_23 | Should be visible? |
|---|---|---|---|---|---|
| Unsupported shape kind/flags | 0 | 0 | 0 | 0 | - |
| Polygon/triangulation fault (`shapes_rejected`) | 0 | 0 | 0 | 0 | - |
| Missing material (extruded, flat fallback) | drawn, counted as fallback in D149 | | | | - |
| Edit-mode only (GENERATED_MESH visibility without PLAY_MODE) | 0 | 6 | 0 | 0 | no (T5) |
| No GENERATED_MESH, own RENDER_MESH (collision hull) | 212 | 273 | 80 | 13 | no (mesh drawn instead) |
| No GENERATED_MESH, RENDER_MESH elsewhere in parent tree | 40 | 184 | 63 | 10 | no (mesh drawn instead) |
| No GENERATED_MESH, no mesh in parent tree (invisible collider) | 6 | 9 | 4 | 4 | no |
| **Undrawn total** | **258** | **472** | **147** | **27** | 0 should be visible |

Details:
- Every RENDER_MESH that stands for a collider (own or in its parent tree) has POS or JOINT,
  so it is placed by the level scene (252 / 463 / 143 / 23 colliders checked).
- Invisible colliders: Things 1-4 in every level are 4-vertex slabs, thickness 500, about
  300,000 units from any mesh, physics material `general_infinite_mass_cant_jump_off.mat`
  (00001414; level boundary walls). Rookie Test also has 2 `long_joint_heavy.mat` (00004c88)
  thickness-10 pieces at one point; Grab and Swing 5 `base_cardboard.mat` thickness-10
  pieces of `obj_toilet_roll.plan` instances whose mesh Thing is in a different parent tree
  (nearest RENDER_MESH 4 units away for one).
- Edit-only (Grab and Swing): 6 `base_circuit_board(_core).mat` shapes of `obj_sack_boy.plan`
  microchips (3 with SWITCH_INPUT).
- PShape flags were 0x6/0x7/0x47/0x807 (collision bits per Toolkit `ShapeFlags`); none of
  them is a visibility bit, so visibility follows GENERATED_MESH alone.

## Decision and change

No undrawn group should be visible in the game, so nothing new is drawn. Shape counts
before = after: 599/857, 589/1,061, 171/318, 9/36.

Change (report only): `lbp_level::shape_mesh::ColliderRoles` / `ColliderRole` split the
level scene's `no_generated_mesh` count into the three rows above
(`no_generated_mesh: collision hull of own RENDER_MESH`, `...: collider in a RENDER_MESH
parent tree`, `...: invisible collider, no mesh in parent tree`). The parent walk is
bounded (`MAX_PARENT_DEPTH` 256, self/out-of-range parents end it). Test:
`colliders_without_generated_mesh_split_by_parent_tree` (synthetic, includes a parent cycle).

Asset counts: **+0 newly viewable, +0 newly Complete** (baseline D149; unit assets;
scope 01.33 snapshot). Levels are not an asset count.

## Evidence

| Rule | Label | Source |
|---|---|---|
| A shape without GENERATED_MESH is not drawn itself (collision hull / invisible collider) | STRONG_INFERENCE (D108 T4) + COMMUNITY_BEHAVIOUR | LBPAS `references/lbp_bgm/.../view3d/core/types/Thing.java` `generateShape`: such shapes get alpha 0 |
| GENERATED_MESH visibility PLAY_MODE=1, EDIT_MODE=2 | CONFIRMED_FROM_COMMUNITY_CODE (D108 T5) | Toolkit d6a2ef80 `cwlib/enums/VisibilityFlags.java` |
| PShape flags 1/2/4 are collision bits, not visibility | CONFIRMED_FROM_COMMUNITY_CODE | Toolkit d6a2ef80 `cwlib/enums/ShapeFlags.java`, `PShape.java` |
| Undrawn colliders sit in RENDER_MESH trees or are far-off boundary slabs | CONFIRMED_FROM_LOCAL_DATA | probe above, four levels |
| Material names | CONFIRMED_FROM_LOCAL_DATA | snapshot MAP paths for the GUIDs |

Toolkit was read for facts only; no code copied.

## Commands

- `shape_visibility_probe SNAP GD 0001e786 0001e793 0001e7f4 0001db4f` (run from the main
  checkout): exit 0, counts above.
- `cargo fmt --all --check`: clean. `cargo test -q -p lbp-level -p lbp-preview`: all pass.
  `cargo clippy -q --workspace --all-targets -- -D warnings`: exit 0.
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`: exit 0.
- `lbp-bevy-viewer --snapshot SNAP --game-dir GD --level-guid <guid> --level-report`: exit 0
  for all four; shapes extruded/placed unchanged, `no_generated_mesh` split as in the table.

## Not tested

No window capture (geometry unchanged, so none was needed; the command would be `--level-guid 0001e786 --qa-hide-ui --qa-capture
local-analysis/d153-shapes-rookie.png`). The game's own rendering of the toilet-roll and
joint colliders was not observed in RPCS3. Other levels; legacy viewer.
