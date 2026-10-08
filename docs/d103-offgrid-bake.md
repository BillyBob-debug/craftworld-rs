# D103 — off-grid bakes for the graph evaluator (2026-10-08)

Follows [d100-grid-graph-bake.md](d100-grid-graph-bake.md), which refused evaluable graphs
whose texture leaves were not on the same-channel integer Repeat grid. Route code only, in
`lbp-preview`: new module `crates/lbp-preview/src/offgrid.rs`; `grid_graph.rs` (shared
step compile/evaluate helpers, `GridGraph.off_grid`, select/load/label hooks); `linked.rs`
(one hook: off-grid bakes are not nearest-sampled); `lib.rs` (`OffGrid` export);
`diffuse_parity` schema 24; `diffuse_route_census` schema 4. No parser, shader or viewer
change: the split uses the existing Bevy dual multiply.

## Census first (b102, 53 unsupported)

13 distinct graphs (16 rows) failed only with `(graph evaluator: requires same-channel
integer Repeat grid samplers)`; their samplers were dumped with a scratch tool (not
committed):

| Shape | Rows | Why off the grid |
|---|---|---|
| BLEND/ADD/MULTIPLY/MIX graphs, all leaves UV0 | 9 (6 hashes) | scales 2.5/3.5/0.5/300, offsets (−1.5, −4), one BORDER slot |
| MULTIPLY tree over UV0 and UV1 (5 leaves) | 1 | two channels, one offset |
| BLEND/MIX roots over UV0 and UV1 | 6 | two channels inside the blend/mix inputs |

## Rule (runs only where D100 refused with the grid reason)

On-grid and per-texel graphs keep the D100 path unchanged (`off_grid = None`, same
selection, same bytes; the D100 bake loop was only factored into `steps`/`evaluate`/
`encode_texel`, unit tests keep the exact D49/D83/D97 bytes).

1. **Transforms.** Every source must be a scale/offset sampler (subtype 0) with |scale|,
   |offset| ≤ 4096; matrices stay refused ("off-grid bake needs scale/offset samplers").
2. **Resampled** (`OffGrid::Resampled { channel, period }`): all sources on one UV channel.
   Tile period per axis = smallest integer `P` in 1..16 with `P × |scale| / r` integral
   (tolerance 1e-4) for every leaf, `r` = 1 Repeat, 2 Mirror. Otherwise "off-grid scales
   have no common period within 16 repeats".
3. **Split** (`OffGrid::Split`): sources on UV0 and UV1. The root's MULTIPLY tree is
   flattened (a one-letter input swizzle passes its lane down; 64 visits); each factor
   must depend on one channel or none, else "off-grid samplers mix UV channels outside a
   separable product". Constant factors join the primary group (channel of the first
   sample in evaluation order); each group gets its own period as in 2.
   `second_sample` = the first sample on the other channel.
4. **Load.** Every texture wrap must be Repeat (1) or Mirror (2), else refused (BORDER and
   clamps are not periodic); the period recomputed with the real wraps must equal the
   selected one. Per group a bake of `N = min(4096, max ⌈P × |scale| × texels⌉)` per axis,
   the larger axis halved until ≤ 4 Mi pixels. At each bake texel centre every leaf is
   sampled **bilinearly in linear light** (RGB sRGB-decoded, alpha linear; Repeat or
   Mirror texel addressing) through its own scale/offset, box-averaged over up to 4×4
   sub-positions when the bake minifies the leaf; the D100 node rules then evaluate the
   graph (only the steps the group needs) and the group's factors are multiplied; output
   sRGB RGB / linear alpha clamped at 8-bit encoding. Cancellation every 16,384 texels.
5. **Display.** Primary bake on `uvC / P` (Repeat, **linear** filtering); for a split the
   second bake is a `DualAsset` on `uvC2 / P2` multiplied per pixel by the Bevy dual path
   (linear values). The legacy viewer keeps its dual refusal; resampled single bakes show
   there with linear filtering.
