# D406: Popit Player Color and Backgrounds pages (Popit part 7)

Before: the Costumes collection's Player Color page and the Tools Bag's Backgrounds page
were empty (their plans have no category, so `plan_sections` skipped them), and
Backgrounds sat last in the Tools Bag. After: Player Color shows the twelve player colours
as tinted Sackboy figures in three columns, Backgrounds shows one column of wide tiles,
and Backgrounds is page 6 of 8.

## MEASURED (RPCS3 frames 2026-10-17, 1280x720, kept in ignored local-analysis)
- Neither page has a Functions row or section headers.
- Player Color: 3 columns x 4 rows, column centres x 970 / 1075 / 1165, row tops y 140 /
  262 / 382 / 502. Each item is a Sackboy silhouette in one colour with a small rounded
  square of a second colour behind its top right; the focused one is drawn larger.
- Backgrounds: one column of wide tiles, x 943-1186, about 75 px tall at an 87 px pitch,
  6 visible; the focused tile grows to about 332 x 108 with a dark border.
- Tools Bag page order: Music 4/8, Sounds 5/8, Backgrounds 6/8, Gameplay Controls 7/8,
  Global Controls 8/8.

## FOUND
- Script class `PlayerColour` (g0000a9c2): `GetPlayerColourCount` returns 12;
  `GetPlayerColourFromIndex__i` takes the index mod 12 and loads four constants (primary,
  secondary, tertiary, emphasis) as 0-255 floats. Entry 0's primary (150, 221, 60) and
  tertiary (29, 220, 205) match what the repo already had.
- The player-colour plans' own icons are 2x2 dark placeholders.

## INFERRED
- The details' `colour` word (one byte repeated, every value below 12) is the palette
  index. The resulting colours and order match the game frame.
- The silhouette is the Body button's last figure frame (already used for the costume
  tool tiles); the game's own texture is not identified.
- Uncategorised plans on other pages form one unnamed section with no header.

## Built
- `PlanStyle` (Grid / Swatch / Wide) per page; Swatch and Wide pages skip the Functions
  row and count focus cells from 0.
- `swatch_sections` draws each player colour from `PLAYER_COLOURS`; `PopitItem` now
  carries the details' `colour`.

## Open
- The game's Backgrounds order (Avalonia first) is not the title order used here.
- Swatch sizes are approximate; the focused swatch's check mark is not drawn.
