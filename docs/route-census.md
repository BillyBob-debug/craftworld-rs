# D64 corpus diffuse route census and same-sample multiply — 2026-10-07

## Route census (diagnostic tool)

`cargo run -p lbp-preview --example diffuse_route_census -- SNAPSHOT GAME-DIR
MATERIAL-CENSUS NEW-JSON` reads every material location listed by the D60 material
census (5163 locations, 44060339 archived bytes, 64 MiB aggregate budget, the
existing per-material input limit, 4 KiB pages) and runs only the diffuse
*selectors*. Texture descriptors, wrap, UVs and pixels are not checked, so a
selected route is not payload compatibility and not an asset count. Unsupported
materials also get a compact signature of their diffuse ancestry for grouping.

Results (d63-diffuse-route-shapes.json, before D64): texture route 4175, solid
color 459, unwired black 7, needs animation base 34, needs supplied instance color
122, unsupported 366. Weighted by mesh dependencies (2716 mesh resources by hash,
dependency-level, not primitive binding): 2269 have every material selectable by
default, 152 need an opt-in, 293 have one or more unsupported materials, 2 have none.

Top unsupported shapes by dependent meshes (meshes blocked by nothing else in brackets):
three-texture product with an independent third transform, 114 materials /
about 132 meshes [84]; same sample on both multiply ports, 17 / 29 [25]; texture ×
COLOR nested in a three-term product, 18 / 23 [13]; wired or animated samplers in
several shapes, about 100 materials.

## Same-sample multiply

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin d6a2ef80…, GfxAssembler MULTIPLY
  (lines 328–347) multiplies its two port inputs; a wire from one sampler to both
  ports yields the same sampled value twice.
- `CONFIRMED_FROM_LOCAL_DATA`: 17 materials (d64-square-material-samples.json) have
  exactly output ← MULTIPLY ← sampler on ports 0 and 1. Pinned oracle parity:
  17 compared / 0 mismatches / 17 projected.
- Rule: the two-texture product branch now also accepts left == right. The second
  sample equals the first (same slot, channel, transform), so the existing product
  path (LCM grid or independent GPU samplers) computes texel × itself at each
  coordinate. Filtering and colour-space fidelity claims are unchanged. The earlier
  test that rejected this case now asserts it as supported.

## Counts (distinct catalog GUID)

New cohort d64-square-mesh-samples.json: every catalog mesh depending on one of
the 17 materials (26 hashes, 938487 bytes). Before: 26 viewable / 0 full diffuse /
0 Complete; after: 26 / 23 / 0; supported groups 36 → 65 of 68; geometry identical.
D61 48-cohort unchanged at 48/43/0. **+0 newly viewable / +0 newly Complete**;
23 assets newly reach full diffuse preparation (not visual verification).
No external geometry oracle was run for the 26 added meshes.
