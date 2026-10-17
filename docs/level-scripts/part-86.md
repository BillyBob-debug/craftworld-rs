# Level scripts, part 86: the Story view's info bubble (2026-10-16, D339)

Part 81 drew the Story title's outline bubble; the game also shows a green bubble with the
selected result's name and author beside its badge.

### What was found

- FOUND (01.33): the planets' per-frame update (0x4381d8) finds the selected result and hands
  it to the info bubble manager (0x7010b8). A page is pushed when the cursor mode is not
  blocked (mask 0x1af) and the stack is empty, so the bubble shows at once for the badge under
  the cursor; a slot result gets the level page (0x702c5c, draw 0x70430c).
- FOUND (0x7006c0, the bubble frame): `GBS_INFOBUBBLE_PRIMARY`, style 13
  (`GCS_SPEECH_BUBBLE`, filled, no outline), child spacing 32, borders 48, corner radius 64,
  no transition, curvature 5, corner jauntiness 5, tail scale (15, 10, 0.6). The fill is the
  Gooey's player colour 0 (frame colour slot 1, 0x487490), set from the profile by 0x438314.
- FOUND (0x70430c, 0x703768): an outer frame of maximum width 1150; the slot's name (text style
  1; "Unnamed Level", key 0xeca1a592, without one), a break, the author (style 5, "By %1",
  key 0x43dd1282). For a story level or group a goodies row follows ("Goodies & prizes won",
  key 0xd9b1b864, style 4): collected x 100 / total, truncated, printed "%d%%" (0x4f57b4);
  hidden when the total is 0. The counts walk the slot's item list against the profile
  (0x43982c; a group sums its levels, 0x347ec).
- MEASURED (the game frame of the Story view, 1280 x 720): the bubble's body is x 731..1026,
  y 282..414, filled (150, 221, 60), the default player colour (the main menu's pills); its
  tail is 45 pixels long, its tip at (686, 348), its base about 0.4 of its length on each side
  of the middle (as part 81's tail). The text is white.
- FOUND (the story slot list): The Cosmos (4, 27) has tag `COSMOS_THEME` and author
  "Dr Herbert Higginbotham"; the other groups likewise.

### What changed

- lbp-runtime: `SearchSlot` keeps the slot's tag, stored name and author.
  `LevelWorld::story_info_bubble` builds the bubble frame above for the default result while
  Story is up (the name by `Slot.toString`'s rules from the loaded tables). `SpeechBubble`
  also reads style 13 as a filled bubble (`filled`), its tail `SPEECH_BUBBLE_TAIL` (96 units,
  the measured 45 pixels at part 81's 0.47 pixels a unit).
- Viewer: the bubble's items are laid out with the Story menu's and placed so the tail's tip
  is the measured offset from the title bubble (INFERRED: the selected badge stays put on the
  screen while Earth turns it to the camera, part 82; 0x701780's placement is not decoded).
  `MenuShape::Bubble { filled }` draws the filled body with its tail on the left, in the
  player's colour; the text in it is opaque white. On the overlay route (no final pass) the
  bubble is the node's background, without a tail.
- Left out: the goodies row (the profile's collected items are not read; the row is hidden,
  as the engine does for a total of 0); the maximum width (no wrapping); curvature and
  jauntiness (the body is a plain rounded rectangle); the badge card itself.

### Verification

`d339_story_info_bubble_holds_the_default_slot_name_and_author` (lbp-runtime),
`d339_filled_bubble_has_its_tail_on_the_left` (viewer). `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; viewer, lbp-preview and lbp-runtime
tests. No viewer capture was taken.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
