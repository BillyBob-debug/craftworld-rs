# Logic, Part 88

See docs/logic.md for Parts 1-49.

## Part 88: a piston's base is its saved length (D1187)

A piston's length rule and limits start from a base length. The sim took that base from
the joint's tweak minimum (tweakTargetMinLength) when the tweak values were valid, and
from the saved length otherwise. D1187 decoded where the game keeps the base.

FOUND (01.33):
- The limits are min/max of PJoint +0xf0 and +0xf0 + +0xc8 (0xc7330, 0xc7350; also
  0xc7d1c, 0xc8218), so a piston is limited to base..base + range.
- The PJoint serialiser 0xb41850 writes, in order: body A +0x10, body B +0x1c, contact A
  +0x30, contact B +0x40, then +0xf0 (0xb418fc), +0xe0 (0xb41914), +0x1d0, +0x1d8, and
  later type +0xc, strength +0xc0, stiff +0x28, slide direction +0xb0, pattern +0xc4 and
  animationRange +0xc8. That is the field order of the parse here, so +0xf0 is the saved
  `length` and +0xc8 is the range.
- The only other writers of +0xf0 are the script setters 0xc7e68, 0xc83bc and 0xc8230,
  reached from script natives at 0x71aff8 and 0x71b044. The length rule 0xc9790, row
  0x960988 and the solver only read it.
- The free rule (0xc9834) returns base + the wave × range, with base +0xf0.
- In row 0x960988, the edge 1.5 × (rule − current) (0x960f7c/0x960f98) and the edge
  cut(rule step, rule − current) (0x960fe0) do not depend on k. The cap k × max(|err|, 10)
  (0x960ec0) does, which matches `piston_band` here.

Change: `free_length`, `length`, `one_shot_length`, `range` and the other piston rules use
the saved `length` as their base. The tweak values still set the limits of the other
joint kinds. The test is d1187_a_piston_rule_starts_from_its_saved_length.

INFERRED: the tweak minimum and maximum are editor values that the game's piston does not
read. Route and parity were unchanged: on the route, every piston's saved length matches
its tweak minimum, or it has no valid tweak values.

Open: with the right base, whether a pattern-0 piston (k 0) still needs the D1183 hold.
The D1182 route fall was measured with the old base.
