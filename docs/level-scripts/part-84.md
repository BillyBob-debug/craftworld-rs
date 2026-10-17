# Level scripts, part 84: the Story globe's jelly on the glass route (2026-10-16, D337)

Part 83 left the Story globe's `proc_victoria_jelly_glassy` (gmat 108277, 0x1a6f5) out:
its program is the glass family of `window_glassy` (D286), which the viewer drew for that
one GUID only.

### What was found

- FOUND (its decoded program, variant 0): `out = fog(sqrt(4 light^2 d0 d1)) a + 6 ta4
  [(pow(R.L, CosinePower) + sat(N.V + rim_round)^2) light^2 + 0.3 refl luma(light^2)]`: the
  lit colour times the alpha, then the x6 specular added, as the window. Its rim term is in
  full where the window's is halved; ta4, the static gloss colour on port 2, is (1.446,
  1.054, 1.274).
- FOUND: lbp-preview dropped the whole surface (bump, gloss, alpha test) of any material
  whose static gloss colour passes 1: the gloss shared the diffuse tint's 0..1 check. The
  jelly, `eye.gmat` (1.809) and 0x43f0 (1.455) in this frame were such materials.
- FOUND: the jelly is only a thin part of the globe. The black continents are two overlay
  layers, `cloth_earth_land_ao` (0x2382f) and `cloth_earth_sea_ao` (0x2382e): COLOR black on
  port 0 and texture x 0.8 on port 1 (alpha), drawn by the game as AO shading over the copper.
  The viewer's solid route reads port 0 only, so they cover the copper opaque. Next (D338).

### What changed

- Viewer: `glass_route` puts a material on the glass route by GUID: `window_glassy` at
  opacity 0.2 with half its rim (as before), the jelly at opacity 1 (INFERRED: its alpha is
  the diffuse textures' alpha times the tint's, not read per pixel) with its rim in full.
  The glass's rim weight is the shader's `glass.w`.
- lbp-preview: a static gloss colour is kept when finite and not negative, whatever its
  size; otherwise the gloss alone is left out with a note, and the rest of the surface is read.
- Still left out: the jelly's reflection term (the second diffuse sample takes graph slot 0).

### Viewer run (part 83's flags, `--lbp-lighting`, `--level-scripts-hold-at 760`)

The jelly's edges take a copper sheen; the continents stay black until the AO layers blend.
Captures stay in the ignored `local-analysis/d337-copper/`.

### Verification

`d337_the_jelly_takes_the_glass_route` (viewer), `d337_a_gloss_colour_above_one_is_kept`
(lbp-preview). `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D
warnings`; viewer and lbp-preview tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
