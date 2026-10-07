# D61 supplied instance color and texture review — 2026-10-06/07

The code for this milestone was written on 2026-10-06 but its record was not
written then; this document and the STATUS D61 entry were completed on
2026-10-07 after re-running the checks listed in STATUS.md.

Bevy and the census example can opt into a **user-supplied** linear RGBA value
for ThingColor nodes. It is never a game-instance default: the default remains
no instance input, and the real per-object color the game would pass is unknown.
The UI states "Supplied linear RGBA; actual game instance values are unverified",
and an asset using it cannot be confirmed Complete.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler TEXTURE_SAMPLE / THING_COLOR](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  (lines 199–224, 268–269): `THING_COLOR` resolves to the per-instance color
  input; for a subtype-0 sampler, input port 0 is added to, port 1 multiplies, and
  port 2 is subtracted from the scaled/offset UV, in that order. A scalar input
  (swizzle `a`) is broadcast to both axes.
  [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java)
  numbers THING_COLOR 2, COLOR 3, MULTIPLY 10.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, branch 0. bounce_pad_glow
  4640ecf1…, neon_lifebar d83895a7…, lcd_stretch_custom ebb4137b… use ThingColor
  variant 8 (`parameters_raw[0] == 8`) as direct diffuse, nested texture tint, or
  alpha subtracted on sampler port 2. Six pinned oracle records
  (d61-blocker-material-oracle.json) were compared.
- `UNKNOWN`: the real instance colors, color space of the game's instance input,
  and any per-frame changes.

## Rules (independently written selector)

`project_instance_color` derives a copy of the graph; source bytes and the
original graph are kept. Within the diffuse ancestry only: each ThingColor must
be unwired, static, subtype 0, variant 8 and becomes a constant COLOR; samplers
may only take one wire, ThingColor `.a` into port 2 (UV subtraction applied after
the existing bounded scale/offset); one nested `A × (COLOR × B)` shape folds to
two samplers plus a retained tint. Other wiring, more than 1024 nodes / 4096 wires,
bad endpoints, nonfinite colors and UVs beyond ±1e6 are rejected. Subtraction is
cancellable every 256 vertices, capped at 100k vertices.

`additional_texture_slots` lists sampler slots reachable from any connected
output channel that the preview does not render. Unconnected descriptors are not
counted. Any group with such slots is marked "Other required texture slots
unrendered", and Complete cannot be confirmed for the asset.

## Counts (distinct catalog GUID, 48-hash D61 cohort)

The cohort adds 8 catalog-linked meshes to D60's 40 (d61-mesh-samples.json).
Pinned mesh geometry/group/UV oracles compare 48/48 with 0 mismatches.
Before the code: 48 viewable / 43 full diffuse / 0 Complete. After, default mode:
48/43/0. Supplied white: 45 full diffuse. Supplied (0.25,0.5,0.75,0.5) with
animation base: 46. **+0 newly viewable / +0 newly Complete.** Full diffuse with a
supplied color is preview coverage only, not game-correct coverage.
