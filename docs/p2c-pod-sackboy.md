# P2c: Sackboy standing in the Pod (2026-10-08)

## Question
In the closed Pod view (RPCS3, /mnt/project-files/lbp2/research/pod-reference-rpcs3.md,
about 0.02) Sackboy stands at the bottom centre facing the camera, in front of the
DualShock: centred at 0.50 of the width, head top 0.56 and feet 0.82 of the height.
Where does the shell level put him?

## Change
- `lbp_preview::level_scene::checkpoints` lists Things with a CHECKPOINT part and a
  valid POS (Thing index, world matrix), in reader order; `LevelScene::checkpoints`
  carries them. The player spawns at a checkpoint; that the Pod uses its first one is
  STRONG_INFERENCE (pod-assets-0133.md lists one Sackboy spawn in the shell).
- With `--pod-shell`, `--select-guid HEX` is no longer ignored in level mode: that mesh
  (Sackboy 0000043f) is drawn under the shell root, feet at the first checkpoint's POS
  origin, upright (Z-up model, D148 rotation), in raw model units. The checkpoint's own
  rotation is not applied (INFERRED upright).
- `--play-animation HEX` plays on it (GPU joints, or `--skinning cpu`); the joint root is
  parented under the figure, so the dolly carries it.
- The status lists where the figure lands in the closed view
  (`PodRig::shell_frac`) next to the RPCS3 numbers, so the PC run measures the scale.
- Playback now poses only the clip's own meshes (`Playing::meshes`), not the level.
- A new load drops the previous shell root (`clear_pod_shell`).

## Evidence and limits
- Scale 1 (model units = level units) is UNKNOWN until the PC line is compared.
- Holding the controller, seen from behind, in the open view is the next slice.
- Tests: `checkpoints_need_the_part_and_a_valid_pos`; the P2b rig test also checks
  `shell_frac` against `PAD_CLOSED`.

## PC check
`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --select-guid 0000043f --play-animation 0001a697 --animation-frame 0 --qa-hide-ui --qa-capture local-analysis/NEW.png`
- Report the "Figure at shell checkpoint" status line (or "Figure not placed").
- Expect Sackboy in front of the DualShock at the bottom centre, facing the camera.
