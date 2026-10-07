# D88 — THING_COLOR with texture samples under the supplied instance color (2026-10-07)

Follows D86 ([d86-thing-color.md](d86-thing-color.md)); backlog rank 3
([backlog.md#thing-color](backlog.md#thing-color)). Route code only (`lbp-preview`);
no parser, disk-layout, shader or viewer change. Default behaviour (no supplied
color) is unchanged: both rules run only on a graph derived by `project_instance_color`.

## Census first

The 88 unsupported 01.33 materials with THING_COLOR in the diffuse ancestry
(`b87-diffuse-route-shapes-0133.json`, grouped by `thing_color_reason` and shape by
script):

| Group | Count | Decision |
|---|---|---|
| BLEND with a `texture × THING` input, or RGB mask (H3) | 36 + 2 under MUL | out: BLEND inputs, mask space and thresholds (D76/D84) |
| ThingColor wired into sampler ports other than scalar `.a` on port 2 (H2) | 14 | out: UV policy, no evidence |
| two or more animated samplers | 12 | out: D78 one-leaf rule |
| ADD in the ancestry (`THING × (THING + texture)` 3, others; incl. the variant-3 ThingColor) | 13 | out: ADD (D89) |
| **`THING × animated texture`** (`10,2,1+1`) | **7** | **this milestone** |
| **`texture × (THING × COLOR)`** | **1** | **this milestone** |
| other: THING × THING.a output, `(T×T) × (THING × T²)`, `T × (THING × T²)` | 3 | out |

## Rules (supplied-color opt-in only)

1. **Folded ThingColor operand** (`instance_fold::fold_constant_operand`). After the
   D86 root fold fails, a static diffuse root MULTIPLY with exactly two unswizzled
   inputs on ports 0/1 may have one input folded: a static MULTIPLY/MIX whose inputs
   pass the unchanged D86 constant rule (at least one projected ThingColor; COLOR,
   CONSTANT, one-component swizzles; no two scalars; MIX weight word 0 unwired) and
   whose only outgoing wire is the one into the root. The operand becomes a derived
   COLOR (`InstanceProjection::folded_operand`); the existing texture × COLOR,
   MIX and D75 selectors then decide. Out-of-range results keep the D86 reason.
2. **Animated texture × instance tint** (`instance_textured::select_instance_diffuse`).
   With both opt-ins (supplied color and animation base), when the existing selectors
   reject the derived graph, the D78 product search (`linked::select_animated_products`)
   is accepted only for exactly one animated sampler × a COLOR that is a projected
   ThingColor or the folded operand: root MULTIPLY, no second/third/fourth leaf, no
   nested product, no MIX/BLEND/sampler-scale state. A plain static COLOR tint keeps
   the D78 rejection ("needs 2+ textures"), so animation-base-only results do not move.
   The animated leaf passes the unchanged D56/D62 rule and `verify_global`.

Hooks outside the new code: `linked.rs` gains `select_animated_products` (the D78
search unchanged) and `binding` takes the optional projection; `lib.rs` exports
`select_instance_diffuse`. Rendering reuses the existing single-sample tint path with
the animation-base transform; the selection stays "supplied" and never Complete.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  `resolve`: THING_COLOR is the 4-wide `iColor` input (268–269); MULTIPLY (non-BUMP) is
  `l * r` with the wider width and no operand-type special case (328–347);
  TEXTURE_SAMPLE (150–267) is the same sample whether its consumer is a COLOR or a
  ThingColor product; COLOR/CONSTANT/MIX forms as cited in D86. The animated-sampler
  base rows keep the evidence of [animation-base-preview.md](animation-base-preview.md)
  and D78. Restated independently; no Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + update): 8 materials moved (below); the
  Toolkit oracle graph rows give the same multiply/sample/color indices and tint bits
  (white for THING_COLOR, oracle-row fold for the operand) for all 8.
- `UNKNOWN` (unchanged from D80 E10): the game value of `COLOR0`, its color space and
  alpha use; animation timing (base rows only).

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (material locations with MAP rows, selection only, not asset counts)

`diffuse_route_census` on 01.33, B87 → D88: unsupported 174 → 166, instance_color_only
236 → 244; all other classes unchanged (texture 6,949, solid 619, unwired 10,
animation-base-only 106, no_map_row 65). Moved: `THING × animated texture` 7
(1e2ff0af, 47cb733c, 6b2ad02c, 7114d2cc, b5cad3fb, d660c949, ea5d0739),
`texture × (THING × COLOR)` 1 (7aadc87a).

Corpus mesh census (distinct catalog GUID, 4,261 mesh GUIDs, all-diffuse):
default 3,665 → 3,665 (unchanged); `--instance-rgba 0.25,0.5,0.75,0.5` 3,894 → 3,895
(+1, baseline rerun on origin/main 2ef7fce: `d88-corpus-instance-rgba-base-main.json`);
`--animation-base --instance-rgba …` 3,954 (B87) → 3,963 (+9). Viewable 4,257 in all
modes; Complete 0.

Newly viewable / newly Complete assets: +0 / +0 by default (opt-in only; supplied
color and animation snapshots never count as Complete).

## Verification (PC, 2026-10-07)

- `cargo fmt --all --check` pass; `cargo test -q -p lbp-preview` 44 pass (2 new D88
  tests); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings` pass.
- `diffuse_route_census` SNAPSHOT `game-data/a1-disc;game-data/a1-update-0133`
  `d68-material-census-0133b.json` → `local-analysis/d88-diffuse-route-shapes-0133.json`.
- `scripts/run-material-oracle.ps1 -Diffuse` with manifests `d88-manifest-{DATA,
  patch_0106,patch_0120,patch_0128}.json` (4/1/2/1 samples) → `d88-oracle-*.json`.
- `diffuse_parity … --animation-base --instance-white` (flags now combinable; schema
  17 adds `instance_white_expected`, recomputed from oracle graph rows) →
  `d88-parity-both-*.json`: 4/1/2/1 projected, 0 mismatches. Default mode on the same
  oracles (`d88-parity-default-*.json`): 0 projected, 0 mismatches (routes unchanged
  without the opt-ins).
- Regression: D86 oracles `--instance-white` 5/5 and 3/3 projected, 0 mismatches
  (`d88-regress-d86-parity-*.json`); D87 oracles default 6/1/18, 0 mismatches
  (`d88-regress-d87-parity-*.json`).
- `corpus_mesh_census` → `d88-corpus-{instance-rgba,both,default}.json`; viewer
  builds `cargo build -q -p lbp-bevy-viewer -p lbp-viewer` pass.

## Synthetic tests (`instance.rs`)

THING × animated sampler selects only with both opt-ins (plain animation-base selector
and default unchanged; static COLOR not projected); animated MULTIPLY and two
animated leaves rejected; static sampler keeps the existing tint route. Operand fold:
texture × (THING × COLOR) tints with the folded value, source graph untouched, also
with an animated sampler; operand with a second consumer or a swizzled root input is
not folded; out-of-range reason kept.

## Not tested

No viewer run or capture (no native windows); the selections feed the existing
single-sample tint and animation-base paths. Real game instance colors, animation
playback timing and 01.00-only data were not examined.
