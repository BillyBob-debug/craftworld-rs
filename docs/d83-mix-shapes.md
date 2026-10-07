# D83 — two-texture alpha MIX on the nearest product grid (2026-10-07)

Backlog rank 4 ([backlog.md#mix](backlog.md#mix), 14 MIX materials on 01.33,
"two-texture alpha MIX 8"). Route code only (`lbp-preview`); no parser, disk-layout,
shader or viewer change. Other MIX shapes and BLEND are untouched.

## Rule

`select_alpha_mix` (D58, COLOR + one sample) now also accepts the shape where both
MIX ports 0/1 are texture samples and port 2 is the alpha of one of them:

    MIX(port0 = T0, port1 = T1, port2 = Tk.a), k in {0, 1}
    out = (T1 - T0) * Tk.a + T0      (all four channels, same scalar factor)

Selection requirements (explicit `Unsupported:` errors otherwise): one static
subtype-0 unanimated MIX with exactly three incoming wires on ports 0/1/2, source
port 0, ports 0/1 unswizzled, port 2 swizzle exactly `a` + four NUL bytes; two
distinct samples; the factor wire reuses one of them; both samplers pass the
existing static unwired UV0/UV1 sampler rule; and both sit on the existing
nearest product grid (`uses_grid`: same UV channel, integer scales 1..16, zero
offset). At load time both textures must be Repeat on both axes (same check as the
D57 product MIX). The stored MIX parameter 0 is kept as `unused_weight_bits`.

`DiffuseSelection.texture_alpha_mix` (`TextureAlphaMix { mix_index, factor_port,
unused_weight_bits }`) carries the shape; `sample_index` is the port-0 sample and
`second_sample` the port-1 sample. `product::pixels_alpha_mixed` bakes the grid:
RGB is mixed in linear space and re-encoded sRGB, alpha linearly, nearest only
(same authored policy as D58, docs/alpha-mix-preview.md). Same 4 Mpixel / 4096 grid
budgets and cancellation points as the D49 product bake. Frontends already treat a
product asset as nearest; labels say "two-texture alpha MIX".

Not supported (stay unsupported with a specific reason): different UV channels or
non-grid scales (6 of the 8 local materials: UV0/UV1 pairs and a 70x overlay need
independent per-sample UVs and a shader mix mode, not a CPU bake); wired or
animated samplers; factor from a third sample or another channel.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`, case
  `BoxType.MIX` (lines 388–410): ports 0/1 via `getWithSwizzle`, a wired port 2
  overrides parameter 0, result `((r - l) * f) + l`. Input node kinds are not
  restricted, so two samples mix the same way as COLOR + sample. Read only; no code
  copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: 01.33 (BCUS98245 disc + updates 01.01..01.33). The
  8 materials of shape `12,1,1 | 1>0:1, 1>0:2a, 2>0:0` all put the factor on port 1
  (overlay alpha), Repeat wrapping, static samplers. Toolkit oracle (MaterialOracle
  `--diffuse`) vs `diffuse_parity` schema 14 on all 8: 0 mismatches, 2 projected.
- `UNKNOWN`: game filtering (nearest is authored), game pixels, other channels.

## Counts (diffuse route selection, unit = material location with a MAP row)

`diffuse_route_census` on 01.33 vs `d78-diffuse-route-shapes-0133.json`:
texture route 6,938 → 6,940; unsupported 209 → 207; other classes unchanged
(solid 603, unwired 10, animation-base 106, instance 228, no MAP row 65).
11 more materials changed only their reason: 6 two-texture alpha MIX now say
"requires same-channel integer Repeat grid samplers"; 5 (THING_COLOR family
graphs) now reach "sampler must be static, unwired UV0/UV1". Not asset counts.
Newly viewable / newly Complete assets: not measured (no mesh cohort run); +0
claimed. Corpus totals unknown.

## Commands (PC, from the main checkout)

- `diffuse_route_census SNAPSHOT "game-data/a1-disc;game-data/a1-update-0133"
  local-analysis/d68-material-census-0133b.json local-analysis/d83-diffuse-route-shapes-0133.json`
  → counts above.
- `scripts/run-material-oracle.ps1 -Diffuse -Archive <DATA.FARC | patch_0105/0106/0110.farc>
  -SampleManifest local-analysis/d83-alpha2-manifest-*.json -Output local-analysis/d83-alpha2-oracle-*.json`
  (5 + 1 + 1 + 1 samples), merged to `d83-alpha2-oracle-merged.json`.
- `diffuse_parity SNAPSHOT GAME-DIRS local-analysis/d83-alpha2-oracle-merged.json
  local-analysis/d83-alpha2-parity.json` → 8 compared, 0 mismatches, 2 projected.
- Regression `diffuse_parity` (01.33 snapshot): d78-oracle-DATA 38/0 mismatches/0
  projected, with `--animation-base` 36 projected (= D78); d58-alpha-search 48/0/47
  (0 two-texture alpha MIX); d55-mix 1/0/1; d57-single-mix 1/0/1.
- `cargo fmt --all --check`, `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`,
  `cargo test -q --workspace` (lbp-viewer diagnostics tests need an existing
  `local-analysis/` dir; pass with it), viewer builds, Python unittest: pass.

## Tests (synthetic)

- `linked::tests::two_texture_alpha_mix_selects_grid_samples_and_rejects_other_shapes`:
  local shape selects with NaN parameter 0 retained, wire order, factor port 0,
  rejects factor from a third sample, `r` swizzle, same sample twice, UV1 vs UV0,
  scale 70, animated sampler, subtype-1 MIX, wired sampler.
- `product::tests::two_texture_alpha_mix_grid_endpoints_factor_port_and_cancel`:
  grid extent, alpha 0/128/255 endpoints, factor port 0/1, bad port, cancellation,
  bad extent and scale budget.

## Not tested

Viewer rendering of the two materials (no native window, no capture); meshes using
them (newly viewable / Complete not measured); 01.00-only oracle d46 (hash not in
the 01.33 snapshot); independent-UV two-texture alpha MIX (6) and the other MIX
shapes (product under alpha MIX, MIX of MULTIPLY with COLOR on non-grid samplers).
