# Logic, Part 70

See docs/logic.md for Parts 1-49.

## Part 70: a piston holds its start angle; the solver's type table (D1169)

Part 9 left open whether the joint type names are off by one at the type 4 case 0x1c9988,
which goes through 0x962590 and wraps an angle.

FOUND (EBOOT 01.33):
- The constraint solver 0x1c89d4 dispatches on `*(joint+0xc)` < 0xb through a table at
  0x1c8d60 (pointer in TOC 0xd984bc). Its entries are 0: 0x1c8dc4, 1: 0x1c972c,
  2: 0x1c97e4, 3: 0x1c98b4, 4: 0x1c9988, 5: 0x1c9a74, 6: 0x1c9b14, 7: 0x1c9bc0,
  8: 0x1c9d2c, 9: 0x1c9eac, 10: 0x1ca05c. Type 3 (winch, Part 8), 4 (piston) and 9
  (motor) are where the names put them, so the names are not off by one.
- The type 4 case needs `+0x28` set and `+0x29` clear, allocates a 0x70-byte row and
  calls 0x962590(row, joint, ..., f1 = k, f2 = strength 0xca760, f3 = lo 0xc7350,
  f4 = hi 0xc7330). k is 5 × strength when `+0xc4` (animation pattern) is 2, unless the
  piston has an input with `+0xa0` (mode) 1 or 4; otherwise 0.
- 0x962590 first calls 0x96178c, a linear row driven by the length rule 0xc95ec per
  frame (like the motor row) with the lo/hi limits and the strength cap. It then takes
  the measured relative angle 0xc9250 minus the rest angle 0xc99d0 (`+0xe0` for types
  1-6), wraps it to [-π, π) and builds an angular row on it.
- 0xcab7c (also gated on `+0x28` set and `+0x29` clear) calls 0xc9af4, which adds the
  measured minus the rest angle to `+0xe0`. So `+0xe0` becomes the relative angle the
  ends had when the joint first ran, and the angular row holds it.

lbp-sim's piston is an avian2d `PrismaticJoint` with a global basis
(`with_basis(Rotation::IDENTITY)`, `JointBasis::FromGlobal`), which keeps the relative
angle at spawn, so it already matches. Nothing changes in the model; the Piston spawn
says why. What k does in the linear row is not known (INFERRED: a stiffness for the
sawtooth pattern), the same open term as the motor's.

Test: `d1169_a_piston_holds_the_relative_angle_its_bodies_start_at` in
crates/lbp-sim/tests/sandbox.rs.

Census (local-analysis/logic/d1169/census1.json, new top-level and per-level `pistons`,
`pistons_turned` and `pistons_sawtooth`): 2,551 pistons in 145 levels; 1,156 of them
start with their ends turned to each other, and 43 use pattern 2. The route is unchanged
and parity passes 5/0.
