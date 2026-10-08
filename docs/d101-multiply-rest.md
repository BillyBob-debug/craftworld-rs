# D101 — remaining MULTIPLY shapes (2026-10-07)

Backlog: the 25 unsupported material locations on 01.33 left in MULTIPLY trees by
D99 (`b99-diffuse-route-shapes-0133.json`): 18 "diffuse multiply requires two direct
textures or one texture and static COLOR", 5 "product tree four textures need two exact
coordinate-sharing pairs", 1 "more than four texture leaves", 1 "COLOR-only product".
Code: `lbp-preview` `product_tree.rs` (rules), hooks in `small_family.rs`
(COLOR-only tree), `solid.rs` (`SolidSelection::product_tree`), `linked.rs` (loader
fourth coordinates, solid label), `diffuse_parity` (schema 23), `compatibility_census`
(schema 13), Bevy `dual.rs` / `main.rs` (fourth transform). No parser or disk-layout
change. BLEND/ADD/MIX nodes (D100) and sampler ports/animation (D102) untouched.

## Breakdown by shape (script over a scratch graph dump; scratch tool not committed)

| Shape under the diffuse MULTIPLY | Locations | Outcome |
|---|---|---|
| Four static texture leaves, at most one exact coordinate pair | 5 (4 hashes) | **texture route** (rule 3) |
| MULTIPLY tree of COLOR leaves only, `(C×C)×C` | 1 | **solid** (rule 2) |
| `(T×T')×CONST` with CONST = 4.888889 | 1 | refused: "product tree folded COLOR/CONST tint outside finite 0..1" (rule 1 folds it; overbright) |
| Five texture leaves | 1 | refused: no fifth sampler binding in either render path |
| ThingColor with ADD and/or FRESNEL (`THING×(THING+T)` ×3, `(T*+(T*+T*))×THING`, ADD of THING and FRESNEL ×2) | 6 | refused (ADD is D100; FRESNEL has no path) |
| ThingColor with BLEND inside the product | 2 | refused (D100) |
| `MIX(C,T,T.a)×T` | 1 | refused (D100) |
| ThingColor × 2-3 animated leaves (`several animated samplers under a folded…` ×4, product tree under instance ×1) | 5 | refused (needs instance + animation opt-ins and projected-leaf parity) |
| ThingColor × static `T×T'×T''²` | 1 | refused: "product tree under a supplied instance color" (projected reason was the four-pair rule) |
| ThingColor × animated matrix samplers (subtype 1, two animations) | 2 | refused (sampler animation, D102) |

