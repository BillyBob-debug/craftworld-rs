# D234: Sackboy animation 1:1 by game vertices (2026-10-11)

RPCS3 01.33, owner's blank Moon level, play mode. Instead of F12 frames, the game's own
CPU-skinned Sackboy vertices are read from guest memory and fitted to our poses, so an
animation difference is a number (RMS, game units; Sackboy is about 200 tall).

## Method

- Buffer: three rotating 0xC000-byte vertex pools (this boot `0x32990000`, `0x39690000`,
  `0x39f90000`, found by diffing memory while walking). Records are 16 bytes (x, y, z, tag).
  The first 2512 vertices are Sackboy (our mesh `0000043f` has 2920; order and count differ,
  so matching is order free). One new buffer per 30 Hz tick.
- Capture: `scripts/rpcs3_vdump.py OUT.bin SECONDS t:key:ms ...` (needs `memscan.py` / `pad.py`
  from `local-analysis/pc-helpers`), `scripts/rpcs3_vsel.py` slices samples. Keys: `d`/`a`
  walk, `x` jump (Cross), `z` Square (opens the Popit, avoid), D-pad = arrows (in create mode
  Down opens the Popit and the other directions steer it: moods need play mode).
- Fit: `cargo run -p lbp-preview --example pose_fit -- SNAPSHOT GAME-DIR 43f DUMP.bin CLIPS`
  with `GAME_VERTS=2512 AXIS=1` (mesh is z-up: game (x, y, z) = mesh (x, z, -y)). ICP of our
  posed vertices onto the game cloud (translation + yaw, four start yaws with `MULTI=1`),
  reports the RMS of the best 80% nearest distances and per-body-group residuals
  (`PGROUPS=1`). `OURS_DUMP=FILE` adds our actual blended play poses (viewer env
  `LBP_DUMP_VERTS=FILE` writes them, `--play-sackboy-input` drives it); `BLEND=a,b` searches
  lerp/additive blends of two clips; `VARIANTS=1` adds hold-root / mirrored variants.
- Noise floor: a perfect clip pose fits at 0.7-0.8 (a few hidden or extra verts, joint 22).

## Results

| Case | Game vs ours | Note |
|---|---|---|
| Stand | 1.3 | floor |
| Walk right, steady (walk_fast) | 0.7-0.8 | exact clip frames |
| Walk left, steady | 1.4-1.8 | same as a plain x-flip of walk_fast (1.5): mirror rule fine |
| Jump, ascent / descent (before) | 2.4-2.9, arms 3-4, hands 3-4 | jump clip at weight 1 |
| Jump landing tail (before) | 2.7-2.9, arms 11-13, hands 16-18 | |
| Jump, all (after) | 1.3-1.5, arms/hands 1-3 | air weight 0.7 -> 0.9 |

Jump finding: in the air the game's jump layer is not at weight 1. Best fits are the jump
clip with 10-40% stand under it (stand or stretch_crouch frame 4 fit equally well): 0.3 at
take-off falling to 0.1 near the end of the descent, then the tail. The weight does not
follow |vy| (it does not dip at the apex). Applied as `JUMP_AIR_START` 0.7 rising to
`JUMP_AIR_END` 0.9 over 30 air ticks (INFERRED from one standing jump; walking jumps and
the 0x20a5f4 follower reading are not reconciled).

## Not resolved

- Stop (walk_fast_stop) and turn poses fit at 3-5, not exact: need tick-aligned captures.
- Head-top bones 22/23/24/27/28 differ 5-10 even at rest (secondary motion or hidden
  parts); not modelled.
- Head-top bones and arms during stop/turn are not modelled.

## Emote inputs, confirmed in play mode (memory reads, same boot)

The Moon starts in create mode (D-pad Up = pause, Down = Popit). Pause menu (Enter) ->
"Change to Play Mode" -> "Discard Changes" (never Save) gives play mode. The mood level
words sit at anim object +0x894[i] (this boot guest `0x36c9f414`, level slots in order
happy, sad, angry, scared; found by scanning for a (1,0,0,0) level row with a 1.0 weight row
after it). Reads after D-pad presses, one press each:

| Check | Result |
|---|---|
| Up / Down / Right / Left = slot 0 / 1 / 2 / 3 | happy / sad / angry / scared: confirmed |
| Same direction again | level 1 -> 2 -> 3 -> off: confirmed |
| Another mood while one is on | replaces at level 1 (angry 2 -> sad: sad 1, angry 0): confirmed |
| Opposite mood (up/down, left/right) while one is on | ends the active mood, the pressed one does not start: confirmed (angry 2 + Left -> all 0; sad + Up -> all 0) |
| Timeout | none (6 s later unchanged): confirmed |

These are the D233 INFERRED rows for the d-pad mapping, cycle and opposite cancel; the
visual emote clips were not compared frame by frame.
