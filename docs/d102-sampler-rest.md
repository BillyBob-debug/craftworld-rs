# D102 — remaining "sampler must be static" materials (2026-10-07)

Follows D78/D92 ([d78-animated-product.md](d78-animated-product.md),
[d92-multi-animated.md](d92-multi-animated.md)), D87 and D96. Route code only
(`lbp-preview`): `animation_base.rs` (rule + scope helper), one-line hook in
`linked.rs::select_diffuse_with_animation_base`, `instance_textured.rs` reuses the
helper. No parser, disk-layout, shader or renderer change. Default selection unchanged.

## Census first (b99, 01.33, by script)

`b99-diffuse-route-shapes-0133.json`, unsupported rows whose reason is "sampler must be
static, unwired UV0/UV1" (19 locations, 18 distinct hashes), plus the three other
reasons named in the task. Graph details read with a throwaway local probe (not committed).

| Cause | Locations (sha1 prefix) | D102 outcome |
|---|---|---|
| Subtype-1 animated sampler whose matrix rows key only Z (mask 4); direct TEX | 1 (e75c6111) | **animation-base-only** (rule 1) |
| One animated sampler × static COLOR (D78 left out "not a 2+ product") | 1 (b8d53987) | **animation-base-only** (rule 2) |
| Sampler UV channel raw 2 (TEX alone, TEX × TEX) | 4 (a6055e9a ×2, fd4cbc97, affc417f) | unsupported: D89 decision, no new evidence (Toolkit maps every raw ≠ 1 to UV0; game meaning unknown) |
| COLOR alone outside 0..1 (blue −0.0054 ×2, alpha 1.09) | 3 (0dd9da97, 4cec35cd, ca1765bc) | unsupported: Toolkit emits the COLOR unclamped and multiplies diffuse into lighting before any framebuffer clamp; no clamp rule to cite |
| ThingColor nested tint with animated or port-folded samplers | 3 (25ff1aa3, 38ab5674, c36d1d22) | unsupported: D92/D96 nested-tint rules (no oracle parity for those rewrites) |
| Four-texture product, one animated leaf, no coordinate sharing | 3 (291c78cb, b3d53f1f, dc56676e) | unsupported: "four textures need two exact coordinate-sharing pairs" (multiply shapes, D101 area) |
| Constant MIX of COLOR and a tinted product (static samplers) | 2 (cca8853b, ee085058) | unsupported: MIX family (D100 area); generic message comes from the default product search |
| MIX(COLOR, animated T × animated T), weight word 0.61, matrix leaf | 1 (38a4993f) | unsupported: rule 1 now passes both leaves; constant product MIX still needs the integer Repeat grid and a matrix leaf is off it |
| Static product tree under ThingColor | 1 (ef22e36c) | unsupported: D99 "product tree under a supplied instance color" |

Other reasons named in the task:

- `822391c0` "diffuse output must be static and unswizzled": diffuse = `(THING × THING.a).rgb`.
  Toolkit evidence exists (a three-lane diffuse is returned as `float4(v, 1.0)`,
  `setupPath` ≈899–907), but the material is instance-only, needs a new solid-selection
  field, and the white-instance parity cannot tell alpha 1 from `a × a` (both 1). Left
  refused; listed for a later instance milestone.
- `89e9b0b8` "static diffuse color outside 0..1": the ADD addend COLOR has alpha −0.236.
  No clamp evidence (same as the COLOR rows above). Refused.
- `30425124`, `352e3edd` "alpha MIX needs COLOR and one texture sample": MIX shapes,
  D100 area. Not touched.

## Rules (animation-base opt-in only)

1. **Partial matrix-row masks.** `animation_base::sampler` accepted a subtype-1 row only
   with component mask 7 (XYZ). It now accepts any nonempty subset of XYZ (1..=7). The
   transform still uses the whole base row `.xyz`; masks with W or above bit 2 and an
   empty mask stay refused, as do every other D56/D62 check (names, lane-count
   divisibility, key counts, distinct row names, unwired UV0/UV1, bounds,
   `verify_global`).
2. **One animated sample × static COLOR.** `animation_base::product_scope` replaces the
   D78 "2+ textures" check: the product search result is accepted when it has a second
   texture (unchanged) or is exactly one animated texture × one COLOR under the root
   MULTIPLY (`single_tinted_leaf`: no further leaf, nested product, MIX, BLEND, ADD,
   product tree or sampler-scale state). Rendering is the existing single-sample tint
   path that D88 already uses for an instance tint.