Side effect: 3 locations whose four-leaf trees have one or two animated leaves
(`sampler must be static`) now select under the animation-base opt-in (rule 3 with
each leaf's own base rows); they stay behind that opt-in.

## Rules (default route unless stated; run only after the fixed routes refused)

1. **CONST leaves.** In a D99 MULTIPLY tree, an unanimated, unwired CONST (box 4)
   contributes parameter word 0 to all four components; CONST4 (box 7) contributes
   words 0..3 like COLOR. They fold with the COLOR leaves (ascending box index, f32
   product from 1) into the tint. CONST2/CONST3 leaves are refused ("vector-width
   product semantics not established"). A folded tint outside finite 0..1 is refused.
2. **COLOR-only trees.** A tree with no texture leaf and at least one COLOR/CONST leaf
   folds (same fold) to a solid color: `select_small_solid` returns
   `SolidSelection { color_index: root, product_tree: Some(..) }`. Original graph only
   (the loader already applies `select_small_solid` only without an instance
   projection); tree errors there fall back to the texture routes' reasons.
3. **Four leaves without two exact pairs.** After the D60 pairing fails, the first leaf
   order (lexicographic over the walk-order leaves) is taken whose third sample passes
   the D65 rule (`relative_third`) and whose fourth passes the same rule
   (`relative_fourth`): exact share with the primary → primary coordinates (identity,
   the D60 behaviour), else exact share with the second → second coordinates, else a
   finite scale/offset relative to a nonzero-scale scale/offset primary or second on the
   same raw channel (f64, rounded once to f32, |v| ≤ 1e6). Instance-subtracted samplers
   are not bases. Checked at selection (census reasons match the loader).
   Rendering: `SharedSampleAsset::relative` is now set for the fourth sample too; the
   Bevy shader samples it at `base × xy + zw` (binding 108; `enabled.z` 1 = primary
   coordinates, 2 = second). Legacy viewer: unchanged dual-path refusal.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`: CONSTANT
  (line 287) yields a scalar float (type 1) from parameter 0; CONSTANT4 (303) a float4 of
  parameters 0..3; COLOR (270) a float4; MULTIPLY (328–347) emits `(l * r)` with the wider
  operand type, so a scalar CONST scales all four components and the product of a static
  tree is the product of its leaves (D99). TEXTURE_SAMPLE (150–254) builds each sampler's
  own UV expression and `SAMPLE_2D` call, so four samplers with four independent
  scale/offset transforms are four independent samples whose product is order-free;
  the relative-coordinate derivation is ours (D65), not Toolkit's. CONSTANT2/3 (290, 296)
  produce float2/float3, whose product with a float4 is not established here. Restated
  independently; no Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + updates 01.01..01.33): Toolkit oracle
  (`MaterialOracle --diffuse`) on all 8 hashes that changed class; `diffuse_parity`
  walks the oracle rows independently (texture leaf multiset, slot/channel/transform of
  each role, CONST broadcast in the fold, COLOR-only fold bits for the solid): 0
  mismatches default and with `--animation-base`.
- `UNKNOWN`: game pixels and filtering; GPU rounding of the relative coordinates (f32
  once vs the game's per-sampler UV math); overbright products (refused); CONST2/3
  product semantics; real instance colors.

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (unit = material location with a MAP row; selection only)

`diffuse_route_census` b99 → `d101-diffuse-route-shapes-0133.json`: unsupported
102 → 93; texture route 6,975 → 6,980 (+5 four-leaf); solid 619 → 620 (+1 COLOR-only);
animation-base-only 121 → 124 (+3 animated four-leaf); instance-only 267, unwired 10,
no MAP row 65 unchanged. Reason changed only (still unsupported, 2): CONST tint bound,
five leaves (fifth binding). One ThingColor reason changed (b63743…: projected four-pair
→ product tree under a supplied instance color).

Corpus mesh census (all 4,261 mesh GUIDs, distinct catalog GUID, all-diffuse = every
primitive's diffuse loads): default 3,683 → 3,687 (+4); `--animation-base` 3,747 →
3,753 (+6). Viewable 4,257 unchanged; Complete 0 (no visual check). Newly viewable +0;
newly Complete +0.

## PC verify (from the main checkout; outputs ignored, all new files)

| Check | Result |
|---|---|
| `diffuse_route_census local-analysis\d68-inventory-0133-verified.json "game-data\a1-disc;game-data\a1-update-0133" local-analysis\d68-material-census-0133b.json local-analysis\d101-diffuse-route-shapes-0133.json` | counts above |
| `scripts\run-material-oracle.ps1 -Diffuse` on `d101-manifest-{DATA,patch_0109,patch_0111}.json` (6 + 1 + 1) | merged `d101-oracle-merged.json` (8) |
| `diffuse_parity … d101-oracle-merged.json d101-parity-default.json` | 8 compared, 0 mismatches, 5 projected |
| same `--animation-base` (`d101-parity-anim.json`) | 8 / 0 / 8 |
| regression `diffuse_parity … d99-oracle-merged.json d101-regress-d99-default.json` | 27 / 0 / 22 (= D99) |
| same `--instance-white` (`d101-regress-d99-instance-white.json`) | 27 / 0 / 27 (= D99) |
| `corpus_mesh_census` default / `--animation-base` | `d101-corpus-{default,anim}.json`, counts above |
| `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (62); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`; `cargo clippy -q -p lbp-bevy-viewer -- -D warnings`; `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `python -m unittest discover -s tests` | pass |
| `cargo test -q --workspace` | pass except 2 `lbp-viewer` diagnostics tests that need the ignored `local-analysis/` directory, absent in the worktree (environmental, untouched code) |

## Tests (synthetic)

`product_tree::tests::d101_constant_leaves_fold_and_color_only_trees_are_solids`
(CONST broadcast, CONST4, COLOR-only solid, overbright and CONST2 refusals);
`d99_four_texture_trees_need_two_sharing_pairs` extended (relative fourth from the
second, D60 chain keeps primary identity, two distinct matrix leaves refused);
`linked` four-chain test updated (unshared chain now selects through the tree).

## Not tested

Viewer rendering of the new materials (no native window, no capture): the Bevy fourth
transform is compiled, not visually checked; legacy viewer on four-texture trees
(refused as before); game pixels; `--instance-rgba` corpus census (no instance shape
changed class).
