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

## OPEN
- Exact gradient colours/alpha (not in script, Gooey defaults), card anchor rule, bubble tail/cord geometry
  (yellow S-curve drawn as dots here), camera constants (no named data found), whether Sackboy turns/tints
  (the 2026-10-11 frames show him from behind, untinted), animation order of the icon frames when focused.
- Create mode needs the real D-pad Down binding; the viewer uses Z for both modes.
