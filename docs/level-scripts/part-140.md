# Level scripts, part 140: a short link's strip (2026-10-19, D393)

Part 139 left open the third strip the link call builds when a link is no longer than
its head. This part reads it, and the shaft's u for an active link. The viewer draws a
short link's start as the game does.

### Found (EBOOT, the link call 0x4a1e40, r2 0xdaa0c0)

- The constants: 1 (-0x5dbc, f29 and f23), -1 (-0x5d0c, f27), 0 (-0x5d80, f25), 0.999
  (-0x5c60), degrees per radian (-0x5c74) and 0.01 degrees (-0x5c40, the shortest arc
  drawn, 0x4a1f84). f17 is the head and f28 the progress.
- 0x4a201c: `f = (progress * angle - head) / angle`, the neck's place along the arc. The
  neck and the tip are found at `f` and at the progress (0x8956bc, 0x4a204c, 0x4a2068).
- When `f` is above 0 (0x4a2078), the shaft and the head are built as part 139 says; the
  head's u always runs 0.5 to 1 (0x4a20f8, f6 = f23).
- Otherwise (0x4a215c) one strip runs from the start to the tip at the head's width (f20)
  and v (f18, f19), its u from `1 - 0.5 / (1 - f)` (0x4a215c-0x4a21a8) to 1 (f6 = f29).
- An active link's shaft (flag 0, 0x4a2134) takes u from `(progress * angle - head) /
  head` at its start to 0 at the neck, so its band repeats along the shaft; a pending
  one's u is 0.25 all along (0x4a2084).
- 0x420890 is the key test of parts 134 and 138: kind 2 is a group whatever its word
  (0x4208a4-0x4208b0). No code stores a literal 2 into a key's kind byte, so kind 2
  comes from loaded data.

### Code

- `link_short_u` (viewer `story_world.rs`); `link_ribbon` uses it for a link no longer
  than its head.

### Next

The active set, once a link is drawn active; what loaded data gives a key kind 2.
