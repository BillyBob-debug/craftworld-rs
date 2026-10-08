# D105 — graph evaluator on supplied-instance graphs (2026-10-08)

Follows [d100-grid-graph-bake.md](d100-grid-graph-bake.md) (evaluator, rule 7: ThingColor
only on the plain substitution), [d86-thing-color.md](d86-thing-color.md),
[d88-thing-color-textured.md](d88-thing-color-textured.md),
[d96-thing-sampler-ports.md](d96-thing-sampler-ports.md) (projection rewrites) and
[d101-multiply-rest.md](d101-multiply-rest.md) (five-leaf product).

Route code only, in `lbp-preview`: new module `src/instance_graph.rs`; hooks in
`instance.rs` (one `InstanceProjection.evaluated` field), `instance_textured.rs` (early
return when `evaluated`), `linked.rs` (the loader calls `project_instance_graph` instead
of `project_instance_color`, one line), `lib.rs` (exports). `diffuse_parity` schema 24
(`instance_evaluated_plain` per row), `diffuse_route_census` schema 4 (ThingColor reason
gets `| D105 <evaluator reason>`). No parser, disk-layout, shader, viewer or
`grid_graph.rs` change. Default behaviour (no supplied colour) is unchanged.

## Census first (script over the 53 b102 unsupported locations; scratch dump not committed)

Graphs of all 53 unsupported 01.33 locations (`b102-diffuse-route-shapes-0133.json`) were
dumped and read node by node. ThingColor sits in the diffuse ancestry of 28:

| Group (b102 reason) | Locations | ThingColor | What blocks the evaluator |
|---|---|---|---|
| ADD supports two texture samples, … plus COLOR | 6 | 6 | **`T×C + THING`: taken** (it was a D89 ADD the loader refuses under a supplied colour); FRESNEL box 18 (2, one also with a port-1-scaled sampler); animated sampler (1); ThingColor variant 3, not 8 (1); `THING + CONST`, no texture sample (1) |
| BLEND inputs must be COLOR or texture samples (no grid suffix) | 6 | 6 | all six mix UV0 and UV1 leaves (nested tint or operand rewrite on top) |
| diffuse multiply requires … (with ThingColor) | 11 | 11 | 6 animated-leaf products (one a product tree under instance), 3 BLEND-in-product or tree mixing UV0/UV1, 2 FRESNEL + ADD |
| sampler must be static (with ThingColor) | 4 | 4 | **`(T0² × THING) × T1` static product tree: taken**; animated leaves (2); MIX(CONST4, CONST4, THING.a) on a sampler port plus an animated leaf (1) |
| diffuse output must be static and unswizzled | 1 | 1 | `THING × THING.a`: no texture sample (a solid, not an evaluator shape) |
| five-leaf product tree | 1 | 0 | off grid, see below |
| others (off-grid BLEND/MIX/ADD, UV channel raw 2, out-of-range COLOR, CONST tint 4.89, COLOR-only matrix samplers) | 24 | 0 | unchanged, not in scope |

So of the instance cases, the rewrite itself was the only blocker for 2: a constant
ThingColor addend under ADD, and a static `(T0² × THING) × T1` product tree. Every other
ThingColor case fails the evaluator on the plain graph too: mixed UV channels (9), animated
or wired samplers (11), FRESNEL (3), no texture sample (2), ThingColor variant 3 (1).

**Five-leaf product** (`f404b6b6…`, `((T0×T1)×(T2×T3))×T4`): not on the integer Repeat
grid, for two independent reasons. Leaf T2 (slot 2) samples UV0 while the other four
sample UV1, and leaf T1 (slot 1) has offset (0, −0.1). D49 grid rule: one UV channel,
integer scales, zero offset. The evaluator already accepts five texture leaves (budget 6
samples), so this is not a missing evaluator case. It belongs to the off-grid bake (D103),
not here. It stays refused with the D101 reason plus the evaluator suffix.

## Rule (supplied-colour opt-in only)

`project_instance_graph(graph, color, animation_base)`:

1. Run `project_instance_color` as before. No projection (no ThingColor) → unchanged.
2. Keep that projection when its derived graph already selects: a solid
   (`select_solid_diffuse` not `Ok(None)`), or `select_instance_diffuse` succeeds with
   anything but a D89 ADD (the loader refuses D89 ADD under a supplied colour).
   Every existing instance-only selection keeps its projection, so its bake is byte-identical.
3. Otherwise build the **plain substitution**: the original graph with only the
   projected ThingColor nodes (`color_nodes`, the unwired static variant-8 nodes of the
   diffuse ancestry) set to COLOR with the supplied four words. Nothing else is
   rewritten. The D86/D88 constant folds and the D88 nested tint then remain the graph's
   own COLOR/MULTIPLY/MIX nodes, which the D100 evaluator already evaluates. Sampler wires
   are kept, so UV alpha subtraction and D96 sampler-port inputs stay refused by the
   evaluator's static sampler rule.
