# Level scripts, part 85: the Story globe's AO overlays blended (2026-10-16, D338)

Part 84 found the Story globe's black continents are two overlay materials drawn opaque.

### What was found

- FOUND: `cloth_earth_land_ao` (0x2382f) and `cloth_earth_sea_ao` (0x2382e) share one graph:
  COLOR (0, 0, 0, 1) on port 0, and a texture (slot 0, UV word 1) times a scalar 0.8 on
  port 1 (alpha). Their program writes the alpha as the port's x times the tint's alpha
  (`MUL H0.w, H1.xxxx, tintcol`): black shading over the copper beneath.
- FOUND: lbp-preview's solid route read port 0 only, so both drew as opaque black. No route
  read port 1, and the material's alpha mode is not used for drawing.

### What changed

- lbp-preview: `load_material` loads a solid material's port 1, when wired, as a diffuse
  (through `rewire_output`, as the glow does) and turns it into an overlay (`alpha_overlay`):
  the display pixels are the solid colour with the port's red as alpha, marked
  `Asset::overlay_alpha`. A port 1 that does not load leaves the solid as it was.
- Viewer: an overlay bucket blends as a decal does (alpha blended, depth bias 8).
- INFERRED: the port's red stands for its x; the tint's alpha is taken as 1.

### Viewer run (part 83's flags, `--lbp-lighting`, `--level-scripts-hold-at 760`)

The continents show copper with dark shading at their coasts and folds, the seas pale.
Captures stay in the ignored `local-analysis/d337-copper/`.

### Verification

`d338_the_overlay_takes_the_alpha_ports_red` (lbp-preview). `cargo fmt --all --check`;
`cargo clippy --workspace --all-targets -- -D warnings`; viewer and lbp-preview tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
