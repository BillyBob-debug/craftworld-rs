# Level scripts, part 136: a link's width and look (2026-10-19, D389)

Part 133 drew every link with an 80-wide shaft and a 9-degree head and left open which
case picks those. This part traces the pick and the link's two looks. No code changes:
the pick depends on an object field (+0xc0) that is not identified yet, so the viewer
keeps 80 and 9.

### Found (EBOOT, the drawer 0x426484, r2 0xdaa0c0)

- The link record's byte +0x4c picks the size (read at 0x426674 and 0x426870): set,
  width 80 (0xda268c) and head 9 (0xda2688); clear, 40 (0xda2694) and 6 (0xda2690); both
  times 0.4 (0xda2698). The 48 and 7.2 pair (0xda269c, 0xda26a0, at 0x4268c0) replaces
  either when both badges' keys have +0x10 == 1 and +0x14 of 3 or 4, the same key test
  as 0x420890 (part 134).
- +0x4c is written only by the record's constructor 0x6ff5d8 (0 at 0x6ff684, 1 at
  0x6ff700), which the progress setter 0x42c928 calls (0x42caa4) when it adds a record.
  It is 1 when both ends' objects resolve (0x8953cc) and each has +0xc0 of 0, 5, 7 or 8.
  0x42c928 clamps the progress to 0..1 and stores it at +0x48, as does 0x42c7e0.
- The call at 0x426998 passes the radius planets+0x1c, the width, segments of 10
  (0xda26b8), the head and the progress. A link is active when the second badge's object
  (+0x30 through 0x6fb9a4) exists, its virtual at +0x38 returns an object whose +0x84
  is 0, and its +0x50 has no bit 4 (0x4266a0-0x426720); otherwise it is pending. This is
  worked out each frame, not stored.
- An active link takes the colour 0xA0FFFFFF (0x426708) and the UVs 0.75, 0.125, 0.25,
  1.0; a pending one the caller's colour (0x426834) and 0.5, 0, 0.125, 0.75 (0x4a1ef8,
  0x4a2008, the flag read at 0x4a1ee0).

### Inferred

- +0xc0 reads as an object class, so +0x4c marks links between two ordinary badges.
  The disc run's link joins two story levels; the viewer keeps 80 and 9 for it.
- Part 133's colour (0.094, 1.0, 0.0117) is the pending colour's, since the target of
  a growing link is still locked.

### Next

Name +0xc0's values, and the pending colour's source at 0x426834.
