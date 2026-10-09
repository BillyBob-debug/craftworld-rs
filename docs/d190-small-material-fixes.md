# D190 small material blockers under white instance colour + animation base (2026-10-09)

Input: the b176 PC run (01.33, disc + update; `corpus_mesh_census --blockers
--instance-rgba 1,1,1,1 --animation-base`, baseline **4,155** all-diffuse distinct mesh
GUIDs of 4,261, 4,259 viewable). Six small blocker groups were investigated from the
recorded reasons, the route-census shape signatures and existing docs only; no game
data was available in this environment. Mixed-UV BLEND/MIX graphs, preview defaults
and material-less primitives are handled elsewhere and are untouched here.

Shape signatures use `shape()` in `crates/lbp-preview/examples/diffuse_route_census.rs`:
node 0 is the box feeding the output's diffuse port, `type.subtype+named`, edges
`from>to:port[swizzle]`. The output box and the output wire itself are not shown.

Toolkit pin for every community-code reference below:
`d6a2ef80b5a09999f08f23a803b516096055e5c9`, file
`lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java` unless stated. Restated
independently; no Toolkit code is copied, adapted or translated.

## Decisions

| # | Material (mesh GUIDs blocked) | Reason before | Decision |
|---|---|---|---|
| 1 | g11161 `stone_lantern.gmat` (5) | texture product requires present unflagged Repeat texture | **fixed if wrap was the cause** (rule A) |
| 2 | g21845 `proximity_switch_colour_light.gmat` (4) | solid diffuse needs an unwired unanimated subtype-zero COLOR | **fixed if the output wire carries an RGB-identity swizzle** (rule B) |
| 3 | g3732 `badge_zip_entrance_cloth.gmat` (2) | BORDER requires nearest same-sample alpha MIX and validated interior UVs | **refused** |
| 4 | g618859 `mesh_football_goal_net.gmat` (2) | static diffuse color outside finite 0..1 preview bounds | **fixed** (rule C) |
| 5 | g204837 `muppets3_animal_to_shirt.gmat` (1) | texture product requires Repeat on both axes | **fixed** (rule A) |
| 6 | 2 meshes `Invalid("nonfinite mesh UV")`, 1 "derived UV exceeds finite preview bounds" | — | **refused** |

### Rule A — three/four-texture products keep per-texture wraps (cases 1, 5)

