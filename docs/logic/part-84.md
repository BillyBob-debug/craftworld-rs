# Logic, Part 84

See docs/logic.md for Parts 1-49.

## Part 84: a non-stiff piston has the same band (D1183)

Part 83 found that the solver gives a stiff piston the band row 0x962590 and any other
piston row 0x960988 plus a block at 0x1c90b8, and held every non-stiff piston until that
row was decoded.

FOUND (01.33):
- Row 0x960988 emits one row along the piston's axis, the normalised anchor delta (a
  degenerate length takes 0x961698). Its length part is the band of 0x96178c: `delta`
  is the rule over the next frame (0xc95ec), `err` the next length less the measured
  one, the edges are `delta` and 1.5 x `err` (TOC -0x1d88, 0x960f98), swapped into order
  at 0x960fec. The caps are k x max(|err|, 10) (TOC -0x1cdc, 0x960ec0) on the `err`
  edge and the strength cap f2 on the `delta` edge (0x960f78).
- It also carries the length limits f3..f4: past either one the bound eases by 0.2
  (TOC -0x1fe0) beyond a slop of 1 (TOC -0x2148, 0x96158c/0x961604). Here the slider's
  limits are the rule's range, as in Part 82.
- Its own stiff branch (0x960c68 to 0x961274) only takes the axis from the joint frame;
  the solver never sends a stiff piston there.
- The block 0x1c90b8 runs for joint types 2, 4 and 6 when not stiff (or when +0x29 is
  also set). It measures angles from the body frames and calls 0x1c58d0 once against
  PJoint +0x50 and, unless +0x54 holds the sentinel 100, once against +0x54.

So the stiff flag does not change the length drive. `slides` drops its stiff test: a
piston with a strength and a k (pattern 2) slides whether or not it is stiff.

INFERRED: the block's rows are angle rows, so a non-stiff piston's angle is not locked;
here both kinds stay prismatic (angle locked) until 0x1c58d0 is decoded. INFERRED, as in
Part 83: a piston with k 0 is held, since only the k-capped `err` edge closes the gap from
its saved length to its rule.

Route (Part 57) unchanged: x -2,870.3, y 5,188.6 at ticks 1,000 and 1,400; x -2,440.6,
y 5,400.9 at ticks 1,600 and 2,007. Parity 5 pass.

Open: 0x1c58d0 and what PJoint +0x50/+0x54 hold, the byte at +0x29, and how a pattern-0
piston closes the gap to its rule in the game.

Test: `d1183_a_pattern_2_piston_with_a_strength_slides_stiff_or_not` (replaces the D1182
test).
