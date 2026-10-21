# Logic, Part 60

See docs/logic.md for Parts 1-49.

## Part 60: aiming spins and stick movers read their stick (D1158)

FOUND (EBOOT 01.33): 0xcb02c and 0xcb074 read a PPhysicsTweak's stick from its input
list (`*(*(tweak+8)+0x60)`): the raw f32 activation (+0x18) of the signal in slot 1 when
the list has at least 2 slots, and in slot 2 with at least 3; 0 for a missing or unwired
slot. Unlike slot 0 (0xcaf9c) there is no deactivation test and no absolute value, so a
signed analogue stick keeps its sign. They are called by the spin packer 0x95fb20
(0x95ff98, 0x95ffa8; configuration 4, Part 50) and by the move packer 0x957df4
(0x957f00, 0x957f10).

Move packer, stick branch (every configuration but 8 and 11 reaches it; the scene only
builds 6 and 7):

- The stick (x, y) is mapped from the square onto the circle: times max(|x|, |y|) over
  its length (0x95840c-0x95845c), so a full diagonal has length 1.
- With localSpace (+0x5c) it is turned by the up axis from 0xcc484, the same turn the
  mover's velocity takes (0x9582b4).
- Row 1 runs along it with target its length, limits -decel × range and +accel × range;
  a zero stick brakes like a mover (±decel × range along the body's velocity, or world
  up). Configuration 6 multiplies the target by range (0x958494-0x9584bc);
  configuration 7 does not, so its wanted speed is at most 1 a tick.

Not modelled: the mirror test (0x957fb4-0x958058: x is negated when the body's
determinant is below 0 unlike byte +0x224), the input scale 0xcbed4 (the slot 0
activation when +0x10 is 2, else 1) and the param_5 path (0x958354).

Model: `LevelWorld::tweak_stick` (crates/lbp-runtime/src/switches.rs), a `stick` field
on `scene::MoveTweak` with the stick `want` in `MoveTweak::rows`, and a stick closure on
`lbp_sim::drive_spins` and `drive_moves` that the viewer fills each tick. Tests:
`d1158_tweak_stick_reads_raw_activations_on_ports_1_and_2` (lbp-runtime) and
`d1158_stick_movers_map_the_stick_onto_a_circle` (crates/lbp-sim/tests/sandbox.rs).

Census (`level_physics --all` over the 01.33 catalog, new `sticks` and `sticks_wired`
counts): 166 stick tweaks (spin 4, move 6 and 7) in 43 levels, 164 with port 1 or 2
wired; most in the Joker levels (33, 13 and 10).

Route: unchanged from Part 57.
