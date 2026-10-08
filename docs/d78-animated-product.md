# D78 — animation-base opt-in for 2+ texture products (2026-10-07)

Backlog rank 2 ([backlog.md#animated-sampler-product](backlog.md#animated-sampler-product),
65 materials on 01.33). Route code only (`lbp-preview`); no parser, disk-layout,
shader or viewer change. Default (static-only) selection is unchanged.

## Rule

With the existing animation-base opt-in (`--animation-base`, Bevy checkbox), the
MULTIPLY product routes now accept **one** distinct animated texture leaf:

| Route | Before (opt-in) | Now (opt-in) |
|---|---|---|
| `T1 × T2` (D49), squared `T × T` (D64) | sampler rule | product, one leaf uses its base rows |
| `(T1 × T2) × COLOR`, `(T × COLOR) × T2` (D75) | sampler rule | tinted product, same |
| `(T1 × T2) × T3` (D65/D67) | sampler rule | same; an animated *independent* third must be subtype 0 (scale/offset) |
| `((A × B) × C) × D` (D53 four chain) | sampler rule | same; exact coordinate sharing still required |

The animated leaf passes the unchanged D56/D62 rule (`animation_base::sampler`,
extracted from the direct selector): unwired UV0/UV1 sampler; subtype 1 with two
distinct named XYZ curves (rows 0/1), or subtype 0 with one named masked curve and
an empty second binding (scale `(x, y)`, offset `(-(z·x), -(w·y))`). Static leaves
keep the D49 sampler rule. A sampler is tried as static first. The snapshot now
records `sample_index`, so `verify_global` reconciles the animated leaf (any role)
against the global declarations; the UI still excludes snapshots from Complete.

Still unsupported (explicit errors): two or more distinct animated leaves (since D92
supported, see [d92-multi-animated.md](d92-multi-animated.md)); animated
MULTIPLY/COLOR nodes; wired or UV2+ animated samplers; subtype 1 with only one named
row (Toolkit applies it per row, existing selector requires both); animated leaves
under MIX/BLEND/THING_COLOR; a single animated texture × COLOR (not a 2+ product).

Rendering reuses existing paths: the selection already carries per-leaf transforms.
A nonzero base offset or non-integer scale leaves the Repeat LCM grid, so both
samplers stay independent (D50 dual path); the legacy viewer keeps rejecting that.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  TEXTURE_SAMPLE (lines 150–266) builds each sampler's UV from that box's own
  `anim`/`anim2` bindings and parameters only, independent of the consuming node;
  MULTIPLY (lines 328–347) multiplies its port 0/1 values component-wise. So an
  animated leaf composes with a product exactly as a static leaf does. No community
  code copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: not yet; needs the PC run below. `diffuse_parity`
  now checks the animated leaf in any role against its base rows (node subtype,
  static bits, base rows, key counts, derived transform) and every static leaf
  against its parameters. Parity schema 13 and compatibility census schema 10 add
  `animation_base.sample_index`.
- `UNKNOWN`: game time, key interpolation, motion, game pixels, filtering, corpus
  asset counts. Same limits as docs/animation-base-preview.md.

## Tests (synthetic, Linux)

`linked::tests::animation_base_opt_in_selects_one_animated_leaf_of_a_static_product`:
animated primary and second leaf, derived scale/offset bits, dual (non-grid) path,
local/global reconciliation by snapshot index, squared animated sample, D75 tint,
animated independent third (subtype 0) and rejection of subtype 1 there, and
rejections: two animated leaves, animated multiply/COLOR, wired/UV2 sampler, uneven
curve, single animated texture × COLOR. Static selection unchanged without opt-in.
Existing animation-base and route tests unchanged and passing.

## PC verify

Exact commands and the expected count change are in the PR body.
