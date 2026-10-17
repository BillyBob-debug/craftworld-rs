# D401: which Popit button opens which inventory page, from the `Poppet` script (Popit part 2)

Date 2026-10-17. Before: the viewer's inventory pages (D235 pass 3) were keyed by button
in a hand-made table (button 6 Body, 0 Stickers, 1 Community Objects, 5 Sounds, 4 Tools and
Used Objects, 10 My Pods), and every listed button opened its page in both modes. Now the
viewer runs the game's `Poppet.DoMainMenuButtonResponse(gooey, index)` for every button in
create and play mode and opens a page only where the script pushes the inventory submode;
the pages are keyed by the inventory collection it pushes. The table stays as the fallback
(log line `popit card: ... pages (button: collection) ...`).

## FOUND (script listing, 01.33 `poppet.ff`)
- `DoMainMenuButtonResponse`: button 0 -> `PushSubModeInventory(1)`, 4 -> 4, 5 -> 3,
  6 -> 0 (plus `poppet/changeskin` and `GatherCurrentCostume`), 10 -> 8; button 1 -> 2 only
  when `GetPlayOrEditMode() == 1`, otherwise `poppet/error`. Button 2 sends
  `SendSimpleMessage(0)` (cursor mode), 3 `PressSuicideButton`, 7 chat (`GetAllowChat`,
  `BeginTextEntry(1)`, or an info message), 8 the tutorials.
- `PushSubModeInventory(c)` clears `LastSelectedIcon`, sets `SinglePlayerMode` from
  `GetNumAvatars() <= 1` and sends `SendSimpleMessage(37, c)`; `c` is an
  `InventoryCollection.EInventoryCollection`.
- `UpdateUI`: submode 0 runs `DoMainMenu`, submode 11 runs the native
  `DoInventoryGrid(LastMainMenuButton, ...)`, after `SetInventorySound`. The page content
  (title, sections, tiles, counter) is native, so it stays MEASURED (D235 pass 3 and 5).
- Both submodes sit in frame 1 opened by `UpdateUI`: style 8, corner radius 64, corner
  jauntiness 10, curvature 16, gradient style 7, drop shadow alpha 0.25, highlight style 0,
  borders 0, sizing `GetDesiredBubbleSize() * 0.01`, clip cursor on.
- Static helpers the native grid can call: `DoTitleBox` (frame borders 24, 32, title text
  style 16), `DoNameBox`, `StartSection` / `DoSectionBreak` (layout (0, 3), borders 16,
  title style 4, dashed `DoHorizontalBreak` style 2 with `HDashScale`, offset -30).
  Not used by the viewer yet.

## Result
- Run (`ui_trace SNAPSHOT GAME-DIR Poppet popit-pages`): create
  `{0: 1, 1: 2, 4: 4, 5: 3, 6: 0, 10: 8}`, play `{0: 1, 4: 4, 5: 3, 6: 0, 10: 8}`. This is the
  D235 button-to-page table; play mode refuses the Goodies Bag page (not on the play card).
- Viewer capture `--popit-page 1` (create): the Community Objects page as before.

## Open
- Collection titles (0 Body, 1 Stickers, 2 Community Objects, 3 Sounds, 4 Tools and Used
  Objects, 8 My Pods) are still the MEASURED table; `DoInventoryGrid` is native.
- `ui_trace` of `UpdateUI` with `Poppet.GetSubMode=11` stops at `GetWorld` before the grid
  call (the host has no world); the dispatch above is read from the listing.
- The pod button (10) condition in play mode; highlight sizing 264 / style 4 vs the 68 px
  focus tile (from D400).
