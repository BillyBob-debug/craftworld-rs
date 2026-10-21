# Logic, Part 76

See docs/logic.md for Parts 1-49.

## Part 76: a pattern-0 motor is held to its sweep (D1175)

Part 75 left pattern 0's angle limits unapplied: centred on the start, the centre ±
range / 2 would have cut our sweep from the start to the start + range in half.

FOUND (EBOOT 01.33):
- 0xc95ec switches on the joint type `+0xc` through a table at 0xc9658: type 0 to 0xc96c0,
  types 3, 4 and 9 to 0xc9790, type 10 to 0xc96f8, the rest return the base `+0xf0`.
- Pattern 0 (0xc9834 to 0xc9898): s = 0xc79c8(frame + phase `+0xd0`), c = 0x88f64c(π s, 1)
  (a thunk to the sin/cos routine 0x759c24; the 1 adds a quadrant, so INFERRED cos). A
  piston's length is base + range × (0.5 − 0.5 c), as our D1102 rule has it.
- A motor (type 9) branches at 0xc9860 to 0xc996c to 0xc9988 and skips the + 0.5: base +
  range × (−0.5 c). So 0xc99d0's target, the rule − `+0xf0` + `+0xe0`, swings about the
  centre `+0xe0`, between centre − range / 2 and centre + range / 2, which are exactly the
  limits 0x1c9eac gives (Part 75).

What we do: our motor rule is the game's + range / 2, which the centre sync (0xc9af4)
cancels, so the turn is unchanged. `motor_angle_limits(params, start)` now gives the
limits relative to the spawn angle, where our rule is at `start`: -start to -start +
range, sorted; the motor's joint (lib.rs, `spawn_joint`) gets them for pattern 0 with a
range under 2π (2,244 motors in the 01.33 catalog, Part 75's census). Pattern 2's limits
still wait on the turn sign (Part 74). INFERRED: a motor wired in mode 1 or 4 ramps from
the base, so its limits are taken at its free-running start.

Tests: `d1174_a_pattern_0_motor_is_limited_to_half_its_range_either_side` (now the sweep)
and `d1175_a_pattern_0_motor_joint_is_limited_to_its_sweep`.
