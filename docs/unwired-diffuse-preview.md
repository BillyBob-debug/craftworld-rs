# D63 unwired diffuse preview — 2026-10-07

A material whose output box has no diffuse (port 0) wire now previews as an
explicit black solid group instead of a neutral unsupported group. The app labels
it "No diffuse wire • black (0,0,0,0) per Toolkit shader generator; game default
unverified • other channels not rendered". It is not an invented texture or a
fallback for a failed binding.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::setupPath](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  (lines 844–922): when no box feeds a port, the generated shader returns
  `float4(0.0)` for every port except opacity (1.0), GLSL bump and baker specular.
  Lines 846–866 make an exception for Blender-family materials, which take
  diffuse/glow/specular colors from output parameters 0–2.
  [BrdfPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BrdfPort.java)
  numbers DIFFUSE 0, SELF_ILLUMINATION 4, REFLECT 6.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00. base_logic_lcd_custom 00020c95
  (3d32e960e2288d59886b531d7fa2a875e6295540): 11 boxes / 10 wires, output
  parameters all zero, wires only into ports 4 and 6; no box type ≥128. The pinned
  oracle records its diffuse route as null.
- `UNKNOWN`: what the original game shader does for an unwired diffuse port.
  Toolkit's generator recompiles materials for the game, but that does not prove
  the shipped shaders behave the same. Glow/reflection appearance is not rendered.

## Rule

`select_solid_diffuse` accepts zero diffuse wires only with one output box that is
subtype 0, unanimated, with all eight parameters zero, and no Blender-family node
(type ≥128) anywhere in the graph. Result: color bits `[0; 4]`, `unwired_diffuse`
set, `color_index == output_index`. Supplied instance color projection passes such
graphs through unchanged. Samplers reachable from the other connected ports are
reported as unrendered texture slots, so Complete cannot be confirmed for the
asset. Census schema 9 adds `unwired_diffuse`; parity schema 11 checks a null
oracle route plus zero color (`unwired_projected`).

Tests: zero result, ignored counts, instance pass-through, nonzero output
parameter, Blender-family node, animated output.

## Counts (48-hash D61 cohort, distinct catalog GUID)

Default: 48/43/0 → 48/43/0 (00020c92 groups 1/3 → 2/3). Supplied color plus
animation base: 47 → 48 full diffuse (00020c92 2/3 → 3/3).
**+0 newly viewable / +0 newly Complete.** Every sampled asset now prepares all
diffuse groups with all options on; this is binding coverage, not visual correctness.
