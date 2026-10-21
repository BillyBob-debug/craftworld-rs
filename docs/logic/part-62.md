# Logic, Part 62

See docs/logic.md for Parts 1-49.

## Part 62: a mirrored body turns its movers the other way (D1160)

Correction to Part 60: the stick mirror test is not "the body determinant below 0 unlike
byte +0x224" read loosely; it is exactly that XOR, the same flip D1159 gives spins.

FOUND, the localSpace turn 0xcc484 (EBOOT 01.33). The parent Thing is *(tweak+8)+0x5c,
else the Thing itself; the determinant of its world basis rows (*(PPos+0xc)+0x40, +0x50,
+0x60) is tested at 0xcc540 (`vcmpgtfp.`, `blt cr6,0xcc6cc`). Both branches first take
the body's local Y (0x113fb0); with the determinant below 0, 0xcc6ec loads +0x220 and
negates it (`fneg`), otherwise 0xcc56c loads +0x220 as it is. Byte +0x224 plays no part.
This serves configurations 6, 7, 8 and 11 with localSpace. The allowUpDown up-axis turn
(0x9587ac, Part 53) keeps +0x220 unnegated.

FOUND, the stick mirror 0x957fb4-0x958058, skipped for configuration 11 (0x957fb0).
r27 is the parent Thing as above; r10 = 1 when its world determinant is below 0, and
`lbz r11,0x224(r31)`, `cmpw r10,r11`; when they differ the sign of stick x is flipped
(0x958040-0x958058). This comes after the stick length (0x957f90) and before the
localSpace branch (0x95805c).

Model: `MoveTweak::mirror` (the owner body's world 2D determinant below 0) and
`MoveTweak::flip` (mirror XOR the chain parity from `chip_turn`). `rows` turns the
localSpace build by minus the angle when mirrored and keeps +angle for allowUpDown; stick
movers negate x when `flip`. INFERRED: the +0x5c Thing is the owner body in our scene,
and its world determinant is its 2D one. Test:
`d1160_mirrored_movers_negate_their_local_turn_and_stick_x` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `chip.move_mirrored` and
`chip.move_flipped` totals and per-level counts): 1,057 movers on a mirrored body and
1,033 flipped in 98 levels. The Rookie Test has none.
