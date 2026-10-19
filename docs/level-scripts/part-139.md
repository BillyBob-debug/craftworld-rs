# Level scripts, part 139: a link's texture (2026-10-19, D392)

Parts 133 and 136 drew a link as a flat ribbon. This part finds the texture the drawer
binds and how its UVs map onto the shaft and the head. The viewer now draws links with it.

### Found (EBOOT, the drawer 0x426484, r2 0xdaa0c0)

- The planets' constructor loads g16570 (`ui/fatline/lines_sdf.tex`) into +0x250 at
  0x42621c (TOC 0xda2678). +0x24c is g26529 (`pin_icons_sdf.tex`) and +0x254 g66721
  (`price_tag.tex`). The drawer binds +0x250 at 0x426548 (`bl 0x49d104`), then sets
  the shader constant 0x64f6da0a at 0x42656c (`bl 0x49d3fc`).
- The texture is 128 by 256 A8R8G8B8 with 9 mips. Alpha is 255 and green 0; red (and
  blue) is a distance field that peaks near 0.33 in the middle of each line. Rows 0-32
  hold the pending shaft, 32-64 the active shaft, 128-192 the pending head and 192-256
  the active head; the heads narrow from x 93-106 to nothing at x 127, an arrowhead.
- The link call 0x4a1e40 builds the shaft with the width (f24, 0x4a20c4) and the head
  with twice it (f20, 0x4a2030, 0x4a2108). The strip builder 0x4a1250 keeps one width
  along a strip, so the head is a rectangle and the texture draws the arrow.
- Pending UVs: shaft u 0.25 throughout, v 0 to 0.125; head u 0.5 to 1 along, v 0.5 to
  0.75. Active UVs use v 0.125 to 0.25 and 0.75 to 1.

### Inferred

- The line's edge is where the field passes about half its peak (0.16, with 0.04 of
  softness). PLACEHOLDER: the line shader and its constant 0x64f6da0a were not read.
- A link shorter than its head takes the third strip at 0x4a21ac (not read); the viewer
  starts the head's u part way in.

### Code

- `LINK_TEXTURE` and `link_coverage` (viewer `story_world.rs`): the field turned into
  white with alpha; `link_ribbon` gives the shaft and the head their UVs.
- The level load decodes the texture from the game's files; `follow_links` binds it
  and draws flat when it cannot be read.

### Next

The line shader's edge, the third strip, and the active set once a link is active.
