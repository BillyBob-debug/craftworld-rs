# D148: default camera and upright Sackboy (2026-10-08)

## Problem

Single-mesh previews looked wrong from the default view: Sackboy (body mesh 0000043f,
`sack_boy.mol`) appeared lying on his back, seen from the crown of his head, as if the
camera looked up at him from below. The level view (yaw 0 / pitch 0) was already level.

## Cause (mesh orientation, not only the camera)

Game meshes are authored in Z-up model space with the front toward -Y
(CONFIRMED_FROM_LOCAL_DATA, sack_boy.mol bind skeleton, 3ds Max Biped names):
Bip01 Head z 108, toes z 0..0.3, bone_eye_pos_left/right (+/-24, -34.2, 166.4),
bone_zip_pull01 y -13.2 (front zip); mesh bounds z 0.15..204.6, x +/-59, y -48..45.
Other bone-0 props agree (tutorial_tv 0001a6a1 z 0..1496 tall and 60 thin in y;
toilet_roll 00014abf z 0..420; pencil 00003199 along z). In levels the POS world
matrices provide the rotation (D113/D120), so level mode is unaffected. The viewer drew
single meshes with raw axes as Y-up, so the model's up pointed at the camera and the
old home view (yaw 0.6, pitch 0.65, camera above) looked down on the top of his head.

## Change (crates/lbp-bevy-viewer/src/main.rs)

- `model_up(z_up)`: -90 degrees about X, (x, y, z) -> (x, z, -y): Y-up, front toward
  +Z (the camera). Proper rotation, so triangle winding and normals stay consistent.
- Static and CPU-skinned single meshes get it as the entity Transform. GPU skinning
  ignores the mesh entity transform, so `gpu_rig` puts it on the joint root
  (`U x N`); inverse binds are unchanged (skin = U x N x world x invBind x N^-1).
- New flag `--model-up z|y` (default `z`; `y` restores raw axes). Synthetic fixtures and
  level scenes are not rotated.
- `Orbit::default()` (single-mesh home, R reset): yaw 0, pitch 0.15, distance 2: from
  the front, camera 0.3 display units above the centre (about eye height of a unit-tall
  upright figure), looking slightly down. Level home stays yaw 0 / pitch 0.
- Tests: `z_up_models_stand_upright_facing_the_default_camera` (bone points and default
  camera position); `gpu_rig_joint_globals_match_cpu_skin_matrices` now runs with and
  without the rotation (static joint = U).

Evidence label for the Z-up rule: CONFIRMED_FROM_LOCAL_DATA for the meshes listed;
applied to every single mesh as a viewer policy (UNVERIFIED for meshes authored
otherwise; use `--model-up y` to compare).

## Commands and results

Run from the main checkout (`$X` = viewer binary, `$gd` =
`game-data\a1-disc;game-data\a1-update-0133;local-analysis\d133-edat-overlay`, snapshot
`local-analysis\d133-inventory-0133-edat.json`; the d133 snapshot needs the edat overlay).

- `cargo fmt --all --check`: ok
- `cargo test -q -p lbp-bevy-viewer`: 26 passed
- `cargo clippy -q --workspace --all-targets -- -D warnings`: ok
- `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`: ok
- `$X --snapshot ... --game-dir $gd --select-guid 0000043f --play-animation 0001a697
  --animation-frame 8 --qa-hide-ui --qa-capture local-analysis\d148-sackboy-front.png`:
  exit 0, "positioned on non-primary monitor 213v0", 43/43 bones driven, GPU joints.
  Sackboy upright, facing the camera (face, eyes, front zip visible), mid-stride, seen
  from about eye level. Before: `local-analysis\d148-sackboy-before3.png` (lying, top of
  head toward the viewer).
- `$X ... --level-path lbp2/th_tutorials/levels/tutorial_23.bin --qa-hide-ui
  --qa-capture local-analysis\d148-tutorial23-camera.png`: exit 0, 14 RENDER_MESH
  Things + 9 shapes + 75 backdrop items; upright, level front view (unchanged policy).

- Same Sackboy command with `--skinning cpu` ->
  `local-analysis\d148-sackboy-front-cpu.png`: identical silhouette bbox
  (540,168)-(791,668) to the GPU capture, 0 pixels differing by more than 40 grey levels.

+0 newly viewable / +0 newly Complete (camera/orientation only; unit distinct mesh GUIDs).

## Not tested

Meshes whose model space is not Z-up; the legacy `lbp-viewer`
(unchanged); interactive orbit feel.
