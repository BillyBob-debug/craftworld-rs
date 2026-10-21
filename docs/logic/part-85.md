# Logic, Part 85

See docs/logic.md for Parts 1-49.

## Part 85: a non-stiff piston holds its angle (D1184)

Part 84 found that the solver's block 0x1c90b8 adds rows for a non-stiff piston and
guessed they free its angle. D1184 decoded the block's calls.

FOUND (01.33):
- The block always calls 0x1c58d0 (at 0x1c96a4) with a point turned by the body's angle
  less PJoint +0x54 (0x1c9510/0x1c9520). It calls it again (0x1c9724) for the angle less
  +0x50 (0x1c92ec), unless +0x50 holds 100 (TOC -0x1bbc, test 0x1c96a8).
- 0x1c58d0 is the generic one-sided contact-row builder, not an angle limit. It fills a
  row from pool solver+0x10 (0x95ba80) or solver+0x20 (0x95c29c), mixes the bodies'
  material bytes (+0x78/+0x79 over 255), and emits nothing when a body's inverse mass is
  0 (0x1c59b0) or the separation along the row passes max(measured, 50) (TOC -0x1f94,
  0x1c5b44).
- +0x50 and +0x54 are the two floats the PJoint printer 0xb41850 lists after
  animationPause (+0xc8..+0xd8) and before modStartFrames (+0x144).

INFERRED: they are aAngleOffset and bAngleOffset (`motion[5]` and `motion[6]` here), and
the rows hold a non-stiff piston near its saved angle on each body, with 100 meaning no
hold on body A. So the angle of a non-stiff piston is held rather than free, and both
kinds stay prismatic (angle locked). No behaviour changes; the D1183 comment on `slides`
is corrected.

Route (Part 57) unchanged by construction (no code path changes).

Open: whether the hold is soft (the row's bias and cap in 0x95ba80 are not decoded),
where +0x50/+0x54 are set when a joint is made, the byte at +0x29, and how a pattern-0
piston closes the gap to its rule in the game.
