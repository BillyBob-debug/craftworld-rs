# P1b: the Pod main menu on the globe (2026-10-08)

## Question
The RPCS3 reference (/mnt/project-files/lbp2/research/pod-reference-rpcs3.md) shows that the
Puter has no flat screen. With the Puter open, the main menu looks like this:
- the cloth Earth globe, with pill labels down its right side;
- the moon, carrying "Me";
- a profile Polaroid between them.
How close can the viewer get with what we have now?

## Change
- `lbp_ui_script::pod::layout(&Frame) -> PodMenu` places the script's buttons:
  - all buttons but the last go down the globe's right side, on an arc between the
    measured Story (0.40/0.32) and Store (0.38/0.53) positions;
  - the last button goes on the moon (0.62/0.40);
  - `DoPlayerBigIcon` becomes the Polaroid (0.50/0.36).
  Positions are fractions of a 16:9 frame.
- Viewer `--pod-menu CLASS` (crates/lbp-bevy-viewer/src/pod_menu.rs):
  - camera: level, looking -Z;
  - globe and moon: spheres placed so they project to the measured centres and radii;
  - the globe spins slowly;
  - pills are UI nodes at the placed points; Up/Down move a cyan, pulsing highlight;
  - Enter prints the chosen label.

## Evidence labels
- Globe/moon/Polaroid centres, radii and the two end pills: CONFIRMED_FROM_LOCAL_DATA
  (RPCS3 captures, about 0.01 frame accuracy).
- Buttons-in-order to pills and last-to-moon: STRONG_INFERENCE (trace order matches the
  capture).
- Arc shape between the ends, Me pill position, FOV, colours, pulse rate: UNKNOWN,
  chosen by eye.
- Globe, moon, Polaroid and background are PLACEHOLDERS until the Pod level/meshes are
  found (PC level search). The window says so unless `--qa-hide-ui`.

## Tests
- `p1b_buttons_go_down_the_globe_and_the_last_onto_the_moon`
- `p1b_one_button_stays_on_the_globe`
- `p1b_frame_points_project_back_to_their_fractions`

## PC check
`lbp-bevy-viewer --pod-menu PodStateMainMenu --snapshot <d68> --game-dir "game-data/a1-disc;game-data/a1-update-0133" --qa-capture local-analysis/NEW.png`
- Report the printed "Pod menu:" line.
- Compare the capture with d156-rpcs3-pod-* (main menu): pill positions, globe and moon
  size and position.
