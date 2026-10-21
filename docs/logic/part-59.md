# Logic, Part 59

See docs/logic.md for Parts 1-49.

## Part 59: tag synchronisers send a tag's offset to a receiver's tag (D1157)

Correction to Parts 57 and 58: the world list at +0x2b8 (count +0x2bc) holds the tag
synchroniser (PTagSynchroniser), not the wormhole. The part table rows in
crates/lbp-level/src/thing.rs give serialized indices; the runtime part enum adds 1 to
every index above 0x28 (crates/lbp-level/src/wormhole.rs: serialized 0x30 is slot 49),
so runtime bit 0x30 is serialized 0x2f, TAG_SYNCHRONIZER. Part 58's "list +0x2ac is the
tag synchronizer" is wrong too.

The part is saved as type s32, colour index s32, name (wide string), radius, min radius,
scale x, scale y (f32), useLayers and visible (bools); `PartData::tag_synchroniser` now
reads it.

FOUND, the list tick 0x132800 (EBOOT 01.33). An entry is skipped while its Thing+0x7f
is set, or while its 0x60 chain is non-null with +0x1c 0. Type 0 is a sender (0x13187c),
type 1 a receiver (0x131f88); other types do nothing.

- Sender: walks the tag map (0xceb8c) for its key and takes the nearest lit tag
  (0x12f6f4 ternary not 0) with minRadius² <= d² <= radius²; a radius below 0.0
  (DAT_00d96820) has no distance test. It passes over tags on its own root Thing and the
  tags synchronisers make themselves (bit 0x40000000). It keeps the tag's offset along
  its own axes over their lengths at +0x90+0x10 and is on; with no tag it is off.
- Receiver: takes an on sender of its key (0x130c90) and, if online, a second source
  (0x2a8538); the shorter offset wins. It makes its tag Thing once (0x131638), and each
  tick places it at its own point plus its unit x axis times offset x times scale x plus
  its unit y axis times offset y times scale y; z is the offset's z with useLayers, else
  its own. On, it registers the tag in the tag map (0xde2d4); off, it removes it (0xd1b58).

Model: `scene::SceneSync` (a synchroniser's kind, key, ring, scale, body, point and unit
axes in its body's frame), `SyncMarks` on the body entities (or one loose entity),
`sync_spots` for their world pose, and `synced_tags(syncs, lit tags)`, which gives each
receiver with a sender its tag as a `TagSpot`. The viewer adds those to the tags its
followers and aiming spins search. INFERRED: "own root" is the sender's own body, and
0x130c90 picks the sender nearest the receiver. Not modelled: the useLayers z (the
online source and the other 0x130c90 caller 0x29ff3c are resolved in Part 68). Test:
`d1157_tag_synchronisers_send_a_tag_offset_and_receivers_place_a_tag` in
crates/lbp-sim/tests/sandbox.rs.

Census (`level_physics --all` over the 01.33 catalog, new `senders`, `receivers` and
`syncs_on_body` counts): 62 senders and 82 receivers in 7 levels, all in the
Cross-Controller levels; 50 of them ride a body. The Rookie Test has none.

Route: unchanged from Part 57 (x -3,789.7 on tick 950; x -2,870.3, y 5,188.6 from
tick 1,000 to about 1,400; x -2,440.6, y 5,400.9 from tick 1,600 to 2,007).
