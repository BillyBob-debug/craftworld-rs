# Logic, Part 80

See docs/logic.md for Parts 1-49.

## Part 80: the motor row's band on the joint (D1179)

Part 79 decoded the motor row 0x95f494's band: the cut `delta` edge with the strength cap
and the 1.5 × `err` edge with a cap of k × max(|err|, 0.1), k = 5 × strength for
pattern 2 only. The joint still pushed both ways toward the cut `delta`.

What we do (D1179): each tick `PistonDrive::band_to` (crates/lbp-sim/src/lib.rs) takes
the ends' relative turn rate (body 2 less body 1) and sets the joint motor's target and
push from the band:
- below the band it drives to the low edge, above it to the high edge, with that edge's
  cap; inside the band it does not push (damping 0);
- the `delta` edge's push is `motor_damping` (MOTOR_DAMPING × strength, Part 74); the
  1.5 × `err` edge's is MOTOR_DAMPING × min(1, k × max(|err|, 0.1)) (INFERRED scale: the
  row is solved off the PPU, so caps are taken as the strength cap is);
- pattern 1, or wired in mode 1 or 4, is still driven at its rate both ways.

So a motor with k 0 (pattern 0 or 3) drives forward to its target but is no longer braked
when its ends turn faster; a pattern-2 motor is braked once it turns past 1.5 × `err` per
frame.

Test: `d1179_a_motor_pushes_to_its_band_and_only_pattern_2_is_braked_past_it`.
