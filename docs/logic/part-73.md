# Logic, Part 73

See docs/logic.md for Parts 1-49.

## Part 73: a speed motor turns at its speed in every mode, once in mode 2 (D1172)

Part 72 left open pattern 1's cut-off in mode 2 in the rate function 0xc7370.

FOUND (EBOOT 01.33, 0xc73bc to 0xc7434):
- Pattern 1 takes speed `+0xd4` and returns speed × the clock rate
  (`*(+0x190)+0x3c`) whatever the mode, wired or not. The length ramp of a wired mode 1
  or 4 piston (Part 72's other branch) is not used.
- Wired (`*(*(joint+8)+0x60)` nonzero) in mode `+0xa0` 2, it first takes
  ((frame - start) × rate + start + delta) × |speed|, the clock's frame (`+0x34`, `+0x38`,
  `+0x3c`, the D1103 clock) times |speed|. When that passes 2π (0xd95130), the
  speed becomes 0 (0xd9513c), so the motor stops after one full turn from its start.
  The frame it is called with is the world frame (f31 in 0x95f494).

lbp-sim: `PistonDrive::turn` returns speed × clock rate × TICK_HZ for pattern 1 in every
mode, and 0 once the clock's frame × |speed| passes 2π for a wired mode 2 one. Before,
a wired mode 1 or 4 speed motor followed the length ramp, which is 0 with time below 1.
INFERRED: the clock starts at 0 when play starts (D1103), so the turn counts from there.

Test: `d1172_a_wired_speed_motor_in_mode_2_stops_after_one_turn` in
crates/lbp-sim/tests/sandbox.rs.

Census (local-analysis/logic/d1172/census1.json, new `motor pattern 1 wired mode M`
keys): of 229 wired speed motors, 110 are mode 0, 23 mode 1 (now turning at their
speed), 1 mode 2 (now stopping after one turn), 95 mode 3 and none mode 4. The Rookie
Test has no speed motor. The route is unchanged and parity passes 5/0.
