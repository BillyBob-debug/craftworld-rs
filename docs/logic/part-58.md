# Logic, Part 58

See docs/logic.md for Parts 1-49.

## Part 58: aiming spins measure from their parent's point (D1156)

Part 56 found that followers (configuration 11) search and aim from their Thing's parent,
not from their body's centre (D1154). The turning tweak that aims at a found target,
configuration 5, does the same.

FOUND, the SPU row packer 0x95fb20 (EBOOT 01.33, part at 0x95fdc8): it takes the
tweak Thing's parent (`*(*(param_2 + 8) + 0x5c)`, else the Thing itself) and, for
configuration 5, passes that Thing to the search 0x1c3e04 with the tweak's distance band,
cone and key (`param_2 + 0xe8`, `+0xec`, `+0xf0`, `+0xf4`). The aim is the found
target's translation less that Thing's (its `+0x64` object, matrix at `+0x110`), as in
the follower packer 0x957df4.

Model: `scene::SpinTweak::at` is that Thing's point in its body's frame, built as
`MoveTweak::at` (a parent with no matrix of its own takes its nearest placed ancestor's,
INFERRED as in D1154). `drive_spins` measures each tweak from its body's position plus
`at` turned by the body. Test: `d1156_spin_aimers_measure_from_their_parent` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `aimers` and
`aimers_offset` counts): 428 configuration-5 tweaks in 61 levels, 369 of them (in 47
levels) with a parent point more than 1 unit off their body's origin. The Rookie Test
has none.

Route: unchanged from Part 57 (x -3,789.7 on tick 950; x -2,870.3, y 5,188.6 from
tick 1,000 to about 1,400; x -2,440.6, y 5,400.9 from tick 1,600 to 2,007).

Notes for later: the world list at +0x2b8 (count +0x2bc, Part 57) holds part bit 0x30,
which is the wormhole part, and the list at +0x2ac part bit 0x2f, the tag synchronizer;
both INFERRED from the part bit table in crates/lbp-level/src/thing.rs. The wormhole
tick 0x132800 places and registers a tag Thing for its type-1 entries (0x131f88).
