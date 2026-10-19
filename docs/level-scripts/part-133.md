# Level scripts, part 133: drawing the links (2026-10-18, D386)

While the Story search is up, each pending link with progress above 0 is drawn on the
Pod's Earth as a green ribbon with an arrowhead, growing from one level's badge towards
the other's as part 132's progress runs.

### What changed

- FOUND (EBOOT): the planets' draw 0x433328 (vtable 0xd745a8 +0x30) calls the link
  drawer 0x426484 at 0x433be4. It walks the link records (planets+0x104/+0x108, 0x50
  bytes each) and draws each through 0x4a1e40 with its progress (record+0x48).
- FOUND (0x42676c-0x426784): the angle between the two badges' directions (+0x200);
  a link under 0.01 radians, or whose badges' angular radii (+0x210, radians, used as
  is) add up to the angle or more, is skipped, as is one with a missing badge or the
  same badge at both ends. 0x4a1e40 also skips progress 0 or below.
- FOUND (0x426918, 0x4a1e40): the arc runs from the first badge's centre (not clipped)
  to the second badge's edge, a slerp at 1 less its radius over the angle. The tip is
  at progress times that arc; the shaft ends an arrowhead's length (degrees) before it.
- FOUND (0x426694-0x4268c8): shaft width 80 or 40 and head length 9 or 6 degrees, both
  scaled by 0.4; 48 and 7.2 when both badges are of one kind. The head is twice the
  shaft's width; the strip builder 0x4a1250 takes 10 degrees a segment. The radius is
  planets+0x1c.
- INFERRED: the colour (0xA0FFFFFF through shader parameter 0x9cc50ca0 and the
  constants at 0xc648f0, about (0.094, 1.0, 0.0117) at alpha 160).
- lbp-bevy-viewer `story_world.rs`: `planet_frame` (factored out of the badges),
  `link_ribbon` builds the ribbon, `LEVEL_BADGE_ANGLE`, the link constants.
- lbp-bevy-viewer `level_play.rs`: `follow_links` builds the ribbons when the progress
  changes and places them each tick where the Story badge is drawn now (its global
  transform less its own place, so the World Thing above the level and the Earth's
  moves both count).
- lbp-runtime: `LevelWorld::link_dirs` gives a link's two badge directions.

Run on the disc with the scripts (Pod menu, Cross on the selected level at tick 720,
finish at 30, hold at 850): back in the Pod, the link's ribbon shows beside the Cosmos
badge, with progress 1.

### Not found (INFERRED or PLACEHOLDER)

- A level badge's radius at +0x210: only the groups' 0.27 was measured (D344). The disc
  run's link spans 13.5 degrees, so a level's is under 0.118; this takes 0.09.
- The ribbon's texture (UVs 0.75 / 0.125 / 0.25 / 0.5 by a pending flag) and the render
  state: the ribbon is flat. It follows the sphere at planets+0x1c (no lift was found)
  while the Earth model's relief rises above it, so the viewer draws it over the Earth
  with a depth bias (without it the relief hides the ribbon).
- Which case gives 80 or 40 and 9 or 6 (this takes 80 and 9), and the 48 / 7.2 kind.

### Next

The ribbon's texture and the level badges' own radius.
