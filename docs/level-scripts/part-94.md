# Level scripts, part 94: the info bubble's size and its goodies row (2026-10-17, D347)

Part 93 left the Story info bubble narrower and taller than the game's, with its goodies
row drawn as two plain texts. This part draws the row as the game's play stat widget and
sizes the bubble's text as the game does.

### What was found

- FOUND (01.33, the level case 0x7049fc, the group case 0x704708): after the author the
  page does a horizontal break (type 1, (0.5, 0.5, 0, 0), 0, 0, colour (1, 1, 1, 1)), then
  a frame with one widget call, 0x496700 (from 0x703ef4): the label (key 0xd9b1b864),
  collected, total, text style 4 and the percent shown. There is no separate percent text.
- FOUND (0x4f5400, the widget's layout): every size comes from S, the style's size times
  the Gooey's scale (+4). The disc's radius r is (S x 1.9 + S) x 1.2 / 2 with a label; the
  gap after it 0.2 r; the percent is font 1 at S x 1.9 and the label the style's font at
  S x 0.9, below it, both at x = 2r + gap. Width 2r + gap + the wider text, height 2r.
- FOUND (0x4f5abc): the disc is a shader quad (resource 0x109ee): all the widget's colour
  once collected reaches the total, all (0, 0, 0, 0.3) at none, else a pie of the colour
  over that shade. The colour is the Gooey's vector at +0x1e0 (0x704088).
- MEASURED (the game frame, the owner's profile): the disc is (195, 49, 160), the
  profile's second colour 0xffc331a0. The title's and author's widths are 1.41 and 1.44
  times the viewer's at scale 1, and the percent's cap height (27 pixels) fits S at a
  Gooey scale of 1.42. The bubble is 348 x 144 pixels.
- FOUND (0x7033a8, 0x7035a4): the title is text style 1 and the author style 5, as part 86
  had them.

### What changed

- lbp-runtime: `GooeyItem::PlayStat` lays out as the widget does and places a
  `PlacedKind::Disc` and its two texts; a horizontal break places a `PlacedKind::Rule`.
  In a filled speech bubble a text is two times its size tall (`BUBBLE_TEXT_LINE`), its
  ink 0.4 of that above the line's centre, and a break takes the frame's child spacing.
- lbp-runtime: the info bubble's texts and widget use `INFO_BUBBLE_SCALE` (1.42), and the
  disc takes the profile's second colour (`profile_stat_colour`).
- Viewer: draws the disc (round, `PLAY_STAT_DISC_INK` of its quad) and the half-white line.

### Viewer run (part 80's flags, `--profile-records`, held at tick 760)

The bubble is 345 x 148 pixels against the game's 348 x 144, with the pink disc, the large
100% and the label under it, and the line under the author. Captures stay in ignored
`local-analysis/d347/`.

### Left out

- The Gooey's scale is fitted, not read from memory, and the line height, the ink's raise
  and the break's gap are fitted to this one frame.
- A part-filled disc is drawn whole (the pie is not drawn).

### Verification

`d347_play_stat_lays_out_a_disc_and_two_texts`, `d340_story_info_bubble_adds_the_goodies_row_after_15_frames`.
`cargo fmt --all --check`; `cargo clippy -p lbp-bevy-viewer -p lbp-runtime --all-targets -- -D warnings`;
viewer and runtime tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
