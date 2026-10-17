# Level scripts, part 88: the profile's per-slot records (2026-10-17, D341)

Part 87 built the info bubble's goodies row but left it hidden: its counts come from the
local profile's per-slot records, which the repo did not read. This part reads them from the
running game and gives the viewer a file of them, so the row shows.

### What was found

- MEASURED (the owner's save, `BCUS98245PROFILE...`, files `0` and `BIG000`): both end in
  the plain magic `FAR4`, but the rest is encrypted (the PS3's protected save data: the count,
  key and table before the magic read as noise; only the last partial block is plain). The
  save cannot be parsed without the console's keys, which the repo does not carry.
- FOUND (01.33, 0x894b2c -> 0x399b04 -> 0x68cbe0): the local profile is
  `*(0xE4F800 + idx * 0x120 + 0x108)`, `idx = *0xE50A04`.
- FOUND (0x199c9c): the per-slot records are `{data, count}` at profile+0x280 / +0x284,
  0x80 bytes each, `SlotID` (type, id) at +0; a record's items `{data, count}` at +0x38 /
  +0x3c, 0x20 bytes each: a resource descriptor (GUID +0, SHA1 +4, type +0x18, 0x26 with the
  top bit set) and the skip word +0x1c (0x43982c).
- FOUND (the profile's vtable +0x40, 0x171c3c -> 0x171a00): "owns the item" is a binary
  search of a sorted array of pointers at profile+0x78 / +0x7c, each to `{GUID, SHA1
  pointer (0: a zero hash), type}`, compared by type & 0x7fffffff, GUID, then SHA1. An item
  without a GUID goes to `*(profile+0x518)`'s vtable +0x40 instead (not followed here; in
  the owner's profile only records of `SlotID` types 1 and 2 hold such items, 123 counted
  ones, which the script counts as not owned).
- MEASURED (the owner's profile in RPCS3, a run not kept): 391 records (210 of `SlotID`
  type 0, 116 of 8, 43 of 1, 22 of 2) and 3272 owned entries. The story levels' records
  hold 1..92 counted items each; most are all owned, a few none.
- MEASURED (the same run, in the Pod; FOUND chain 0x139308: players at `*0xDC7B00` +0x1e4,
  0x6b0 each, colours at `*(player+0x16c)` +0x4c..+0x58): the one player's four colours are
  0xffceb424, 0xffc331a0, 0xff3bcfc1, 0xffe60f64, and that object is not the local profile.
  The game frame's green title and bubble (part 86) are not the first of these (a yellow),
  so the Gooey's player colour comes from elsewhere. Open.

### What changed

- lbp-runtime: `parse_profile_records` reads lines `TYPE ID COLLECTED TOTAL` into
  `LevelWorld::profile_goodies`' map.
- Viewer: `--profile-records PATH` fills `profile_goodies` in the scripted level, so the
  Story bubble's goodies row shows for a slot the profile has a record of.
- `scripts/rpcs3_profile_records.py OUT.txt`: reads the records from LBP2 01.33 running in
  RPCS3 (memory reads only) and writes that file, counting as 0x43982c does.
- Left out: the save file itself (encrypted); a GUID-less item's owner check; 0x43982c's
  recursion into linked levels.

### Verification

`d341_profile_records_are_read_by_slot_id` (lbp-runtime). Viewer capture (scripted Pod, held
at tick 760, `--profile-records` from the owner's profile, not kept): the bubble for The
Cosmos (the viewer's default result) shows "100%" and "Goodies & prizes won"; its levels'
records are all owned. The game frame of part 86 shows Da Vinci's Hideout at "0%", which
the measured records do not give (its levels' records are mostly owned). INFERRED: that frame
came from another profile state. `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; lbp-runtime tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
