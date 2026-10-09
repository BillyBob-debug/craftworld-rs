# P2c: Sackboy standing in the Pod (2026-10-08)

## Question
In the closed Pod view (RPCS3, /mnt/project-files/lbp2/research/pod-reference-rpcs3.md,
about 0.02) Sackboy stands at the bottom centre facing the camera, in front of the
DualShock: centred at 0.50 of the width, head top 0.56 and feet 0.82 of the height.
Where does the shell level put him?

## Change
- `lbp_preview::level_scene::checkpoints` lists Things with a CHECKPOINT part and a
  valid POS (Thing index, world matrix), in reader order; `LevelScene::checkpoints`
  carries them.
- With `--pod-shell`, `--select-guid HEX` is no longer ignored in level mode: that mesh
  (Sackboy 0000043f) is drawn under the shell root, upright (Z-up model, D148 rotation),
  at model scale (model units = level units, STRONG_INFERENCE).
- First PC run (2026-10-08): feet at the first checkpoint's POS origin (Thing 4) put him
  at -0.12..0.05 h, above the frame. The game's spawn offset from the checkpoint is
  UNKNOWN, so the spot is now fitted: `PodRig::fit_figure` solves the depth from the
  projected height (0.26 h) and the height from the feet (0.82 h), centred at 0.50 w.
  The status prints the solved feet in level units next to the checkpoint's, so the two
  can be compared.
- `--play-animation HEX` plays on him (GPU joints, or `--skinning cpu`); the joint root
  is parented under the figure, so the dolly carries it.
- Playback poses only the clip's own meshes (`Playing::meshes`), not the level.
- A new load drops the previous shell root (`clear_pod_shell`).

## P2d: holding the controller, input, `--pod-open`
- Open, RPCS3 shows him from behind holding the controller at about 0.50/0.90. Past
  half-open (switch point UNKNOWN) `pod_figure` turns him half a turn on the same spot
  and swaps in `--pod-hold-animation HEX` when given; closing reverses both. The hold
  animation's GUID is not known yet (needs a catalog search on the PC).
- Gamepad: Square toggles the Puter (as Space), Circle closes (as Esc/Backspace,
  STRONG_INFERENCE from the usual LBP back button). With the Puter open, D-pad
  Up/Down and Cross move and choose the menu pill (as the arrows and Enter).
- `--pod-open` starts open (QA captures of the menu view); R still returns to the
  closed view.

## Evidence and limits
- The fit is a placement to match the reference, not the game's spawn rule.
- Tests: `checkpoints_need_the_part_and_a_valid_pos`; the P2b rig test also checks
  `shell_frac` against `PAD_CLOSED` and that `fit_figure` lands on `SACKBOY_CLOSED`.

## PC check
Closed: `lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f --play-animation 0001a697 --animation-frame 0 --qa-hide-ui --qa-capture local-analysis/NEW.png`
Open: the same with `--pod-open` and another new capture path.
- Report the "Figure fitted ..." status line.
- Expect Sackboy at the bottom centre in front of the DualShock facing the camera
  (closed), and from behind below the menu (open, pills showing).
