# D189 — per-fragment graphs over mixed UV0/UV1 samples (2026-10-09)

Follows [d100-grid-graph-bake.md](d100-grid-graph-bake.md) (evaluator),
[d103-offgrid-bake.md](d103-offgrid-bake.md) (off-grid bakes; mixed-channel graphs left
refused) and [d105-instance-graphs.md](d105-instance-graphs.md) (plain instance
substitution). Extends the D99 Bevy dual path ([d99-product-trees.md](d99-product-trees.md))
from "primary × one second sample" to an N-sample program on either UV channel.

Code: new `crates/lbp-preview/src/fragment.rs`; hooks in `offgrid.rs` (one fallback in
`plan`, `OffGrid::Fragment`, label), `grid_graph.rs` (loader dispatch), `linked.rs`
(`Asset.fragment`, retained bytes), `lib.rs` (exports), `fragment: None` in the other
`Asset` constructors; `lbp-bevy-viewer/src/dual.rs` (shader mode + bindings 120–128),
`main.rs` (prepare/upload); `lbp-viewer/src/app/mesh.rs` (refusal);
`diffuse_parity` schema 25; `compatibility_census` adds a `fragment_graph` field when
present. No parser, disk-layout or blend-maths change.

## Census first (PC 01.33 route census at 0097c07, `pc-batch2.md` §2)

Shape signatures from `diffuse_route_census::shape()`: nodes `type.subtype+named-anims`
in walk order (local 0 = the diffuse root), edges `from>to:port[swizzle]`. Types: 1
TEXTURE_SAMPLE, 2 THING_COLOR, 3 COLOR, 10 MULTIPLY, 12 MIX, 16 BLEND. All rows below are
static (no named animations) and every sampler is subtype 0 (scale/offset).

Decoded as `BLEND(port 0, port 1, mask port 2)` / `MIX(port 0, port 1, factor)`; `Tn` =
sample at local index n, `THING` = ThingColor; which samples sit on UV0 vs UV1 is not in
the signature (the refusal says both channels occur).

| GUID | Material | Shape (decoded) | Refusal before D189 |
|---|---|---|---|
| 16046 | indianrubble | BLEND(T7×T6, T4×T3, T1) | evaluator: mixes UV channels outside a separable product |
| 242447 | summergames_gymnast_hr | BLEND(T6×T4, T4×THING, T1) | same, under ThingColor (D105 plain substitution) |
| 242464 | summergames_tracknfield_lg | BLEND(THING×T2, T2, T1) | same, under ThingColor |
| 242465 | summergames_tracknfield_to | BLEND(T3, THING×T3, T1) | same, under ThingColor |
| 242467 | summergames_wrestler_he | BLEND(T5×THING, T2, T1) | same, under ThingColor |
| 255162 | knightsofold2_dragon_he_details | BLEND(T4×T6, T4×THING, T1) | same, under ThingColor |
| 108243 | da_vinci_tower_sm | BLEND(T1, T2, T1) | two-texture BLEND requires same-channel grid + evaluator suffix |
| 15658 | wheel_tricycle | MIX(COLOR, T3×T2, weight) | static MIX requires same-channel grid + evaluator suffix |
| 19661 | basketball_black | BLEND(T3, T2, T1) | three-sample BLEND has no bake + evaluator suffix |

Every row compiled under the D100 rules and failed only D103's channel split: some
input of the root BLEND/MIX depends on both channels, so no bake per channel exists, and
a single baked texture cannot hold a blend of samples addressed by two UV sets.

## Rule

Runs only where D103 returned exactly "off-grid samplers mix UV channels outside a
separable product" (`offgrid::NOT_SEPARABLE`); every other selection, bake and label is
unchanged (on-grid, resampled, split, and every refusal with another reason).

