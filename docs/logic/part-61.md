# Logic, Part 61

See docs/logic.md for Parts 1-49.

## Part 61: a tweak on a microchip takes its turn on the board (D1159)

Correction to Part 50: +0x220 and +0x224 are not built over the plain Thing+0x50
parent chain. 0xcd2e0 is the same walk as the chip-table angle 0x110140 (Part 45,
D1143): it stops at the first Thing that is not on a microchip's board.

FOUND, 0xcd2e0 (EBOOT 01.33). T = *(tweak+8); it zeroes +0x220 and +0x224, then loops
while T+0x5c is neither 0 nor T, and only while the link's G = (T+0x50)+0x50 is
non-null and has the microchip part (bit 0x21).

- With a PPos (T+0x2c) the term comes from T's local matrix, which SetLocalPosition
  0xf2e80 writes at *(PPos+0xc)+0 (the world translation is at +0x70); the term is
  atan2 of its y axis, atan2(l_y.x, l_y.y), which is minus the in-plane turn. The link
  is mirrored when the local determinant is below 0.
- Without a PPos the term is word 5 of T's chip table entry and the mirror is the byte
  at word 8 (the same entry as D1143).
- A mirrored link negates the sum after adding its term and flips +0x224. T then moves
  to its parent; the sum is not wrapped.

The packer 0x95fb20 uses both: r25 is +0x224 XOR the body basis determinant below 0
(Part 50), and the branch at 0x960250 (`cmpwi cr7,r25,0`) picks f13 = 1.0
(-0x2148(r2) = 0xd97fb0) unflipped or -1.0 (-0x1f70(r2) = 0xd98188) flipped; 0x960560
`fmadds f0,f13,f12,f0` adds f13 × +0x220 (loaded at 0x96039c) to the aim angle.

Model: `scene::chip_turn(things, boards, i)` gives the turn and mirror parity over the
same walk, using each Thing's local matrix (its world basis over its parent's) or the
chip table entry. `SpinTweak.angle` holds the turn; `flip` is now the chain parity XOR
its body's world determinant below 0 (before it was the tweak's own world basis);
`target_along` adds f13 × angle to the error. `MoveTweak.angle` takes the same turn.
INFERRED: the chip table angle is in radians, and the sign: a tweak turned θ on its
chip aims its body's up axis turned by θ, so a gyroscope on a turned chip holds its body
at -θ. Test: `d1159_tweaks_on_a_chip_take_their_turn_on_the_board` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `chip_turned` and
`chip_flipped` counts, totals under `chip`): 2,330 spin and move tweaks have a turn on
their boards and 1,127 spins are flipped, in 144 levels; the most are in the Cross-
Controller boss (128), Mr. Freeze (90) and the Joker boss (83). The Rookie Test has 15
turned and 22 flipped.

Route: unchanged from Part 57 (x -3,789.7 on tick 950; x -2,870.3, y 5,188.6 from
tick 1,000 to about 1,400; x -2,440.6, y 5,400.9 from tick 1,600 to 2,007).
