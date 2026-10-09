# Pod vs RPCS3 comparison (2026-10-08)

Reference frames: the user's own LBP2 01.33 in RPCS3 0.0.43 (local `local-analysis/d156-rpcs3-pod-*.png`,
never committed). Ours: `lbp-bevy-viewer` at main 4968218 (`local-analysis/d172-pod-*.png`), command:
`--level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --pod-menu PodStateMainMenu --select-guid 0000043f
--play-animation 0001a697 --pod-hold-animation 000042b2 --animation-frame 0 --qa-hide-ui [--pod-open]`.
Fractions are of frame width (w) and height (h), measured on 480 px previews (about +-0.02).

| View | Item | RPCS3 | Ours |
|---|---|---|---|
| Puter closed | window opening | 0.35-0.67 w, 0.28-0.69 h | 0.33-0.67 w, 0.21-0.64 h |
| Puter closed | DualShock centre / width | 0.50/0.70, 0.15 w | 0.50/0.69, 0.17 w |
| Puter closed | Sackboy | 0.50 w, 0.56-0.82 h | 0.47-0.53 w, 0.56-0.84 h |
| Puter closed | globe centre | 0.47/0.48 | 0.47/0.47 |
| Puter open | tent visible | no | yes (arch and side panels) |
| Puter open | globe centre, radius | 0.35/0.40, 0.22 h | 0.32/0.39, 0.21 h (D157) |
| Puter open | DualShock centre / width | 0.50/0.86, 0.17 w | 0.50/0.83, 0.19 w |
| Puter open | Sackboy (from behind) | centre about 0.90 h | centre 0.91 h |
| Story | Da Vinci's Hideout planet | rendered | not rendered yet |

`--pod-shell-camera 0,-250,1130,0,100,-1500` (the PlanetsWorld "not in use" preset taken as a shell
camera) frames much closer than RPCS3 (window 0.27-0.88 w x 0.17-0.82 h), so that preset is not in
shell-level coordinates as used here.

## Biggest remaining gaps

1. Side panels: grey. Their pod.mol primitive 2 has an unrendered runtime texture slot 0; the Pod
   display textures 000027fb (draws black) and 00004277 (flat light lavender) do not reproduce the
   swirly art, so the binding is still unknown.
2. Puter open: the camera stops short; the tent arch and side panels stay in frame where RPCS3 shows
   only the room beyond the window.
3. Closed view: the window top is about 0.07 h too high (vertical fit or camera tilt).
4. Story planet (Da Vinci's Hideout) view not rendered.
5. Polaroid is a grey placeholder; the dolly transition between closed and open is not animated.
