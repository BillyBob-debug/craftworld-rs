# Lane state: sackboy model (2026-10-18)

## Read first
1. `docs/d700-sackboy-model.md`: D703 (eye width) and the Open list (where the game narrows
   the eyes, zip pull brightness, program decoding).

## State
- PR for branch `sackboy-model-4` (worktree `E:\Projects\wt-sackboy-model`): D703 narrows
  the eyes to the game's measured width and adds `--window-scale N`; merge via coordinator.
- RPCS3 closed. Decision numbers: D700 series, next free D704.

## Facts
- Close game frames: launch with `rpcs3_launch.launch(('--no-gui', '--config', COPY, L.ISO))`
  where COPY is a copy of `config/config.yml` with `Resolution Scale: 300` (nothing
  persistent changes). The window itself shows white; F12 via bgpad saves real 3840x2160
  frames (`local-analysis/pc-helpers/f12cap.py OUT.png 640`, waits for the write, deletes
  the original). Pod: X, X (intro, profile), then the default Pod view.
- Viewer match: the D700 Pod play command plus `--window-scale 3`; our Sackboy face sits
  45 px lower than the game's at 3x (crop game 1790,1450 / ours 1795,1495, 240 px).
- Eye blobs (luma < 12): game left eye 23-26 x 31-32 px; ours before 29 x 32, after D703
  24-26 x 32, eye spacing ours 91 px vs game 87.
- The game's eyes sit in a soft dark ring (socket AO); ours have a crisp edge.
- `eye_probe`: eye vertices follow only `bone_eye_left/right` (27, 28); no morph reaches them.
- Pull gap: knit and pull programs end alike, so look at the pull's light inputs
  (aomap u8 / `aotweak`, rim terms) or its texture data.

## Next
1. Find where the game narrows the eyes (EBOOT: eye bones, eye material, draw scale) and
   replace `EYE_WIDTH` with that rule.
2. The dark ring around the eyes.
3. Zip pull brightness (D701).
