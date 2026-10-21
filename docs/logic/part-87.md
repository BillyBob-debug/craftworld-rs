# Logic, Part 87

See docs/logic.md for Parts 1-49.

## Part 87: a non-stiff piston's angle rows are hard (D1186)

Part 85 found that a non-stiff piston's block 0x1c90b8 adds rows through the contact-row
builder 0x1c58d0, but left open whether they hold softly or firmly. D1186 decoded the
row the builder fills.

FOUND (01.33):
- The builder picks its pool on a stack byte (0x1c5c1c). Both piston calls store 0 there
  (0x1c95d8, 0x1c9708), so the rows always come from pool solver+0x10 through the filler
  0x95ba80 (0x1c5e8c), not solver+0x20 (0x95c29c).
- Both calls also pass a 1 (0x1c95c4, 0x1c96f8) that makes the filler zero the friction
  term (0x95bc58), so the rows are frictionless.
- The filler reads neither the joint's strength (+0xc0) nor the strength cap 0xca760.
  Its only material read is the bodies' bytes +0x78/+0x79 over 255 (0x95bb4c..0x95bc24).
- Bias (0x95bef4..0x95bf30): 0.4 × max(separation − 1, 0) for a separation of 0 or more
  (TOC -0x1edc 0.4, -0x2148 1.0), and the separation itself below 0. It is scaled by the
  effective mass and stored at row+0x30.
- Geometry: the angle (an atan2 polynomial at 0x1c9600..0x1c9664) only builds the
  row's direction. The second call (0x1c9724) holds the A anchor's coordinate along that
  direction against the B anchor's. The anchors are the joint's local anchors +0x30 on
  body +0x10 and +0x40 on body +0x1c, put into the world at 0x1c8cb4..0x1c8d3c.
- The threshold 50 is the float 50.0 at 0xd98164 (TOC -0x1f94), in solver units.

INFERRED:
- These are hard, one-sided contact rows (the contact-row type) that keep each anchor on
  the line through the other along the saved angle's direction. A prismatic joint's
  perpendicular and angle rows do the same, so the model here already matches, and there
  is no softness to add.
- The impulse cap at row+0x4c is min(the mixed material term, 800 over a sum of two
  inverse masses) (0x95bf58). The sum was not decoded.
- The signs of the two directions were not confirmed.

Change: none to behaviour; the note on `slides` records the finding. Route and parity
unchanged.
