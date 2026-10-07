# D87 — small "Other" diffuse families (2026-10-07)

Backlog rank 6 ([backlog.md#small](backlog.md#small), "about 66" materials). Route
code only (`lbp-preview`, new module `src/small_family.rs`); no parser, disk-layout or
shader change. THING_COLOR, BLEND and MIX shapes are untouched.

## Shapes in the D84b census (scripted grouping)

Grouped by `shape` and `reason` from `local-analysis/d84b-diffuse-route-shapes-0133.json`
(207 unsupported), excluding any shape that contains THING_COLOR (2), MIX (12) or
BLEND (16): 84 materials. Node notation `type.subtype+named-animations`; the reason is
the D84 route reason.

| Shape | n | D84 reason | D87 decision |
|---|---|---|---|
| CONST4 alone (`7.0+0`) | 9 | sampler must be static | **taken**: solid color |
| COLOR × COLOR (`10,3,3`) | 7 | diffuse multiply requires... | **taken**: solid color (product) |
| TEX with CONST on sampler port 1 (`1,4`) | 7 | sampler must be static | **taken**: folded UV scale |
| TEX with CONST2 on sampler port 1 (`1,5`) | 2 | sampler must be static | **taken**: folded UV scale |
| COLOR alone (`3.0+0`) | 3 | sampler must be static (solid route refused) | unsupported: components outside 0..1 (−0.0054, alpha 1.09); output clamping unknown |
| TEX alone (`1.0+0`, 3) / TEX × TEX (`10,1,1`, 1) | 4 | sampler must be static | unsupported: sampler UV channel raw 2; Toolkit's generator treats every value other than 1 as UV0, game meaning unknown |
| TEX alone animated (`1.1+2`), TEX × animated TEX / COLOR × animated TEX | 3 | sampler must be static | unsupported: named animation (animation-base family) |
| Any ADD (11) in the ancestry (two textures, texture + COLOR, larger trees) | 8 | sampler must be static / multiply rule | unsupported: additive composition has no render path; the COLOR + TEX case also has alpha −0.24 |
| Nested products with repeated leaves, 2+ animated leaves, CONST inside products, four-texture sharing mismatch, product × COLOR with COLOR-only inner multiply | rest | as listed | unsupported: unchanged earlier rules (rank 2 / other ranks) |

After D87, 59 of the 182 unsupported materials are outside THING_COLOR/BLEND/MIX
(largest: animated 2-texture product `10,1+1,1+1` 12; ADD-containing 8).

## Rules

1. **CONST4 alone.** The diffuse wire comes straight from a CONSTANT4 node (type 7):
   subtype 0, no named or keyed animation, no incoming wires, diffuse wire source
   port 0, no swizzle. Its parameters 0..3 are the RGBA solid color (same 0..1 finite
   guard as COLOR). Rendered by the existing solid path (`SolidSelection`,
   `color_index` = the CONST4 node).
2. **COLOR × COLOR.** A static MULTIPLY (subtype 0, unanimated) with exactly two
   inputs, one on port 0 and one on port 1, both from unwired unanimated COLOR nodes,
   source port 0, no swizzle. Each COLOR must pass the 0..1 guard; the color is the
   component-wise product, each component one f32 multiply. `color_index` is the
   multiply, `product_of` the two COLOR operands (port 0, port 1). Rendered by the
   existing solid path; the summary label says "(COLOR × COLOR product)".
3. **Sampler port-1 UV scale.** The diffuse wire comes straight from a subtype-0,
   unanimated TEXTURE_SAMPLE on UV0/UV1 whose only incoming wire is on port 1 from an
   unwired unanimated CONSTANT (4, scalar used for both axes) or CONSTANT2 (5, X/Y),
   source port 0, no swizzle, finite factor. The sampler's own scale/offset
   `(sx, sy, ox, oy)` and factor `(fx, fy)` are regrouped as
   `uv × (sx·fx, sy·fy) + (ox·fx, oy·fy)` (each product one f32 multiply) and fed to
   the existing single-texture scale/offset path. `DiffuseSelection::sampler_scale`
   keeps the CONST index, factor bits and the unfolded source bits. Ports 0 and 2 and
   every other input combination are still rejected by the unchanged sampler rule.

D87 solids are applied only to the original graph (`select_small_solid`), never to an
instance-color derived graph, so THING_COLOR × COLOR materials do not move into the
instance-color class.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, method `resolve`:
  CONSTANT4 yields a float4 of parameters 0..3 (no animation lookup); COLOR without a
  named animation yields a float4 of parameters 0..3; MULTIPLY outside the BUMP port
  multiplies its port 0 and port 1 values component-wise; for a subtype-0 sampler
  without named animations the UV is scaled/offset by its parameters, then the port-0
  input is added, the port-1 input multiplies, the port-2 input subtracts (via
  `getWithSwizzle`); CONSTANT yields a scalar and CONSTANT2 a float2. Box numbering
  from `cwlib/enums/BoxType.java` (CONSTANT 4, CONSTANT2 5, CONSTANT4 7, MULTIPLY 10).
  Rules above are independent statements; no Toolkit code copied, translated or
  executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: Toolkit oracle (`run-material-oracle.ps1 -Diffuse`)
  over all 25 newly selected materials (3 archives) and `diffuse_parity`
  (schema 15): 25 compared, 0 mismatches, 25 projected (9 with `sampler_scale`,
  7 with `product_of`, 16 solid). Parity recomputes the COLOR product and the folded
  transform independently from the oracle's parameters and wiring.
- Regression: `diffuse_parity` on `d75-oracle-DATA-1.json`: 48 compared, 0 mismatches.

## Applicability and limitations

- Toolkit's generator is a community reconstruction; game-GPU output is not verified.
  The regrouped UV transform can differ from `((uv·s)+o)·c` by f32 rounding.
- Solid colors are not clamped; out-of-range COLOR/CONST4 stay unsupported.
- With a supplied instance color, `project_instance_color` still rejects any sampler
  input other than its own port-2 alpha subtraction, so a port-1-scaled sampler is
  unsupported in that opt-in mode (same outcome as before D87).
- The texture summary label shows the folded transform, not the CONST separately.

## Counts (01.33, selection census, material locations with MAP rows; not asset counts)

`diffuse_route_census` d84b → d87: unsupported 207 → 182; solid_color 603 → 619 (+16);
texture_route 6,940 → 6,949 (+9); instance-color 228, animation-base 106, unwired 10
unchanged. Newly viewable / newly Complete meshes: not measured (mesh census not run);
Complete requires a visual check.

## Verification

- `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (36 pass, 3 new tests);
  `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`;
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `cargo test -q --workspace`
  (pass; lbp-viewer diagnostics tests need an existing `local-analysis/` directory).
- `diffuse_route_census SNAPSHOT GAME-DIRS d68-material-census-0133b.json
  local-analysis/d87-diffuse-route-shapes-0133.json`.
- `scripts/run-material-oracle.ps1 -Diffuse -Archive <farc> -SampleManifest
  local-analysis/d87-oracle-manifest-{0,1,2}.json -Output
  local-analysis/d87-diffuse-oracle-{0,1,2}.json` (DATA.FARC 6, patch_0115 1,
  patch_0128 18), then `diffuse_parity ... local-analysis/d87-diffuse-parity-{i}.json`:
  6/1/18 compared, 0 mismatches.
- Not tested: viewer captures (no native windows), mesh-level corpus census, python
  unittest suite (no Python change).
