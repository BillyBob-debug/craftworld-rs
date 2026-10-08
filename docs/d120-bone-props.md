# D120 joint-attached meshes in the level scene (L5 bones subset)

2026-10-08 America/Chicago. D113 skipped every RENDER_MESH Thing without its own POS
("no POS, joint/bone placement not implemented"): 122 of 455 in `main_01_rookie_test`.
D120 finds what those Things are, places the ones the evidence covers, and keeps the
rest skipped with specific reasons. POS placement is unchanged (same `pos.world` copy).

## What the skipped Things are (local data)

`level_bone_probe` (new `lbp-preview` example, summary counts only) on 01.33:

- main_01_rookie_test: all 122 are `JOINT+RENDER_MESH` Things (no POS, no parent, empty
  `boneThings`), both JOINT endpoints carry POS. Meshes: piston 000043f2 (4 bones, 33),
  string 00004c7f (25), chain 00004c80 (4), motor bolt 000054cd (5), sprung bolt 000054d0
  (4), bolt 000054d8 (51); bone names `joint_piston_bone_01`, `joint_string_bone_01`,
  `joint_bolt`, ... (types 4, 5, 3, 9, 8, 7).
- Corpus, all 236 in-band levels (`--all`): 13,499 RENDER_MESH Things without POS =
  13,080 JOINT Things with both endpoints on POS (angular types 7/8/9: 3,385 / 268 /
  3,355; linear types 1..6: ELASTIC 212, SPRING 442, CHAIN 558, PISTON 3,177, STRING
  1,533, ROD 150) + 11 JOINT Things with an endpoint lacking POS (10 type 0, 1 type 9) +
  408 non-JOINT Things (`RENDER_MESH+TRIGGER+STICKERS+SCRIPT` 384, `RENDER_MESH+AUDIO_WORLD`
  24, in 89 levels; `boneThings[0]` is the Thing itself, its parent has no POS). The D106
  "bone placement" remark therefore means joint rendering, not prop-on-bone attachment.

## Rules and evidence

