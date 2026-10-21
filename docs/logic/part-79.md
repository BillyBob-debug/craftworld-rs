# Logic, Part 79

See docs/logic.md for Parts 1-49.

## Part 79: the motor row's second edge and pattern 2's k (D1178)

Part 75 found k (5 × strength for pattern 2) and left the row's use of it INFERRED.

FOUND (EBOOT 01.33), the motor row 0x95f494:
- k arrives in f1 and is kept in f29 (0x95f4e8); f2 is the strength cap, f3 and f4 the
  angle limits.
- `err` is wrap(target(t + 1) − the measure 0xc9250) and `delta` is wrap(target(t + 1) −
  target(t)), the targets from 0xc99d0 (pi, 2pi and 1 / 2pi at TOC -0x1d94, -0x1d90,
  -0x1d8c). For a motor (`+0xc` set, 0x95fa0c) `delta` is cut to between 0 and `err`
  (0x95fa28, 0x95fb10) unless pattern 1 or wired in mode 1 or 4 (0x95fa44, 0xc7444).
- The second edge is 1.5 × `err` (TOC -0x1d88, 0x95f734) with a cap of
  k × max(|err|, 0.1) (TOC -0x202c = 0x3dcccccd, 0x95f740 to 0x95f750).
- 0x95f778 sorts the two edges with their caps (the swap at 0x95f78c, else 0x95f9f8) and
  stores the row: +0x30 holds the two caps and lo × m, hi × m, m = 1 over the ends' sum of
  `+0x34` (vrefp and a Newton step); +0x20 holds m and the limit terms.
- INFERRED: the strength cap goes with the `delta` edge and k's with the 1.5 × `err` edge;
  the row is solved off the PPU (no PPU solver found), so the pairing is not read. With k
  0 that edge has no force; a pattern-2 motor is also braked once it turns faster than
  1.5 × `err` per frame.

What we do: `motor_k` and `motor_band` (crates/lbp-sim/src/lib.rs) give k and the band;
the joint does not use them yet, since avian's motor pushes both ways toward one target
velocity (no one-sided band). No behaviour change.

Test: `d1178_only_a_pattern_2_motor_has_a_k_term`.
