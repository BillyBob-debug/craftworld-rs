# Level scripts, part 97: the overlays' turn axis and the badge's first fill (2026-10-17, D350)

Part 96 made a Story badge's overlays spring in from nothing whenever the Story search
opened, turned about the card's Y. This part checks the axis in the game's draw and finds
who sets the card's grow flag: the badge's first fill shows its overlays settled.

### What was found

- FOUND (01.33, 0x6fef14-0x6ff0c8, inside 0x6fe974): the angle (current spin times pi/8)
  goes through an inline sine and cosine (range step 2/pi at r2+0x6dd8, pi/2 split
  r2+0x6de0 / r2+0x6de4, polynomials r2+0x6de8-0x6dfc). Two rows are built with the lane
  masks r2+0x6dc8 (X) and r2+0x6dd0 (Z): (cos, 0, -sin) and (sin, 0, cos), and they mix the
  scaled mesh's X and Z rows; the Y row is kept. So the turn is about the mesh's Y, the
  card's depth axis (part 90's badge basis): the overlay turns in the card's plane, with
  glam's `from_rotation_y` sign. The viewer already did this; the second read in part 96's
  handoff (the card-plane Z) was wrong.
- FOUND (who sets card+0x22c, a byte): the two card constructors clear it (0x6fc5c0,
  0x6fc868), the planets' rebuild clears it for every card (0x42330c, after 0x6fbaa0), and
  the per-card fill loop of 0x42a92c sets it to 1 (0x42b288) after the slots are made
  (0x6fcd30 at 0x42b1dc, which reaches 0x6fca08) and placed (0x6fd540). Its only other
  reader is 0x6fca08 (0x6fcbb4). So the slots of a card's first fill start at rest (scale 1,
  their spin); only slots made on later updates spring in from 0.
- FOUND: 0x6fe974 is called only from 0x4226b0 (0x4227f8 per card, 0x422944), which is
  called at the end of 0x42f43c, slot 0x24 of the planet's vtable 0xd745a8; 0x42f43c first
  runs the fill 0x42a92c. Update and draw are one call; the spring has no time step.
- INFERRED: that call runs once per game tick (its caller, CPlanets slot 0x1c at 0x43d900,
  is reached through a dispatcher not traced); the rebuild (0x42324c, from 0x4235fc and
  0x436cc4) runs when the Story search opens, so each opening is a first fill.

### What changed

- Viewer: `OverlayGrow::settled`; `grow_badge_overlays` shows overlays settled when none
  was shown on the previous update (the badge's first fill), and springs in only overlays
  shown after that.

### Left out

- The game's spins (the generator state), the dispatcher above CPlanets slot 0x1c, and
  whether a hidden slot shrinks out (drawn while its scale is above 0).

### Verification

`d350_overlay_turn_is_the_games_row_mix`, `d350_first_fill_is_settled`,
`d349_overlays_spring_in_from_nothing`. `cargo fmt --all --check`; `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
