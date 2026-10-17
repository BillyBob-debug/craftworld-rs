# Level scripts, part 93: the Story badge's medals and the info bubble's place (2026-10-17, D346)

Part 92 left out the gold icons along the badge card's bottom and the info bubble, which
covered the card. This part draws the icons the game shows and puts the bubble's tail on
the card.

### What was found

- FOUND (01.33): a badge has overlay slots 1..12 next to its own mesh (slot 0, 0x6fcfd0).
  Twelve wrappers (0x6fcd30..0x6fcf98) attach them through 0x6fca08, each with its GUID
  from the TOC (r2+0x6c98 down to r2+0x6c6c): padlock 0xeb1, medal_finish 0xd12f,
  medal_played 0x13890, medal_collect 0xd12d, medal_ace 0xd12e, medal_heart 0xd16d,
  medal_playlist 0x152b0, badge_rosette 0x13ee0, badge_versus 0x22f20, badge_cutscene
  0x22f1f, medal_lbp1 0x23a8c and medal_move 0x2edc4.
- FOUND (0x6fe398, 0x6fe974): each slot has a fixed offset in the card's own space. The
  values come from a 13 x 2 table, and its second column is the polaroid's. Finish and
  played sit at (0, -20, -200), collect at (150, -40, -150) and ace at (-150, -10, -150).
  Heart and playlist sit at (-/+150, -10, -50), lbp1 at (-150, -40, 150) and move at
  (150, -20, 150). Each icon turns about the card's Y by a random value in [-1, 1] times
  pi/8, and its scale springs to 1 while the slot is shown.
- FOUND (the refresh 0x45098c): the slot's SearchResultSlot (`*(result+0x30)`) keeps a flags
  word at +0x50, which the shown slots follow. Ace is bit 3 and collect bit 4. Finish and
  played come from the slot's state at +0x84.
- MEASURED (RPCS3 memory, the owner's profile, Story view): Earth's six group results,
  SlotIDs 4 22..27, each have finish, collect and ace shown (mask 0x34), with flags 0x18
  and state 3.
- MEASURED (the game frame): the info bubble's tail tip is at the card's right rim, at
  about half the card's height. Part 86's fixed offset from the Story title put the tip
  about 100 pixels left of that at the viewer's scale, over the card.

### What changed

- `scripts/rpcs3_profile_records.py`: also writes `overlays TYPE ID MASK` lines. These are
  the shown slots of each planet result it finds in memory, keyed by the SearchResultSlot's
  SlotID.
- lbp-runtime: `ProfileRecords::overlays` reads those lines.
- Viewer: `load_badge` loads the shown overlays (`BADGE_OVERLAYS`) and `add_badge` places
  them at their offsets, turned by a fixed per-slot spin (`overlay_spin`, standing in for
  the game's random one). The `Story badge` line lists them.
- Viewer: `anchor_info_bubble` moves the info bubble each frame, before the menu is
  painted, so its tail's tip is on the drawn card (`INFO_BUBBLE_ON_CARD`).

### Viewer run (part 80's flags, `--profile-records`, held at tick 760)

The spade, the Sackboy face and the money bag are drawn gold below the card, as in the
game. The bubble now starts at the card's right edge and no longer covers it. Captures
stay in ignored `local-analysis/d346/`.

### Left out

- The overlay spin is not the game's random draw, and the icons don't grow in (the game
  springs them from 0 when result+0x22c is set).
- The bubble's own size and its goodies row (the pink dot and large 100%) still differ.
  The light mat inside the card's rim (part 92) is still unchecked.

### Verification

`d346_profile_overlays_line`, `d346_overlays_sit_on_the_card`. `cargo fmt --all --check`;
`cargo clippy -p lbp-bevy-viewer -p lbp-runtime --all-targets -- -D warnings`; viewer and
runtime tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
