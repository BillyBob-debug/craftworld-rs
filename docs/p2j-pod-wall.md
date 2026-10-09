# P2j: Pod side panels show the room wall art; game camera findings

## Side panels

- pod.mol 00002f6c primitive 2 uses window_glassy 0001d14c: slot 0 tissue_paper_bump
  00002649 (bump port only), diffuse constant 0.118, specular 0.6, alpha_mode 4
  (CONFIRMED_FROM_LOCAL_DATA, PC research D178).
- A translucent-glass trial (`--glass`, first version of this PR) changed nothing visible:
  nothing sits behind the panels in our scene (PC D181). Removed.
- `--runtime-texture 00002f6c:0:0001f3b9` (default_wall_diffuse, through window_glassy's
  own sampler UV0 x 2) gives swirly yellow/orange/red/green/white art close to RPCS3 in
  colour and style; 0001f3c1 (default_wall02) is mostly white (PC D184, by eye).
- Change: with `--pod-shell`, that rule is now the default (`pod_wall_default`). An
  explicit `--runtime-texture` for 00002f6c slot 0 wins; `--no-pod-wall` turns it off.
  Evidence: STRONG_INFERENCE (visual match); the game's binding is unconfirmed.
- Remaining gap: RPCS3's panels look like deep windows with perspective and are wider in
  the frame; ours is a flat texture repeated x2.

Test: `p2j_pod_wall_default_yields_to_an_explicit_slot_rule` (lbp-bevy-viewer).

## Game camera (recorded for the next step; no code yet)

- PlanetsWorld moves pod_planets Thing 0 (World) every tick. The dumped matrix (D180) is
  reproduced exactly, row vectors (world = x W, translation in the last row), by
  W = [T(-eye) S(2) T(eye)] [Rx(pitch) T(offset) Ry(yaw)] with view 0 values pitch 0.15,
  yaw -0.07, offset (0,0,-15000), eye = cur_pos (scale 2 about the eye, then pitch,
  offset, yaw). CONFIRMED_FROM_LOCAL_DATA (VM run, emulated natives).
- Camera: SetCameraFromToUp gets the presets (not in use (0,-250,1130)->(0,100,-1500), in use
  (0,-300,900)->(0,500,-1500)). RPCS3 memory in the in-use menu: aspect 16:9, near 10,
  far 100000, horizontal scale 1.0296 (hfov 88.3 deg, vfov 57.3 deg), view a pure 18.4 deg
  pitch, matching the in-use preset. CONFIRMED_FROM_LOCAL_DATA (emulator memory).
- Projection with these (Earth Thing 1 at (-2295.7,1132.2,40.2), Moon Thing 4 at
  (1332.1,529.3,-51.6)): Earth x 0.374 and Moon x 0.631 against RPCS3's 0.35 and 0.605 (the
  separation matches), but y is 0.27-0.33 too low and the Earth radius (bounds 1600) is
  0.10 h against 0.22 h. Our script run never enters the main-menu view (view 1), so the
  menu's World matrix is the open question; a level aim would fix y (0.37/0.44) but
  contradicts the measured pitch.
