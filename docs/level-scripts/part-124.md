# Level scripts, part 124: the start checkpoint (2026-10-18, D377)

A level with several checkpoints now starts Sackboy at the one the game picks, not at
the first in Thing order. By the entrance score alone the first is not the top scorer in
81 of 203 multi-checkpoint levels.

### What the game does (FOUND, 01.33 EBOOT)

- 0x5b46c, called only from 0x1ab508 (which first resets each checkpoint's +0x34 to
  0x14, +0x10 to -1 and +0x14 to 0), walks the checkpoint list in order. For revisions
  above 0x26d (level byte +0x575 set) each scores +2 when active (byte +0x40), then by
  the category byte at +0x7e: 'n' +4, 'p' or 'q' +1, 'r' -3, 'o' or anything else 0.
  Signed compare from -100, `best < score`: the highest wins, the first on ties, and it
  is activated quietly (0x5a5b8 with 1).
- The classifier (0x3ed2c) takes the checkpoint component (0x10a748), then: 'r' when
  0x584e4 finds GUID 0x2d4f at resource+0x20 behind thing+0x50; else 'n' when 0x58770
  finds 0x7a06, 0x1d1b5, 0x1d1bc or 0x1dd3f (the decompile shows it returning 0; the
  disassembly does not); else 'q' when +0x41 is set; else 'p' when +0x38 is at least 2;
  else 'o'. 0x59928 sets +0x41 1 and +0x38 1, 0x59874 sets +0x41 0 and +0x38 2.
- Old revisions: among active checkpoints the largest +0xc wins; with none active, the
  lowest float at thing+0x2c ->+0xc +0x70 (INFERRED world x) is made active.
- When a world flag (+0x40 behind 0xd931c8 +0x88 ->+0x58) is set, nothing is scored:
  every 'r' checkpoint is deactivated and every other one activated.

### In the level data

A scan of the 221 levels with a CHECKPOINT Thing: the checkpoint Thing has no mesh; its
parent carries the render mesh, and the 'n' GUIDs sit there (0x7a06 on 106, 0x1d1bc on
90, 0x1d1b5 on 7, 0x1dd3f on 2; the parents' plan is 0x7bf7). No level has 0x2d4f.
INFERRED: thing+0x50 reaches the parent's render mesh, and 'n' is the entrance gate.
The Pod level has no checkpoint.

### What changed

- `lbp-level`: `Checkpoint::active` keeps the stored activeFlags.
- `lbp-preview`: `start_checkpoint_score` scores as above, from the parent's mesh GUID;
  `checkpoints` puts the winner first, so every spawn that took the first checkpoint
  takes the game's. INFERRED: a double holds 6 spawns (a single 3), so 'p' is
  `max_spawns_left >= 6`.
- Not ported: the old-revision rule and the world flag.

Parity 5 pass; the Story run is unchanged.

### Next

The warning popup's drawing and the state 6/7 load.
