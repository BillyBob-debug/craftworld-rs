# D96 — ThingColor on texture-sampler ports under the supplied instance color (2026-10-07)

Follows D86/D88 ([d86-thing-color.md](d86-thing-color.md),
[d88-thing-color-textured.md](d88-thing-color-textured.md)) and D87's port-1 scale
([d87-small-families.md](d87-small-families.md)). Route code only (`lbp-preview`); no
parser, disk-layout, shader, viewer or `linked.rs` change. Default behaviour (no
supplied color) is unchanged: the new rule runs only inside `project_instance_color`.

## Census first

The 14 unsupported 01.33 material locations whose THING_COLOR reason was "supplied
instance input only supports scalar alpha subtraction on sampler port 2"
(`b93-diffuse-route-shapes-0133.json`, by script; nodes local to the diffuse root):

| Shape (root first) | Locations | What feeds the sampler port | D96 outcome |
|---|---|---|---|
| `THING × TEX`, TEX port 1 = CONST | 2 (3d0460d4, e333131d) | static CONST (no ThingColor) | **instance-only** |
| `TEX × TEX`, both port 2 = THING (unswizzled) | 2 (40ad4c68 ×2) | ThingColor `.xy` subtract | **instance-only** |
| `TEX × THING`, TEX port 2 = CONST × THING.a | 2 (917025e9, dd042f2a) | scalar subtract | **instance-only** |
| two-texture alpha MIX, port 2 = CONST × THING.a | 2 (5320ea0f, 7779148c) | scalar subtract | folds; still out: D83 integer Repeat grid |
| same, port 2 = MAKE_FLOAT2(CONST × THING.a, CONST) | 3 (bbc56145, db8bdd5f, f8d5ad94) | float2 subtract | folds; still out: D83 integer Repeat grid |
| `TEX × (THING × animated TEX)`, port 2 = MIX(CONST4, CONST4, THING.a) | 1 (25ff1aa3) | float2 subtract | folds; out: nested instance tint (no parity) |
| ADD root with FRESNEL, TEX port 1 = CONST | 1 (b3dddc6d) | static CONST | folds; out: ADD under instance (D89) |
| `(THING + FRESNEL) × TEX`, TEX port 1 = CONST | 1 (e809d355) | static CONST | folds; out: FRESNEL/ADD |

## Rule (supplied-color opt-in only)

`instance_sampler::fold_sampler_ports`, called from `project_instance_color` for every
sampler in the diffuse ancestry with incoming wires, except the unchanged D80 shape
(one ThingColor `.a` on port 2, kept as the coordinate subtraction):

1. The sampler is subtype 0 with no named or keyed animation and UV0/UV1; otherwise
   "sampler port inputs need a static subtype-0 UV0/UV1 sampler".
2. Ports 0, 1, 2 each wired at most once; a port value is evaluated from static,
   unanimated, subtype-0 nodes only, source port 0, depth ≤ 8, ≤ 64 visits:
   projected ThingColor / COLOR (4 lanes), CONSTANT (scalar), CONSTANT2/3/4 (2/3/4
   lanes), MULTIPLY `l × r`, MIX `(r − l) × w + l` (w = port 2, else word 0),
   MAKE_FLOAT2 of scalar inputs (unwired lane = its word). A swizzle picks named lanes
   (`xyzw`/`rgba`); a swizzled scalar, vector inputs to MAKE_FLOAT2 and any other box
   type (FRESNEL, samplers, ADD, …) are explicit unsupported reasons.
3. The UV operand is a scalar broadcast to both axes or the first two lanes of a vector.
   With sampler scale `s`, offset `o` and ports `a` (add), `m` (multiply), `b`
   (subtract): `((uv × s + o) + a) × m − b` is regrouped as
   `uv × (s × m) + ((o + a) × m − b)`, each operation applied only when its port is
   wired. The result must be finite, within the UvTransform bounds, nonzero scale.
4. The derived sampler gets the folded scale/offset and loses its incoming wires;
   `InstanceProjection::sampler_port_folds` records sampler index, source bits, folded
   bits and each port's evaluated bits. The existing selectors then decide.
5. `select_instance_diffuse` refuses folds combined with the nested-tint rewrite
   (`folded_sampler`): "folded sampler port inputs under a nested instance tint".

