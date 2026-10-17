# Handoff: Popit lane, part 7 (PC, 2026-10-17)

## Read first
1. `docs/d406-popit-colour-and-backgrounds.md` (layouts, palette, Open).
2. `crates/lbp-bevy-viewer/src/popit_play.rs`: `PlanStyle`, `swatch_sections`, the
   `style != PlanStyle::Grid` block in `popit_page_ui`, the swatch/wide branch in the nav.
3. Part 6 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part6.md`).

## State
- PR #131 (branch `popit-7`, worktree `E:\Projects\wt-popit`): D406 plus this handoff.
  The coordinator merges once `rust` + `python` are green.
- RPCS3 was started once by a helper (scratch config copy, no patches) and closed.

## Facts
- Game frames (ignored, never commit): canonical repo `local-analysis/d406/`
  `game-player-colour.png`, `game-backgrounds.png` (1280x720, RPCS3 F12; bgcap.py gave
  black frames this time). `d406-compare.jpg` = ours vs game.
- Capture script: `local-analysis/d406-cap.sh` in the worktree (`--popit-page 6 --popit-tab
  4` = Player Color, `5 5` = Backgrounds). Delete old PNGs first (capture must be new).
- Palette source: script `PlayerColour` g0000a9c2, listing in canonical
  `local-analysis/d133-script-listings-0133/9f77c916...txt`. Name keys
  PLAYER_COLOUR_LBP2_01..12_NAME = indices 0..11; description keys run 1, 12, 11, ... down.
- The Pod Popit has no Tools Bag; create mode only. Costumes opened on Body (4/5).

## Next
1. Backgrounds order (game starts with Avalonia) and the focused swatch's check mark.
2. Page slide (animator `0x462500`), card placement (D403 Open), highlight sizing (D400).
3. Use decision numbers D407 and up.
