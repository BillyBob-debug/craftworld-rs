# Logic, Part 74

See docs/logic.md for Parts 1-49.

## Part 74: a motor pushes by its strength, clamped to [0, 1] (D1173)

Part 72 left the motor row's strength cap out of the model.

FOUND (EBOOT 01.33):
- The strength passed to the motor row 0x95f494 (f2) comes from 0xca760: the PJoint
  strength `+0xc0` clamped to [0, 1] (TOC 0xd95134 = 1.0, 0xd9513c = 0), times 0xca2a4.
- 0xca2a4 sums `*(body+0x110)+0x38` over the joint's ends at `+0x10` and `+0x1c` whose
  `+0xe` is 0, returns 0 when the sum is 0, and otherwise switches on the type through
  a table at 0xca398. A motor (type 9, 0xca73c) gives max(1, 4 × 0xc746c) over the sum,
  where 0xc746c is the motor's speed magnitude (pattern 0: |range / time × 0.01| when
  time is at least 1; pattern 2: range; pattern 1 and wired modes 1 and 4 through
  0xc7538). Type 10 gives 0.8 over the sum.
- The row stores the strength (0x95f6c0) and, unless it is FLT_MAX (0x95f768 against
  TOC 0xd97ff4; a motor's never is), bounds the push by strength × |the ends' speed
  error| (0x95fab8). So a motor at strength 0 does not turn its ends.

lbp-sim now scales the motor's speed damping by the clamped strength
(`motor_damping`, INFERRED: the clamped strength is the share of the speed error the
motor removes). The `*(body+0x110)+0x38` factor (INFERRED: an inverse mass sum) is left
out, as avian's solver already weights by mass.

Still open: the row's second bound, 1.5 × `err` (TOC 0xd98370) with a cap of
k × max(|err|, 0.1), and the piston k term (Part 70). The turn sign: 0xc9250 measures
each end's angle as an inlined atan2(−m1, m0) of `*(*(obj+0x2c)+0xc)+0x40` and returns
the `+0x1c` end's less the `+0x10` end's, wrapped. Whether that matches ours
(`m[1].atan2(m[0])`, scene.rs) depends on the matrix layout (UNVERIFIED).

Test: `d1173_a_motor_pushes_by_its_strength_clamped_to_one` in
crates/lbp-sim/tests/sandbox.rs; the older motor tests now give their motors strength 1.

Census (local-analysis/logic/d1173/census2.json, new `motor strength` keys): of 3,356
motors, 74 are at strength 0, 90 below 0.25, 15 below 0.5, 12 below 1, 2,897 at 1 and
268 above 1.
