# D89 — diffuse ADD shapes and sampler UV channel raw 2 (2026-10-07)

Backlog rank 6 ([backlog.md#small](backlog.md#small)): the D87 leftovers "ADD-containing
8" and "UV channel raw 2 (4)". Route code only: new module
`crates/lbp-preview/src/add.rs`, small hooks in `linked.rs` (selection field, ADD root
dispatch, pixel/grid bake branches, label, instance guard), `product::extent` made
`pub(crate)`. No parser, disk-layout or shader change. THING_COLOR, BLEND and MIX
shapes are untouched.

## Shapes in the B87 census (scripted grouping)

From `local-analysis/b87-diffuse-route-shapes-0133.json` (174 unsupported), shapes with
an ADD (box 11) and no THING_COLOR (2), MIX (12) or BLEND (16): 8 material locations,
7 distinct materials. Graphs were dumped with a temporary local example (not committed).

| Material (MAP path) | Diffuse shape | Locations | D89 decision |
|---|---|---|---|
| `oddworld_munch_base.gmat`, `oddworld_munch_ft.gmat` | ADD(TEX, TEX), UV0, integer scales (1/4, 1/3), Repeat | 2 | **taken**: two-texture grid sum |
| `starhawk_outlaw_rope.gmat` | ADD(TEX × COLOR, COLOR), TEX scale 6 | 2 (patch_0113, patch_0114) | **taken**: per-pixel bake |
| `oddworld_munch_he/hd/to.gmat` | ADD(TEX, TEX × TEX) | 3 | unsupported: three-texture sum needs a new bake or shader |
| `mesh_football_goal_net.gmat` | ADD(COLOR, subtype-1 TEX) | 1 | unsupported: COLOR alpha −0.236 outside 0..1 |

ADD shapes containing THING_COLOR (9 more locations) stay out of scope; they now report
an ADD-specific reason instead of "sampler must be static".

UV channel raw 2: `generated_paint.gmat` (2 locations; TEX alone),
`wheel_tricycle_valve.gmat` (TEX alone), `vin_metal_dull_crane.gmat` (TEX × TEX, second
sample on raw 2). Decision: **unsupported, documented** (below).

## Rules (ADD)

1. **ADD root.** The diffuse wire comes from an ADD (11) that is subtype 0, unanimated,
   and has exactly two incoming wires, on ports 0 and 1, both source port 0 and
   unswizzled. Otherwise: "ADD needs a static node with unswizzled inputs on ports 0
   and 1".
2. **Two-texture sum.** Both inputs are distinct static texture samples accepted by the
   existing sampler rule (unwired, unanimated, UV0/UV1). Port 0 is `sample_index`,
   port 1 `second_sample`. They must use the same UV channel and integer scales 1..16
   with zero offset (the D49/D83 product grid), and Repeat on both textures at load
   time; otherwise unsupported. The grid pixel is `linear(a) + linear(b)` per RGB
   channel (sRGB decode/encode as in the product bake), alpha `a + b`, each clamped to
   0..1 at 8-bit encoding; nearest grid sampling.
3. **Texture (× COLOR) + COLOR.** One input is an unwired, unanimated COLOR within 0..1
   (the addend). The other is either one static sample, or a static MULTIPLY whose
   ports 0/1 are one static sample and a different unwired unanimated COLOR within
   0..1 (the tint). Each pixel becomes `linear(sample) × tint + addend` per RGB
   channel and `a × tint.a + addend.a` for alpha, clamped at encoding; the existing
   single-texture transform/wrap path is unchanged.
4. **Other ADD forms** are explicit errors: texture + two-texture product ("needs a
   three-texture bake (no render path)"), COLOR + COLOR, nested ADDs and anything
   else ("ADD supports two texture samples, or a texture (optionally × COLOR) plus
   COLOR").
5. **Instance color.** D89 applies to the original graph only. Under a supplied
   instance color the loader refuses an ADD selection, and `diffuse_route_census`
   does not count an instance-derived ADD as instance-color only (one THING_COLOR ADD
   material would otherwise have moved class).

Clamping and linear-space summation are authored preview policy (consistent with the
product/MIX bakes), not verified game-GPU behavior.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, method `resolve`,
  case `BoxType.ADD`: the generated value is the port-0 input plus the port-1 input
  (via `getWithSwizzle`), result width the wider of the two; with only port 0 wired
  it passes port 0 through, with neither it throws. MULTIPLY (non-BUMP) and COLOR as
  in D87. Box numbering `cwlib/enums/BoxType.java` (ADD 11). Independent statements;
  no Toolkit code copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: Toolkit oracle (`run-material-oracle.ps1 -Diffuse`) on
  all 7 ADD locations of the three archives holding the newly selected materials
  (patch_0109 5, patch_0113 1, patch_0114 1); `diffuse_parity` schema 17 re-reads the
  ADD root, its two port wires, the samplers and both COLORs from the oracle rows:
  7 compared, 0 mismatches, 4 projected (2 two-texture, 2 TEX × COLOR + COLOR), the
  3 three-texture ADDs unsupported with route equal.
- Regression: `diffuse_parity` on `d75-oracle-DATA-1.json`: 48 compared, 0 mismatches.

## UV channel raw 2: not supported

- `CONFIRMED_FROM_COMMUNITY_CODE` (same pin): `GfxAssembler.resolve`, case
  `TEXTURE_SAMPLE`, picks the `zw` half of `iUV` when parameter 4 is 1 and the `xy`
  half for every other value, so the Toolkit generator renders raw 2 as UV0. The
  generated shader (`lib/cwlib/src/main/resources/shaders/templates/cg/brdf.cg`) has
  only `iUV` (float4, TEXCOORD0: UV0 and UV1) and a separate `iDecalUV` (TEXCOORD3);
  `cwlib-gl` `MaterialBaker`/`MultiMaterialBaker` and `SceneExporter` feed UV0, UV1
  and the mesh's last attribute as the decal UV.
- `CONFIRMED_FROM_LOCAL_DATA`: the two meshes that use raw-2 materials
  (`wheel_tricycle.mol` valve primitive, `vin_crane_machine.mol` primitive 2) both have
  three UV attributes; the third varies per vertex (valve 37 distinct of 64 vertices,
  range about 1.79..1.91 × 0..0.16; crane 0..4 × 0..1.68) and differs from UV0 on
  every vertex. No mesh in the snapshot uses `generated_paint.gmat`.
- Conflict, `UNKNOWN`: raw 2 can mean "UV0" (Toolkit's fallback) or "the third mesh
  attribute" (which the meshes provide, and which Toolkit treats as decal UV). The
  choice changes which texels appear; no game evidence or visual check separates
  them. The sampler rule keeps rejecting raw values above 1 ("sampler must be static,
  unwired UV0/UV1"), and `selected_uv` still refuses channels above 1. Resolving it
  needs a game-side reference (capture or executable shader/vertex-layout evidence).

## Counts (01.33; selection census = material locations with MAP rows, not assets)

`diffuse_route_census` b87 → d89: unsupported 174 → 170; texture_route 6,949 → 6,953
(+4); solid 619, instance-color 236, animation-base 106, unwired 10 unchanged.

`corpus_mesh_census` default (distinct catalog GUIDs, all 4,261 mesh GUIDs):
viewable 4,257 → 4,257 (+0 newly viewable); all-diffuse 3,665 → 3,669 (+4:
`00032edd` oddworld_munch_ft, `000399a9`/`000399ad`/`000399b5` starhawk outlaw
he/lg/to); supported groups 8,525 → 8,529. Newly Complete: 0 (needs the owner's
visual check; no captures taken).

## Verification

- `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (45 pass, 3 new in
  `add.rs`); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`;
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `cargo test -q --workspace`
  (pass except the 2 lbp-viewer diagnostics tests that need an existing
  `local-analysis/` directory, absent in the worktree).
- `diffuse_route_census SNAPSHOT GAME-DIRS d68-material-census-0133b.json
  local-analysis/d89b-diffuse-route-shapes-0133.json` (the earlier
  `d89-diffuse-route-shapes-0133.json` predates rule 5: instance-only 237).
- `scripts/run-material-oracle.ps1 -Diffuse -Archive <patch_0109|0113|0114>
  -SampleManifest local-analysis/d89-oracle-manifest-{0,1,2}.json -Output
  local-analysis/d89-diffuse-oracle-{0,1,2}.json`, then `diffuse_parity ...
  local-analysis/d89-diffuse-parity-{i}.json`: 5/1/1 compared, 0 mismatches.
- `corpus_mesh_census SNAPSHOT GAME-DIRS local-analysis/d89-corpus-default.json`.
- Not tested: viewer captures (no native windows), visual correctness of the clamped
  sums, opt-in corpus modes (`--animation-base`, `--instance-rgba`), python unittest
  suite (no Python change).