1. **Selection** (`fragment::plan`): every source is a scale/offset sampler with |scale|,
   |offset| ≤ 4096 (the D103 transform check; matrix samplers keep "off-grid bake needs
   scale/offset samplers"); UV channel 0 or 1; at most **6 distinct texture slots** (the
   shader's graph bindings; the D100 sample budget is also 6) and at most **32 steps**
   (the D100 node budget). `GridGraph.off_grid = OffGrid::Fragment { samples, textures }`,
   no second sample.
2. **Program.** The D100 compile (`grid_graph::steps`) is encoded unchanged
   (`fragment::encode`): Sample (texture index, channel, scale.xy, offset.xy), Value
   (COLOR/CONSTANT4 words, CONSTANT broadcast), MULTIPLY, ADD, MIX (wired port 2 or
   parameter-word-0 weight), BLEND (D94 thresholds from `blend_thresholds`); inputs are
   step indices with the D100 one-letter lane broadcast. Every parameter is the f32 the
   D100 compile read, so no value is re-rounded. A supplied instance colour arrives as
   the D105 plain-substitution COLOR, so it is folded exactly like any COLOR.
3. **Load** (`fragment::load`): each slot needs wrap Repeat (1), Mirror (2) or ClampToEdge
   (3) on both axes (sampler address modes; raw 5 CLAMP/BORDER and mirror-once refused),
   decodes with the existing descriptor rules, and must fit the viewer's GPU budget
   (≤ 4096 per axis, ≤ 4 Mi pixels) so the census counts only what the viewer can bind.
   The asset's `sampling_uv` is raw UV0, `fragment.uv1` raw UV1; texture 0 is the primary
   texture; no bake is made.
4. **Bevy** (`dual.rs`, `graph_info.x` = step count): graph textures 0..2 reuse bindings
   100/103/105, 3..5 are 120/122/124; programs in `graph_code`/`graph_param` uniforms.
   Per fragment each sample reads `uvC × scale + offset` with `textureSampleGrad`
   (gradients from `dpdx/dpdy` of the raw UVs taken in uniform control flow, scaled by the
   sample's scale), on `Rgba8UnormSrgb` images (RGB decoded, alpha linear: the D100 decode
   rule); the steps run in order with the D100 semantics; RGB is clamped to 0..1 (D100
   clamps only at encoding) and drawn opaque. Lighting/fog follow as for every material.
   The P2q bump map is not drawn on these materials (UV1 is the graph's).
5. **Legacy viewer:** refuses ("Unsupported in legacy viewer: per-fragment UV0/UV1 graph;
   use Bevy viewer"); no approximation is shown there.
6. **Labels:** `OffGrid::label()` = "preview approximation (not game behaviour): UV0/UV1
   graph evaluated per fragment in the Bevy viewer (N samples over K textures, viewer
   bilinear filtering, linear light, clamped to 0..1 at output; legacy viewer refuses)";
   the route census writes it as the texture-route reason (as for D103); `diffuse_parity`
   reports `bake: fragment_graph_bevy`.

`fragment::evaluate` (exported as `evaluate_fragment`) is the f32 CPU reference of the
shader loop; unit tests compare it with the D100 evaluator.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java` (read only; restated, nothing
  copied, adapted or translated): the node rules are those already cited in D100
  (`resolve` 140–141, COLOR 270–286, CONSTANT 287–289, CONSTANT4 303–310, MULTIPLY
  328–347, ADD 349–386, MIX 388–410, BLEND 438–456, THING_COLOR 268–269) and
  TEXTURE_SAMPLE (150–225) samples `iUV.<channel>` scaled and offset by its own words per
  pixel (171, 197–207). The Toolkit output is a per-pixel shader expression, so
  evaluating the same DAG per fragment is the same structure; D189 changes only where it
  is evaluated, not what.
- `STRONG_INFERENCE`: lane broadcast and BLEND thresholds as in D100/D94.
- **Preview policy, not game behaviour (labelled):** the viewer's sampler filtering (Bevy
  default linear; mip selection from the scaled gradients if mips exist), f32 GPU
  arithmetic, the output clamp, opaque alpha.
- `UNKNOWN`: game pixels, filtering and mips; GPU rounding; real instance colours.
- `CONFIRMED_FROM_LOCAL_DATA`: **not yet** — the oracle/parity runs below are pending on
  the PC (no game data in this container).

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Which listed GUIDs should now pass (expected, not measured)

- **16046, 108243, 15658, 19661** (default route): the only recorded reason past the
  fixed route was the evaluator's channel-split refusal, which now selects
  `Fragment`; samplers are subtype 0 and static per the shape signature. They load if
  their slots are Repeat/Mirror/ClampToEdge and within the GPU budget (not visible in the
  signatures; any raw-5/BORDER slot keeps them refused with the new D189 wrap reason).
- **242447, 242464, 242465, 242467, 255162** (supplied instance colour only, D105 plain
  substitution): their D105 reason was the same evaluator refusal, so with
  `--instance-rgba` they should select `Fragment`. With the default route (no supplied
  colour) they stay refused (ThingColor is not evaluated without a colour, D100 rule 7).
  The white-anim mesh census showed the plain "BLEND inputs must be COLOR or texture
  samples" reason without the D105 suffix; if those meshes take a different instance
  projection than the route census (e.g. a fold that blocks `instance_supported`), they
  will stay blocked — check the reason in the D189 output.
- Per the b176 white-anim blocker table: g16046 sole-blocks 4 GUIDs, g108243, g15658 and
  g19661 one each; the summergames/dragon rows are part of the 11 "BLEND inputs" group.
  Expected: about +7 to +20 all-diffuse GUIDs over the 4,155 baseline, depending on the
  wraps above and on other materials in the same meshes.

## PC verification (from the main checkout; outputs under ignored `local-analysis/`)

```
cargo build --release -p lbp-preview --examples
target/release/examples/corpus_mesh_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d189-corpus-white-anim.json --blockers --instance-rgba 1,1,1,1 --animation-base
```
(argument order as for `b176-blockers-white-anim.json`.) Compare `all_diffuse_guids`
with the baseline **4,155** (`b176-blockers-white-anim.json`); no GUID may be lost (diff
the GUID lists); new neutral reasons should only be D189 wrap/budget refusals.

```
target/release/examples/diffuse_route_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json local-analysis/d189-route-0133.json
```
Expect only the rows above to move from unsupported (34) to texture_route (default) or
instance-only (ThingColor), with the D189 label.

Oracle and parity: build a manifest for the 9 materials, run
`scripts/run-material-oracle.ps1 -Diffuse -Archive <farc> -SampleManifest local-analysis/d189-manifest-<farc>.json -Output local-analysis/d189-oracle-<farc>.json`,
merge to `d189-oracle-merged.json`, then
`diffuse_parity SNAPSHOT GAME-DIRS local-analysis/d189-oracle-merged.json local-analysis/d189-parity-default.json`
and the same with `--instance-white` (`d189-parity-instance-white.json`): expect 0
mismatches; the default run projects 16046/108243/15658/19661, `--instance-white` all 9.
Regressions (latest STATUS entry, D103–D105 line): rerun `d100`, `d101`, `d99`, `d78`
(`--animation-base`), `d75`, `d83`, `d86`, `d87`, `d88`, `d92`, `d96`, `d64`, `d65`, `d67`,
`d46` ×2, `d89-0`, `d97-a`, `d103` (default and `--instance-white`), `d104`, `d105`
oracles into `d189-regress-<name>.json` vs an `origin/main` build: identical except rows
that were unsupported with the mixed-channel evaluator reason (expected in `d103` and
`d97-a`, which carried those reasons) now projecting with `bake: fragment_graph_bevy`.

Viewer: `target/debug/lbp-bevy-viewer.exe --snapshot local-analysis/a2-inventory-verified.json --game-dir game-data/a1-disc --select-guid <mesh guid using 108243 or 16046> --qa-capture local-analysis/d189-<guid>.png`
and a mesh using a summergames material with `--instance-rgba 1,1,1,1`.

## Checks run here (Linux container)

`cargo fmt --all --check`; `cargo clippy -q -p lbp-preview -p lbp-bevy-viewer --all-targets -- -D warnings`;
`cargo test -q -p lbp-preview`; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`;
`cargo test -q -p lbp-bevy-viewer authored_shader` (naga validation with every branch):
all pass (`lbp-preview` 108 unit tests). Not run: workspace tests, Python tests.

## Tests (synthetic)

`fragment::tests::d189_mixed_uv_blend_and_mix_select_the_fragment_path` (16046-like
BLEND of cross-channel products, static MIX, two-texture BLEND with a UV1 mask; matrix
sampler still refused); `fragment::tests::d189_encoded_steps_evaluate_like_the_d100_evaluator`
(encoded program vs `grid_graph::evaluate` at 50 UV pairs, texture/step budgets,
malformed input codes). Refusal tests that the fragment path now takes were updated:
`add` (cross-channel ADD), `grid_graph` (cross-channel BLEND of a product),
`offgrid` (cross-channel BLEND still `NOT_SEPARABLE` in `targets`), `linked` (off-grid
port-0 alpha MIX, MIX of COLOR and a UV0×UV1 product, two- and three-sample BLEND on two
channels).

## Not tested / left

- Viewer rendering (no window or capture here): the shader is naga-validated with all
  branches, not visually checked; the bind group now holds up to 10 extension textures
  plus StandardMaterial's, which should fit desktop limits but is unverified on the
  user's GPU.
- `fragment::load` (decode, wraps, raw UV arrays) has no unit test; only selection,
  encoding and the CPU reference are tested. The CPU census counts a binding when that
  loader succeeds.
- Game data: none of the GUID claims above is measured; oracle parity is pending.
- Left out: matrix samplers, raw UV channel 2 inside mixed graphs, BORDER/raw-5 slots,
  more than 6 textures, animated graphs (the evaluator is static-only), and the D99
  port-0 alpha MIX now selects here only when its channels differ.
