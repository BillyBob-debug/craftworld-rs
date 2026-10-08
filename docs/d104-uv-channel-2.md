# D104 — sampler UV channel word 2 (2026-10-08)

Backlog item "UV channel raw 2 (4)" left open by [D89](d89-add-uv2.md). Question: which
mesh coordinates does a TEXTURE_SAMPLE whose parameter 4 (channel word) is 2 sample?
D89 named two candidates: UV0 (Toolkit generator fallback) or the mesh's third UV
attribute (which Toolkit treats as decal UV). **Answer: UV0.** The game's own compiled
fragment programs fetch word-2 samplers from the same interpolant components as word-0
samplers. The third attribute is never used for a fetch.

## Rule

`lbp_preview::sampler_uv_channel(word)` (`crates/lbp-preview/src/uv_channel.rs`):
0 -> UV0, 1 -> UV1, **2 -> UV0**, any other word -> unsupported (none observed). Every
place that checked `word > 1` now uses it: `linked::sampler`, `animation_base`,
`instance_sampler::fold_sampler_ports`, `small_family` (sampler UV scale).
`SamplerSelection.uv_channel` holds the mapped mesh attribute, so the UV0 data and the
grid rules (same-channel products, ADD, MIX) treat word 2 like word 0. `selected_uv`, the
mesh UV parser and both viewers are unchanged (viewers receive already-selected UV
arrays). Refusal texts are unchanged ("sampler must be static, unwired UV0/UV1").
`diffuse_parity` compares the oracle's raw word through the same mapping.

## Evidence

| # | Finding | Label | Source |
|---|---|---|---|
| E1 | Generator: TEXTURE_SAMPLE uses `iUV.zw` when parameter 4 is 1 and `iUV.xy` for every other value; reflection-port samplers ignore the word. | `CONFIRMED_FROM_COMMUNITY_CODE` | Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9`, `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java`, `resolve`, case `TEXTURE_SAMPLE` (as in D89). |
| E2 | Conflict: Toolkit's glTF exporter passes parameter 4 unchanged as the glTF `texCoord`, so word 2 would export as `TEXCOORD_2` (third attribute). `MaterialBaker`/`SceneExporter` feed the last attribute as decal UV. Neither is a statement about game shading. | `CONFIRMED_FROM_COMMUNITY_CODE` (conflict) | Same pin, `lib/cwlib/src/main/java/cwlib/io/exports/MeshExporter.java` `createMaterial` (`setTexCoord(channel)`); `lib/cwlib-gl/.../jobs/MaterialBaker.java`. The fork in `references/lbp_bgm` additionally maps word 257 to `zw`. |
| E3 | Corpus words (01.33, 7,980 distinct material hashes): sampler boxes with word 0: 17,427; word 1: 4,016; word 2: 9 (9 materials); none above 2. | `CONFIRMED_FROM_LOCAL_DATA` | `uv_channel_probe` (below). |
| E4 | Each GfxMaterial's shader blob holds 10 `CGB` programs whose ucode is stored as pairs of 16-bit indices into a trailing table of 8-byte halves (instruction = two halves, 32-bit words halfword-swapped, NV40-class fragment encoding). Decoded: 71,775 of 71,820 programs. | `CONFIRMED_FROM_LOCAL_DATA` | Read-only decode of the user's 01.33 material blobs; field layout from the public NV30/NV40 fragment-program encoding (Mesa nvfx documentation). Interpolant use matches Toolkit's `brdf.cg` template (TEXCOORD0 UV pair, 1 tangent, 2 normal, 3 decal UV with fog in `.z`, 4 eye vector). |
| E5 | Data flow from interpolant components to each texture fetch (fetch unit = texture slot): word 0 -> `TEXCOORD0.xy` in 140,222 fetches (6,913 materials); word 1 -> `TEXCOORD0.zw` in 35,163 (2,291 materials); **word 2 -> `TEXCOORD0.xy` in all 54 fetches of the 6 materials whose word-2 sampler is UV-addressed** (`generated_paint`, `wheel_tricycle_valve`, `vin_metal_dull_crane`, `princesses2_rapunzel_to_dress`, `marv_midbuild_winref01`, `pressure_switch_plastic`). No word mixes components; no fetch reads `TEXCOORD3` (decal UV) as coordinates. The other 3 word-2 materials (`gravity_alias_he`, `_he_2sided`, `_to`) use the sampler only on the reflection port (output port 6), addressed from normal/eye vectors. Remaining fetches are computed (bump-perturbed reflection etc.: word 0 1,892, word 1 343) or lightmap/eye (word 0 33). | `CONFIRMED_FROM_LOCAL_DATA` | `fpdec.py`/`texflow.py`/`shaderscan.py` on the blobs; summary `local-analysis/d104-uv-channel/shader-fetch-scan.json`. |
| E6 | Inference: word 0 and word 2 compile to the same coordinates, and word 0 is the mesh's first UV attribute (D40/D44 oracle and mesh UV parity), so word 2 samples mesh UV0. The vertex program that fills `TEXCOORD0` was not decoded. | `STRONG_INFERENCE` | E5 plus D40/D44. |
| E7 | Mesh side: 4,242 decodable mesh hashes have 1 (11), 2 (1,737) or 3 (2,494) UV attributes. All 7 mesh primitives bound to word-2 materials have 3 attributes; the third is a separate unwrap (in 0..1 for costumes, an atlas region starting near 0.004, i.e. the decal UV), different from UV0 and UV1 on every vertex. | `CONFIRMED_FROM_LOCAL_DATA` | `uv_channel_probe`. |
| E8 | Texture fit is not decisive and argues against "third attribute": rasterising each primitive's triangles per channel over the word-2 texture, the third attribute fits none (rapunzel normal map and marv window bump fit UV0; the valve texture and the crane AO map fit UV1 best). The valve and crane therefore preview their textures at UV0 as the game does, which may look like a stretched sliver; that is the authored game behavior per E5, not a preview choice. | `CONFIRMED_FROM_LOCAL_DATA` (coverage metrics); visual appearance `UNKNOWN` | `uvcov.py`, `local-analysis/d104-uv-channel/probe-0133b.dump/coverage.json` and overlays. |

Applicability: LBP2 PS3 GfxMaterial branch 0 on 01.33 (disc + patches). Words above 2
and other games/branches are unobserved and stay unsupported. Raw decoded shader
listings are research material and were not committed; only the independent finding
is used. No Toolkit or game code is copied, translated or executed in the product.

## Counts (01.33; selection census = material locations with MAP rows, not assets)

`diffuse_route_census` b102 -> d104: unsupported 53 -> 49; texture_route 6,989 -> 6,993
(+4: `generated_paint.gmat` x2, `wheel_tricycle_valve.gmat`, `vin_metal_dull_crane.gmat`);
solid 620, animation-base 126, instance-color 296, unwired 10 unchanged.

`corpus_mesh_census` default (distinct catalog GUIDs, all 4,261 mesh GUIDs): viewable
4,257 -> 4,257 (+0 newly viewable); all-diffuse 3,695 -> 3,695 (+0; the two meshes
`000150dc` wheel_tricycle and `00003d2b` vin_crane_machine each gain one supported
group, 1 -> 2 of 3, but keep other unsupported groups); supported groups 8,562 -> 8,564.
Newly Complete: 0 (no visual check).

## Verification

- `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (69 pass; new
  `uv_channel` test, updated ADD/linked/animation-base/small-family cases now use word 3
  as the unobserved refusal); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`;
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`; `cargo test -q --workspace
  --no-fail-fast` passes except the 2 known `lbp-viewer` diagnostics tests that need an
  existing `local-analysis/` in the worktree.
