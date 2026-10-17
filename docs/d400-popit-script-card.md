# D400: the Popit card from the game's `Poppet` script (Popit part 1)

Date 2026-10-17. Before: the `--play-sackboy` Popit (D235, docs/d235-popit.md) drew its create
and play cards from hand-copied tables (button order, icon sheets, cell sizes, frame counts,
tooltip keys, default focus). Now the viewer runs the game's own `Poppet.DoMainMenu(Gooey)` on
the Gooey host (`crates/lbp-ui-script/src/popit.rs`) and takes all of these from it; the
tables stay as the fallback when the catalog has no scripts, and the log line
`popit card: ...` says which was used.

## FOUND (script listing, 01.33 `poppet.ff`)
- `DoMainMenu` opens frame 10000: `SetFrameLayout(3, 3)`, borders 0, child spacing 16 x 16,
  highlight sizing 264, highlight style 4, scroll style (0, 5); `SetDefaultNode(10003)`
  (the Popit Cursor, button 2). Edit or play is `Thing.IsEditing`.
- Each button is `DoImageSDFButtonNamed(id = 10001 + index, icon, sdf colour
  (0.039, 0.498, 0.012), size 128 x 128, tint, uv (0, 0, 1, 1), flags)`. The icon is the
  `Resource` field `.init` filled with `LoadAnimatedTextureByFilename(guid)`; the tint is
  grey 0.5 (flags 0) when `Profile.IsLBP2GameProgressionFlagSet` is false, white (flags 256)
  when true.
- The tutorial button (8) needs `GetPlayOrEditMode() == 1`, `GetNumPS3s() == 1`,
  `AmIOverallPartyHost(player)` and not `IsTutorialLevel()` (`DoMainMenuEdit` +13..+44).
- `GetMainMenuTooltip(index)` returns the button's LAMS key (an `if index == k` chain).
- With those answered as a single-player create session the run gives create rows
  `6,1,8 / 4,2,0 / 7,5,3`, focus 2, and play rows `6 / 4,0 / 7,3`, focus 6: the D235 order.
  Without the answers the card has 8 buttons (no tutorial) and every button is grey.

## FOUND (animated textures, `ATXb` type 45)
- Ten icon `.atx` files (revision 0x38a, flags 7, 13 to 15 body bytes): a texture reference
  (flags integer, type byte 2 = GUID, compact GUID), frame count, cell width, cell height,
  and an integer 1 (meaning UNKNOWN). Parser `lbp-resource/src/animated_texture.rs`. Every
  icon's sheet GUID, frames and cell equal the D235 table (the sheet is the `.atx` GUID - 1
  for all ten).

## MEASURED
- The script's pitch (128 + 16 = 144 units) over the measured 46.5 px pitch gives 0.323 px
  per unit, the scale D235 measured for the icons (41 px for 128 units).
- RPCS3 frame 2026-10-11 12:56:43 (create mode, Moon) against the viewer
  (`--play-sackboy --popit-open --window-16-9`): same nine buttons in the same places, same
  icons, focus tile on the Popit Cursor with its "Popit Cursor" pill. The card's outline and
  gradient are unchanged from D235.

## Tools
- `ui_trace ... Poppet DoMainMenu__Q5Gooey NEW.json TRANS... Thing.IsEditing=true
  Game.GetPlayOrEditMode=1 Game.GetNumPS3s=1 Game.AmIOverallPartyHost=true
  Profile.IsLBP2GameProgressionFlagSet=true`: an argument `Class.Base=VALUE` pins a native's
  return value (`GooeyHost::answers`).
- The Gooey host now returns `Resource.Load*ByFilename__g` as `Ref(guid)` and records
  `SetDefaultNode` as a `DefaultNode` frame property.
- Inputs: the Poppet script is in the decrypted patch overlay, so use
  `--snapshot local-analysis/d133-inventory-0133-edat.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"`.

## Open
- The pod button (10, play mode in the own Pod) is not in either card run (its condition is
  not answered); its icon stays from the table.
- Highlight sizing 264 and style 4 do not yet set the focus tile (68 px, measured).
- Next Popit slices: the inventory pages (`DoInventoryGrid`, submode 11) from the script,
  cursor mode, more frame comparisons.
