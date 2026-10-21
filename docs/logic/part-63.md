# Logic, Part 63

See docs/logic.md for Parts 1-49.

## Part 63: the input scale 0xcbed4 scales movers and caps followers and spins (D1161)

Replaces the INFERRED reading of Parts 51, 52 and 60 that the input scale 0xcbed4 is 1.

FOUND, 0xcbed4(tweak) (EBOOT 01.33). It returns 0.0 (0xd952c8) when the Thing
(tweak+8) has +0x7f set. When the Thing's input chain +0x60 is non-null, its first word
is non-zero, chain[4] is non-zero and the tweak's behavior (+0x10) is 2, it returns
*(float*)(chain[4]+0x18), the signed activation of input slot 0. Otherwise it returns
1.0 (0xd952c4, bytes 3f800000). INFERRED: chain[4] is input slot 0's signal record.

FOUND, the mover target, row 1 (0x958128). Configuration 6 branches to 0x958494, which
takes target × range (+0x50) at 0x9584b8 and returns to 0x958134. Configuration 11
skips the scale (`beq 0x958174`). Every other configuration calls 0xcbed4 at 0x958144
and multiplies the target by it, signed, at 0x95816c (`fmuls`), storing row +0x30 at
0x958170.

FOUND, the clamps. The follower speed is clamped to ±velRange × |0xcbed4| at
0x958724-0x958738 (Part 52), and the spin speed likewise at 0x9607ec-0x960868 (Part 51).

For comparison, the activation 0xcaf9c (behaviors 0 and 1) returns 0 with +0x7f set and,
with slot 0 wired, the ternary +0x1c ≠ 0 for behavior 0 or |+0x18| for behavior 1, else 1.
Not modelled: `tweak_activation` does not yet test +0x7f.

Model: `SpinTweak::scale` and `MoveTweak::scale`, filled each tick by
`World::tweak_input_scale` (lbp-runtime switches.rs) through `drive_spins` and
`drive_moves`; 1 until a host fills them. `rows` multiplies the non-follow target by
`scale` and clamps the follower by range × |scale|; `target_along` clamps the spin
speed by range × |scale|. Test:
`d1161_the_input_scale_scales_movers_and_caps_followers_and_spins` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `chip.input_scaled` total and
per-level `input_scaled`): 376 spin and move tweaks with behavior 2 in 41 levels. The
Rookie Test has none, and the route is unchanged.
