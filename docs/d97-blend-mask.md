# D97 BLEND mask colour space and two-texture BLEND — 2026-10-07

Follows [d94-blend.md](d94-blend.md) (threshold rule; mask space `UNKNOWN`),
[d84-blend-route.md](d84-blend-route.md) and [d76-blend.md](d76-blend.md). Backlog row
[blend](backlog.md#blend).

Question: **is a BLEND mask texture sampled raw (byte/255) or sRGB-decoded when the
factor is computed?**

Answer: **sRGB-decoded on the red channel** (alpha stays linear), as `STRONG_INFERENCE`
from community code plus local data. No executable investigation was needed or done.
BLEND now accepts red masks and two-texture shapes; on 01.33 this routes **9 BLEND
material locations under the supplied instance colour** (0 on the default route).

## Routes tried (AGENTS.md order)

1. **Documentation.** None found that states LBP2's RSX texture gamma setup.
2. **Open-source references** (`references/`, Toolkit pin
   `d6a2ef80b5a09999f08f23a803b516096055e5c9`, MIT; read only, nothing copied,
   translated or executed in the product). D84/D94 had only searched `lib/cwlib`; this
   time `lib/cwlib-gl` (Toolkit's OpenGL material baker) was included. Found (R1–R4 below).
3. **Texture headers / local data.** A local probe over all 01.33 material samplers and
   the D95 texture census (R5–R7 below).
4. **Executable / shader-program investigation:** not attempted. Routes 2–3 answered the
   question at the same evidence level the project uses for the threshold rule, and no
   decrypted executable exists locally. It remains the way to upgrade the label (below).

## Findings

| # | Finding | Label | Source |
|---|---|---|---|
| R1 | Toolkit's material baker uploads every texture as `GL_SRGB_ALPHA` unless the texture carries `noSRGB`, then as plain `GL_RGBA`. | `CONFIRMED_FROM_COMMUNITY_CODE` | `lib/cwlib-gl/src/main/java/cwlib/gl/objects/GlTexture.java`: constructor passes `texture.noSRGB` as `linear`; `Link` chooses the internal format from it. |
| R2 | The same baker renders the GfxAssembler shader (incl. the `BLEND` case: lerp by `mask.x`) with those textures bound per sampler slot, i.e. the BLEND factor reads the **sRGB-decoded** red of a non-noSRGB mask. Output goes to an sRGB framebuffer except for normal maps. | `CONFIRMED_FROM_COMMUNITY_CODE` | `lib/cwlib-gl/src/main/java/cwlib/gl/jobs/MaterialBaker.java`: shader from `GfxAssembler.generateBakedShaderSource`; per-slot `new GlTexture(textures[i])`; `SetSRGB(!isNormalMap)`. |
| R3 | `noSRGB` is set only from the TEX `BUMP` footer or GTF header flags bit 0 (`isBumpTexture`); a comment maps that bit to `BUMPTEX \| NOSRGB_TEX`. | `CONFIRMED_FROM_COMMUNITY_CODE` | `lib/cwlib/src/main/java/cwlib/resources/RTexture.java` (`process`), `structs/texture/CellGcmTexture.java` (flags), `enums/ResourceFlags.java` (`NOSRGB_TEX = 0x400`). |
| R4 | sRGB texture formats decode R, G and B; alpha is returned unchanged. | `CONFIRMED_FROM_DOCUMENTATION` | OpenGL 3.3 core specification, sRGB texture colour conversion (applies to R1/R2's `GL_SRGB_ALPHA`). |
| R5 | The per-texture marker is meaningful in the data: of 21,773 sampler references in the 01.33 material census (locations with MAP rows), samplers that feed only the BUMP output port are `no_srgb` in 5,962 of 6,000 references (99.4%); samplers that feed only DIFFUSE in 5 of 8,163 (0.06%). | `CONFIRMED_FROM_LOCAL_DATA` | Probe joined with `b96-texture-census-0133.json` (`decoded.no_srgb` = GTF flags bit 0 / TEX BUMP footer, D90/D95 decoder). |
| R6 | All 63 distinct BLEND mask textures (72 mask sampler references in 64 materials) decode without `no_srgb` (49 BC1, 14 BC3). Material texture descriptor flags are 0 on every one of the 21,773 references, so neither the texture nor the material marks a mask as linear. | `CONFIRMED_FROM_LOCAL_DATA` | Same probe. |
| R7 | The space matters: 26 of 63 masks have >5% of texels with red strictly between 0 and 255 (8 have >25%); only 10 are pure 0/255, where both readings agree. With the corpus thresholds 0/1 a red byte of 128 gives factor 0.216 decoded vs 0.502 raw. | `CONFIRMED_FROM_LOCAL_DATA` | Same probe (red-channel histograms of the decoded top mip). |
| R8 | Conclusion: the game samples BLEND masks like any other non-noSRGB texture, i.e. sRGB-decoded on R/G/B; the factor uses `m = decode(red)`, alpha masks `m = alpha/255`. | `STRONG_INFERENCE` | R1–R7. The texture unit has no knowledge of how the shader uses a channel; the only linear marker observed (R3/R5) is absent on every mask (R6). Not observed on hardware or in the executable. |

Conflicts: none found. The existing preview policy already treats colour samples as
sRGB-decoded (RGB lerp/multiply in linear space since D49/D58), so R8 makes the mask
consistent with that policy.

Applicability: LBP2 PS3 GfxMaterial branch 0, 01.33 census; textures without the
BUMP/noSRGB marker. A noSRGB mask texture (none in the corpus) would be read raw by the
same reasoning but is not special-cased: the decoded texture's `no_srgb` is not
consulted by the BLEND bake (0 users).

## Rule (implemented)

`select_blend` (`crates/lbp-preview/src/linked.rs`), `BlendDiffuseMask`,
`TextureBlendMask`, `BlendPort`, `blend_mask_value` (`crates/lbp-preview/src/blend_mask.rs`),
`product::pixels_blend_masked`.

- Mask wire swizzle: none, `r` or `x` → red, `m = sRGB-decode(byte)`; `a` or `w` →
  alpha, `m = byte/255`; anything else → `BLEND mask must be the red or alpha channel`.
- Factor and lerp unchanged from D94: `f = clamp((m − p0)/(p1 − p0), 0, 1)`,
  `out = port0 + (port1 − port0)·f`, RGB in linear space, alpha linear, nearest filtering.
- Inputs on ports 0/1: COLOR (static, unwired) or texture sample; mask: texture sample.
  Products/MIX/THING_COLOR under an input stay refused (`BLEND inputs must be COLOR or
  texture samples`); ports 0 and 1 must be different boxes.
- One distinct sample (COLOR + sample, mask = that sample): D84 per-texel bake, now
  with red or alpha.
- Two distinct samples (two samples with the mask reusing one, or COLOR + sample with
  the mask from another sample): baked on the D83 nearest Repeat grid (same UV
  channel, integer scales 1..16, zero offset; Repeat both axes at load) →
  `two-texture BLEND requires same-channel integer Repeat grid samplers` otherwise.
- Two COLORs with a mask sample, or three distinct samples → refused (no bake).
- Under the supplied-instance-colour opt-in, a ThingColor port is projected to COLOR
  by the existing D86/D88 projection and then follows the same rule.

Label: `BLEND <port0> to <port1> by mask red (sRGB-decoded) of texture N, thresholds p0/p1`
(two-texture) or `BLEND mask red (sRGB-decoded)|alpha of sample, …` (one texture).

## Counts (01.33, unit = material location with a MAP row; not asset counts)

`diffuse_route_census` b96 → d97: texture 6,953 → 6,953; solid 619; unwired 10;
animation-base-only 121; **instance-only 253 → 262**; **unsupported 138 → 129**; no MAP
row 65. Class changes: 9 locations (8 distinct material hashes), all THING_COLOR shapes
`BLEND(mask s1.red, ThingColor, s3)` (7) and `BLEND(mask s1.red, s2, ThingColor)` (2).
All 50 BLEND-root locations changed reason. The 41 still unsupported: 37 with a product
under an input (10 plain + 27 THING_COLOR), 3 with three distinct samples, 1 two-texture
material off the grid.

`corpus_mesh_census` (all 4,261 mesh GUIDs, unit = distinct catalog GUID): default
4,257 viewable / 3,669 all-diffuse / 0 Complete (= b96 baseline 3,669);
`--instance-rgba 0.25,0.5,0.75,0.5` 4,257 / **3,902 → 3,910 all-diffuse** / 0 Complete
(b96-corpus-inst.json → d97-corpus-inst.json). Newly viewable +0, newly Complete +0;
the +8 all-diffuse GUIDs exist only under a supplied (unverified) instance colour.

## Verification

- `cargo fmt --all --check`; `cargo test -q -p lbp-preview` (57 unit tests; new: red
  mask decode, two-texture texel/grid bake, selection of red/two-texture shapes and
  refusals); `cargo clippy -q -p lbp-preview --all-targets -- -D warnings`;
  `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`.
- `diffuse_route_census SNAPSHOT "game-data/a1-disc;game-data/a1-update-0133"
  local-analysis/d68-material-census-0133b.json local-analysis/d97-diffuse-route-shapes-0133.json`
  (release, from the main checkout) → counts above.
- Oracle: `scripts/run-material-oracle.ps1 -Diffuse -Archive <farc> -SampleManifest
  local-analysis/d97-manifest-<farc>.json -Output local-analysis/d97-oracle-<farc>.json`
  for all 50 BLEND locations (DATA.FARC and patch_0109/0111/0113/0114/0116/0118/0122/0125),
  merged into `d97-oracle-merged-a.json` (31) and `-b.json` (19).
  `diffuse_parity` (schema 21, new BLEND branch: root type/subtype, threshold words,
  port and mask wires/swizzles, COLOR words or the oracle's white ThingColor, second
  sampler): default a 31/0 mismatches/0 projected, b 19/0/0; `--instance-white`
  a 31/0/9, b 19/0/0. Regression: d96-oracle-DATA `--instance-white` 4/0/3,
  d83-alpha2 8/0/2, d88 `--animation-base --instance-white` 4/0/4, d78 38/0/0 (= before).
- Raw research output (uncommitted, ignored): `local-analysis/d97-research/`
  (`d97-mask-probe-0133.json`, probe source `d97_mask_probe.rs.txt`, census trial).

## Not tested / limitations

- No game capture: R8 is inferred, not observed. Upgrade path: targeted, read-only
  look at the user's executable for where texture control words get their gamma bits
  from the noSRGB flag (question unchanged from d94).
- No viewer QA of a BLEND material; the 9 routed locations are oracle-parity checked
  for structure only, not pixels.
- Products under BLEND inputs (37 locations) and three-sample BLEND need a three-texture
  bake; not implemented.
- Game filtering order (factor before/after filtering) remains `UNKNOWN`; nearest is
  the authored preview policy.