Before: when a third sample was present, `load_material` (`crates/lbp-preview/src/linked.rs`)
required the primary and second textures to be Repeat/Repeat ("texture product requires
Repeat on both axes" for the primary; the second's failure shared the "present unflagged
Repeat texture" message with absent/flagged descriptors). The check dates from the D60
baseline, before D82 wrap modes reached the independent dual path.

Now: the primary/second Repeat requirement for three/four-texture products is removed.
Absent (`kind == 0`) or flagged second descriptors are still refused with the same
message. Shared third/fourth textures still require present, unflagged Repeat textures
(`load_shared_sample`), because the Bevy viewer draws them with a Repeat sampler.

Why this is not a guess:
- `uses_grid` is false whenever a third sample exists, so these products never take the
  Repeat-periodic LCM grid bake; they always use the dual path, where the primary and
  second images get their own per-axis samplers (`adapter::image_axes` with `Asset::wrap`
  and `DualAsset::wrap`), exactly as two-texture dual products already do (D50/D99/D160).
  The new unit assertion in the D78 three-texture test pins this invariant.
- Wrap values per slot: `CONFIRMED_FROM_COMMUNITY_CODE`, `enums/TextureWrap.java` and
  `RGfxMaterial::serialize` wrapS/wrapT (docs/d82-wrap-modes.md); the D82 display rules
  are reused unchanged.
- Each TEXTURE_SAMPLE has its own transform (lines 197–208) and MULTIPLY is a
  componentwise product (lines 328–347): `CONFIRMED_FROM_COMMUNITY_CODE`. The third
  sample's relative coordinates (D65) are derived from the transformed, unwrapped base
  coordinates, which wrap mode does not change.
- Game filtering/rounding: `UNKNOWN` (as D65).

Applicability: depends on which clause refused case 1. D97 measured descriptor flags 0
on every material texture reference in 01.33 (`CONFIRMED_FROM_LOCAL_DATA`), so the
g11161 refusal was either a non-Repeat second texture in a three-texture product (now
lifted) or an absent second slot (still refused, same reason string). Case 5's reason is
specific to the primary wrap and is lifted. A now-loaded product can still meet the
shared-third Repeat rule or BORDER rule (case 3) with a new reason.

### Rule B — RGB-identity swizzle on the diffuse wire for solids (case 2)

Shape `10.0+0,2.0+0 / 1>0:0,1>0:1a` is `THING × THING.a`. Under a supplied colour the
D86 fold turns the MULTIPLY into one derived COLOR (white: (1,1,1,1)), but the solid
selector then rejected the output wire; D86 recorded this material as "blocked by the
output wire". `select_solid_diffuse` (`crates/lbp-preview/src/solid.rs`) now also accepts
a diffuse-wire swizzle of exactly `xyz`, `rgb`, `xyzw` or `rgba` (NUL-padded); every other
swizzle, a nonzero source port and an animated/subtyped output stay refused with the old
reason. The solid label names the swizzle; `SolidSelection::output_swizzle` records it.

Evidence (`CONFIRMED_FROM_COMMUNITY_CODE`): `setupPath` (line 844, resolve at 876)
resolves the output's source box with the wire's source port and does not apply the
wire swizzle; `getWithSwizzle` (lines 84–96), used for all other wires, appends the
swizzle and narrows the width, after which a 3-wide value returns as `float4(v, 1.0)`.
For the four accepted swizzles both readings give the source RGB unchanged and differ
only in alpha, which the solid preview does not display (Bevy draws the solid with
alpha 1). Which reading the game uses: `UNKNOWN`. The route stays "supplied" (never
Complete), as all D86 folds.

Applicability: whether g21845's output wire really carries one of these swizzles is
**not verified here** (the shape signature omits the output wire). If its blocker is a
source port, another swizzle or output animation, the PC run will show the same reason
and gain 0.

### Rule C — ADD addend outside 0..1 (case 4)

Shape `11.0+0,3.0+0,1.1+0 / 1>0:1,2>0:0` is D89 rule 3 `ADD(matrix TEX, COLOR)`; D89
recorded the COLOR alpha as −0.236. The ADD bake already computes
`linear(sample) × tint + addend` (alpha `a × tint.a + addend.a`) in f64 and clamps each
channel to 0..1 at 8-bit encoding, so an out-of-range addend has a defined preview value.
`crates/lbp-preview/src/add.rs` now requires the addend only to be finite ("ADD COLOR
addend must be finite" otherwise); the MULTIPLY tint inside an ADD keeps the 0..1 bound.
When the addend is outside 0..1 the label adds "preview approximation (not game
behaviour): addend outside 0..1, sum clamped at encoding".

Evidence: ADD is `l + r` of the two swizzled inputs (lines 349–386,
`CONFIRMED_FROM_COMMUNITY_CODE`); COLOR is its four words as float4 (270–285). Clamping
and linear-space summation are the existing D89 preview policy (`STRONG_INFERENCE` at
best); game blending of an out-of-range sum (e.g. negative alpha before alpha test or
blending) is `UNKNOWN`. The subtype-1 sampler is static (no named animation) and uses
the existing matrix transform route.

### Case 3 — BORDER (refused)

`crate::border_domain::addressing` refuses raw wrap 4 (BORDER) outside the nearest
same-sample alpha MIX route. Toolkit names value 4 BORDER but no reader in the pin
defines a border colour (D82, `CONFIRMED_FROM_COMMUNITY_CODE` for the name only), and
the route census shows g3732 takes the D103 off-grid bake on a common [2, 2] UV tile,
i.e. its sampling coordinates span more than one period, so border texels would be
reached. Showing them would require inventing a border colour. Stays refused
(`UNKNOWN` border colour), reason unchanged.

### Case 6 — nonfinite and oversized UVs (refused)

`Invalid("nonfinite mesh UV")` comes from `lbp-mesh` `decode_attributes`, which refuses
the whole UV block when any channel/vertex holds NaN/inf. Whether those vertices are
referenced by triangles, or the channel is used by the material, is not known from the
recorded reasons; substituting a value would be a guess, and splitting the refusal per
channel changes mesh-crate output for every mesh. "derived UV exceeds finite preview
bounds" (`affine.rs`, |uv| > 1e6 after a non-identity transform) is an authored
precision budget: f32 spacing at 1e6 is 0.0625, so texture lookups would be
meaningless. Both stay refused. Needed evidence: per-channel/per-vertex dump of the two
meshes (channel, vertex indices, triangle references) and the transform of the third.

## Tests (synthetic only)

- `add.rs`: addend alpha −0.236 selects and is labelled; NaN addend and a 1.5 tint are
  refused; pixel bake with a negative addend alpha clamps (255 → 195, 40 → 0).
- `instance.rs` `d190_rgb_identity_output_swizzle_reaches_the_solid_route`: the g21845
  shape with `rgb`/`xyz`/`rgba`/`xyzw` folds to white solids under the white projection;
  `a`, `bgr`, `rg`, `xyzx`, `rgbaa` stay refused; unswizzled output keeps zero metadata.
- `linked.rs` D78 three-texture test: `uses_grid` and `grid_with_wrapping` are false for
  a three-texture selection (the invariant rule A relies on).

Commands run here: `cargo fmt --all --check`, `cargo clippy -q -p lbp-preview
--all-targets -- -D warnings`, `cargo test -q -p lbp-preview`,
`cargo build -q -p lbp-bevy-viewer -p lbp-viewer` (passed; built with `CARGO_PROFILE_DEV_DEBUG=0` after the first attempt hit a full container disk). All passed.

## PC verification (not run here)

```text
cargo run --release -p lbp-preview --example corpus_mesh_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d190-corpus-white-anim.json --blockers --instance-rgba 1,1,1,1 --animation-base
```

Compare all-diffuse distinct mesh GUIDs with the 4,155 baseline (b176-blockers-white-anim.json).
Expected at most +12 (5 + 4 + 2 + 1, sole-blocked counts); certain only for case 4's
selection rule and case 5's wrap clause. Check the g11161, g21845, g618859 and g204837
keys: a remaining reason names the next blocker. Also re-run the default census
(no flags) to confirm unaffected output; case 2 needs the instance opt-in, cases 1, 4
and 5 also move in the default run if they were sole-blocked there.

Not tested: real game data, visual inspection in the viewer, Toolkit oracle parity for
the changed shapes, the legacy viewer (it refuses independent-sampling products).
