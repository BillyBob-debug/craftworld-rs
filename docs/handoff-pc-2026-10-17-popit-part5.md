# Handoff: Popit lane, part 5 (PC, 2026-10-17)

## Read first
1. `docs/d404-popit-page-switching.md` (FOUND page strip rules, page lists, Open).
2. `crates/lbp-bevy-viewer/src/popit_play.rs`: `COLLECTION_PAGES`, `PageData::visible` /
   `current`, the L1/R1 branch in `popit_input`, the counter block in `popit_page_ui`.
3. Part 4 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part4.md`) for build and captures.

## State
- PR #126 (branch `popit-5`, worktree `E:\Projects\wt-popit`): D404 plus this handoff.
  The coordinator merges once `rust` + `python` are green.
- Scratch (ignored): `local-analysis/d404/` (captures `tools-tab4.png` / `tools-tab1.png`,
  side-by-side `d404-pages.jpg` with the game frame). `local-analysis/d404-cap.sh` in the
  worktree builds and captures (blank level, create mode, `--popit-page 5 --popit-tab N`).
- RPCS3 was not started.

## Facts
- Native page strip `0x463b98` (via `DoInventoryGrid`); collection instance +0x38 = current
  page. Visibility filter `0x471a80`; scroller per page `0x477274`; slide animator
  `0x462500` (speed not read). Builder `0x465688` gives every page's (type, subtype, key).
- Page types from the builder (for loading other pages' contents): Costumes 0x100010,
  Head 0x10/0x1fd, Body 0x10/0x3e02, Materials 0x20, Player Colour 0x80000, Stickers 8,
  Decorations 4, Eye Camera 0x4000, My Pictures / Community Photos 0x400, Collected 2,
  My / Community Objects 0x80, Tools 0x10000, Gadgets 0x8040, Gameplay Kits 0x200,
  Music 0x20201000, Sounds 0x400000, Backgrounds 0x100.
- Game frame 2026-10-11 18:12:52 (RPCS3 screenshots folder): Sounds "5/8", arrows x 928 /
  1212, y 350.

## Next
1. Contents for the non-default pages (plan pages by the types above), then the slide.
2. Card placement beside Sackboy in the Pod (D403 Open).
3. Highlight sizing 264 / style 4 vs the 68 px focus tile (D400).
4. Use decision numbers D405 and up.
