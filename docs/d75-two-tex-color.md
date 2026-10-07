# D75 — two textures × static COLOR under two multiplies (2026-10-07)

Backlog rank 1 ([backlog.md#two-tex-color](backlog.md#two-tex-color), 113 materials
on 01.33). Route code only (`lbp-preview`); no parser, disk-layout or shader change.

## Rule

`select_diffuse` now accepts exactly two static MULTIPLY nodes over two texture
samples and one static COLOR, in either nesting:

| Shape | Before | Now |
|---|---|---|
| `(T1 × T2) × COLOR` (either outer order) | "diffuse multiply requires two direct textures or one texture and static COLOR" (multiply rule; backlog signatures 23 / 5) | two-texture product + tint |
| `(T1 × COLOR) × T2` (any port order) | COLOR passed to the sampler check: "sampler must be static, unwired UV0/UV1" (sampler rule; the mislabel noted in D67) | two-texture product + tint |

Both reduce to the existing D49 two-texture selection (`sample_index`,
`second_sample`) plus the existing D46 static tint (`tint`, `color_index`);
`inner_multiply_index` records the inner multiply. One sample on both product
inputs is that sample squared (D64 rule) and is tinted the same way.
Every selected multiply is subtype 0, unanimated, two incoming edges on unique
ports 0/1, source port 0, no swizzle. COLOR must be unanimated, unwired and finite.
Samplers keep the unchanged D49 rule: static, unwired, UV0/UV1, known subtype.

Rendering reuses existing paths. Same-channel integer Repeat scales bake the
nearest grid product (D49) and the tint is then applied to the baked product in
linear RGB, alpha multiplied (D46 table). Otherwise the primary texture is tinted
and the second is sampled independently (D50 dual path), as the instance nested
tint (D61) already does. The legacy viewer keeps rejecting dual sampling.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve, MULTIPLY non-BUMP and TEXTURE_SAMPLE/COLOR cases](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  MULTIPLY multiplies its port 0/1 values component-wise; COLOR is a constant
  float4. A component-wise product is commutative and associative, so both shapes
  equal `T1 × T2 × COLOR`. Box/port enums as cited in tinted-preview.md. No
  community code copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: not yet. Needs the PC oracle/parity run below;
  `diffuse_parity` gains a D75 branch that checks root/inner/leaf/COLOR/tint/second
  sampler fields against the Toolkit graph records.
- `UNKNOWN`: float rounding order (product baked to 8 bits, then tinted), game
  pixels, filtering, corpus asset counts.

Not taken (no evidence beyond the existing rule): animated samplers (backlog rank 2,
animation-base opt-in is single-sampler only), UV2+ channels, wired samplers,
deeper nesting, a third texture with COLOR. Those still report the sampler or
multiply rule.

## Tests (synthetic, Linux)

`linked::tests::two_textures_and_static_color_under_two_multiplies_select_a_tinted_product`:
both nestings, both port orders, squared sample, grid vs dual, ignored counts,
and rejection of animated/wired/NaN COLOR, swizzled inner edge, animated or UV2
samplers, wired sampler inputs, an extra inner input, a COLOR-only product and a
cycle through the root. Existing route tests unchanged and passing.

## PC verify

See the PR body for exact commands and the expected change (unsupported 386 →
between 273 and 358; texture route 6,828 up by the same amount).
