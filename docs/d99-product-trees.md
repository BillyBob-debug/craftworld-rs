# D99 — static product trees and off-grid two-texture alpha MIX (2026-10-07)

Backlog [rank 6 "deeper products"](backlog.md#small) and [rank 4 MIX](backlog.md#mix)
("6 need per-sample UVs"). Route code in `lbp-preview` (new `product_tree.rs`, small
hooks in `linked.rs`, `instance_textured.rs`), a parity branch in `diffuse_parity`
(schema 21) and one Bevy shader mode. No parser or disk-layout change. BLEND (D97) and
UV channel raw 2 are out of scope and untouched.

## Census first (b96 baseline, 138 unsupported)

`b96-diffuse-route-shapes-0133.json` grouped by script after dumping the diffuse
ancestry of all 138 graphs (scratch tool, not committed): 52 contain BLEND (excluded),
4 use UV channel raw 2 (excluded). Of the remaining 82 the two largest families with
a render path were:

| Family | Locations | Render path |
|---|---|---|
| MULTIPLY-only trees of TEX/COLOR leaves the fixed shapes refuse (`(T×C)×C`, `(T×C)×(T×C)`, `((T×C)×T)×C`, `(T×T)×(T×C)`, `T×(T×T')` squares, balanced four-leaf, 5-leaf) | 24 static + 4 with animated leaves (16 now select) | existing single / two-texture / third / four-chain roles |
| Two-texture alpha MIX off the integer Repeat grid (UV0/UV1 pairs ×4, UV1 base + UV0 ×70 overlay ×2) | 6 | Bevy dual path with a new mix mode |

## Rule 1 — product trees (default route; also under the opt-ins)

Runs only after the fixed product routes returned an error (`select_products` →
`select_fixed_products`, then `product_tree::select`); already-selected materials are
unchanged.

1. The diffuse wire (unswizzled, port 0) reaches a MULTIPLY. Every MULTIPLY in the tree
   is subtype 0, unanimated, with exactly two inputs on ports 0 and 1, source port 0,
   no swizzle. Leaves are texture samples (box 1) or COLOR (box 3, unanimated, unwired).
   Any other box keeps the fixed route's reason. A box reached twice counts twice
   (squared). Walk budget: 64 visits, depth 8.
2. COLOR leaves fold into one tint: component-wise f32 product from 1 in ascending box
   index order (with multiplicity); the result must be finite and within 0..1.
3. 1–4 texture leaves (with multiplicity), each through the existing sampler rule
   (static, or with the animation-base opt-in its own base rows):
   - 1: texture × tint; 2: the two-texture product (grid or dual) × tint;
   - 3: the D65/D67 third role: an exactly shared pair takes second/third (identity
     relative sampling; a squared leaf always shares); otherwise every leaf must be a
     nonzero scale/offset and some order must give the third a base on its channel;
   - 4: the D60 chain: two exact coordinate-sharing pairs (first/fourth, second/third).
4. `DiffuseSelection.product_tree` (`ProductTree { root, multiply_nodes, texture_leaves,
   color_leaves }`), `multiply_index = root`, `color_index = None`. The loader applies
   the tint to the primary pixels (or the baked grid), as for D75.
5. Refused under a supplied instance color (`select_instance_diffuse`: "product tree
   under a supplied instance color"): no oracle parity for projected ThingColor leaves.

## Rule 2 — off-grid two-texture alpha MIX (default route, Bevy)

D83 shape `MIX(T0, T1, Tk.a)`. When the samplers are not on the same-channel integer
grid (or not both Repeat), the selection now succeeds if the factor is the **port-1**
sample alpha (k = 1, all 6 local materials); a port-0 factor off the grid is refused
("needs the factor on the port-1 sample"; the primary alpha is not available after
Bevy's opaque base-color path). The loader builds the existing `DualAsset` and the Bevy
shader mixes instead of multiplying when `enabled.w = 1`:
`color = mix(T0, T1, T1.a)` on linear values (same as the D83 bake's linear RGB mix),
linear filtering. The legacy viewer keeps its dual-path refusal.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`: the `LOOKUP`
  cache (lines 140–141) resolves a box once, so a box wired twice is the same value
  used twice; COLOR (270–286) is the four static parameter words; MULTIPLY (328–347)
  emits `(l * r)` component-wise for diffuse (the `- 0.5` form is BUMP only); MIX
  (388–410) emits `((r - l) * f) + l` with a wired port 2 overriding parameter 0.
  Hence any static MULTIPLY tree equals the product of its leaves, and COLOR leaves
  commute into one tint. Restated independently; no Toolkit code copied, adapted or
  translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + updates 01.01..01.33): Toolkit oracle
  (`MaterialOracle --diffuse`) on all 27 materials that changed class;
  `diffuse_parity` walks the oracle's MULTIPLY rows independently, compares the texture
  leaf multiset with the selected roles, recomputes the tint from the oracle COLOR rows
  and checks every leaf's slot/channel/transform: 0 mismatches.
- `UNKNOWN`: game pixels and filtering; f32 rounding order on the game GPU (the fold
  differs from per-pixel evaluation by rounding only; Toolkit prints COLOR with `%f`,
  parity uses parameter bits); real instance colors.

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (unit = material location with a MAP row; selection only)

`diffuse_route_census` vs `b96-diffuse-route-shapes-0133.json`: texture route
6,953 → 6,975 (+22: 16 product trees, 6 alpha MIX); instance-only 253 → 258 (+5:
THING alpha MIX whose D96 sampler-port folds now meet the off-grid rule); unsupported
138 → 111; solid 619, unwired 10, animation-base-only 121, no MAP row 65 unchanged.
Reason changed only (still unsupported, 7): four texture leaves without two sharing
pairs (5), five texture leaves (1), COLOR-only triple product (1).

Corpus mesh census (all 4,261 mesh GUIDs, distinct catalog GUID, all-diffuse = every
primitive's diffuse loads): default 3,669 → 3,683 (+14); `--animation-base` 3,733 →
3,747 (+14); `--instance-rgba 0.25,0.5,0.75,0.5` 3,902 → 3,921 (+19); both 3,983 →
4,002 (+19). Viewable 4,257 unchanged; Complete 0 (not certified: no visual check).
Newly viewable +0; newly Complete +0.

Left unsupported outside BLEND (59): ThingColor shapes 28 (multiply 14, ADD 9, sampler
4, swizzled output 1); sampler rule 15 (UV2 4, animated leaves in four-leaf chains /
MIX, out-of-range COLOR roots); four leaves without sharing pairs 5; ADD of a texture
and a product 3; MIX of COLOR and products 3; 5-leaf product 1; COLOR-only product 1;
others 3 (two non-tree multiplies, one COLOR root outside 0..1). One animated
texture × static COLOR (inside the sampler 15) stays outside D78's 2+ scope.

## PC verify (from the main checkout; outputs ignored)

| Check | Result |
|---|---|
| `diffuse_route_census ... local-analysis/d99-diffuse-route-shapes-0133.json` | counts above (`-pass1` identical before the D65 guard) |
| `run-material-oracle.ps1 -Diffuse` on `d99-manifest-{DATA,patch_0106,patch_0110,patch_0127}.json` | 24 + 1 + 1 + 1 materials, merged `d99-oracle-merged.json` |
| `diffuse_parity ... d99-oracle-merged.json d99-parity-default.json` | 27 compared, 0 mismatches, 22 projected |
| same `--instance-white` (`d99-parity-instance-white.json`) | 27 / 0 / 27 |
| regressions `d99-regress-*` vs main build `*-base` | identical except d83-alpha2-merged 8/0/2 → 8/0/8; d75-DATA-1 48/0/41, -2 48/0/39, d78 anim 38/0/36, d58 48/0/47, d64 17/0/17, d67 18/0/18, d87-2 18/0/18, d89-0 5/0/2, d88/d92 both 4/0/4, 17/0/14, d96 white 4/0/3 |
| `corpus_mesh_census` default / `--animation-base` / `--instance-rgba` / both | `d99-corpus-{default,anim,inst,both}.json`, counts above |
| `cargo fmt --all --check`, `cargo test -q --workspace`, `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`, `cargo clippy -q -p lbp-bevy-viewer -- -D warnings`, both viewer builds, `python -m unittest discover -s tests` | pass |

## Tests (synthetic)

`product_tree::tests::{d99_single_texture_under_nested_colors_folds_the_tint,
d99_two_and_three_texture_trees_take_existing_roles,
d99_four_texture_trees_need_two_sharing_pairs, d99_fold_is_an_ascending_f32_product}`;
updated `linked` tests (squared three-leaf and duplicate four-leaf trees, `(T×C)×C`,
off-grid alpha MIX port 1 selects / port 0 refused, matrix animated leaf as primary),
`instance::tests::d88_constant_thing_operand_folds_into_texture_tint` (product tree
refused under a supplied color).

## Not tested

Viewer rendering of the new materials (no native window, no capture), so the Bevy mix
mode is compiled but not visually checked; legacy viewer on the new routes (dual
refusal unchanged, single-texture trees use its existing tint path); game pixels;
product trees under a supplied instance color (refused); THING_COLOR trees.
