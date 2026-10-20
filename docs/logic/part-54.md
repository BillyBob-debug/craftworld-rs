# Logic, Part 54

See docs/logic.md for Parts 1-49.

## Part 54: the followers' tag key colour (D1152)

The route stopped at x 910.8 because follower 2499 ("Rise 9", configuration 11,
followKey) found no tag. Its tweak's colour read 12 where the tag's read 6.

FOUND, the search: with followKey (tweak+0xf4) the tag search 0x1c3e04 calls 0xceb8c
with the tweak's colour (+0xd8) and name (+0xdc); that leads to 0xceac0, a linear scan of
0x18-byte entries that returns the first exact int32 pair match (cmpw at
0xceb1c-0xceb2c). The fallback 0xceb58 uses the map at world+0x328. Among the matches the
nearest is kept (strict d < best, best starting at maxRange) in an x window with margin
0xd3500(world), after the filters 0x12f6f4, 0xf9a64, root +0x5c, 0x88f7dc and the cone
test against DAT_00d98014.

FOUND, the tag side: the inserter 0xddf98 (via 0xde2d4 from 0x13152c, SWITCH_KEY bit
0x1a) copies PSwitchKey +0xc (colour) and +0x10 (name) verbatim. The serializer 0xb71e34
reads +0xc through 0xc1dc90, an interned wstring at +0x10 and a u8 at +0x24.

FOUND, the tweak side: 0xb7834c reads the colour into tweak+0xd8 through the same
0xc1dc90 when the version is at least 0x28e. 0xc1dc90 is a thunk to 0x8a9240: with
stream+0x2c bit 0 (compressed integers) it reads a LEB128 (0x25024) and zigzags it,
(v >> 1) ^ -(v & 1); otherwise 4 bytes. The only other writer of +0xd8 is the script
setter 0x3e024 (cases 4 and 0x12).

Model: the PPhysicsTweak reader takes the colour as an s32, as the PSwitchKey reader
already did; it read a raw i32, so compressed colours came out doubled. The test
`follow_key_tweak_colour_reads_like_the_tag_colour` checks both colours and the key.

Census (followKey followers whose (colour, name) has a tag; census-all-41 before,
census-all-42 after):

| configuration | pair before | name only before | pair after | name only after | neither |
|---|---|---|---|---|---|
| 11 | 692 | 1,457 | 2,080 | 69 | 31 |
| 5 | 147 | 326 | 461 | 12 | 18 |

Route: the x 910.8 stop is gone. Sackboy now stands at x -2,467.6 from about tick 1,000,
blocked on +x by lift 2643 (follower MoveTweak 2652: configuration 11, followKey key 35,
range 28, accel and decel 0.125). The lift aims at its tag at (-2,204, 5,539), rests on
Thing 1330 and touches gyroscope 1320 on its left. INFERRED: that its up-left push jams
it against the gyroscope.

Not modelled: the 69 and 12 name-only followers; the lift jam (the 1/mass rate, Part 53).
