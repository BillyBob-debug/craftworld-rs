# Logic, Part 83

See docs/logic.md for Parts 1-49.

## Part 83: the stiff flag picks the piston's row (D1182)

Part 82 let a piston that is not stiff and has a strength slide by the band of row
0x96178c, guessing at what the stiff flag does. D1182 decoded it.

FOUND (01.33):
- The stiff bool is PJoint +0x28 (the serializer 0xb41850 writes it at 0xb41a34, between
  strength +0xc0 and slideDir +0xb0). Every reader also tests the byte at +0x29, so the
  condition is stiff and not +0x29. What +0x29 holds is not known.
- The solver's joint-type switch (0x1c8d0c, table 0x1c8d60) sends a piston (type 4) to
  0x1c9988. A stiff piston gets the prismatic row 0x962590 (called at 0x1c9a68), which
  calls the length row 0x96178c of Parts 81 and 82. Any other piston gets row 0x960988
  (called at 0x1c9b08) with the same f1-f4, and an extra block after the switch
  (0x1c90b8) that a stiff piston skips. The motor case 0x1c9eac does not read the flag.
- f1 is 5 x 0xca760 for pattern 2 (0 when wired in mode 1 or 4), else 0, as for motors.
  f2 is the strength cap 0xca760 = clamp(strength, 0, 1) x a per-type scale (0xca2a4),
  so a strength of 0 gives a cap of 0. f3 and f4 are min and max of +0xf0 and
  +0xf0 + +0xc8 (0xc7350, 0xc7330).

So the band of Part 82 is the stiff piston's, not the springy one's. `slides` now drives
only a stiff piston. A first run that drove every stiff piston with a strength dropped
Sackboy to y 3,977.4 by tick 1,600: the route's stiff pistons are pattern 0, so their k is
0, and many start far from their rule's length (one measured 814.5 against a rule of
129.3). With no k the band does not push inside it, so those pistons never close that
gap. INFERRED: a stiff piston also needs a k (pattern 2, `motor_k`) to slide; one
without is held at its length as before. Any other piston is held until row 0x960988
is decoded. INFERRED: a cap of 0 pushes nothing.

Route (Part 57) unchanged: x -2,870.3, y 5,188.6 at ticks 1,000 and 1,400; x -2,440.6,
y 5,400.9 at ticks 1,600 and 2,007. Parity 5 pass.

Open: row 0x960988 and the block 0x1c90b8 (non-stiff pistons; docs/runtime-evidence
says stiff locks the angle), the byte at +0x29, and how a stiff pattern-0 piston closes
the gap to its rule in the game.

Test: `d1182_only_a_stiff_pattern_2_piston_with_a_strength_slides` (replaces
`d1181_only_a_springy_piston_with_a_strength_slides`).
