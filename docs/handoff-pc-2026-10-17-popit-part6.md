# Handoff: Popit lane, part 6 (PC, 2026-10-17)

## Read first
1. `docs/d405-popit-page-contents.md` (page types, what stays empty, Open).
2. `crates/lbp-bevy-viewer/src/popit_play.rs`: `COLLECTION_PAGES` (type names per page),
   `plan_sections`, `PageData::others`, the `home` / `others` switch in `popit_page_ui`.
3. Part 5 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part5.md`).

## State
- PR #127 (branch `popit-6`, worktree `E:\Projects\wt-popit`): D405 plus this handoff.
  The coordinator merges once `rust` + `python` are green.
- Scratch (ignored): `local-analysis/d405-cap.sh` in the worktree captures six pages
  (`--popit-page P --popit-tab T`) into the canonical repo's `local-analysis/d405/`
  (`--qa-capture` must be under the cwd's `local-analysis`). Side-by-side `d405-pages.jpg`.
- RPCS3 was not started.

## Facts
- Items per page (default page excluded, 36 = cap): stickers [0, 36, 8, 1, 1]; goodies
  [36, 36, 36, 0]; tools bag [8, 36, 36, 36, 0, 0, 0, 1]; costumes [36, 36, 0, 36, 0].
  Player Colour and most Backgrounds have no category, so `plan_sections` skips them.
- The capture of Tools Bag tab 0 timed out once (no PNG); the other five captured.

## Next
1. Player Colour / Backgrounds: show uncategorised plans (one unnamed section) and check
   against a game frame.
2. Page slide (animator `0x462500`, speed not read), then card placement (D403 Open) and
   highlight sizing (D400).
3. Use decision numbers D406 and up.