| Rule | Evidence |
|---|---|
| PJoint field order (a, b ref Thing; aContact, bContact v3; length, angle, offsetTime; invertAngle; settings; boneIdx; boneLengths v4; type; ...; renderScale; ...; hideInPlayMode; behaviour) | CONFIRMED_FROM_COMMUNITY_CODE Toolkit [d6a2ef80](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9) `lib/cwlib/.../structs/things/parts/PJoint.java` serialize (already followed by the D107 reader, which now keeps the values in `parts::Joint`); CONFIRMED_FROM_LOCAL_DATA: 129/129 joint Things in the 5 D106 scene levels equal the oracle on every kept field |
| Type numbers 0..10 LEGACY, ELASTIC, SPRING, CHAIN, PISTON, STRING, ROD, BOLT, SPRING_ANGULAR, MOTOR, QUANTIZED, angular from 7 | Toolkit PJoint.java EJointType comment (community notes, not code); INFERRED |
| Contact world point = endpoint POS world x contact (contact is in the endpoint's local frame) | INFERRED_FROM_LOCAL_DATA: angular joints' two contact points coincide in the layer plane (< 1 unit) for 6,541 / 7,008 (93 %), differing only in depth by layer steps (0/100/200/300); STRING distance within 10 % of `length` 1,185 / 1,533, ROD 127 / 150 |
| A mesh vertex goes to world through bone world x invSkinPose(bone) | CONFIRMED_FROM_COMMUNITY_CODE Toolkit `lib/cwlib/.../io/imports/StaticMeshBuilder.java` (matrices[0] = POS x bones[0].invSkinPoseMatrix; bone Things by POS animHash) and `lib/cwlib-gl/.../exporter/SceneExporter.java` registerThing (boneThings[i] POS x invSkinPose[i]); neither has a rule for JOINT Things (no POS, no boneThings), so Toolkit places them nowhere |
| Joint bone world frame axis-aligned with world (bone local Z = layer depth), bind scale | INFERRED_FROM_LOCAL_DATA: every joint bone skin pose is a scaled rotation mapping local Y to model +Z and local Z to model -Y; POS Things' POS matrices carry the same scale without that rotation (e.g. spawn point skin 5.55 x rotation, POS 5.55 x in-plane rotation); a bolt mesh (disc in model XZ, depth along -Y) then faces the camera with its depth along the joint's A->B direction (pure Z for bolts) |
| `boneLengths` = column norms of the joint mesh's last bone skin pose | INFERRED_FROM_LOCAL_DATA (6 meshes in main_01); recorded, not used |

Placement (`lbp_preview::level_scene::joint_world`, preview approximation):
pA, pB as above; P0 = bone-0 skin-pose rotation (columns normalized, must be orthonormal
within 1e-3), t0 its translation.
- Pivot (types 7..10): world = T(pA) x P0^T x T(-t0) (bone 0's bind point on contact A).
- Span (types 1..6): the mesh's extent along the bind axis bone0->bone1 (in the
  de-rotated frame, from the model bounds) is fitted onto pA..pB; the axis is rotated
  onto pA->pB keeping local Z nearest world Z; the cross-section stays rigid.
- Skipped with reasons: JOINT type outside 1..10, endpoint without POS or with a bad
  matrix, mesh without bones / bone-0 pose not a scaled rotation, span mesh with one
  bone, coincident bind bones or contacts, no bounds, non-finite result; Things without
  POS and without JOINT ("no POS and no JOINT (no placement source)").

Known approximation limits (labelled in panel/QA log): no skinning, so string/piston end
pieces stretch with the span and the piston's two parts do not telescope; `angle`,
`renderScale` (36 bolts at 2.0 in main_01), `boneIdx` (pistons [2, 3]), animation and
`hideInPlayMode` are not applied. Angular joints whose contacts disagree (7 %) are placed
at contact A.

## Counts (01.33, headless `--level-report`)

Unit: RENDER_MESH Things (tag-table entries). Baseline D113.

| | tutorial_23 | main_01_rookie_test |
|---|---:|---:|
| placed / with RENDER_MESH, before (D113) | 14 / 14 | 333 / 455 |
| placed / with RENDER_MESH, after | 14 / 14 | 455 / 455 |
| newly placed from JOINT (pivot / span) | 0 | 122 (60 / 62) |
| skipped | 0 | 0 |
| distinct meshes loaded | 8 | 49 (was 43) |
| draw entities | 39 | 948 (was 684) |
| shear dropped | 0 | 0 |

POS items under `--level-placement pos-only`: `pos.world` copied as in D113 (unit tests
assert exact matrices). The table counts are identical in both placement modes. Asset counts: +0 newly viewable, +0 newly Complete (no per-asset route changed);
0 levels Complete.

## Oracle comparison

`scripts/LevelOracle.java` scene rows now include a `joint` object (a/b refs, contacts,
length, angle, settings, boneIdx, boneLengths, type, renderScale, hideInPlayMode). Run on
the D106 scene manifest: 5 levels parsed_exact; 129 joint Things (pod 2, chick_flick 3,
rookie 124), 129/129 equal to the Rust reader on every field (f32), contact points
recomputed from the oracle's POS world matrices equal ours (max abs difference 9.4e-4 on
coordinates up to ~3e4, f64 rounding). Toolkit computes no world matrix for JOINT meshes,
so the D120 matrices themselves have no oracle; 126 of the 129 are placed (3 joints have
no RENDER_MESH).

## POS placement: POS x inverse bind of bone 0 (default since the D120 follow-up)

