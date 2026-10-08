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

## PC result for the budget change
- The globe draws (screenshot page v6, Pod section). Its seas are grey: one material
  route is unsupported.
- cloth_earth 000027bd: 25,440 vertices, 59,031 triangles, 9 primitives, 512x512 BC1
  textures tiled x11.
- Thing POS (-2296, 1132, 40), row scale 0.96. Inverse-bind placement shows scale x1.405
  and a 90 degree rotation against POS-only.
- Placed radius is about 1,600 level units.

## Change: Pod camera and menu over the real level
- `--pod-camera N` (with a level) sets the home view:
  - level and looking -Z, as in the RPCS3 frame;
  - placed so Thing N's drawn bounds (the globe) project to the reference centre
    (0.35/0.40 of a 16:9 frame) with radius 0.22 of the height;
  - `pod_menu::pod_camera` solves it for the viewer's 45 degree FOV.
- Only the projection is matched. The game's camera is script-driven (UNKNOWN), and the
  FOV is the viewer's.
- The radius is half the largest extent of the globe's drawn bounds, an approximation of
  the visible silhouette.
- `--pod-menu CLASS` together with a level draws the pills over the level view instead
  of the placeholder scene:
  - reference fractions are mapped into the 1280x800 window, keeping offsets in heights;
  - Up/Down/Enter work as in P1b.
- Test: `p2_pod_camera_projects_the_globe_to_its_reference_place`.

## PC check (pod camera)
`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera <globe Thing index> --pod-menu PodStateMainMenu --qa-hide-ui --qa-capture local-analysis/NEW.png`

The globe's Thing index is in `--level-report` / the d155 notes.

Expect:
- the globe left of centre;
- the pills down its right side;
- the moon to the right (its height differs: it orbits in the game).

Put the capture on the screenshot page.

## Next
- Supported material route for the globe's seas.
- The Pod shell level (000027b9) composed with the view, Sackboy at the controller, and
  the open-Puter dolly.

## PC check
1. `lbp-bevy-viewer --snapshot <d68> --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-guid 00018fdb --qa-capture local-analysis/NEW.png`
   The globe should now draw. Report its triangle and vertex counts and its mesh bounds.
2. Report the level's placed position and radius of the globe in display units (the
   transform the level mode applies), for the camera solve.
