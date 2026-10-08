# P2: the Pod from its real levels (2026-10-08)

## Findings (PC task A, /mnt/project-files/lbp2/research/pod-assets-0133.md)
The Pod is two levels, both read by lbp-level (CONFIRMED_FROM_LOCAL_DATA):
- Shell 000027b9 `gamedata/levels/pod/pod.bin`: pod.mol 00002f6c, DualShock 000042af,
  Sackboy spawn; 19 Things, 4/4 meshes. Very likely the profile's podLevel (the save's
  only LEVEL dependency; the descriptor itself is encrypted, INFERRED).
- View beyond the window 00018fdb `lbp2/pod/levels/pod_planets.bin`:
  - globe cloth_earth 000027bd at (-2296, 1132, 40);
  - moon cloth_moon 00003e6e at (1332, 529, -52), scale 2.1, `orbitingmoon.ff`;
  - backdrop plan 0001d054 (room 0001d050, bulb 0000d6c8, 34 stars);
  - world script `planetsworld.ff`.
- There is no camera Thing; the camera comes from the scripts.

## Change: preview geometry budgets
The globe was skipped because it exceeds the 50,000-triangle preview budget.
- `lbp_preview::MAX_TRIANGLES` is now 500,000 and `MAX_VERTICES` 1,000,000. These are
  shared by mesh loading, the UV derivations (affine, matrix, border, instance) and the
  Bevy adapter.
- They remain explicit refusals above the limits (the guard tests now use the constants).
- The globe's own counts are not known here (PC check).

## Next
- Solve the Pod camera from the globe's level position and the RPCS3 frame position and
  radius, so `--pod-menu` can draw the real pod_planets level with the pills on the real
  globe.
- This needs the globe mesh bounds and the viewer's level-to-display transform from the
  PC.

## PC check
1. `lbp-bevy-viewer --snapshot <d68> --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-guid 00018fdb --qa-capture local-analysis/NEW.png`
   The globe should now draw. Report its triangle and vertex counts and its mesh bounds.
2. Report the level's placed position and radius of the globe in display units (the
   transform the level mode applies), for the camera solve.
