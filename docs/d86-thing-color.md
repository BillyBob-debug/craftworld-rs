# D86 — THING_COLOR constant fold under the supplied instance color (2026-10-07)

Implements the parts of the D80 proposal ([d80-thing-color.md](d80-thing-color.md))
that the 01.33 census and Toolkit evidence support. Backlog rank 3
([backlog.md#thing-color](backlog.md#thing-color)). Route code only (`lbp-preview`);
no parser, disk-layout, shader or viewer change. Default behaviour (no supplied
color) is unchanged: the fold runs only inside `project_instance_color`.

## Census first (D80 step 1)

`diffuse_route_census` (schema 3) adds `thing_color_reason` per unsupported material
whose diffuse ancestry holds box 2, plus `thing_color_reason_counts` and
`thing_color_node_counts` (subtype, parameter words, outgoing `type:port[swizzle]`).
Run on 01.33 before the route change (`d86-pre-...`, code otherwise = D84b),
96 of the 207 unsupported materials contain THING_COLOR:

| Bucket | Count | Owner |
|---|---|---|
| projected, BLEND inputs / mask rules (H3) | 27 + 9 | D76/D84 BLEND |
| projection: ThingColor or other wire into sampler ports / non-`a` (H2) | 14 | out of scope (UV policy) |
| projected, "diffuse multiply requires two direct textures or one texture and static COLOR" | 14 | 6 are all-constant roots (this milestone), 8 deeper products |
| projected, sampler static/unwired rule | 11 | animated/wired samplers |
| projected, animation-base rules (one animated sampler, 2+ textures) | 9 + 7 | rank 2 animation |
| projected, static MIX rule | 2 | all-constant MIX (this milestone) |
| projected, output static/unswizzled | 1 | THING × THING.a; blocked by the output wire, stays |
| projection: ThingColor not variant 8 (word 0 = 3) (H1) | 1 | stays; one material, no evidence |
| projected, product × COLOR rule | 1 | deeper product |

All other ThingColor nodes are subtype 0 with words `[8,0,0,0,0,0,0,0]`.
Swizzled ThingColor wires (`.a` into MULTIPLY/MIX) occur almost only inside graphs
already blocked by H2/H3, so the general D80 step 2 substitution would move nothing
on its own; swizzles are handled inside the fold below.

## Rule

Under the supplied-color opt-in, after ThingColor nodes become derived COLORs, a
diffuse root that is a static (subtype 0, unanimated) MULTIPLY (10) or MIX (12)
with exactly two inputs on ports 0 and 1 is folded into one derived COLOR when:

- at least one input is a projected ThingColor;
- every input is an unwired static COLOR (incl. projected ThingColor) or CONSTANT
  (4), source port 0, swizzle either empty or one component (`x y z w` / `r g b a`)
  followed by NULs;
- not both operands are scalars (CONSTANT or one-component swizzle);
- MIX has no port-2 wire (weight = parameter word 0 as f32).

Value: MULTIPLY `l × r` component-wise; MIX `((r − l) × w) + l`, l = port 0,
r = port 1; a scalar operand is broadcast to four components. The result must be
finite and within 0..1 (`DiffuseTint::validate`), otherwise the explicit reason
"Unsupported: folded supplied instance color outside finite 0..1 preview bounds".
A non-matching shape is left untouched, so existing selectors keep their reason.
`InstanceProjection` gains `folded_root` and `swizzled_color_edges`
(ThingColor node, swizzle byte). Code: `crates/lbp-preview/src/instance_fold.rs`.
The material stays "supplied" (never Complete); the label keeps "Supplied instance
RGBA … (unverified)".

Not changed: ThingColor into sampler ports (H2), variant ≠ 8 (H1), nested or
deeper constant subgraphs, wired MIX weights, multi-component swizzles, two-scalar
results, and BLEND/animated compositions (their own milestones).

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  `resolve` THING_COLOR is the 4-wide `iColor` input (lines 268–269); COLOR is a
  constant float4 of words 0..3 (270–285); CONSTANT is the scalar word 0, width 1
  (287–289); MULTIPLY (non-BUMP) is `l * r` with the wider width (328–347); MIX is
  `((r - l) * f) + l` with f = word 0 unless port 2 is wired (388–405);
  `getWithSwizzle` appends the wire's swizzle and sets width to its length (84–96).
  Scalar-to-vector broadcast follows Cg/GLSL scalar promotion. Restated
  independently; no Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33, disc + update): 8 materials moved (below);
  Toolkit oracle graph rows give the same folded bits for all 8.
- `UNKNOWN` (unchanged from D80 E10): the game value of `COLOR0`, its color space
  and alpha use. The census uses supplied white, which only exercises the fold.

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (material locations with MAP rows, selection only, not asset counts)

`diffuse_route_census` on 01.33, D84b → D86: unsupported 207 → 199,
instance_color_only 228 → 236; all other classes unchanged (texture 6,940, solid
603, unwired 10, animation-base-only 106, no_map_row 65). Moved shapes: COLOR ×
THING 4, MIX(COLOR, THING) 2, CONSTANT × THING 1, THING × THING 1.
Newly viewable / newly Complete assets: +0 / +0 by default (opt-in only; supplied
color never counts as Complete).

## Verification (PC, 2026-10-07)

- `cargo fmt --all --check` pass; `cargo test -q -p lbp-preview` 37 pass (3 new D86
  tests); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings` pass.
- `diffuse_route_census` SNAPSHOT `game-data/a1-disc;game-data/a1-update-0133`
  `d68-material-census-0133b.json` → `local-analysis/d86-diffuse-route-shapes-0133.json`
  (pre-change bucket run: `d86-pre-diffuse-route-shapes-0133.json`).
- `scripts/run-material-oracle.ps1 -Diffuse`: disc DATA.FARC 5 samples
  (`d86-oracle-disc.json`), `patch_0128.farc` 3 samples (`d86-oracle-patch_0128.json`).
- `diffuse_parity … --instance-white` (new flag; recomputes the fold from oracle
  graph rows with THING_COLOR = white): disc 5/5 folded, 0 mismatches; patch_0128
  3/3, 0 mismatches. Default mode on the same oracles: 0 mismatches, 0 projected
  (routes unchanged without the opt-in).

## Synthetic tests (`instance.rs`)

COLOR × THING folds only with a supplied color (default selectors unchanged, source
graph untouched, wire-order independent); CONSTANT broadcast; out-of-range reason;
animated root, wired operand, no ThingColor operand and third input are not folded;
`.a` broadcast and `x`≡`r`; `xy`, `ax`, `q`, non-NUL tail rejected; two scalars not
folded; THING × THING squares; MIX parameter weight, wired weight not folded, NaN
weight rejected. `instance_fold.rs` covers the swizzle parser.

## Not tested

No viewer run or capture (no native windows); the fold feeds the existing solid
route, whose rendering is unchanged. Bevy/legacy viewer builds were not rebuilt in
this session. 01.00-only data not re-censused. Game fidelity of any supplied color
remains unverified.
