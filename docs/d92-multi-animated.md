# D92 — two or more animated product leaves (animation-base opt-in) (2026-10-07)

Backlog rank 2 ([backlog.md#animated-sampler-product](backlog.md#animated-sampler-product))
and the "THING_COLOR with 2+ animated samplers" part of rank 3. Route code only
(`lbp-preview`); no parser, disk-layout, shader or renderer change. Default
(no opt-in) selection is unchanged.

## Rule

Under the existing animation-base opt-in (`--animation-base`, Bevy checkbox), the D78
product routes now accept **any number of distinct animated texture leaves** (at most
four exist: primary, second, third, fourth). Each animated leaf passes the unchanged
per-leaf D56/D62 rule (`animation_base::sampler`) and uses **its own** base rows; static
leaves keep the D49 sampler rule; a sampler is still tried as static first.

- `DiffuseSelection::animation_base` stays the first animated leaf;
  new `animation_base_more: [Option<AnimationBaseSnapshot>; 3]` holds the others
  (first-use order, a squared sampler counts once). `animation_base::AnimatedLeaves`
  collects them; `verify_global` reconciles every leaf against the global declarations.
- Shape rules are unchanged: an animated *independent* third must still be subtype 0
  (D65 scale/offset), the four chain still needs exact first/fourth and second/third
  coordinate sharing. D78 never attached the snapshot on the four chain (it was refused
  by the 2+ check); it now does.
- THING_COLOR: two or more animated leaves are accepted under a supplied color only when
  the ThingColor (or a D88 folded operand) is the direct tint of the product, i.e. the
  D75 `(T1 × T2) × COLOR` route on the derived graph. Refused with an explicit error when
  the projection folded a nested `T × THING` sampler (nested tint) or has alpha
  subtraction: those rewrites have no oracle parity in `diffuse_parity`, and the loader
  already refuses nested tint with three samples.

Still unsupported: animated MULTIPLY/COLOR nodes; wired or UV2+ animated samplers;
subtype 1 with one named row; animated leaves under MIX/BLEND/ADD; one animated texture ×
static COLOR (not a 2+ product); the nested/alpha-subtract instance forms above.

## Affected materials (01.33 census, b90 baseline)

From `b90-diffuse-route-shapes-0133.json` by script: 44 unsupported with "sampler must be
static" (largest: `T1a × T2a` 12; `(T1a × T2a) × T3` 2; three-animated 1; others MIX,
COLOR, deeper chains, alpha-subtract) and 9 THING_COLOR materials whose projection
stopped at "animation base allows one animated product sampler".

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`, case
  TEXTURE_SAMPLE: each sampler builds its UV from its own `anim`/`anim2` names (`t<name>`
  uniforms) and parameters, independent of other samplers and of the consuming node;
  `t<name>` uniforms are declared once per material parameter animation with its base
  value (shader/bake generation, `parameterAnimations` loop). MULTIPLY multiplies port 0/1
  component-wise. So several animated leaves compose exactly as one does (D78). No
  community code copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: Toolkit oracle (`MaterialOracle --diffuse`) on all 24
  materials that changed class in the first census pass, `diffuse_parity` with
  `--animation-base` and with `--animation-base --instance-white`: 0 mismatches. Parity
  schema 19 checks every animated leaf against its own base rows and the supplied white
  ThingColor tint on the D75 product branch; compatibility census schema 12 adds
  `animation_base_more`.
- `UNKNOWN`: game time, key interpolation, motion, game pixels, filtering, real
  instance colors, corpus asset counts.

## PC verify (01.33, from the main checkout)

| Check | Result |
|---|---|
| `diffuse_route_census ... d92-diffuse-route-shapes-0133b.json` | unsupported 162 -> 144, animation-base-only 106 -> 121, instance-only 244 -> 247; others unchanged (texture 6,953, solid 619, unwired 10) |
| first pass without the nested-tint guard (`d92-diffuse-route-shapes-0133.json`) | 138 / 121 / 253; the 6 extra were nested-tint folds, now refused |
| `run-material-oracle.ps1 -Diffuse` on `d92-manifest-{DATA,patch_0106,_0110,_0120,_0128}.json` | 17 + 1 + 1 + 3 + 2 materials |
| `diffuse_parity ... d92-oracle-* --animation-base` | 24 compared, 0 mismatches, 15 projected (all 15 new animation-base-only) |
| same `--animation-base --instance-white` | 0 mismatches, 18 projected (+3 instance) |
| regress `d78-oracle-DATA.json --animation-base` | 38 / 0 / 36 (baseline 38 / 0 / 36) |
| regress `d88-oracle-*.json --animation-base --instance-white` | 8 / 0 / 8 (baseline unchanged) |
| `corpus_mesh_census --animation-base` (`d92-corpus-animation-base.json`) | all 4,261 mesh GUIDs: 4,257 viewable, all-diffuse 3,721 -> 3,733 (+12), 0 Complete |
| `corpus_mesh_census` default (`d92-corpus-default.json`) | 4,257 / 3,669 / 0, unchanged |
| unit tests, fmt, Clippy `-p lbp-preview`, both viewer builds, `python -m unittest` | pass |

Not tested: game pixels, viewer captures of the new materials, `--instance-rgba`
corpus run, legacy viewer on the dual (non-grid) path (unchanged refusal).

## Tests (synthetic)

`animation_base::tests::d92_each_animated_product_leaf_uses_its_own_base_rows`,
`animation_base::tests::d92_tinted_three_and_four_leaf_products`, the D78 test now
accepting two animated leaves, and `instance::tests::d88_animated_texture_times_thing_needs_both_opt_ins`
(two animated leaves under a direct ThingColor accepted; nested tint refused).
