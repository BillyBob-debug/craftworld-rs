# Handoff: Popit lane, part 4 (PC, 2026-10-17)

## Read first
1. `docs/d403-popit-in-the-pod.md` (Square rule in the Pod, card check, open items).
2. `docs/d402-popit-titles-and-pod-button.md` "FOUND" (default pages per collection).
3. `crates/lbp-bevy-viewer/src/popit_play.rs`: `PAGES`, `popit_input`, `popit_page_ui`.
4. Part 1 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part1.md`) for inputs and captures.

## State
- PR #123 (branch `popit-4`, worktree `E:\Projects\wt-popit`): D403 plus this handoff.
  The coordinator merges once `rust` + `python` are green.
- Scratch (ignored): `local-analysis/d403/` (viewer logs, captures, side-by-side with the
  game frame). RPCS3 was not started.

## Facts
- Build env: `local-analysis/env.sh` in the worktree (copy of `E:\Projects\d321-env.sh` with
  `CARGO_TARGET_DIR=/e/Projects/wt-popit/target`); `source` it in bash. A release build of the
  viewer takes over 10 minutes from cold: run it in the background.
- Run the viewer from the main checkout root (`E:\Projects\LBP2 Rust Rewrite`, where
  `game-data/` and `local-analysis/` inputs are) with the worktree's exe.
- Pod with Popit capture: part 1's inputs plus `--level-guid 00018fdb --pod-camera 1
  --pod-shell 000027b9 --play-sackboy --level-scripts --window-16-9 --preview-lighting
  --popit-open --popit-mode play --qa-hide-ui --qa-capture local-analysis/<new>.png`. The
  process does not exit after the capture: wrap it in `timeout 200`. The early frame looks from
  inside the shell (pre-existing, also with the Popit closed).
- RPCS3 frames already on disk (owner's, never commit): `local-analysis/pc-helpers/pod-popit*.png`
  (Popit in our Pod, 2026-10-11) and `pod-page*.png` (a Pod page open), useful references
  for page switching.

## Next
1. Page switching within a collection (L1/R1; default pages in D402; check `pod-page*.png`).
2. Card placement beside Sackboy in the Pod (game frame: card left of him, tooltip above).
3. Highlight sizing 264 / style 4 vs the 68 px focus tile (D400).
4. Use decision numbers D404 and up.