Rule (CONFIRMED_FROM_COMMUNITY_CODE, Toolkit d6a2ef80
`lib/cwlib/src/main/java/cwlib/io/imports/StaticMeshBuilder.java`: matrices[0] = POS
world x bones[0].invSkinPoseMatrix; `lib/cwlib-gl/src/main/java/cwlib/gl/exporter/SceneExporter.java`
registerThing: bone node = POS x invSkinPoseMatrix): a POS-placed RENDER_MESH Thing is
drawn at POS world x the stored inverse skin pose of mesh bone 0
(`PosPlacement::InverseBind`, `lbp_preview::level_scene::mul44`). Meshes without bones
keep POS. Multi-bone Things still use bone 0 only (boneThings / animHash per bone not
applied). Visual correctness UNVERIFIED (no capture).

`--level-placement pos-only` (viewer) / `PosPlacement::PosOnly` (LevelScene) restores the
D113 placement exactly (`pos.world`), for side-by-side captures; `inverse-bind` is the
default. Panel / QA log / `--level-report` print the rule, the number of Things whose
matrix changed versus POS only, and the five meshes that moved most (translation shift,
scale factor, rotation).

| | tutorial_23 | main_01_rookie_test |
|---|---:|---:|
| POS Things changed vs D113 / unchanged / no bones | 14 / 0 / 0 | 245 / 88 / 0 |
| scene Z bounds, pos-only | -2037 .. 1103 | -1248 .. 726 |
| scene Z bounds, inverse-bind | -527 .. -110 | -628 .. 228 |

Largest changes: tutorial_tv 0001a6a1 (shift 591, scale x2.33, 180 deg), 0000441c
(main_01, shift 672, x2.33, 180 deg), toilet roll 00014abf (shift 262-287, no rotation),
padlock 00000eb1 (x7.0, 90 deg), pencil 00003199 (x34.9: D113 drew it 35 times too
small), pressure switch 00005585 (90 deg). The scene depth extent shrinking to the layer
range is consistent with props no longer being turned 90 degrees out of the layer plane;
a plausibility signal, not a visual check.

## Commands

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check                                                       # ok
cargo test -q -p lbp-level -p lbp-preview -p lbp-bevy-viewer                  # all ok (new joint_things_place_from_endpoints)
cargo clippy -q -p lbp-level -p lbp-preview -p lbp-bevy-viewer --all-targets -- -D warnings   # ok
cargo build -q -p lbp-bevy-viewer -p lbp-viewer                               # ok
python -m unittest discover -s tests                                          # OK (skipped=1)
target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path lbp2/th_tutorials/levels/tutorial_23.bin --level-report
target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path main_01_rookie_test/main_01_rookie_test.bin --level-report
# each also with --level-placement pos-only (D113 placement; JOINT counts unchanged)
cargo build -q --release -p lbp-preview --example level_bone_probe
target/release/examples/level_bone_probe <snapshot> <game dirs> --all                        # corpus summary above
target/release/examples/level_bone_probe <snapshot> <game dirs> <level path> local-analysis/<new>.json   # joint rows
```

Oracle (javac/java from `local-analysis/toolchain/java`, cwlib jar at the pin, the same
steps as `scripts/run-level-oracle.ps1` with a worktree-local classes directory):
`LevelOracle --self-test` passed; `LevelOracle <d106-level-manifest-scene.json>
local-analysis/d120-level-scene-joints.json <DATA.FARC> <patch_*.farc...>` -> 5
parsed_exact. Comparison script: scratch Python over that file and the five probe JSONs
(not committed).

## Not tested

No window, capture or visual check: orientation (which side faces the camera), bolt
size, string/piston appearance and the inverse-bind POS placement are unverified
(Integrator: capture with and without `--level-placement pos-only`). Levels other than
the five scene levels were not run through `--level-report` (corpus numbers are reader
level, not scene runs). Skin streams are not decoded, so true per-bone skinning was not
attempted. The D107 `level_parity` and census examples were not rerun (reader byte
consumption unchanged; only values are now kept).
