# Handoff: Popit lane, part 2 (PC, 2026-10-17)

## Read first
1. `docs/d401-popit-pages-from-script.md` (what D401 found and built, open items).
2. `crates/lbp-ui-script/src/popit.rs` (`run_responses`, `call_with_index`).
3. `crates/lbp-bevy-viewer/src/popit_play.rs`: `PAGES`, `FALLBACK_OPENS`, `script_cards`.
4. Part 1 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part1.md`) for inputs and captures.

## State
- PR #119 (branch `popit-2`, worktree `E:\Projects\wt-popit`): D401 plus this handoff.
  Ready for review with CI pending; the coordinator merges once `rust` + `python` are green.
- Scratch (ignored): `local-analysis/d401/` (community capture, UpdateUI / DoTitleBox /
  DoSectionBreak traces). RPCS3 was not started.

## Facts
- `ui_trace SNAPSHOT GAME-DIR Poppet popit-pages` prints button -> collection per mode.
- Page content is the native `DoInventoryGrid(LastMainMenuButton, ...)` (submode 11 in
  `UpdateUI`); only its frame chrome and the static `DoTitleBox` / `DoSectionBreak` helpers
  are script (constants in the D401 doc).
- `ui_trace UpdateUI` with `Poppet.GetSubMode=11` stops at `System.GetWorld` (no world in
  the host) before reaching the grid.

## Next
1. Page titles per collection (find where `DoInventoryGrid` gets them: InventoryPage /
   InventoryCollection scripts g000176d8 / g000176da, or LAMS keys).
2. Pod button (10) condition in play mode; highlight sizing 264 / style 4 vs the 68 px tile.
3. Use decision numbers D402 and up.