6. **Labels.** `GridGraph.label()` and the source label append `OffGrid::label()`:
   "preview approximation (not game behaviour): …"; `diffuse_route_census` writes it as
   the texture-route reason (instance-only rows append it in brackets) and states it in
   `scope`; `diffuse_parity` reports `policy:
   evaluated_static_graph_offgrid_preview_approximation`, `bake:
   offgrid_resampled_bilinear_tile | offgrid_split_dual_multiply` and the label.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java` (read only; restated, nothing
  copied, adapted or translated): node semantics as cited in D100 (`resolve` 140–141,
  COLOR 270–286, CONSTANT 287–289, CONSTANT4 303–310, MULTIPLY 328–347, ADD 349–386, MIX
  388–410, BLEND 438–456); TEXTURE_SAMPLE (150–225) samples `iUV.<channel>` scaled and
  offset by its own words per pixel (171, 197–207), so each leaf has its own coordinates
  and MULTIPLY is per-pixel and commutative (the split's regrouping is exact before
  quantisation).
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + updates 01.01..01.33): Toolkit oracle on all
  15 hashes that changed class or reason; `diffuse_parity` compares the program node for
  node (D100 `graph_equal`) and now re-derives the off-grid plan from the oracle rows
  alone (sample channels, separable oracle MULTIPLY factors, periods from the oracle
  scale words, second sample channel): 0 mismatches.
- **Preview policy, not game behaviour (labelled):** the common tile, bake extent,
  bilinear leaf sampling, 4×4 box minification, linear display filter and the per-group
  clamp before the split product. The game samples each leaf on the GPU with its own
  filtering/mips; none of that is observed here.
- `UNKNOWN`: game pixels and filtering, mip selection, f64 vs GPU rounding, real instance
  colours, BORDER colour.

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (unit = material location with a MAP row; selection only, not assets)

`diffuse_route_census` b102 → d103 (`d103-diffuse-route-shapes-0133-b.json`): texture route
**6,989 → 6,999 (+10)**; instance-only **296 → 298 (+2)**; unsupported **53 → 41 (−12)**;
solid 620, unwired 10, animation-base-only 126, no MAP row 65 unchanged. 18 rows / 15
distinct hashes changed:

- → texture route, Resampled (9 rows / 6 hashes): BLEND inputs (4), BLEND three samples
  (2), ADD of a texture and a product (1), diffuse multiply (1, `9f39…`, see below),
  product tree tint outside 0..1 (1, scale 300 with CONSTANT 4.89).
- → texture route, Split (1): five-leaf product tree.
- → instance-only, Split under the supplied colour (2): diffuse multiply with ThingColor.
- Reason only, still unsupported (6): BLEND/MIX roots over two channels (not separable):
  BLEND inputs 2, three-sample BLEND 1, two-texture BLEND 1, static MIX 1, alpha MIX 1.

`9f39…` selects but its first leaf's slot is BORDER, so loading refuses it as before
("BORDER requires …", 2 primitives in the corpus); the selection census does not see
wraps (same caveat as D100).

`corpus_mesh_census` (all 4,261 mesh GUIDs, distinct catalog GUID): default all-diffuse
**3,695 → 3,700 (+5)**; `--instance-rgba 0.25,0.5,0.75,0.5` **3,962 → 3,969 (+7)**; none lost
(GUID sets vs `b102-corpus-{default,inst}.json`); viewable 4,257 unchanged; the only new
load reason is the BORDER refusal above. Newly viewable +0; newly Complete +0 (no visual
check).

## PC verify (from the main checkout; outputs under ignored `local-analysis/`)

| Check | Result |
|---|---|
| `diffuse_route_census SNAPSHOT "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json local-analysis/d103-diffuse-route-shapes-0133-b.json` (release) | counts above (`d103-diffuse-route-shapes-0133.json` = same classes, earlier label text) |
| `scripts/run-material-oracle.ps1 -Diffuse -Archive <farc> -SampleManifest local-analysis/d103-manifest-<farc>.json -Output local-analysis/d103-oracle-<farc>.json` for DATA.FARC and patch_0113/0122/0106/0109/0116 | 8+3+1+1+1+1 = 15, merged `d103-oracle-merged.json` |
| `diffuse_parity SNAPSHOT GAME-DIRS local-analysis/d103-oracle-merged.json local-analysis/d103-parity-default.json` | 15 compared, 0 mismatches, 7 projected (6 resampled, 1 split) |
| same `--instance-white` → `d103-parity-instance-white.json` | 15 / 0 / 9 (6 resampled, 3 split) |
| Regressions `d103-regress-d100-{default,instance-white}.json`, `d103-regress-d99-default.json` | 36/0/9, 36/0/36, 27/0/22 (unchanged) |
| `corpus_mesh_census` default / `--instance-rgba 0.25,0.5,0.75,0.5` | `d103-corpus-default.json`, `d103-corpus-inst.json`, counts above |
| `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (72 pass); `cargo test -q --workspace --exclude lbp-viewer`; `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`; `cargo clippy -q -p lbp-bevy-viewer -- -D warnings`; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `python -m unittest discover -s tests` | pass |

## Tests (synthetic)

`offgrid::tests::{d103_same_channel_offgrid_graphs_select_as_labelled_resamples,
d103_separable_products_split_by_channel_and_others_are_refused,
d103_resample_reproduces_texels_and_evaluates_the_graph,
d103_split_bakes_multiply_back_to_the_whole_graph}` (period, matrix and cross-channel
refusals, texel round trip at scale 1, Mirror addressing, 4×4 minification, cancellation,
period budget, COLOR folded into the primary group). Updated refusal tests that the
approximation now takes: `add` (scale 1.5 ADD, three-texture ADD), `linked` (zero base
scale, fractional BLEND second sampler), `product_tree` (zero scale, five leaves),
`grid_graph` (cross-channel BLEND reason text).

## Not tested / left

Viewer rendering of any off-grid material (no native window, no capture): the split's
dual multiply and the linear display are compiled, not visually checked. Game pixels and
filtering. On-grid byte identity is asserted by unit tests and identical parity, not by a
corpus pixel-hash comparison. Left unsupported: BLEND/MIX roots whose inputs mix UV0/UV1
(6 rows; a GPU-side formula over ≥3 bakes would be needed), BORDER/clamp leaves off the
grid, matrix samplers off the grid, scales without a period ≤ 16.
