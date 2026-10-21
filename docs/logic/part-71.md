# Logic, Part 71

See docs/logic.md for Parts 1-49.

## Part 71: a direction piston stops when a signed source is off (D1170)

Parts 7 and 8 left mode 1's rate with the ternary 0 as -1 (INFERRED), since which type
a switch output has was not mapped.

FOUND (EBOOT 01.33):
- The per-frame pass 0xcfedc jumps on the mode through the table at 0xcffa4 (pointer in
  TOC 0xd95408): 0: 0xcffb8, 1: 0xd0178, 2: 0xd0148, 3: 0xd0114, 4: 0xd00c8. The signal
  is `*(*(*(joint+8)+0x60)+0x10)`.
- The mode 1 case 0xd0178 reads the ternary at signal+0x1c. When it is nonzero the rate
  is the ternary as a float. Otherwise it calls 0x900730(signal+0x34, signal+0x38) at
  0xd01a8, the switch and port of the source: a nonzero result gives the rate 0.0, a
  zero one the constant -1.0 at 0xd9540c (0xbf800000). The rate then goes to 0xc7648.
- 0x900730 is the port test lbp-runtime already models as `output_signed`: switch types
  2, 10 and 20 always, 38 for outputs 10-17, 44 for 10-24, 55 for 7-19, and 36 and 37
  as the output they forward.

So a direction piston (or winch or motor in mode 1) whose source is signed, such as a
three-way switch, stops when the source is off, and one fed by an ordinary switch runs
back. `piston_scale` takes a new `signed` argument, and `SwitchWorld::input_signed`
gives it for a joint's input port 0; the viewer passes it with the ternary and the
activation.

Tests: `d1170_a_piston_input_is_signed_from_a_three_way_switch_or_a_stick` in
crates/lbp-runtime/src/switches.rs and the `piston_scale(1, false, 0, 0., true)` case in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_scripts --all` into local-analysis/logic/d1170/census1.json, new
`joint type T mode M input signed S` counts): of the wired mode 1 joints, 3 of 1,498
pistons and 15 of 474 motors have a signed source; no winch does. Motors also have 12
signed sources in mode 3 and 6 in mode 4. The Rookie Test has none (27 mode 1 pistons
and 1 mode 1 motor, all unsigned), so the route is unchanged and parity passes 5/0.
