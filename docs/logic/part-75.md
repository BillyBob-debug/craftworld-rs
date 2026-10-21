# Logic, Part 75

See docs/logic.md for Parts 1-49.

## Part 75: the motor's angle limits and its k term (D1174)

Part 72 put the motor row's lo/hi at 0xc7350/0xc7330; those are the piston's. The
motor's come from the solver's motor case.

FOUND (EBOOT 01.33), the solver's motor case 0x1c9eac (type 9):
- k (f1 of row 0x95f494) is 0 (TOC 0xd97fa8) unless the pattern `+0xc4` is 2. For
  pattern 2 (0x1ca1c8) it is 0xca760 (strength) × 5.0 (TOC 0xd984d8), or 0 when the
  joint is wired (`*(*(joint+8)+0x60)` set) in mode `+0xa0` 1 or 4. The piston and
  winch cases (0x1ca204, 0x1ca240) use the same 5 × strength.
- The angle limits (f3, f4): with a range `+0xc8` of 2π (TOC 0xd98368) or more, or a
  pattern other than 0 and 2, -FLT_MAX to FLT_MAX (0x1ca27c). Pattern 0: the centre
  `+0xe0` ± range / 2.0, sorted. Pattern 2 (0x1ca6f0): the centre to the centre + range,
  sorted.
- In row 0x95f494 (0x95f6f8, 0x95f774) the target band is the [min, max] of the clipped
  angle change and 1.5 × the wrapped error (TOC 0xd98370); the strength caps one side and
  max(|err|, 0.1) × k the other. INFERRED: with k 0 (every motor but pattern 2) the
  1.5 × err side has no force.

What we do: `motor_angle_limits` (crates/lbp-sim/src/lib.rs) returns pattern 0's limits
relative to the start angle, but the joint does not use them yet. With the centre synced
when the motor first runs (0xc9af4, Part 70) the game's own pattern-0 target runs from
the start to the start + range (0xc99d0, as our D1108 rule does), so limits of the centre
± range / 2 would cut it in half. Either `+0xe0` at the solver is not the synced centre
or the pattern-0 rule is centred (Part 10's "centre ± range / 2" sweep); UNVERIFIED.
Pattern 2's limits and k wait on the turn sign (Part 74).

Census (01.33 catalog, 3,356 motors, key `motor limits`): 2,244 centre ± range / 2, 28
centre to centre + range, 33 none (range 2π or more), 1,051 none (pattern).

Test: `d1174_a_pattern_0_motor_is_limited_to_half_its_range_either_side`. Route and
parity unchanged (no behaviour change; with the limits applied the route was unchanged
too).
