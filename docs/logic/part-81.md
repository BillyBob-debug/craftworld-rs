# Logic, Part 81

See docs/logic.md for Parts 1-49.

## Part 81: the piston's length row and its k term (D1180)

Part 70 left open what k (5 × strength for pattern 2) does in the piston's linear row
0x96178c.

FOUND (EBOOT 01.33), 0x96178c, called first by the type-4 row 0x962590 (bl at 0x962644)
with the solver's joint (r4, kept in r30 at 0x961884) and f1-f4 (k, strength cap, lo,
hi) unchanged:
- The measure L is the distance between the two world anchors (`+0x30`, `+0x40` through
  the body matrices), stored at row+0x20 (0x961b30), its axis normalised when L > 0.01
  (TOC -0x20b8). With `+0x28` set and `+0x29` clear it projects onto a body-fixed axis
  (`+0xb0`, 0x962078).
- t is the frame as a float (0x9618e0). The targets are the length rule 0xc95ec at t and
  t + 1 (TOC -0x2150, -0x2148; 0x961950, 0x961984), `delta` = target(t + 1) − target(t)
  (0x9619ac). Pattern 1 (or wired in mode 1) takes 0xc7370 as target and `delta`
  (0x962018).
- `err` = target(t + 1) − L (0x961d80). For a joint with a type (`+0xc`, 0x961d88,
  0x961da0) `delta` is cut to between 0 and `err` (0x961de4, 0x9624bc), unless wired
  in mode 1 or 4.
- The second edge is 1.5 × `err` (TOC -0x1d88, 0x961d9c) with a cap of
  k × max(|err|, 10) (TOC -0x1cdc = 10, 0x961c84 to 0x961cc8): the motor row's band
  (Part 79), with a floor in game units of length instead of 0.1 rad.
- 0x961df0 sorts the edges with their caps into row+0x40 = {capA, capB, lo, hi}; the
  `delta` edge has the strength cap f2, unscaled for a joint with a type (the velocity
  scale at 0x961cfc runs only when `+0xc` is 0).
- No separate limit rows: row+0x30 = {mass, X, lo − L, hi − L} (0x961e2c to 0x961efc),
  X 0 or -FLT_MAX by a stack flag (INFERRED: an impulse floor). Past hi (0x962390) or
  below lo (0x962408) the limit is eased to L + 0.2 × (lim − L ∓ 1) (TOC -0x1fe0, -0x2148),
  0 within 1 unit.

What we do: `piston_band` (crates/lbp-sim/src/lib.rs) gives the band, sharing
`motor_band`'s rule. The piston is still a slider held at its length, so neither the
strength cap nor k is applied yet. No behaviour change.

Test: `d1180_a_piston_row_has_the_motor_band_with_a_floor_of_10`.
