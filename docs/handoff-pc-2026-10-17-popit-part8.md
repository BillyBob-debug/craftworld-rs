# Handoff: Popit lane, part 8 (PC, 2026-10-17)

## Read first
1. `docs/d408-popit-page-slide.md` and `docs/d407-popit-backgrounds-order-and-check.md`.
2. `crates/lbp-bevy-viewer/src/popit_play.rs`: `popit_slide`, `PageFixed` / `PageClip` /
   `OldPage`, the old-node loop at the top of `popit_page_ui`, `owned_backgrounds`.
3. Part 7 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part7.md`).

## State
- PR #132 (D407) merged. PR #134 (branch `popit-9`, worktree `E:\Projects\wt-popit`):
  D408 plus this handoff; the coordinator merges once `rust` + `python` are green.
- RPCS3 is closed. The measuring run launched it minimised, but the game window still
  became the foreground window once; a later run needs a launch that never activates it.

## Facts
- Slide frames (ignored, never commit): canonical `local-analysis/d408/` `r1-dDDD-K.png`,
  `l1-dDDD-K.png` (DDD ms after the key), `sheet-r1.jpg`; boot/trial scripts `boot.py`,
  `trials.py`, `fit.py` there. Ours: `d408/ours/k0.5.png`; capture script
  `local-analysis/d408-cap.sh` in the worktree (delete old PNGs first).
- `--popit-slide-at K` presses R1 once the page is up and holds the slide at fraction K.
- Backgrounds use a DLC heuristic in place of the profile's inventory; the check mark
  sits on palette entry 0 because `--profile-records` colours do not reach the Popit.

## Next
1. Card placement (D403 Open) and highlight sizing (D400).
2. Pass `--profile-records` colour to the Player Color check mark.
3. Use decision numbers D409 and up.
