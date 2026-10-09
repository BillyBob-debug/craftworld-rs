# P2k: the Pod planets view through the game's own camera (`--pod-game-camera`)

Question: the planets view used eyes fitted to the globe's place in RPCS3 frames. Mid-dolly
it misplaced the moon (PC, D170). Can the game's own camera be used?

## Findings (2026-10-08 America/Chicago; PC VM traces D169/D180/D174 and RPCS3 memory reads)

All CONFIRMED_FROM_LOCAL_DATA unless noted.
- The camera stays at the PlanetsWorld presets, with no base offset: not in use
  (0,-250,1130) to (0,100,-1500); in use (0,-300,900) to (0,500,-1500); up +Y.
- Projection (RPCS3 parameter block, in-use menu): 16:9, near 10, far 100000, horizontal
  scale 1.0296. That is vfov 2 atan(1/1.8305) = 57.3 deg. The view is a pure 18.4 deg
  pitch, matching the preset. Only the in-use menu was read; closed is assumed the same
  (STRONG_INFERENCE).
- PlanetsWorld moves the whole level every tick with pod_planets Thing 0 (World), column
  vectors: `W = T(eye) S(2) T(-eye) Rx(pitch) T(offset) Ry(yaw)`, eye = camera position.
  View 0 (not in use: pitch 0.15, yaw -0.07, offset (0,0,-15000)) reproduces the D174 VM
  matrix exactly. View 1 (main menu: pitch 0.35, yaw -0.07, offset (-700,400,-5000))
  reproduces the RPCS3 menu read exactly (translation (-1400,4480,-10019)). The VM's Mat44
  product had swapped operands before D174.
- In the menu, the Earth and Moon do not sit at their stored POS. RPCS3 world matrices
  divided by W give Earth (-1250,0,0) at scale 1.3505 (spun about its axis) and Moon
  (2000,0,0) at scale 1.5444, in World space. Who sets these (PlanetsView) is not traced;
  the values are read, not derived.
- Check (unit test): through this camera the Earth and Moon centres land at (0.334,0.417)
  and (0.615,0.415) open (RPCS3 (0.35,0.40) and (0.605,0.417)). Closed (view 0), the Earth
  lands at (0.46,0.48) (reference 0.47/0.48), and the moon at 0.56 w, as in the RPCS3
  mid-dolly frame.

## Change

`--pod-game-camera` (with `--pod-camera 1 --pod-shell 000027b9`):
- Moves the Earth (the `--pod-camera` Thing) and the Moon (Thing 4) to the menu placement.
  Each is scaled about its first model item's origin; the rotation is kept (no spin).
- Hangs the planets level under a root carrying `N W(t) N^-1` (N = display normalization).
- Points the planets camera from the presets (`PodRig::with_planets_camera`) at vfov 57.3.
  The shell layer follows a turned planets camera (`X -> P + s Rp R^-1 (X - S)`), and the
  shell eyes are fitted at the same FOV.
- Over the dolly, the view (0 to 1) and the eye blend with the same smoothstep. The game
  blends the view (SmoothStep 0.05/tick) and the camera (Smooth3D) separately, so
  mid-dolly frames are approximate (INFERRED).
- The info panel shows "Game camera (...): Thing 1: N items from (...) scale ... to ...".
  The "from" should equal the stored POS: Earth (-2296, 1132, 40) scale 0.961.

Tests: `p2k_planets_world_matches_the_game_and_frames_the_globe_and_moon`,
`p2k_turned_planets_camera_keeps_the_shell_projection` (lbp-bevy-viewer).

Not tested here: any capture (no game data in the cloud session). Not modelled: the
Earth's spin, the Story planet views (4-7), the crash zoom, the theme transition.

## PC check

`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f --play-animation 0001a697 --pod-hold-animation 000042b2 --animation-frame 0 --pod-game-camera --qa-hide-ui --qa-capture local-analysis/NEW.png`

Run it closed, with `--pod-t 0.5`, and with `--pod-open`. Compare each with the RPCS3
frames: Earth and Moon centre and radius, window and pad edges, and Sackboy.
