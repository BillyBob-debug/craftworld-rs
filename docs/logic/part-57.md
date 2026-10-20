# Logic, Part 57

See docs/logic.md for Parts 1-49.

## Part 57: a tag switched off by its input cannot be followed (D1155)

After Part 56 lift 2643 (follower 2652, key 35) rose at level start to the key-35 tag at
y 5,539, though in the game it waits. All 15 key-35 tags in the Rookie Test (Things 1404,
1416, 1515, ... 2543, x -2,204, y 5,539 to 12,049) are loose tags, each fed on its input
port 0 by output 2 of the circuit board that is its parent; microchip 1326 drives those
boards. Our sim treated every tag as present.

FOUND, the keyed search 0x1c3e04: for each tag of the key in the x window it calls
0x12f6f4 on the tag and goes on only when the second word of the result, the signal's
ternary, is not 0, before the team test 0xf9a64 and the range. 0x12f6f4 gives
{0, 0, -1} for a Thing whose +0x7f is set, else the signal on the Thing's input port 0
(`thing+0x60`, entry 4, words +0x18, +0x1c, +0x20), else {1.0, 1, -1}: this is
`LevelWorld::tag_signal`. So a tag whose wired input is off is not found. The tag
sensor 0x118d5c already skipped such tags (Part 43); followers and aiming spins
(configuration 5) now do too.

Read and ruled out: the tag map itself (0xceb8c, register 0xde2d4 into 0xddf98,
unregister 0xd1b58) changes with a tag's key (0x12fa14) and when a Thing enters or leaves
the world (0xe4d1c, 0xd7ce0, 0x19f16c, 0x1a15e0), not with its signal. The world list at
+0x2b8 (count +0x2bc) holds part 0x30 (added in 0xe4d1c and 0xde6c8, removed in 0xd7ce0)
and its tick 0x132800 registers a tag Thing it places itself (type 1, 0x131f88); that
part is not read further here.

Model: `TagSpot` and `TagMarks` carry the tag's Thing; `lit_tags` keeps only the tags whose
ternary is not 0, and the viewer passes it `LevelWorld::tag_signal` for `drive_moves` and
`drive_spins`. Test: `d1111_keyed_followers_find_tags_with_their_colour_and_name` in crates/lbp-sim/tests/sandbox.rs
(the D1155 lines at its end).

Census (scratch example over the 01.33 catalog, the wire targets of every Thing's switch
outputs): 19,629 tags (PSwitchKey) in 378 levels, 5,316 of them with an input wired to
the tag, in 139 levels; the Rookie Test has 507 tags, 470 wired. A wired tag is found
only while its input's ternary is not 0.

Route: Sackboy matches the D1154 run to tick 1,000 (x -2,870.3) but no longer falls:
he stands at x -2,870.3, y 5,188.6 to about tick 1,400, then at x -2,440.6, y 5,400.9
from tick 1,600 to 2,007.

Next: what part 0x30 is and its type-1 tag placement (0x131f88); spin followers' parent
point (0x95fdc8).
