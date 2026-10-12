# D235: Popit overlay in `--play-sackboy` (create and play menus)

Date 2026-10-11. Viewer: `--play-sackboy ... --popit-mode edit|play [--popit-open]`; Z or the
pad's West button toggles it, arrows move the focus while it is open. Code:
`crates/lbp-bevy-viewer/src/popit_play.rs`. Static pass: `local-analysis/results/2026-10-11-ghidra-popit.md`
(not committed). Game frames (RPCS3 F12, private, never committed) are in RPCS3's screenshot folder.

## FOUND (game script / EBOOT)
- The Popit menu is the Fish script `gamedata/scripts/poppet.ff` (class `Poppet`) on Gooey, not C++ layout.
  Card: GCS_GRADIENT_FILL frame, corner radius 64, jauntiness 10, curvature 16; grid frame centred, child
  spacing 16x16, icons 128x128 white SDF buttons (`SDF_INFO(10,127,3)`).
- Order: create `6,1,8 / 4,2,0 / 7,5,3` (costume, goodies, tutorials; personal star, cursor [default focus],
  stickers; text chat, tools, hold-to-retry); play `6 / 4,0 / 7,3` (no goodies, cursor, tools; pod button only in
  the own Pod). Tooltip strings are LAMS keys (see `ITEMS`), "Close Popit" is key 0x9c2aeec8.
- Icons: animated B8 sheets (`.atx` cell/frames): see `ITEMS` (guid, cell, frames). The sheets hold the shape at
  0.1-0.3 and 0 outside, so the silhouette edge is 10/255 (3/255 soft); values 127+ are the face detail.
- Open/close: `PPoppet::SetOpen` 0x515da8; 10 ticks, smoothstep (0x4099ac). Square glyph = cell (3,0) of
  `ui/poppet/poppet_icons_dxt5.tex` (000020c4).
- `poppet_settings.pop` is cursor/input speeds only (CursorSpeed 0.65, Deadzone 0.3, 45 items per page).

## MEASURED (RPCS3 1280x720, Moon, 2026-10-11) - screen space, not game constants
- Create card 170x177 px, 3x3 cells at 46.5 px pitch, icon 41 px, focused tile 68 px teal (69,203,184) with a
  56 px icon; label pill 27 px tall, (206,180,36), below the card for the bottom row, above it for the top row;
  card colours (over the sky) top (185,106,135) -> bottom (182,148,106); card bottom 40 px above Sackboy's head.
- Play card 109x146 px (rows 1,2,2), focus tile on the costume button, label "Customize Character" above.
- Camera on open (create): about 1.1x closer, scene drops 58 px; the viewer applies zoom 1.1 and an aim drop.

## MEASURED pass 2 (RPCS3 create frames 12:56:41-45, play frames 16:37-16:38, 1280x720)
- Card 170x177 (edges at half intensity: x 554..724, y 260..436); fill is **radial from the tail attach point**
  on the bottom edge (x = 0.547 of the width in create, about 0.51 in play), yellow (205,178,40) at the point to
  pink (183,100,143) at the farthest top corner; ramp table `CARD_RAMP` (t = distance / farthest corner).
  Matches the `GRADIENT_TETHER` guess (gradient style 7). Play frames fit the same ramp within 4/255.
- Tooltip pill 30 px tall, width about 13 px per character; stacked tail 11 px wide x 13 px long (8 long when
  it points sideways). Top row: pill above, tail down, tip 19 px above the tile. Middle row (create): pill to the
  right of the tile, tail tip 8 px off the tile edge, pill centre 4 px below the tile centre. Bottom row mirrors
  the top (INFERRED). Pill centre sits 5 px right of the tail tip.
- Cord: 2-3 px yellow, leaves the card bottom 21 px right of its centre going down, bows right, then turns left
  onto Sackboy's chest; it wiggles (animated wave), the viewer draws one static cubic.
- Sackboy shows his back to the camera while the Popit is open in both modes (create and play frames), untinted;
  the viewer turns him by pi over the open blend.

## MEASURED pass 3: inventory pages (create mode, RPCS3 frames 18:12-18:13, Cross opens, Circle returns)
- The pages (Body/costume, Stickers, Tools "Sounds" list, Community Objects) are native `DoInventoryGrid`, all with
  the same chrome: panel x 918.5..1222.5, y 53.5..667.5 (304x614), same ramp as the card with the attach point
  on the left edge at 0.57 of the height (cord ends there), title (handwritten font, 30 px) at (936,84),
  "Functions" header y 133 with a dashed yellow rule at y 148, function tile centred (957,180) 64 px teal,
  section headers (19 px bold) with rules, 6 icon columns at x 957 + 46*k, rows 46 px apart, page counter
  "n/N" bottom centre (1070,646).
