# P1: the Puter screen in the 3D view (2026-10-08)

## Question
The user wants the Pod itself, not a flat menu window: the menu is what the Pod computer
(the "Puter") shows. First slice: draw the script menu (U12b-U12d) onto a screen inside
the 3D viewer, so it can be placed in the pod room once the room is found (PC task A).

## Status after the RPCS3 reference (PC, 2026-10-08)
The real Puter has no flat screen. The main menu is the 3D cloth Earth globe in the
workshop, with pill labels curving down its right side (Story, Community, Recent Activity,
Store); the moon is "Me", and a profile Polaroid sits between them
(/mnt/project-files/lbp2/research/pod-reference-rpcs3.md, CONFIRMED_FROM_LOCAL_DATA).
So the quad below is a debug view, not the game's look. What carries forward is the
plumbing: `add_gooey`, `GooeyTarget` and `run_script` let the 3D viewer host the script's
tree. The globe menu (pills placed on the globe, driven by the same tree) is the next
slice.

## Change (crates/lbp-bevy-viewer)
- `gooey_ui::add_gooey` adds a gooey screen (input, rebuild, focus) to any app;
  `run_gooey` keeps its own window and uses it. `GooeyTarget` names a camera that renders
  to a texture; the rebuilt root gets `UiTargetCamera` for it.
- `ui_script::run_script` loads the scripts and runs one function (shared by
  `--ui-script` and the Puter).
- `puter.rs`: `--puter-screen CLASS [--puter-function NAME] [--puter-at X,Y,Z[,WIDTH]]`
  in the main 3D viewer. The tree renders through a second `Camera2d` (order -1) into a
  1280 x 720 texture (the size the Gooey host answers to screen queries), shown unlit on
  a 16:9 quad facing +Z. Arrows/Enter move and activate focus (keyboard only; mouse
  picking on the quad is not wired). The count lines are printed, not drawn.
- The texture camera spawns in PostStartup so the egui panel stays on the 3D camera.

## Evidence and limits
- Placement is a viewer choice (default centre 1.1, 0, 0, width 1.2 display units, to
  the right of a single upright mesh); the game's screen placement and camera are
  UNKNOWN until the PC reference captures (task B).
- The script runs once; activating a button does not run the script again (P3).
- Test: `p1_puter_placement_parses_and_rejects_bad_values`.

## PC check
`lbp-bevy-viewer --snapshot <d68 snapshot> --game-dir "game-data/a1-disc;game-data/a1-update-0133" --select-guid 0000043f --puter-screen PodStateMainMenu --qa-capture local-analysis/NEW.png`
Expect Sackboy with a screen to his right showing the five labels (Story, Community,
Recent Activity, Store, Me); the egui panel still on the main view. Also try arrows on the
second monitor: the yellow focus outline should move between the buttons.
