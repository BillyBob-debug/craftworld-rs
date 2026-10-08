# D100 — bounded evaluator for static diffuse graphs (2026-10-07)

Follows the per-shape bakes of [d83-mix-shapes.md](d83-mix-shapes.md) (alpha MIX on the
grid), [d89-add-uv2.md](d89-add-uv2.md) (ADD), [d97-blend-mask.md](d97-blend-mask.md)
(BLEND masks) and [d99-product-trees.md](d99-product-trees.md) (MULTIPLY trees).
Backlog: products under BLEND inputs, three-sample BLEND, ADD of a texture and a product.

Route code only, in `lbp-preview`: new module `crates/lbp-preview/src/grid_graph.rs`;
hooks in `linked.rs` (one `DiffuseSelection` field, one `or_else` after the D99 tree, a
loader early return, label and nearest-sampling lines, `selected_uv` made `pub(crate)`),
`instance_textured.rs` (instance guard), `lib.rs` (exports); `diffuse_parity` schema 23.
No parser, disk-layout, shader or viewer change.

## Rule

The evaluator runs **only after** every fixed route and the D99 product tree refused a
graph (`select_products`: fixed → `product_tree::select` → `grid_graph::select`), so
already-selected materials keep byte-identical selections and bakes. It never runs in
the animation-base pass (static graphs only).

1. **Root.** One output box, one unswizzled source-port-0 wire on diffuse port 0, static
   output. The root's value must be four-component.
2. **Compile** (`grid_graph_program`, public for parity): walk from the root with a cache
   per box (a box wired twice is one value), cycle check, budgets **32 distinct boxes,
   depth 12, 6 texture sample boxes**, graph bounds 1024 boxes / 4096 wires with checked
   endpoints. Every input wire has source port 0 and is unswizzled or has a one-letter
   swizzle (`r/x g/y b/z a/w`, broadcast to four lanes); anything else is refused.
