# P3h: glow (output port 4)

Question: material output port 4 carries an emissive term. The port census (#108) shows 665
wired graphs: type12 mix 214, single sampler 205, type10 multiply 140, static colour 106,
unwired type2 30. The decoded shaders add glow unlit and write output alpha
`tintcol.w / (1 + dot(glow, glow))` (`oracles/results/2026-10-09-shading-fpdec.md`, CONFIRMED
from fpdec). Node types 10 (RGB multiply) and 12 (lerp, weight from port 2 or param[0]) are
CONFIRMED (`2026-10-10-node-types-10-12.md`) and are exactly what the diffuse graph reader
already compiles, so no new node rules.

Change (only where the LBP surface extras are on: `--lbp-lighting` or `--bump-gloss`; drawn
only in the LBP-shaped lighting):
- `surface::rewire_output` copies the material graph with port 4's wire moved to port 0, and
  the unchanged diffuse route (`load_diffuse_material`, output port argument) loads it: a solid
  colour, or one baked texture on a UV set. No instance colour is applied to glow.
- the viewer adds `glow` to the lit colour before the square-root output: scale 1 (INFERRED;
  the NV40 scale field is open in the oracle).
- a map must use the diffuse UVs, UV1 that is free or already holds the same coordinates, and
  no per-fragment graph/dual/third/fourth/product (it rides graph slot 4, graph_info.w and the
  last graph_param row: no new bindings). Anything else is named, not drawn:
  "glow map not drawn (...)".
- the bound glow shows in the surface label ("glow r/g/b", "glow map (WxH)").

Not done: output alpha from glow (the viewer writes opaque alpha), bloom, per-fragment glow
graphs mixing UV sets, glow animation (static values only, the diffuse loader refuses
animated ones), a glow map whose diffuse partner uses several textures.

No GPU in the cloud: not rendered. PC verification wanted: fmt/test/clippy on 1.99; census
unchanged; `surface` label counts of "glow" draws on Bravery Test and tutorial_23; before/after
at gameplay distance on a level with lamps/screens (tutorial_23 lamp, logic-board screens);
check nothing else changes.

## Verification follow-up (2026-10-10)
`2026-10-10-glow-output-stage.md` and `2026-10-10-glow-port4-col0.md` (fpdec): glow is added
unscaled before tintcol x4 and the sqrt (our order is right), constant glows come from the
stored colour, so bright white spheres and red lamps are authentic. Alpha is
`tintcol.w * lerp(1/(1+4*dot(g,g)), 1, sat(TC3.z))` (scale x4 INFERRED); still not drawn.
Change: a colour box (type 3) feeding port 4 is bound to the program as constant `ta<k>`
(k = box index; 2026-10-10-glow-gate, CONFIRMED over 8,043 gmats). When the shader blob names no
`\0ta<k>\0` (raw scan, any variant) the program has no glow add, so glow is skipped (the
frozen cube 00005407 and three others). Texture/mix sources are not decided and pass. An
earlier alpha-mode-4 gate was wrong (the cube is mode 1) and is gone. `--no-glow` and a glowing count in
`--level-report` ease A/B checks.
