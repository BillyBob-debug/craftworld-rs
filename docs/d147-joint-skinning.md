# D147 span-joint skinning in the level scene

2026-10-08 America/Chicago. D120 placed linear (span) JOINT meshes by stretching the
whole mesh along the contact A -> B line, so piston sleeves, string/chain end pieces
and the piston rod all stretched with the span. D147 skins those meshes with their own
skin streams instead: end pieces stay rigid at the contacts and only geometry weighted
across both ends (bridging cord, piston rod) stretches. Angular (pivot) joints are
unchanged (their bones coincide or move together; rigid placement = D120).

## What the joint meshes contain (local data)

`joint_skin_probe` (new `lbp-preview` example, summary numbers only) on
`main_01_rookie_test` (01.33 + D133 edat overlay):

| mesh | bones (bind position along model Z) | skin weights |
|---|---|---|
| piston 000043f2 | 0 `joint_piston_bone_01` +26.65, 1 `_02` -26.16, 2 `_03` +26.43, 3 `_04` -26.16 (2, 3 children of 0) | bone 0: 161 verts, model z 9..88; bone 1: 160 verts, z -88..-9; bones 2/3 blended 100/0, 75/25, 50/50, 25/75, 0/100 along z +26.7..-26.2 (67 verts, the rod) |
| string 00004c7f | 0 +26.63, 1 -26.19 | bone 0: z 9..88 (170), bone 1: z -88..-9 (174), all 100 % |
| chain 00004c80 | 0 +26.63, 1 -26.19 | same split (177 / 167) |
| bolts 000054cd / 000054d0 / 000054d8 (pivot) | 2 bones each, coincident or close | rigid per bone |

Every bone's skin pose has the same rotation (local Y -> model +Z), so one bind frame
serves the whole mesh. Piston `boneIdx` is [2, 3] (the rod bones), strings/chains [-1, -1].

## Rules and evidence

| Rule | Evidence |
|---|---|
| Skin streams: per-vertex 4 bone indices (stream 1 bytes 3, 7, 11, 15) and weights | CONFIRMED_FROM_COMMUNITY_CODE, already implemented and recorded in D115 (`lbp_mesh::decode_skin`, docs/skinned-preview.md); reused unchanged |
| A vertex goes to world through bone world x invSkinPose(bone), linear blend over its weights | CONFIRMED_FROM_COMMUNITY_CODE for the per-bone product (Toolkit [d6a2ef80](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9) `lib/cwlib/src/main/java/cwlib/io/imports/StaticMeshBuilder.java`, `lib/cwlib-gl/.../exporter/SceneExporter.java` registerThing); linear blend as D118 |
| Joint bone world frames: Toolkit has none (PJoint.java only serializes `boneIdx`/`boneLengths`; no reader uses them) | CONFIRMED_FROM_COMMUNITY_CODE (absence, grep of the pin) |
| Each bone is rigid (bind scale, de-rotated as D120) and follows the endpoint on its side: bind axial position past the middle of the mesh extent along bone0 -> bone1 -> contact B, else contact A | INFERRED_FROM_LOCAL_DATA: the probe above (two rigid end halves per mesh, piston rod blended between bone 2 on the A side and bone 3 on the B side); stretching a rigid half reproduces D120, so the end-piece rule is the only one that keeps end caps/sleeves undistorted |
| A group's mesh end (axial `lo`) lands on pA, B group's end (`hi`) on pB, axis rotated onto pA -> pB keeping local Z nearest world Z | INFERRED, same end-to-contact convention as D120 (`lo`/`hi` fitted to the contacts); contacts from D120 |

Implementation (`lbp_preview::level_scene`): `joint_span_skin` (per-bone matrices, f32,
relative to contact A), `skin_span_positions` (CPU linear blend; influences past the
bone list or with non-positive weight dropped; a vertex with no usable weight follows
contact A and is counted), `skin_span_joints` (post-pass after `build_scene`: one
skinned copy of the mesh per span item, `MeshSource::JointSkin`, item matrix =
translation to contact A; failures keep the D120 stretch and are counted by reason).
`read_mesh_skin` decodes a mesh's skin streams once per mesh. `lbp_mesh::Geometry`
(and its metadata types) gained `Clone` for the copy. The viewer needs no change: the
skinned copies are ordinary model payloads. Panel / QA log / `--level-report` print
"Span joints skinned (D147, inferred ...)" with the counts.

