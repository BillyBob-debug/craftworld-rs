# D404: Popit page switching within a collection (Popit part 5)

Before: an inventory page opened from the Popit's main menu showed one page and a fixed
"1/1" counter. After: L1 / R1 (Q / E on the keyboard) step through the collection's pages,
the counter shows the page's place among the visible pages, and page arrows sit at the
panel's sides.

## FOUND (EBOOT 01.33, read as data only)
- The page strip is native: `0x463b98`, reached from `DoInventoryGrid` (0x614270 ->
  0x56c7fc -> 0x520c0c). Collection instance: page array +0x20, count +0x24, current page
  +0x38 (kept per collection; the profile saves it, D402).
- Previous page on gooey bit 0x1000 (pad L1), next page on 0x8000 (pad R1); clicking the side
  arrows gives the same bits. L2 / R2 are not page keys. The pad-bit naming follows the
  cellPad layout (INFERRED).
- No wrap: the search runs from the current page towards the end and stops there. Pages
  failing the visibility filter (`0x471a80`: empty pages of custom type 0, some flagged
  pages, the Background page unless a separate condition holds) are skipped. Each change
  plays `poppet/menunextpage`.
- "%d/%d" is the visible page's number over the visible page count, drawn only when more
  than one page is visible, with left / right arrows (each with a new-items marker).
- Default pages per collection, in order (title keys from the builder 0x465688; the shared
  pages are attached by creation order, INFERRED):
  - 0: Costumes, Head, Body, Materials, Player Colour.
  - 1: Stickers, Decorations, PlayStation Eye Camera, My Pictures, Community Photos.
  - 2: Materials, Collected Objects, My Objects, Community Objects.
  - 3: Tools, Gadgets, Gameplay Kits, Music, Sounds, Gameplay Controls (0x775ac2b6),
    Global Controls (0xd0cee297) and Backgrounds (0x51618d4b).
  - 4: Tools and Used Objects, Heart Items. 8: My Pods.

## MEASURED
- RPCS3 create frame 2026-10-11 18:12:52: the Tools Bag's Sounds page reads "5/8" at the
  bottom centre, with arrows at x 928 and 1212, y 350 (720p). All eight Tools Bag pages are
  visible on that profile, the Background page included.
- Play mode (2026-10-12): Personal opens Heart Items with no counter, so it is the only
  visible page there.

## Built
- `popit_play`: `COLLECTION_PAGES` (titles from the game's text table, English fallbacks);
  `PageData::visible` / `current`; `Popit::tabs` keeps each collection's page.
  `popit_input` steps with L1 / R1 and resets the grid focus to the first cell (INFERRED).
  `popit_page_ui` draws the page's title, the counter and the arrows on the side that has a
  page. QA flag `--popit-tab N` starts the `--popit-page` collection on page N.

## Open
- Only the profile's open page (the measured one) has contents; the other pages show their
  title, counter and arrows over an empty panel.
- Which pages the game hides depends on the profile's contents (empty custom-type-0 pages);
  the viewer keeps every default page outside play-mode Personal.
- The slide between pages (one horizontal scroller, input blocked while it runs) is not
  animated; the new page appears at once.
