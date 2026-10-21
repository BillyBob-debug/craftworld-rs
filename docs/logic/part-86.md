# Logic, Part 86

See docs/logic.md for Parts 1-49.

## Part 86: the byte beside a piston's stiff flag is editor-only (D1185)

Every solver reader of a joint's stiff flag (PJoint +0x28) also tests that the byte at
+0x29 is clear (Part 83). D1185 looked for who sets that byte.

FOUND (01.33):
- A scan of the text segment for byte stores at offset 0x29 finds one PJoint writer:
  the setter 0xcacec (`stb` at 0xcad30). It saves whether the joint is effectively stiff
  (+0x28 set and +0x29 clear), stores the byte, and calls 0xcab7c (at 0xcad50) when that
  changes.
- The setter is reached through the wrapper 0x898f5c only from the joint-editing gadget:
  0x5a6bb4 (value 1 when the gadget takes hold of a joint, 0 when it lets go or swaps
  joints) and 0x55d720 (value 1 on a joint it has just made, at 0x55dbbc).
- The PJoint serialiser 0xb41850 handles type, strength, stiff (0xb41a44), slide
  direction and pattern, and never touches +0x29.
- The stiff flag has its own setter 0xcad64, which native SetStiff (0x6496d8) reaches;
  it makes the same check and also calls 0xcab7c.
- 0xcab7c runs only for an effectively stiff joint and re-reads the pose from the end
  anchors.

INFERRED:
- +0x29 suspends stiffness while the editor holds a joint. A loaded level never sets it,
  so it is 0 in play and the stiff test is +0x28 alone. The zero start is inferred: the
  constructor's store was not found.
- 0xcab7c re-bases the stiff rest pose when stiffness turns on, by SetStiff or by the
  editor letting go. Scripts that call SetStiff are not modelled yet.

Change: none to behaviour. The piston model no longer reads the stiff flag (D1183); the
note on `slides` records the finding. Route and parity unchanged.