- Left of the focused tile: yellow tooltip pill (right edge 8 px off the tile), "USE!" + Cross pill 62 px above
  and "More!" + Square pill 55 px below, both tilted and 55-60 px left of the pill centre.
- Camera: Sackboy turns to face the camera, stands centred with feet 165 px below the frame middle, about 270 px
  tall (about 2.6x the gameplay camera); the cord runs from the panel's left edge to his hips. The Popit button
  focus is remembered per page; the viewer only models the first page of each button.
- Viewer: `--popit-page BUTTON` (6 costume, 0 stickers, 1 goodies, 5 tools, 4 community, 10 pod) starts there;
  X / pad South opens, C / pad East closes. Icons come from the disc plans by category (the game shows the
  profile inventory), section names are humanised tags, counts are not modelled.

## MEASURED pass 4: cursor mode (create frame 18:35:20)
- Square (Z) opens the Popit in create mode as well; Cross on the Popit Cursor button closes it and starts cursor
  mode: a 22 px yellow ring (2 px stroke, centre dot) at the frame centre (639,358), a thin yellow cord from
  Sackboy's hips to the ring, bottom prompt "Set Camera" with the R3 glyph (same pill as Close Popit). Camera
  stays the gameplay camera. Viewer: `--popit-cursor` (QA), WASD / left stick move the ring, C / Circle leave,
  Z / Square leave and reopen the Popit.

## MEASURED pass 5: Tools = the Sounds list (frame 18:12:52, 5/8)
- The Tools button opens the sound-object list, not the tool plans: title "Sounds", no Functions row, one column.
  Rows 46 px apart from y 140, icon 38 px at x 957 (focused row 64 px, teal bar x 960..1213, 40 px tall), label at
  x 1000 (21 px bold). Order and speaker colours match the game exactly for the 11 visible rows (Alerts,
  Animals, Contraptions, Destruction, Earth Wind and Fire, Electric, Human Body, Human Vox, Impacts, Launchers,
  Mechanisms).
- Source: `sound` plans under `gamedata/lbp2/unthemed/` sorted by title. The LBP1 `palette_music_*` sounds, DLC packs
  (Cross-Controller, DC, Move, Muppets, Nightmare, ...) are absent from the measured list (INFERRED: not owned /
  not shown on that profile). Probe: `crates/lbp-preview/examples/popit_pages_probe.rs`.
- The tool plans (`palette_tools_*`: Capture Object, Corner Editor, Electric, Flame, Horrible Gas, Material Changer,
  Unlethalize, Freezatron) are the "Functions" tiles of the other pages; the Community page shows Capture Object.
- Counter "n/N" is not item or page index: Body 4/5, Stickers 1/5, Sounds 5/8, Community 4/4 (OPEN).
- Button -> page (frames 18:50-18:51, create mode): Goodies Bag opens "Community Objects" (Functions: Capture
  Object + a ghosted tile, empty-message bubble, counter 4/4); Personal (star) opens "Tools and Used Objects"
  (Functions row of 14 tiles: Popit Cursor first, then arrow, splat, paint can, sticker tool, scissors, checker,
  capture, 4 triangles (bolt, flame, sun, gas), triangle-X, torn paper; then "Gameplay Kits" with the Popit stone);
  Tools Bag opens Sounds. The function tile icons are not the plan thumbnails (those decode to SDF placeholders):
  sheets `0001cfb9` (cursor/can/sticker/splat/scissors) and `0001d150` (bolt/flame/gas/sun/X/bracket/camera/bag)
  hold related glyphs; the triangle tiles and tile order are OPEN. Viewer shows only the first tile (Popit Cursor).
- Circle in cursor mode returns to the open Popit (focus Popit Cursor); returning from any page resets the focus
  to the Popit Cursor (the 2026-10-11 f handoff said "not reset": wrong).

## OPEN
- Exact gradient colours/alpha (not in script, Gooey defaults), card anchor rule, bubble tail/cord geometry
  (yellow S-curve drawn as dots here), camera constants (no named data found), whether Sackboy turns/tints
  (the 2026-10-11 frames show him from behind, untinted), animation order of the icon frames when focused.
- Create mode needs the real D-pad Down binding; the viewer uses Z for both modes.