4. Take the plain graph when it is not a solid and `select_evaluated` (D100 compile + grid
   rule, `grid_graph::select` on the static pass) accepts it. The projection is then
   `{ color, color_nodes, evaluated: true }` with every rewrite field empty, so
   `grid_graph::instance_supported` holds and the D100 loader runs unchanged.
5. `select_instance_diffuse` with `evaluated` returns `select_evaluated` only; no fixed
   route, product tree or animation-base route runs on the plain graph.
6. Otherwise the step-1 projection and its reason are returned unchanged.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`
  (pin checked by `run-material-oracle.ps1`),
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`: THING_COLOR
  (268–269) is the four-wide instance colour, a value like a COLOR box, so substituting it
  by a COLOR with the supplied words and evaluating the unchanged graph is the same
  expression as Toolkit's. ADD/MULTIPLY/MIX/BLEND rules are as cited in D100. Restated
  independently; no Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + updates 01.01..01.33): Toolkit oracle
  (`MaterialOracle --diffuse`) on both changed materials. `diffuse_parity --instance-white`
  walks the oracle rows from the diffuse root on its own (ThingColor = white COLOR, D100
  `graph_equal`) and compares them node by node with the program compiled from the plain
  graph: 0 mismatches. The folded or rewritten graph is never what is compared.
- `STRONG_INFERENCE`: the evaluator's f64 order of operations on a COLOR × texture product
  equals the folded tint the D86/D88 routes apply (multiplication is order-free up to
  rounding). Only the plain graph is baked, so no folded value is used.
- `UNKNOWN`: real instance colours; game pixels and filtering; overbright ADD results
  (the bake clamps at 8-bit encoding, as in D89/D100).

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (unit = material location with a MAP row; selection only, not assets)

`diffuse_route_census` b102 → `d105-diffuse-route-shapes-0133.json`: unsupported
**53 → 51 (−2)**, instance-only **296 → 298 (+2)**. Texture route 6,989, solid 620,
animation-base-only 126, unwired 10, no MAP row 65 unchanged. Changed: `5b9d1ee8…`
(`T×C + THING`, per-texel) and `ef22e36c…` (`(T0²×THING)×T1`, grid). Remaining
ThingColor reasons (D105 suffix): sampler must be static 11, off the integer grid 9,
FRESNEL box 18 3, no texture sample 2, ThingColor variant 3 1.

`corpus_mesh_census` (all 4,261 mesh GUIDs, distinct catalog GUID): `--instance-rgba
0.25,0.5,0.75,0.5` all-diffuse **3,962 → 3,963 (+1, GUID 000077bf)**; default 3,695
unchanged. No GUID lost (sets compared with `b102-corpus-{inst,default}.json`); viewable
4,257 unchanged. Newly viewable +0; newly Complete +0 (no visual check).

## PC verify (run from the main checkout with this worktree's release binaries; new files)

| Check | Result |
|---|---|
| `diffuse_route_census local-analysis\d68-inventory-0133-verified.json "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d68-material-census-0133b.json local-analysis\d105-diffuse-route-shapes-0133.json` | counts above |
| `scripts\run-material-oracle.ps1 -Diffuse -SampleManifest local-analysis/d105-manifest-DATA.json -Output local-analysis/d105-oracle-DATA.json` | 2 samples (both disc DATA.FARC) |
| `diffuse_parity … d105-oracle-DATA.json d105-parity-default.json` | 2 / 0 mismatches / 0 projected (default unchanged: both unsupported) |
| same `--instance-white` (`d105-parity-instance-white.json`) | 2 / 0 / 2 (both `instance_evaluated_plain`, grid_graph) |
| same `--instance-white --animation-base` (`d105-parity-instance-white-anim.json`) | 2 / 0 / 2 |
| regression `diffuse_parity … d100-oracle-merged.json d105-regress-d100-instance-white.json --instance-white` | 36 / 0 / 36; rows identical to `d100-parity-instance-white.json` |
| regression `… d96-oracle-DATA.json d105-regress-d96-instance-white.json --instance-white` | 4 / 0 / 3; rows identical to `d100-regress-d96-oracle-DATA-white.json` |
| `corpus_mesh_census … d105-corpus-inst.json --instance-rgba 0.25,0.5,0.75,0.5` / `d105-corpus-default.json` | 3,963 / 3,695 |
| `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (71); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `cargo test -q --workspace --exclude lbp-viewer` | pass |

## Tests (synthetic)

`instance_graph::tests::{d105_existing_instance_selections_are_kept,
d105_nested_instance_tint_evaluates_the_plain_graph,
d105_refusals_keep_the_projection_and_name_the_reason}`: a D88 nested tint the fixed routes
take keeps its projection; `BLEND(T1,T2,T3.r) × (THING × T0)` (nested tint refused by the
D100 guard) evaluates the plain graph on the grid with the ThingColor as a COLOR node and
wires untouched; an off-grid leaf and a ThingColor `.a` sampler-port wire keep the old
projection and give the evaluator's reason.

## Not tested

Viewer rendering of the two materials (no native window, no capture); game pixels;
`lbp-viewer` diagnostics tests (need the ignored `local-analysis/` directory in the
worktree); instance colours other than white in parity (the census uses 0.25/0.5/0.75/0.5).