3. **Instance color unchanged.** Under a supplied color this shape is still accepted only
   as the D88 form (COLOR is the projected ThingColor or folded operand, no folded nested
   sampler); otherwise "animated sampler × COLOR under a supplied color needs the direct
   instance tint". `linked::select_animated_products` was only used for that D88 second
   pass and is removed (the base selector now covers it).

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`:
  `resolve` TEXTURE_SAMPLE subtype 1 (≈227–252): a named `anim` replaces the X row by
  `t<name>.xyz`, `anim2` the Y row, independent of which lanes are keyed;
  `generateShaderSource` / `generateBakedShaderSource` (≈932–945, 994–1007) declare each
  `t<name>` from the full four-lane `baseValue`. So the base snapshot of a partially keyed
  row is its whole base vector. MULTIPLY (≈328–347) is component-wise, and a sampler's
  UV depends only on its own bindings (D78), so an animated sample × COLOR composes as a
  static one does. COLOR (≈270–285) is emitted as written, no `saturate`; `setupPath`
  (≈899–907) returns a three-lane diffuse as `float4(v, 1.0)`. Restated independently; no
  Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33): Toolkit oracle rows for all 4 materials that changed
  class; `diffuse_parity` checks each animated leaf against the oracle's base rows, key
  counts and static words and the COLOR tint against the oracle's static multiply: 0
  mismatches.
- `UNKNOWN`: game time, key interpolation and motion; game pixels; filtering; whether the
  game GPU clamps out-of-range diffuse COLOR; UV channel raw 2 meaning.

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (selection census, material locations with MAP rows; not asset counts)

`diffuse_route_census` on 01.33, b99 → d102 (`d102-diffuse-route-shapes-0133.json`):
unsupported 102 → 98, animation-base-only 121 → 123, instance-color-only 267 → 269;
texture 6,975, solid 619, unwired 10, no_map_row 65 unchanged. Moved: e75c6111 (rule 1),
b8d53987 (rule 2) to animation-base-only; 5d5b1438, a04005fd (THING × two subtype-1
samplers with Z-only rows, rule 1 under the D92 direct tint) to instance-only.

Corpus mesh census (4,261 mesh GUIDs, distinct catalog GUID, all-diffuse), b99 → d102:
default 3,683 → 3,683; `--animation-base` 3,747 → 3,748 (+1: 000150ed);
`--animation-base --instance-rgba 0.25,0.5,0.75,0.5` 4,010 → 4,011 (+1). Viewable 4,257
in all modes; Complete 0.

Newly viewable / newly Complete assets: +0 / +0 by default (opt-in only; animation-base
snapshots never count as Complete).

## Verification (PC, 2026-10-07)

- `cargo fmt --all --check` pass; `cargo test -q -p lbp-preview` 62 pass (new
  `animation_base::tests::d102_partial_row_masks_and_single_animated_tint`; the D78 and
  two D88 tests now expect the D102 scope); `cargo clippy -q -p lbp-preview --all-targets
  -- -D warnings` pass; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer` pass;
  `python -m unittest discover -s tests` 34 OK (1 skipped).
- `run-material-oracle.ps1 -Diffuse` with `d102-manifest-{DATA,patch_0106}.json` (3/1)
  → `d102-oracle-*.json`; `diffuse_parity` → `d102-parity-*.json`: default 0 projected;
  `--animation-base` 2/0 mismatches (DATA), 0 (patch); `--animation-base
  --instance-white` 3 + 1 projected, 0 mismatches.
- Regression: `d78-oracle-DATA.json --animation-base` 38 / 0 / 36 (baseline 38/0/36);
  `d92-oracle-DATA.json --animation-base --instance-white` 17 / 0 / 14 (baseline
  17/0/14); D88 oracles `--animation-base --instance-white` 8 / 0 / 8 (unchanged).
- `corpus_mesh_census` → `d102-corpus-{default,animation-base,both}.json`.

## Not tested

No viewer run or capture (no native windows); the moved materials use the existing
matrix / scale-offset single-sample and tint paths. Game pixels, real instance colors,
01.00-only data and the refused subgroups above were not examined further.
