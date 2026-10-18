# D414: the Popit tooltip takes the game's side rule (Popit part 11)

Before: the tooltip pill leaned only when the card sat near a screen edge or a centred
pill would not fit (D413); everywhere else it was centred above the top row, below the
bottom row, or always to the right of a middle-row button. After: the x side follows the
game's whole rule, so on a centred card the outer buttons of the top and bottom rows lean
outward (INFERRED to match the Moon create frame's top-right focus, pill to the right;
not yet compared side by side) and a middle row's
left-hand buttons put the pill on their left.

## The rule (FOUND, EBOOT 0x417b78, root-frame Gooey units)
- The root is the topmost ancestor of the focused node (0x894aec -> 0x479e44, parent
  pointer +8). In `Poppet.UpdateUI` that is frame 1, the bubble round the page (the main
  menu's frame 10000 opens and closes inside it), so for the main menu: the card.
- off = half the tooltip + (80, 110); (130, 180) once a tooltip with body text has shown
  more than 59 frames. Candidate centres lo = node.min - off, hi = node.max + off;
  t = (node centre - root.min) / (0.95 root size); p = lerp(lo, hi, t) per axis.
- Safe area NDC x +-0.88, y -0.65 (or -0.80) to 0.88, shrunk by button prompts.
- x side, normal root: left does not fit -> right; right does not fit -> left; root
  centre NDC x > 0.6 -> left, < -0.6 -> right; else when p.x is outside the root or p.y
  is not, the outward side (t.x >= 0.499 -> hi); else (out on y only) the lerped x.
- y: when p.y is outside the root it snaps to lo / hi by t.y (0.499), then is clamped
  into the safe area; x is never clamped. Out on both axes: p *= 0.9 (corner pull).
- Wide root (wider than 0.75 of the safe area): the inward side, unchecked; neither side
  fitting: centred, inner vertical side. Flag T+0x49: (node centre x, hi.y).
- The stored top-left (p - half) snaps for 15 frames, then eases 0.4 per frame; z +-10.
  The tail is not placed here (INFERRED: the tooltip renderer draws it).

## Port
- `pill_side` takes the card's and the button's x spans (the 128-unit icon cell, not the
  68 px focus tile), the pill width and px per unit (UNIT_PX), and returns the side in
  the game's order. Which row puts the pill above, below or beside stays the MEASURED
  row rule: the y test is too close to the card edge with the port's pill height to
  decide it (top-row p.y lands within a few px of the card top).
- A leaning pill keeps its MEASURED D413 geometry; the beside-left place mirrors the
  MEASURED beside-right one. The game's candidate distance (half + 80 units, ~26 px) and
  the 0.9 corner pull are not used for drawing.
- Check (disc + update only, no decrypted copy): the D409 MEASURED Pod command with
  `--snapshot d79-inventory-0133.json --game-dir "game-data/a1-disc;game-data/a1-update-0133"`:
  pill above and right of the top-left button, as before (card left of NDC -0.6).

## Open
- The y side from the rule itself (needs the tooltip's own Gooey size), the wide-root
  path, the ease, and the tail's renderer.
