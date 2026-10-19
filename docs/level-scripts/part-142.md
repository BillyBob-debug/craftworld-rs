# Level scripts, part 142: the active look (2026-10-19, D395)

Part 141 found that a link draws active while its target level is still locked. This
part draws that look in the viewer. No new EBOOT reads: the values are parts 136, 139
and 140's.

### Code

- `LINK_ACTIVE_COLOUR`, `LINK_ACTIVE_SHAFT_V` and `LINK_ACTIVE_HEAD_V` (viewer
  `story_world.rs`): white at alpha 160 (0xA0FFFFFF, 0x426708), v 0.125 to 0.25 across
  the shaft and 0.75 to 1 across the head (0x4a1ef8, 0x4a2008).
- `link_ribbon` takes `active`: the shaft's u runs from `neck / head` at its start to 0
  at the neck (0x4a2134), so the band repeats along it; the head's u is unchanged.
- `follow_links` draws a link active while `World::slot_unlocked` is false for its
  target, and pending (the caller's colour) once it is unlocked.

### Inferred

- A link grows towards a locked level, so it is drawn active while it grows and turns
  pending when the level unlocks at the end of the link (state 3 of the unlocking
  thread). Not compared with RPCS3 yet.

### Next

The line shader's program and the active band's motion (a capture of an unlock).