- `diffuse_route_census local-analysis/d68-inventory-0133-verified.json
  "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json
  local-analysis/d104-diffuse-route-shapes-0133.json` (release, from the main checkout).
- `scripts/run-material-oracle.ps1 -Diffuse -SampleManifest
  local-analysis/d104-oracle-manifest-0.json -Output local-analysis/d104-diffuse-oracle-0.json`
  (DATA.FARC, 3 locations) and `-Archive game-data/a1-update-0133/USRDIR/patches/patch_0111.farc`
  with manifest/output `-1` (1 location); `diffuse_parity` -> `d104-diffuse-parity-{0,1}.json`:
  3 and 1 compared, 0 mismatches, all projected.
- Regressions: `diffuse_parity` on `d75-oracle-DATA-1.json` 48/0/41 and
  `d89-diffuse-oracle-0.json` 5/0/4 (compared/mismatches/projected).
- `corpus_mesh_census ... local-analysis/d104-corpus-default.json` (247 s).
- Research: `uv_channel_probe SNAPSHOT GAME-DIRS MATERIAL-CENSUS NEW-JSON [--blob-dir
  NEW-DIR]` (new example) -> `local-analysis/d104-uv-channel/probe-0133{,b,c,d}.json`
  (+ `.dump/` texture/triangle dumps); shader blobs went to the session scratch
  directory only; Python decode/scan scripts kept under
  `local-analysis/d104-uv-channel/scripts/` (ignored).
- Not run: mesh oracle / `mesh_parity` (mesh UV parsing unchanged), opt-in corpus modes,
  python unittest suite (no Python product change), viewer captures (no native windows).

## Not tested / open

Visual correctness of the four newly selected materials (valve and crane may show a
small texture region, see E8); the vertex program feeding `TEXCOORD0`; words above 2;
what the third mesh attribute feeds besides decals/fog.
