# Logic, Part 64

See docs/logic.md for Parts 1-49.

## Part 64: a follower always packs its cross row (D1163)

Correction to Part 12: the second call of 0x957df4 is skipped when decel × velRange is 0
only for configurations other than 11; a follower always packs row 2.

FOUND, the caller 0x1c7ed0 (EBOOT 01.33). After the first call (0x1c8030, param_5 0,
row slot +0xd0), it loads +0x58 (decel) and +0x50 (velRange), multiplies them (0x1c8040)
and compares with 0.0 (-0x2150(r2), 0x1c8048). When the product is not 0 it packs the
second row (0x1c80b0, param_5 1, slot +0xd0 + 0x10). When it is 0, 0x1c81a4 tests the
configuration (+0xc) against 11 and returns to the loop (0x1c7f58) for any other; a
follower goes on to the second call. INFERRED: -0x2150(r2) is 0.0, as Part 12 read it.

FOUND, the param_5 path of 0x957df4 (0x958354). With param_5 set the row's target
(+0x30) is 0 and its upper limit (+0x38) is decel × velRange; configuration 11 then
sets both limits to ±accel × velRange (0x95839c). Both rows' limits are finally × the
activation 0xcaf9c × the 1/mass entry +0x3c (1/mass is still left out, as for D1109).
This is Part 12's row 2; the param_5 path itself needs no new model.

Model: `MoveTweak::rows` packs the cross row when decel × range is not 0 or the tweak
is a follower. Test: `d1163_a_follower_always_packs_its_cross_row` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `chip.follow_cross` total and
per-level `follow_cross`): 127 followers with decel × velRange 0 in 28 levels, 2 of them
in the Rookie Test.
