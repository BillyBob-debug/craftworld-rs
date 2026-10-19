# Level scripts, part 157: the planets' resting point (2026-10-19, D1010)

The planets' view turns the Earth toward a point of the planet object (the script's
`UpdatePlanetsView`). With nothing selected the viewer passes a placeholder point,
(0, 0, 1), and the source of the game's point was not read.

### Found

- 0x421708 returns nothing; it loads the point into a vector register (v2), which
  0x42115c hands with a bool to the script call 0x65a1ac. The call site clears w.
- Its order: a locked focus first (planet byte +0x216 set: the vector at +0x220), then
  the selected entry (+0x94): its record's place (+0x40), else the vector at +0x190.
  The first two clear the "arrived" byte +0x215.
- +0x220 and +0x216 are written only by 0x4214d8, the native `SetLockFocus__r`
  (bound at 0x6102b8); 0x420854 clears +0x215 and +0x216. Nothing here binds
  `SetLockFocus__r` yet.
- With the planets object's byte +0xb1 set, 0x4208fc loads +0x190 instead.
- +0x190 has two getters (0x4208e8, 0x422950 with w cleared) and no store in the
  planet code (0x41f000 to 0x423000).

### Inferred

- +0x190 is the base Thing's position; this was not read. The placeholder stays until
  the planet's position is known.
- The bool is the planets object's byte +0xb0 when set, else the planet's +0x215.

### Code

- Comments only: `PLACEHOLDER_PLANET_POINT` and the planets' arguments name the paths
  above, and the native planets world's header lists what is ported since D383.
