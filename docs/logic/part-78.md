# Logic, Part 78

See docs/logic.md for Parts 1-49.

## Part 78: a pattern-2 motor is held to its sweep (D1177)

Part 75 left pattern 2's limits (0x1ca6f0: the centre `+0xe0` to the centre + range,
sorted, for a range under 2π) unapplied while the turn sign was UNVERIFIED (Part 74).

FOUND (EBOOT 01.33), 0xc98d0, free-running pattern 2 in the length rule 0xc9790:
- With time `+0xcc` of 1 (TOC 0xd95134) or more, u = fmod(f11, time + pause `+0xd8`)
  (0x88fe2c); while u is under time the rule adds range `+0xc8` × (1 − u / time), else 0.
- It has no motor branch (unlike pattern 0's at 0xc9860, Part 76), so 0xc99d0's target
  is the centre + range × (1 − u / time): it sweeps the centre to the centre + range,
  which is exactly 0x1ca6f0's limits.

What we do: our pattern-2 rule (`SceneJoint::free_length`, D1102) sweeps [0, range] as
the game's does, so `motor_angle_limits` now gives pattern 2 the same limits relative to
the spawn angle as pattern 0: -start to -start + range, sorted. The limits follow our own
rule, so they hold whichever way Part 74's turn sign comes out. Census (Part 75): 28
motors. Pattern 2's k = 5 × strength (Part 75) is still not modelled.

Test: `d1177_a_pattern_2_motor_joint_is_limited_to_its_sweep`. Route and parity unchanged.