3. **Node rules** (anything else, including THING_COLOR on the default route, is refused
   and the fixed route's reason is kept):

   | Box | Rule | Width |
   |---|---|---|
   | TEXTURE_SAMPLE (1) | existing static sampler rule (`linked::sampler`: unwired, unanimated, UV0/UV1, subtype 0/1); texel RGB sRGB-decoded, alpha linear | 4 |
   | COLOR (3), CONSTANT4 (7) | static, unwired, four words each finite in 0..1 | 4 |
   | CONSTANT (4) | static, unwired, one word finite in −16..16, broadcast | 1 |
   | MULTIPLY (10) | static subtype 0, inputs exactly on ports 0/1; `l × r` per lane | max |
   | ADD (11) | same ports; `l + r` per lane | max |
   | MIX (12) | ports 0/1, optional port 2; `(r − l) × f + l`, `f` = port 2 (per lane) or parameter word 0 (finite, −16..16) | max |
   | BLEND (16) | ports 0/1/2; D94 thresholds (`blend_thresholds`); `m` = first lane of port 2; `l + (r − l) × clamp((m − p0)/(p1 − p0), 0, 1)` | max |

4. **Sources.** Distinct (slot, UV channel, transform) of the samples.
   - One source: evaluated **per texel** of that texture (any transform the sampler rule
     accepts; display uses the texture's own sampling coordinates and wrap, like D84).
   - Two or more: the **D49 nearest Repeat grid** — same UV channel, integer scales
     1..16, zero offset (`product::scales`); grid = LCM per axis ≤ 4096, ≤ 4 Mi texels;
     every texture Repeat on both axes (checked at load). Otherwise the fixed reason is
     kept with ` (graph evaluator: requires same-channel integer Repeat grid samplers)`
     appended.
5. **Bake.** f64 per lane, RGB in linear space, alpha linear; output sRGB-encoded RGB and
   linear alpha, each clamped to 0..1 at 8-bit encoding; a non-finite value is an error;
   cancellation every 4,096 texels. Displayed with nearest sampling. This is the same
   authored policy as the D49/D83/D89/D97 bakes; the unit test checks the evaluator gives
   the D49 product RGB, the D83 alpha-MIX bytes and the D97 two-texture BLEND bytes.
6. **Selection.** `DiffuseSelection.grid_graph = GridGraph { root, nodes, samples,
   sources }`; `sample_index`/`slot`/UV fields from the first sample in evaluation order;
   all other route fields empty. The loader re-derives the program from the same graph.
7. **Supplied instance colour (opt-in only).** ThingColor is evaluated only on the graph
   derived by `project_instance_color` (ThingColor → COLOR with the supplied words), and
   only when the projection made no other rewrite (`instance_supported`: no nested tint,
   folded sampler/root/operand, swizzled colour edge, sampler-port fold or UV alpha
   subtraction). Otherwise "graph evaluator under a folded or rewritten instance input".

Label: `Diffuse only • evaluated graph: N nodes, S texture samples, K sources • UVc
nearest Repeat grid [w, h]` (or the transform, wrap and `per-texel, nearest`).

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java` (read only; restated
  independently, nothing copied, adapted or translated): `resolve` caches a box once
  (`LOOKUP`, lines 140–141); `getWithSwizzle` (83–96) appends the wire's swizzle to the
  input value; COLOR (270–286) emits the four static words; CONSTANT (287–289) one float,
  CONSTANT4 (303–310) four; MULTIPLY (328–347) `l * r` for non-BUMP ports; ADD (349–386)
  `l + r`; MIX (388–410) `((r - l) * f) + l` with a wired port 2 overriding word 0; BLEND
  (438–456) `(in2 - in1) * saturate(in3.x * k - c) + in1`; result width = widest input;
  THING_COLOR (268–269) is the instance colour. These are the same rules already cited
  in D83/D89/D97/D99; D100 composes them.
- `STRONG_INFERENCE`: a one-letter swizzle yields a scalar that the Cg-style generated
  expression promotes to all lanes (scalar promotion of the target language, not
  observed on hardware); BLEND thresholds as D94 (`STRONG_INFERENCE` there); mask lanes
  sRGB-decoded like any colour sample (D97 R8 — the evaluator decodes every sample's
  RGB, so red/green/blue masks are decoded and alpha is linear).
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + updates 01.01..01.33): Toolkit oracle on all
  36 materials that changed class; `diffuse_parity` (schema 23, `graph_equal`) walks the
  oracle's rows independently from the diffuse root (box types, port sets, source ports,
  swizzle lanes, COLOR/CONSTANT/MIX-weight/BLEND-threshold words, sampler slot/channel/
  subtype/transform; ThingColor = white under `--instance-white`) and compares it with the
  preview program node for node: 0 mismatches.
- `UNKNOWN`: game pixels and filtering (nearest is authored), f64 vs GPU rounding, clamp
  points inside the game shader (the preview clamps only at encoding), real instance
  colours, BUMP/other channels (not evaluated).

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (unit = material location with a MAP row; selection only, not assets)

`diffuse_route_census` b99 → d100: texture route **6,975 → 6,984 (+9)**; instance-only
**267 → 294 (+27)**; unsupported **102 → 66 (−36)**; solid 619, unwired 10,
animation-base-only 121, no MAP row 65 unchanged. 36 locations / 36 distinct hashes, all
from unsupported; no other class changed.

| Earlier reason | → texture | → instance-only |
|---|---|---|
| BLEND inputs must be COLOR or texture samples (37) | 4 | 21 |
| ADD of a texture and a product needs a three-texture bake (3) | 2 | 0 |
| ADD supports two texture samples, or … plus COLOR (9) | 0 | 3 |
| diffuse multiply requires two direct textures … (18) | 0 | 3 |
| alpha MIX needs COLOR and one texture sample (2) | 1 | 0 |
| sampler must be static, unwired UV0/UV1 (19; MIX of COLOR and a tinted product) | 2 | 0 |

Reason changed only (evaluable but off the grid, still unsupported, 21): BLEND inputs 6,
three-sample BLEND 3 (all three), four-leaf product trees 5, diffuse multiply 2, and one
each of two-texture BLEND, static MIX, alpha MIX, ADD of a texture and a product, 5-leaf
product tree (see `d100-diffuse-route-shapes-0133.json`). Still outside
the evaluator: unknown box types, animated or wired nodes, UV channel raw 2, COLOR
outside 0..1, ThingColor projections with folds.

`corpus_mesh_census` (all 4,261 mesh GUIDs, distinct catalog GUID): default all-diffuse
**3,683 → 3,691 (+8)**; `--instance-rgba 0.25,0.5,0.75,0.5` **3,929 → 3,958 (+29)**;
none lost (GUID sets compared with `b99-corpus-{default,inst}.json`); viewable 4,257
unchanged; no new load failure reason. Newly viewable +0; newly Complete +0 (no visual
check).

## PC verify (from the main checkout; outputs under ignored `local-analysis/`)

| Check | Result |
|---|---|
| `diffuse_route_census SNAPSHOT "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json local-analysis/d100-diffuse-route-shapes-0133.json` (release) | counts above |
| `scripts/run-material-oracle.ps1 -Diffuse -Archive <farc> -SampleManifest local-analysis/d100-manifest-<farc>.json -Output local-analysis/d100-oracle-<farc>.json` for DATA.FARC and patch_0109/0110/0114/0116/0118/0122/0125 | 5+2+4+8+5+3+6+3 = 36, merged `d100-oracle-merged.json` |
| `diffuse_parity SNAPSHOT GAME-DIRS local-analysis/d100-oracle-merged.json local-analysis/d100-parity-default.json` | 36 compared, 0 mismatches, 9 projected (7 grid, 2 per texel) |
| same `--instance-white` → `d100-parity-instance-white.json` | 36 / 0 / 36 (28 grid, 8 per texel) |
| Regressions `d100-regress-<oracle>-<mode>.json` vs an `origin/main` build (`-base`) | d58 48/0/47, d64 17/0/17, d67 18/0/18, d75-1 48/0/41, d75-2 48/0/39, d78 anim 38/0/36, d83-alpha2 8/0/8, d87-2 18/0/18, d88 both 4/0/4, d92 both 17/0/14, d96 white 4/0/3, d99 27/0/22 and white 27/0/27: every row identical. d89-0 5/0/2 → 5/0/4, d97-a white 31/0/9 → 31/0/17, d97-b 19/0/0 → 19/0/4, white 19/0/0 → 19/0/17, d97-a 31/0/0: base-projected rows identical; changes are unsupported → evaluated graph only, or the appended evaluator reason |
| `corpus_mesh_census` default / `--instance-rgba 0.25,0.5,0.75,0.5` | `d100-corpus-default.json`, `d100-corpus-inst.json`, counts above |
| `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (66 pass); `cargo test -q --workspace --exclude lbp-viewer`; `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `python -m unittest discover -s tests` | pass; `lbp-viewer` diagnostics tests need an existing `local-analysis/` dir (absent in the worktree), as before |

## Tests (synthetic)

`grid_graph::tests::{d100_three_sample_blend_of_a_product_selects_on_the_grid,
d100_thing_color_forms_only_under_a_supplied_instance_color,
d100_add_of_texture_and_product_and_budgets, d100_texel_semantics_match_the_fixed_bakes,
d100_single_source_graphs_bake_per_texel}`. Existing tests that asserted a refusal now
assert the evaluator selection where the evaluator legitimately takes the shape (one-letter
swizzles, wired MIX factor, three-sample BLEND/MIX, ADD of a texture and a product, COLOR
masks) and keep the refusal for multi-letter swizzles and off-grid samplers
(`linked`, `add`, `product_tree` tests).

## Not tested

Viewer rendering of any evaluated material (no native window, no capture), so pixels are
unit-tested only on synthetic textures; the legacy viewer path (it uses the same
`display_pixels`/nearest flags); game pixels and filtering; ThingColor graphs with folds
or UV alpha subtraction (refused); animated graphs (refused).
