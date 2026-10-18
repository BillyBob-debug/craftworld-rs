# Handoff: Popit lane, part 10 (PC, 2026-10-18)

## Read first
1. `docs/d412-popit-pod-facing-and-pill-side.md` (D412 / D413 and its Open list).
2. `docs/d411-pod-popit-without-decrypted-scripts.md` (no-decrypted-copy fallback).
3. `crates/lbp-bevy-viewer/src/popit_play.rs`: `pill_side`, `popit_turn`.

## State
- PR #138 (D411) merged. PR #140 (branch `popit-12`, worktree `E:\Projects\wt-popit`):
  D412 / D413 plus this handoff; green, the coordinator merges.
- RPCS3 was not used this part. No viewer left running.

## Facts
- No-decrypted run: `--snapshot local-analysis/d79-inventory-0133.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133"` (canonical repo paths). With the edat
  overlay: `d133-inventory-0133-edat.json` plus a third layer `local-analysis/d133-edat-overlay`.
- All 426 game scripts are only in patch.edat: without it every `--level-scripts` mode
  loads 0 scripts. The Pod now falls back to `POD_REST_CAMERA` (auto, no flag); planets are
  not placed in that mode.
- `--qa-capture` must be a new PNG under the worktree's own `local-analysis/` (cwd).
- Tooltip rule research: EBOOT 0x417b78, notes and disassembly in canonical
  `local-analysis/d411/` (`pre_ann.asm`). Root-frame size (card vs larger) is unresolved.
- Game Pod frame `local-analysis/pc-helpers/pod-popit3.png`: Sackboy three-quarter right,
  looking up at the card; ours now faces the camera (no look-up / turn yet).
- Decision numbers: D414 and up.

## Next
1. Sackboy's facing / look-up pose while the Popit is open (D412 Open).
2. Full tooltip placement (lerp, offsets, Moon top-right case; D413 Open).
3. Pod planets without the scripts (D411 Open), if no Pod lane takes it.
4. Highlight sizing 264 / style 4 vs the 68 px focus tile (D400 Open).
