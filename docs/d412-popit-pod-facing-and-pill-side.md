# D412 / D413: Sackboy faces the camera with the Popit open in the Pod; the tooltip pill leans (Popit part 10)

Before: in the Pod, opening the Popit turned Sackboy by pi, so he showed his back
(owner report, 2026-10-18), and the focused button's tooltip pill sat centred above the
top-left tile. After: in the Pod he keeps his own facing (toward the camera when he
stands), and the pill sits above and to the right with its tail running from the pill's
near end to the tile's corner, as in the game frame.

## D412: facing
- `popit_turn` keeps the D235 turn (back to the camera, MEASURED on the level frames)
  everywhere except the Pod. The Pod's RPCS3 play frame shows his face: a three-quarter
  view to the right, eye and front zip visible, looking up at the card.
- OPEN: the game's own facing rule while the Popit is open (the Pod frame's three-quarter
  turn and the level frames' back view are not one constant turn).

## D413: pill side (FOUND rule, 0x417b78)
- The game places a tooltip from the focused node and its root frame: a root centred left
  of NDC -0.6 prefers the right side, right of +0.6 the left, and a side the pill would not
  fit inside the safe area (NDC x +-0.88) is never taken. `pill_side` applies this to the
  pills above the top row and below the bottom row; INFERRED: the root frame is the card,
  and a centred card keeps the centred pill (D235).
- Leaning pill, MEASURED on the Pod play frame (1280x720): its near end 9 px past the
  tile's side, its bottom 26 px above the tile's top; the tail a wedge about 30 px wide at
  the pill, its tip 1 px past and 3 px above the tile's corner.
- Check: the Pod capture (D409 MEASURED command) next to `pod-popit3.png`: pill above and
  right of the top-left tile, tail down to the tile's top-right corner.

## Open
- The full rule (off = half + (80, 110) gooey units, lerp of the two candidate centres by
  the tile's place in the root, the pull to 0.9, the wide-root path) is not ported; the
  Moon create frame's top-right focus shows the pill to the right, which needs the root
  frame's size (card or larger) to decide.
