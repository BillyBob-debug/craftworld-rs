# D408: Popit page slide (Popit part 8)

Before: L1 / R1 swapped the page contents in a single frame. After: the old page slides
out and the new one slides in, side by side and cut at the panel's edges, with the game's
timing and easing. The panel card, the page counter, the arrows and the tooltip stay put.

## MEASURED (RPCS3 2026-10-17, play-mode Pod Popit, Body 4/5 <-> Player Color 5/5, one F12
frame per trial at 33 ms steps after the key, kept in ignored local-analysis/d408)
- The move takes 10 frames at 30 Hz (333 ms). The contents' shift per frame, in px at
  1280x720: 0, 6, 26, 58, 102, 159, 212, 253, 280, 294, 297. The pages are 297 px apart.
- R1 moves the contents left, so the new page comes in from the right; L1 mirrors it.
- Both pages are visible at once, each a pure translation (no scale, no visible fade),
  clipped to the panel. The panel, its gradient and the counter do not move. The counter
  switches to the new value at once, and the page title slides with the contents.
- The tooltip pill and its prompts are not cut at the panel and do not slide.

## FOUND (EBOOT 01.33, read as data only)
- `0x463b98` sets the collection's page (+0x38), focuses the new page's gooey node and
  plays `poppet/menunextpage`. Each frame the collection draws every page into one frame
  sized to the panel. `0x462500` only marks the page helpers for a refresh; it is not the
  animator.

## INFERRED
- The slide is the gooey frame scrolling to the newly focused page node. Its speed lives
  in the frame class (vtable not located), possibly as a scroll style setting.
- Smoothstep fits the measured fractions within 0.06 (the game is slightly ahead early).

## Built
- `Popit::slide` (direction, seconds) is set by a page change. `popit_page_ui` keeps the
  old contents as `OldPage` under their own `PageClip` node when the page turns.
- `popit_slide` moves each new content node into the panel-sized clip node and shifts
  old and new by 297 px x smoothstep over 1/3 s. `PageFixed` marks what does not slide.
- QA flag `--popit-slide-at K`: one R1 once the page is up, held at fraction K.

## Open
- The tooltip's text during the slide follows our rebuild, not a measured rule.
