# Handoff: Popit lane, part 1 (PC, 2026-10-17)

## Read first
1. `docs/d400-popit-script-card.md` (what D400 found and built, open items).
2. `crates/lbp-ui-script/src/popit.rs` (header comment, `answers`, `run_card`).
3. `crates/lbp-bevy-viewer/src/popit_play.rs` lines 1-60 and `script_cards` / `poppet_vm`.
4. `docs/d235-popit.md` "MEASURED pass 3" (inventory pages) for the next slice.

## State
- PR #118 (branch `popit-1`, worktree `E:\Projects\wt-popit`): D400, the card from
  `Poppet.DoMainMenu`. Opened with CI pending; the coordinator merges once it's green.
- Builds used `CARGO_TARGET_DIR=<repo>\target-d312`; toolchain via the main checkout's
  `scripts/use-local-rust.ps1` (the worktree has no local toolchain).
- Ignored scratch: `local-analysis/d400/` (captures, compare-edit.jpg), `d400-popit-*.json`
  traces, `d400-atx*/` dumps.

## Facts
- Inputs with scripts: `--snapshot local-analysis/d133-inventory-0133-edat.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"`. The d68
  snapshot has no Poppet class (the viewer falls back to the tables).
- `ui_trace` must run from the main checkout root (output path must start with local-analysis).
- Poppet listing: `local-analysis/d133-script-listings-0133/9e6063c13a1bbff3cd48a7c8435234ae6e1e8b0d.txt`.
- Viewer capture (hidden): `--level-path gamedata/levels/palettes/borders/blank_level_large.bin
  --play-sackboy --no-activate --window-16-9 --popit-open --qa-capture local-analysis/<new>.png`.
- The game's create-mode frames are in RPCS3's screenshots folder, 2026-10-11 12:56:41-45
  (`local-analysis/d235-popit/p2-edit.png` is a viewer capture, not a game frame).
- RPCS3 was not started in this part.

## Next
1. Inventory pages from the script: trace `Poppet` functions around `PushSubModeInventory__i`,
   `UpdateGlobalUI__...EInventoryPageCustomType` and the `DoInventoryGrid` native, with
   answers; replace `PAGES` / `PANEL` constants the same way as D400.
2. Pod button (10) condition in play mode; highlight sizing 264 / style 4 vs the 68 px tile.
3. Use decision numbers D401 and up.
