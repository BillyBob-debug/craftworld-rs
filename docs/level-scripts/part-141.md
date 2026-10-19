# Level scripts, part 141: the link shader and the active test (2026-10-19, D394)

Part 139 left the line shader's edge open, and part 136 the meaning of the active test.
This part reads the constant the link drawer sets and the classes the test can pass for.
No code changes: the edge is in a fragment program not yet found, and the state the
active test reads is not named, so the viewer keeps its placeholder edge and draws
every link pending.

### Found (EBOOT, r2 0xdaa0c0)

- 0x49d3fc sets the vec4 `(f2, f3, f1, t)` under the name hash 0x64f6da0a on the global
  render state (`*(TOC -0x5d68) + 0x60`, through 0x896edc). `t` is the word at 0xe446b4,
  written at 0x4b4100 as a frame counter (0xe4c308) over 100 (-0x5a80).
- The link drawer passes 0, 1 and 0 (0x42655c-0x426568), so the constant is
  `(0, 1, 0, t)`. Twenty other callers set it; 0x4b7650 sets the same `(0, 1, 0)` and
  0x4b808c `(0, 1, x)` with `x` clamped to 0..1. The constant names no shader.
- The drawer binds only the texture (0x49d104) before it, so the program is whatever
  the planets' caller (0x433be4 in 0x433328) left bound.
- A badge element's object (+0x30) has one of three vtables, none with RTTI. 0xd76638
  (the base) and 0xd76528 return null from +0x38 (0xa3f8ac), so a link to them is always
  pending. 0xd765a0 (0x100 bytes, key at +0x7c, built by 0x4518d4, 0x451c78 and the
  factory 0x451a5c for type 1) returns itself (0xa3f96c).
- For that class +0x84 starts at 1 (0x451954, 0x451d04) or a byte of the element's data
  (0x451b34), and its refresh (0x450998, store 0x450a14) sets it from 0x1a7efc (stub
  0x89616c) for the key at +0x7c. +0x50 is set from +0x54 at the refresh (0x450b08);
  nothing in the class code sets bit 4 (0x450c0c-0x451264 set 1, 2 and 8 to 0x400).
- 0x1a7efc gives 0 to 3. A key of type 4 or 9 joins its children's: 3 if all are 3, 1
  if any is 1, 2 if mixed, 0 only if all are 0. A level's key finds its slot (0x1a6cd8)
  and a record (0x199c9c, table +0x280, stride 0x80): 0 (0x1a8270-0x1a82b4) when the
  slot's +0xb8 is set, the record's +0x32 bit 1 is clear and 0x89127c and 0x3ab98 (always
  0) give 0; else 3 if its +0x2e is set, 1 if its +0x2c is 0, 2 otherwise.
- The slot serializer 0xab3848 names +0xb8 InitiallyLocked (+0xa8 PrimaryLinkLevel,
  +0xb0 Group, +0xb9 Shareable, +0xba IsSubLevel, +0x5c ShowOnPlanet).

### Inferred

- The shader's fourth component is time in seconds if the counter runs at 100 a second;
  the first three are a mode the link does not use.
- A link draws active when its target is a 0xd765a0 badge whose per-key state (+0x84)
  is 0: a slot that is InitiallyLocked and not yet unlocked (+0x32 bit 1 read as the
  player's unlocked flag). The arrow is drawn active while it grows towards a locked
  level, pending once the level is open.

### Next

Find the bound fragment program (a capture of an unlock), and draw the active look.
