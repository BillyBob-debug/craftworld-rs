# Level scripts, part 87: the info bubble's goodies row (2026-10-16, D340)

Part 86 drew the Story info bubble with the slot's name and author; the game frame also shows,
under a divider, a round icon, "0%" and "Goodies & prizes won". This part adds that row's
rules; its counts come from the profile, which the repo does not read yet.

### What was found

- FOUND (01.33, the level page draw 0x70430c, jump table 0x704584): a story level (`SlotID`
  type 0, 0x7045ac) and a story group (type 4, 0x7046b0) build the row, others do not. Both
  wait until the page has counted 15 frames (0xdb8c0c, read at 0x70465c and 0x7046e4; the
  count is page+0x38) and draw nothing while the total is 0. The row (0x7030b8) is
  `DoHorizontalBreak(1, 0, 0, 0.5)` (0x4924cc) and a play stat (0x496700) with label key
  0xd9b1b864 ("Goodies & prizes won", 0x703200), text style 4, the percent flag and the
  Gooey's colour slot +0x1e0.
- FOUND (0x4f57b4): the percent is collected x 100 / total, truncated, printed "%d%%"
  (UTF-16 at 0xc6b7b0), or "100%" (0xc6b7c0) once collected reaches the total.
- FOUND (the counts come from the profile, not the slot list): the level branch looks up
  the local profile's per-slot record (0x894b2c -> 0x89581c -> 0x199c9c: a sorted array of
  0x80-byte entries keyed by `SlotID` at profile+0x280, at most 4000, evicted by a 64-bit
  time at +8; looked up without creating one) and counts it with 0x43982c: every 0x20-byte
  entry of its list at +0x38 whose word +0x1c is 0 adds 1 to the total, and 1 to the
  collected count when the profile's vtable +0x40 says it owns that item. No record: no
  call, total 0, no row. INFERRED: the record is `PlayedLevelData` (its name sits in the
  profile's field names, 0xc786b8) and its list the items the level holds.
- FOUND (the group branch 0x89628c -> 0x347ec, notes of part 86): the group sums 0x43982c
  over the slots whose group is it, of type 0..2 or 8, with slot+0xba false and slot+0xc0 0
  or 5. FOUND (the slot serializer 0xab3a40..: `DeveloperLevelType` +0xc0,
  `CollectabubblesRequired` +0xd8, `CollectabubblesContained` +0xe4, `IsSubLevel` +0xba):
  so the filter is "not a sub-level, `DeveloperLevelType` 0 or 5", and the slot list's own
  collectabubble lists are not the counted list (they are at +0xd8 / +0xe4, not +0x38).
- MEASURED (the story slot list, `lbp2story_slots.slt`, a probe run not kept): 78 of its
  82 slots carry collectabubble entries; the first twelve have `CollectabubblesRequired`
  entries of count 0 and `CollectabubblesContained` counts of 1..3. INFERRED: these are
  not the bubble's total (above).
- INFERRED (the game frame's "0%"): the owner's profile holds records for some of The
  Cosmos's levels with items it does not own; a fresh profile has none and shows no row.

### What changed

- lbp-runtime: `SearchSlot` keeps `is_sub_level` and `developer_level_type`.
  `LevelWorld::profile_goodies` holds the profile's `[collected, total]` by `SlotID` (empty:
  the profile is not read). `LevelWorld::story_goodies` gives a level's own counts or a
  group's sum by the filter above. `story_info_bubble` adds the row once
  `PlanetsSearch::info_bubble_since` (the `PWorld.Frame` the bubble came up, set in
  `advance_frame`) is 15 frames back and the total is not 0: a `HorizontalBreak`, the
  percent (`goodies_percent`) and the label (the loaded table's key 0xd9b1b864, else the
  English text), both text style 4.
- Left out: reading the profile (so the row stays hidden in the viewer, as for a fresh
  profile); 0x43982c's recursion into linked levels; the play stat's round icon, its
  colour and the value's larger text (0x496700 not decoded); the divider is a row break
  only (the menu drawing has no line item).

### Verification

`d340_story_info_bubble_adds_the_goodies_row_after_15_frames` (lbp-runtime: no records ->
no row; a group's levels 1/3 and 0/4 with a sub-level and a type-3 level left out -> "14%"
after 15 frames, not before). `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; viewer, lbp-preview and
lbp-runtime tests. No viewer capture was taken.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
