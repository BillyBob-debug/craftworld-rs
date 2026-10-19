# Level scripts, part 149: the line shader's programs (2026-10-19, D1002)

Parts 139 and 141 left open the program that draws a link, and whether its constant
0x64f6da0a (`(f2, f3, f1, t)`) moves the active band. This part reads the UI shaders.

### Found

- The UI programs are `gamedata/pshaders/gooeyps_*.fpo` (fragment) and one vertex
  program, `gooeyvs.vpo` (GUID 0x3da9), in DATA.FARC. A file is a short header and a
  zlib stream holding a CGB container with NV40 microcode.
- 0x4b3468 builds the table of 30 shader GUIDs (EBOOT data 0xc26c80-0xc26cfc): among
  them fill 0x19111, sdf 0x18d51, line_bezier 0x18ef1, line_bezier_sdf 0x1a14d,
  pinribbon 0x2a4fa, texture 0x18e9e and gooeyvs 0x3da9.
- `gooeyvs`'s parameters are modelViewProj, SdfScales, inner_rect, outer_rect, tint,
  fill, border and texture_vrange: none is a time.
- `gooeyps_sdf` samples unit 0 at TC0 and smooths the edge with screen derivatives
  (constants 250, -0.002, 0.5, 0.002), blending COL0 over COL1; `line_bezier_sdf` and
  `pinribbon` read no time either.
- The constant setter 0x167934 looks a name hash up in the bound program's sorted
  (hash, register) table and writes vertex constants (method 0x1efc).
- 0x4b4030's 0x38 selects a vertex attribute format (methods 0x1740, 0x1680), not a
  program.

### Inferred

- With the gooey programs bound, 0x64f6da0a matches no parameter, so no shader scrolls
  the band with `t`: the active look is the texture rows and the u part 140 gives. The
  viewer already draws it so; no code change.
- The link most likely uses `gooeyps_sdf` (its texture is a distance field); the draw's
  index into the GUID table was not read.

### Next

Confirm the bound programs from a capture of an unlock, if the active band looks
different there.
