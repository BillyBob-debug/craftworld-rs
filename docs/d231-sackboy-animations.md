# D231: Sackboy's own clips in play mode (animation plan slices 1 and 2)

Plan: `local-analysis/results/2026-10-11-sackboy-animation-plan.md` (slices 1 and 2).
Labels: FOUND = read from game data / census; INFERRED = our choice, not traced in the EBOOT.

## What changed

- `--play-sackboy` with a level draws Sackboy as his body mesh `0000043f`
  (`sack_boy.mol`, FOUND) without `--select-guid` (still accepted to pick another mesh).
  The default naked costume pieces add no geometry (INFERRED, plan section 1).
- Slice 1, `crates/lbp-sim/src/sackboy_anim.rs` (Bevy-free): `pick_clip(&Sackboy,
  grounded, prev) -> AnimState { clip, frame }` once per 30 Hz tick, after `Sackboy::step`.
  `Sackboy` now exposes `jump_tick()`, `dip_tick()` and `prev_input()`.
- Slice 2: `lbp_preview::skinned::load_clip_bank` reads the mesh once, binds the first clip
  and binds the others to the same rig (`Rig::rebind`, `SkinnedClip::bind`); the viewer
  hard-cuts with `SkinnedClip::swap`, so one GPU joint hierarchy serves every clip.
  `--play-animation HEX` still overrides with one clip at walk pace (old path). The Sackbot
  walk `0001a697` is no longer needed.
- `--play-sackboy-input` logs now end with `clip NAME frame F` once the figure is loaded.

## Clips (GUIDs and frame counts FOUND: MAP + d116 census; all 30 fps)

| ClipId | GUID | frames | loop |
|---|---|---|---|
| stand | 1241 | 19 | wraps |
| walk_slow / walk_medium / walk_fast | 1242 / 1243 / 1244 | 13 | wraps |
| walk_z_forwards / walk_z_backwards | 2fa7 / 2fa5 | 13 | wraps |
| jump_stand / jump_walk | 1249 / 124a | 31 | holds last frame |
| stretch_crouch (land squash) | 125a | 9 | holds last frame |

Loop vs hold is INFERRED (every clip has loopStart 0; the game decides in code).

## Rules (every one INFERRED; the EBOOT readers of the clip fields are not traced)

1. Take-off tick (`jump_tick() == Some(1)`): jump_walk if |vx| > 5, else jump_stand, frame 0.
2. Airborne: keep the jump clip, 1 frame per tick, hold frame 30; a fall without take-off
   enters jump_* at frame 15.
3. Landing: stretch_crouch frame = `dip_tick` for the first 9 ticks of the 15-tick dip.
4. Lane slide (|lane - z| > 0.5): walk_z_forwards toward +z (camera), else walk_z_backwards.
5. |vx| < 1: stand. Else walk bands by |vx|: slow <= 10, medium <= 20, fast <= 30; rate
   = |vx| / band top, clamped to 0.5..1, x FootAnimationSpeed 1 (FOUND value, INFERRED use).
   A band change keeps the 13-frame stride phase. Hard cuts everywhere else.

## Verification

- `cargo test -q -p lbp-sim -p lbp-bevy-viewer -p lbp-preview`: pass (8 new picker tests,
  three driving the real controller; 1 new `Rig::rebind` test). Clippy -D warnings, fmt clean.
- Viewer, Rookie Test (`--level-path main_01_rookie_test/main_01_rookie_test.bin`, d68 0133
  snapshot, no `--select-guid`): "clip bank 9/9 bound to one rig", stand 19 frames at 30 fps.
  Scripted run `-:1300,r:30,rj:10,r:25,-:20,b:1,-:15,j:3,-:40,l:4,-:30` logged
  stand -> walk_slow -> walk_medium -> walk_fast -> jump_walk -> stretch_crouch -> (ledge fall)
  jump_walk f15 -> stretch_crouch -> stand -> walk_z_backwards (5 ticks) -> stand ->
  jump_stand -> stretch_crouch -> stand -> walk_slow/medium -> stand.
- QA capture at the default follow camera: `local-analysis/d231-sackboy-anim-rookie.png`
  (stand pose, mesh drawn; not committed).

## Not verified

No RPCS3 comparison: band thresholds, pacing, take-off speed threshold, fall entry frame,
squash length and loop/hold are all guesses. No blending, facing still snaps. Slice 3
(Ghidra, readers of r30+0x290..0x6c0 and `BlendFrames`) should replace these rules.
