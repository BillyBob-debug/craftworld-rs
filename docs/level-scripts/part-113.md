# Level scripts, part 113: Sackboy plays in the launched level (2026-10-18, D366)

After the switch (part 111) and the entrance camera (part 112) the launched level was
only drawn: Sackboy and the physics world stayed the Pod's. Sackboy now plays in a level
the Pod launches, from its entrance, as `--play-sackboy` does on a level loaded directly.

### What changed

- `lbp-bevy-viewer` (`sackboy_play.rs`): the physics read is factored out of `install`
  (`read_physics`, `checkpoint_spawn`, `spawn_line`). `start_respawn` reads the launched
  level's physics on a worker thread (the same snapshot and game dirs, `LEVEL_STACK_BYTES`
  stack); the `respawn` system then despawns the Pod's bodies and Sackboy, drops the
  Pod's Sackboy state, and hands the new scene and spawn to `spawn_world`, which builds
  them as for a level loaded directly. `show_sackboy`'s framing flag moved into `Play`
  (`framed`) so the new Sackboy is framed again.
- `level_play::switch_level` calls `start_respawn` whenever play is on (every Pod run).
  When the read fails or the level has no checkpoint, Sackboy stays in the Pod and the
  log says so.

MEASURED (the part 109 runs, scripted and native, hold at 900, `--preview-lighting`):
both read Trash Valley's physics after the switch (809 shapes: 438 dynamic, 152 static;
44 joints) and spawn Sackboy at checkpoint Thing 33, (-11560, 19306, 200); the capture
shows him on the kerb at the entrance with the arcade machines behind (106737 and 106724
colours). No panics.

INFERRED: the launched level's Sackboy uses the same spawn rule as `--play-sackboy`
(first checkpoint plus `SPAWN_OFFSET`). Not done: the game's entrance animation (Sackboy
coming out of the pipe), the loading screen and the game state 6/7 load.

### Next

The warnings (0x402b24) and the select sound; then the cursor's flight and sound, and
the other searches.