Also: when the diffuse ancestry has no ThingColor, `project_instance_color` now returns
the original graph unprojected instead of refusing samplers with inputs, so D87's
port-1 scaled samplers keep their default route under the opt-in (D87 listed this as a
limitation).

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`, Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`: `resolve`,
  TEXTURE_SAMPLE subtype 0 without anim (≈196–225): UV scaled by words 0/1, offset by
  words 2/3, then `+ port0`, `* port1`, `- port2`, a value without a component suffix
  of width > 1 taken as `.xy`, a scalar as is; animated or subtype-1 samplers ignore
  ports. THING_COLOR is the 4-wide `iColor` (268–269); CONSTANT/2/3 widths (287–301);
  MULTIPLY (328–347) and MIX `((r − l) × w) + l` with wired or word-0 weight
  (388–409); MAKE_FLOAT2 (411–434: scalar inputs as is, wider inputs averaged or
  `luminance`, hence refused here); `getWithSwizzle` (83–96). Restated independently;
  no Toolkit code copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA` (01.33): Toolkit oracle rows for the 6 changed
  materials; `diffuse_parity --instance-white` recomputes each folded transform from
  the oracle's sampler, wire and constant rows (`oracle_sampler_fold`) and the tint
  from the white ThingColor: 0 mismatches.
- `UNKNOWN`: real game instance colors (D80 E10); whether game shaders round the
  per-pixel order differently (the regrouped transform differs from per-pixel
  evaluation by f32 rounding only; Toolkit's text output also prints constants with
  `%f`, the parity uses parameter bits).

Applicability: LBP2 PS3 GMAT graphs (branch 0) on 01.33; Vita refused (D77).

## Counts (selection census, material locations with MAP rows; not asset counts)

`diffuse_route_census` on 01.33, b93 → d96: unsupported 144 → 138, instance-color-only
247 → 253; texture 6,953, solid 619, unwired 10, animation-base-only 121, no_map_row 65
unchanged. Moved: 3d0460d4, e333131d, 40ad4c68 (×2), 917025e9, dd042f2a. Remaining
8 of the 14 now report the specific downstream reason (table above).

Corpus mesh census (4,261 mesh GUIDs, distinct catalog GUID, all-diffuse), b93 → d96:
`--instance-rgba 0.25,0.5,0.75,0.5` 3,899 → 3,902 (+3: 0001027c, 000106a0 by the
fold; 0004dd94 a D87 default all-diffuse mesh no longer refused under the opt-in);
`--animation-base --instance-rgba …` 3,980 → 3,983 (+3); default 3,669 → 3,669
(unchanged). Viewable 4,257 in all modes; Complete 0.

Newly viewable / newly Complete assets: +0 / +0 by default (opt-in only; supplied
color snapshots never count as Complete).

## Verification (PC, 2026-10-07)

- `cargo fmt --all --check` pass; `cargo test -q -p lbp-preview` 53 pass (3 new D96
  projection tests, 1 swizzle unit test, D80 alpha test updated for the fold);
  `cargo clippy -q -p lbp-preview --all-targets -- -D warnings` pass;
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer` pass.
- `diffuse_route_census` → `local-analysis/d96-diffuse-route-shapes-0133.json`.
- `run-material-oracle.ps1 -Diffuse` with `d96-manifest-{DATA,patch_0128}.json` (4/2)
  → `d96-oracle-*.json`; `diffuse_parity --instance-white` (schema 20,
  `instance_folded_samplers` per row) → `d96-parity-white-*.json`: 3/4 and 2/2
  projected, 0 mismatches (25ff1aa3 refused by rule 5); with `--animation-base` the
  same; default mode 0 projected, 0 mismatches.
- Regression `d96-regress-*.json`: D86 oracles `--instance-white` 5/5, 3/3; D88 and
  D92 oracles `--instance-white --animation-base` 0 mismatches; D87 oracles default
  and `--instance-white` 6/1/18 projected, 0 mismatches.
- `corpus_mesh_census` → `d96-corpus-{inst,both,default}.json`.

## Not tested

No viewer run or capture (no native windows); folded transforms feed the existing
scale/offset and product paths. Real instance colors, 01.00-only data and the two-
texture alpha MIX shapes on a non-integer grid were not examined further.