Known limits (preview approximation): when the contacts are closer than the two rigid
end pieces, the pieces overlap; `angle`, `renderScale`, `length` (rest length) and
animation are not applied; pivot joints unchanged; `boneIdx` / `boneLengths` recorded
but not used.

## Counts (01.33 + edat overlay, headless `--level-report`)

Unit: RENDER_MESH Things. Baseline D120.

| | tutorial_23 | main_01_rookie_test |
|---|---:|---:|
| placed / with RENDER_MESH (unchanged) | 14 / 14 | 455 / 455 |
| JOINT Things placed (pivot / span) | 0 | 122 (60 / 62) |
| span joints skinned (D147) / span total | 0 / 0 | 62 / 62 |
| span joints kept at the D120 stretch | 0 | 0 |
| skinned vertices without usable weight | 0 | 0 |
| draw entities (viewer, incl. backdrop) | unchanged | 1197 (= baseline) |
| shear dropped (no focus) | 0 | 0 |

Rookie span joints by mesh: piston 000043f2 33, string 00004c7f 25, chain 00004c80 4.
The 62 skinned copies are extra scene meshes (`MeshSource::JointSkin`); the three
original joint meshes stay loaded with 0 instances.

Visual check: one capture on the non-primary monitor (log: "Preview window positioned
on non-primary monitor 213v0 before showing"), `--level-focus-thing 21` (a piston),
`local-analysis/d147-rookie-joints.png` (not committed). At the focus point a thin rod
with a small rigid end piece runs between the attached pieces; no stretched sleeve is
visible. Which end piece is which, and the exact sleeve sizes, are not confirmed.

Asset counts: +0 newly viewable, +0 newly Complete (no per-asset route changed);
0 levels Complete.

## Commands

Game dirs `G` = `game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay`,
snapshot `local-analysis/d133-inventory-0133-edat.json`.

```powershell
. ./scripts/use-local-rust.ps1
cargo fmt --all --check                                              # ok
cargo test -q -p lbp-mesh -p lbp-preview -p lbp-bevy-viewer          # all ok (new span_joints_skin_rigid_ends_and_stretch_the_bridge)
cargo clippy -q --workspace --all-targets -- -D warnings             # ok
cargo build -q -p lbp-bevy-viewer                                    # ok (separate CARGO_TARGET_DIR: the shared debug exe was in use)
cargo build -q --release -p lbp-preview --example joint_skin_probe
target/release/examples/joint_skin_probe <snapshot> <G> main_01_rookie_test/main_01_rookie_test.bin   # table above
lbp-bevy-viewer --snapshot <snapshot> --game-dir <G> --level-path main_01_rookie_test/main_01_rookie_test.bin --level-report   # exit 0
lbp-bevy-viewer ... --level-path lbp2/th_tutorials/levels/tutorial_23.bin --level-report                                       # exit 0
lbp-bevy-viewer ... --level-path main_01_rookie_test/main_01_rookie_test.bin --level-focus-thing 21 --qa-hide-ui --qa-capture local-analysis/d147-rookie-joints.png   # exit 0
```

New viewer flag `--level-focus-thing N` frames the view on Thing N's item (bounds
widened threefold) for QA captures; it changes the display normalization only (the
focused Rookie view reports 1 shear-dropped Thing from f32 precision, 0 without it).

## Not tested

Other levels than main_01_rookie_test and tutorial_23 in the viewer; corpus-wide skin
decode of every joint mesh (only the 6 Rookie Test joint meshes probed); no oracle
exists for joint world matrices (Toolkit places none). Visual correctness beyond the
one capture is unverified (end orientation, which end piece is which).
