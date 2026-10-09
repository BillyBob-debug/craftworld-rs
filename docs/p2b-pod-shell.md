# P2b: the Pod shell over the window view, and opening the Puter (2026-10-08)

## Question
In the game, the default Pod view looks out of the fabric tent (shell level 000027b9)
through its window onto the planets view (00018fdb). Pressing Square dollies the camera
through the window in about 1 s to the menu on the globe. In the RPCS3 previews
(/mnt/project-files/lbp2/research/pod-reference-rpcs3.md, PC d156, about 0.02):
- the globe moves from 0.47/0.48 to 0.35/0.40 and grows about 3x;
- the DualShock moves only from 0.50/0.70 (width 0.15) to 0.50/0.86 (width 0.17);
- no tent fabric is visible once open.
So the two levels are seen by separate cameras (STRONG_INFERENCE).

## Change
`--pod-shell 000027b9` together with `--level-guid 00018fdb --pod-camera 1` loads the
shell as a second level and draws it as a layer over the view.

`pod_menu::PodRig` places the shell:
- planets eyes: closed from `GLOBE_CLOSED` (0.47/0.48; radius 0.075 INFERRED from
  globe-moon spacing), open from `GLOBE` (P2);
- shell eyes: from the DualShock (mesh 000042af, Thing 5) bounds and `PAD_CLOSED` /
  `PAD_OPEN`;
- the viewer has one camera, so the shell root transform is `P + scale (X - S)`.
  Scaling about the eye keeps the shell's projection from its own eye, and `scale` puts
  the shell's far side at half the closed eye's distance to the globe.

Space opens or closes the Puter:
- both eyes move together over `DOLLY_SECONDS` (1 s, smoothstep easing UNKNOWN);
- the menu pills (`--pod-menu`) show once fully open.

The home view (R) is the closed view.

## Evidence and limits
- The cameras look -Z with no tilt (the RPCS3 views look level). The FOV is the
  viewer's 45 degrees, so only the measured projections are matched.
- Sackboy standing: P2c (docs/p2c-pod-sackboy.md).
- The shell's own lighting and settings are not used (the view's apply).
- Tests: `p2b_rig_keeps_each_layer_where_its_reference_puts_it` (both layers project to
  their reference points closed and open; the shell stays in front of the globe).

## PC check
`lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --qa-hide-ui --qa-capture local-analysis/NEW.png`
- Expect the closed view: tent walls and window, globe and moon through the window,
  DualShock at the bottom centre, no pills.
- On the second monitor, press Space: the view should dolly to the open menu.
- Compare both with the RPCS3 previews (window opening 0.35-0.67 x 0.28-0.69).
