# Logic, Part 56

See docs/logic.md for Parts 1-49.

## Part 56: followers measure from their parent's point (D1154)

After Part 55 Sackboy stood still at x -1,092.7 on body 1951. Its followers (Things 1957,
2260 and 2480, keys 18, 19 and 20) each sit on a board under a chip at (-944.0, 5,119.0),
and their tags sit on chip 1364 at the same point, but our sim measured from the body
centre (-787.5, 5,275.9), so every follower saw its tag about 220 away and kept pulling.

FOUND, the follower's search point: 0x957df4 takes the tweak Thing's parent
(`*(*(param_2+8)+0x5c)`, else the tweak Thing itself) and passes it to the tag search
0x1c3e04, which measures distance from that Thing's world translation
(`*(*(thing+0x2c)+0xc)+0x70`); 0x957df4 then subtracts the same point from the target's
position. The spin path 0x95fdc8 reads the same parent and calls the same search, but
spin followers (configuration 5) are not changed here.

Model: `MoveTweak::at` holds that point in the body's frame, taken from the parent's
placed matrix; `drive_moves` turns it with the body. A parent with no matrix (a chip's
board) falls back to its own nearest placed ancestor (INFERRED). Test:
`d1154_followers_measure_from_their_parent` in crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all`, local-analysis/logic/d1154/census-all.json): followers
(configurations 6-8 and 11) and those whose point is more than 1 from their body centre.

| levels with followers | followers | offset followers | levels with one |
|---|---|---|---|
| 119 | 1,653 | 1,399 | 105 |

Most per level: main_04_close_shave 106 of 107, toystoryminigame2 71 of 71,
move_main_4_b 63 of 63, main_04_waste_disposal 60 of 60, main_05_king_klong 57 of 61.

Route: body 1951's followers now sit on their tags and it stops drifting. Lift 2643
(follower 2652, key 35, min 0, max 2,100) now measures from about (-2,204, 5,119), where
no key-35 tag sits; the nearest is at y 5,539, so the lift rises about 420 at level start,
before Sackboy reaches it. Sackboy matches the D1153 run to tick 950 (x -3,789.7), then
falls at x -2,870.3 on tick 1,000 and ends at x -2,450.6, y 1,725.8 on tick 2,007.

Read and ruled out: 0x88f7dc (thunk to 0x550e70) is a player check gated on a global's
+0x180, not a tag on/off test; 0x12f6f4 rejects only a tag whose Thing +0x7f is set.

Next (INFERRED): the column tags under lift 2643 are likely visible one at a time
(tag inputs, or tags entering and leaving the 0xceb8c map, inserter 0xddf98 via 0xde2d4
from 0x13152c). Not modelled: spin followers' parent point (0x95fdc8).
