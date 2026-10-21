# Logic, Part 72

See docs/logic.md for Parts 1-49.

## Part 72: a motor never runs past its target (D1171)

Part 10 left out the motor row's position term and strength cap (0x95f494).

FOUND (EBOOT 01.33):
- The row builder 0x95f494 takes the row (r3), the joint (r4), f1 = k, f2 = strength,
  f3 = lo (0xc7350) and f4 = hi (0xc7330). 0xc7330 returns the larger of the base `+0xf0`
  and `+0xf0` + range `+0xc8`, and 0xc7350 the smaller.
- A motor with pattern `+0xc4` 1, or wired with mode `+0xa0` 1 or 4, takes 0x95fa44: its
  rate is 0xc7444, which calls 0xc7370. Otherwise (0x95f618) it calls 0xc99d0 at the
  frame and the frame + 1.
- 0xc7370: pattern 1 gives speed `+0xd4` × the clock rate (`*(+0x190)+0x3c`); with
  pattern 1 wired in mode 2 the speed is dropped to 0 when a term from the clock's
  `+0x34`, `+0x38` and `+0x3c` passes 2π. Any other pattern gives range / (time × 0.5) ×
  the clock rate when time is 1 or more (constants 2π 0xd95130, 1 0xd95134, 0.5 0xd95138,
  0 0xd9513c), else 0. This is what `PistonDrive::turn` already returns on that path.
- At 0x95f688 the row wraps to [-π, π) both `delta` (the angle's change to the next
  frame) and `err` (the angle at the next frame less the measure 0xc9250). Off the rate
  path, it cuts `delta` to between 0 and `err` (0x95fa28 for `delta` > 0, 0x95fb10
  otherwise), so the motor never runs past its target and never turns back. On the rate
  path it takes `delta` as it is.
- Its second bound is 1.5 (0xd98370) × `err` with the cap k × max(|err|, 0.1) (0xd980cc);
  the bounds swap when needed. Both are scaled by the ends' effective mass (the sum of
  `*(body+0x110)+0x34`, reciprocal). The bound on `delta` has the strength cap.
- The limit rows take the measure wrapped about the centre `+0xe0` and push it back
  inside lo and hi with a slop of 0.01 (0xd98040).

lbp-sim: `PistonDrive::turn_to` takes the measured relative angle (body 2 less body 1,
as avian's motor counts it) and, off the rate path, syncs the angle rule to it when the
motor first runs (0xc9af4) and cuts the turn to between 0 and the error. `drive_motors`
reads the ends' rotations. INFERRED and left out: the strength cap, the second bound
(k is 0 except for pattern 2, Part 70) and the 2π cut-off of pattern 1 in mode 2. The
sign of the game's angle against avian's is still UNVERIFIED.

Test: `d1171_a_motor_off_its_rate_path_never_runs_past_its_target` in
crates/lbp-sim/tests/sandbox.rs.

Census (local-analysis/logic/d1171/census1.json, new `motor pattern P wired W driven` or
`clamped` keys): of 3,356 motors, 1,508 are driven at their rate (822 pattern 1 free,
229 pattern 1 wired, 457 pattern 0 wired in mode 1 or 4) and 1,848 are cut to their
target (1,297 pattern 0 free, 523 pattern 0 wired in another mode, 28 pattern 2). The
Rookie Test has 4 free pattern 0 motors, now cut, and its wired motor 58, driven as
before. The route is unchanged and parity passes 5/0.
