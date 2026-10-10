# P2w: level scenes are lit like the game by default

2026-10-09 (America/Chicago). Plan: `lbp2/research/shading-lighting.md` item 1.

Before: level scenes were drawn unlit unless `--preview-lighting` (Lambert) or `--lbp-lighting`
was passed. The LBP-shaped model (P2l-P2t: hemisphere ambient, point-shaped sun, two rims,
square-then-sqrt with tintcol 4, bump and gloss, game fog in output space) only ran when asked.

After: with `--level-guid` / `--level-path`, the LBP-shaped model is on unless the command line
says otherwise:
- `--preview-lighting` keeps the older Lambert preview.
- `--no-lbp-lighting` draws the level unlit, as before.
- `--lighting-preset N|main` works as before; the default preset is 0 when one is stored.
Non-level views (single meshes, UI, browsers) are unchanged.

Evidence status: the constants and shader shape are RSX-fitted on the Pod frame (161 of 162 main
pass draws share one block; Ghidra confirms the scale rules, oracles/results/2026-10-09-ghidra-
light-constants.md). That a Moon level uses the same block, and which preset it blends to, is
INFERRED until the requested Moon-level capture. The Pod fog start (5,810 open / 25,810 closed)
and fogcol are Pod frame values; other levels reuse them until the fog source is traced.

Not changed here (waiting on the PC batch): the sun is still a distant direction rather than a
point with the unresolved position offset, and presets blend by index 0 only. At the 300,000-unit
sun distance a point and a direction differ by little over a level's extent.
