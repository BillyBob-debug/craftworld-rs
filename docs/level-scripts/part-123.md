# Level scripts, part 123: the checkpoint's spawn hold (2026-10-18, D376)

Sackboy's entrance in the level the Pod launches now holds him still for the game's 11
frames at the checkpoint before the controls take him. The other spawns keep their
measured timing.

### What the game does (FOUND, 01.33 EBOOT)

- 0x5a2d8 (checkpoint, player, force) spawns a player at a checkpoint; it is called from
  0x806a4 and from the creature's spawn-at-checkpoint (0x7fc3c). 0x5a134 gates it: spawns
  are left (checkpoint+0x10 above 0, forced, or the start checkpoint) and the player is
  not already spawning.
- It resets the body through 0x58de0 (velocity zeroed, pose reset); if 0x59b04 fails the
  player state (byte owner+0xc14, setter 0x7b454) becomes 6. It appends a 0x14-byte entry
  to the spawning list (vector checkpoint+0x28, count +0x2c): the player, a timer set to 0
  and a slot ((previous + 1) mod 5, 0x58480). 0x3d044 clears the physics part's bit 0 and
  0x41044 parents the player to the checkpoint (parent at thing+0x50). The spawn count
  drops unless the checkpoint is infinite or the spawn is forced.
- The checkpoint's update (0x5b118, from 0xd5164) adds 1 to each entry's timer; above 10
  it removes the entry and releases the player (0x58a24): unparent, part bit 0 set again,
  body flag bit 0 cleared, 0xf9d40. Nothing moves the player during the hold, which lasts
  11 frames. The update also drives the parent's value at +0x28 (1.0, 0.5, 0.0), the ring
  colour and checkpoint+0x34.
- Sounds: a checkpoint-spawn sound when the list was empty, a pod-spawn sound when
  game+0x3b5 is set; the activate sounds are in 0x5a5b8.
- The start checkpoint (0x5b46c): with the level byte +0x575 at 0 (revisions up to 0x26d)
  the lowest world x wins. Otherwise each checkpoint scores +2 when active, and by its
  Thing's category byte +0x7e (classifier 0x3ed2c) 'p' or 'q' +1, 'r' -3 ('n' +4 is a dead
  branch); the highest wins and is activated quietly (0x5a5b8 with 1). INFERRED: 'r' is
  the entrance (resource 0x2d4f), 'q' infinite (+0x41), 'p' double (+0x38 at least 2),
  'o' single. Not ported: the entrance's -3 needs checking first.

### What changed

- `lbp-sim`: `SPAWN_HOLD_TICKS` (11), `SackboyBody::hold` and `spawn_sackboy_held`; a held
  body skips the controller, stays still and is not grounded until the hold runs out.
- Viewer: the launched level's respawn spawns Sackboy with the hold. INFERRED: the game
  holds every checkpoint spawn; the other spawns keep their measured timing.

Parity 5 pass; the Story run is unchanged.

### Unsettled

Whether the player is drawn during the hold, the constants at 0xd931cc-0xd931e0, and the
"out of the pipe" motion, which is not in the checkpoint code (INFERRED the level's own
entrance object).

### Next

The warning popup's drawing, the start-checkpoint scoring and the state 6/7 load.
