# Level scripts, part 96: the badge overlays' spin and spring-in (2026-10-17, D349)

Part 93 put a Story badge's overlays (the medals, padlock and rosettes) on the card, each
turned by a fixed per-slot hash and shown at full size at once. This part follows the
game: the spin is the game's random draw, and each overlay springs in from nothing.

### What was found

- FOUND (01.33, 0x6fca08, 0x6fcc0c and 0x6fcce4): one draw of the game's shared generator
  (the word at game+0x400, x = (x 0x2e90edd + 0xe908fc1) & 0x3fffffff, the same generator
  as Sackboy's idle) when a shown slot's mesh is first made: r = 2 ((x >> 16) & 0x3fff)
  6.1038882e-5 - 1 (r2+0x6c68), in [-1, 1]. Slot 0 (the card) takes spin 0 (0x6fcfd0).
- FOUND: each slot keeps a scale record (card+0x68 + 12 s) and a spin record (card+0x104 +
  12 s), each {target, current, previous}. With the card's grow flag (card+0x22c) set,
  a new slot starts at scale {1, 0, 0} and spin {r, 0, 0} (0x6fcc78); without it both
  start settled.
- FOUND (0x6fec44, inside 0x6fe974, for every slot with a mesh): next = current + 0.7
  (current - previous) (r2+0x6db0 = 0x3f333333) + 0.25 (target - current) (r2+0x6d14 =
  0x3e800000), for the scale and the spin alike. The draw takes the current spin times
  pi/8 (r2+0x6dd4, 0x6feee0) and the current scale on all three axes (0x6feee8); a slot is
  drawn while it is shown or its scale is above 0 (r2+0x6c50).
- FOUND: no stagger: all of a badge's new overlays start together.
- INFERRED: the generator's state at the draw depends on about 44 other users, so the
  viewer starts it at a fixed seed (1); one update per game tick (0x6fe974 per frame or
  per step was not settled); who sets card+0x22c.

### What changed

- Viewer: `overlay_spins` (the game's generator, one draw per shown slot in slot order),
  `OverlayGrow` (the two-record spring) and the `grow_badge_overlays` tick system: while
  the Story search is up each overlay grows from scale 0 and spin 0 to scale 1 and its
  spin, overshooting as the game's does; hidden, it resets.

### Left out

- The game's actual spins (its generator state is not known), the tick rate of the update,
  and the rotation axis is still the card's Y from part 93.

### Verification

`d349_overlay_spins_are_the_games_generator`, `d349_overlays_spring_in_from_nothing`,
`d346_overlays_sit_on_the_card`. `cargo fmt --all --check`; `cargo clippy -p
lbp-bevy-viewer --all-targets -- -D warnings`; viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
