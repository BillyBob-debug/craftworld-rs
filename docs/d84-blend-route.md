# D84 BLEND (box 16) diffuse route, phase 1 — 2026-10-07

Implements phase 1 of the rule proposed in [d76-blend.md](d76-blend.md), only as far
as the pinned Toolkit evidence supports. Backlog row [blend](backlog.md#blend).
Result on 01.33: **0 of the 14 BLEND-without-THING_COLOR materials become routable**;
all 50 BLEND-root materials now carry a BLEND-specific census reason instead of the
misleading "sampler must be static" fallback.

## Rule (implemented)

`select_blend` in `crates/lbp-preview/src/linked.rs` (called when the diffuse wire's
source is type 16, after the MIX branch); `BlendDiffuseMask` and `blend_factor` in
`crates/lbp-preview/src/blend_mask.rs`. Checks, in this order (first failure is the
census reason):

1. BLEND root static: subtype 0, no animated parameters → `BLEND root must be static subtype 0`.
2. Exactly three incoming wires on ports 0/1/2, source port 0, ports 0/1 unswizzled →
   `BLEND needs exactly unswizzled ports 0/1 and a mask on port 2`.
3. Ports 0 and 1 are COLOR (3) or TEXTURE_SAMPLE (1) leaves; products, MIX,
   THING_COLOR etc. → `BLEND phase 1 inputs must be COLOR or texture samples`.
4. Mask swizzle is exactly `a` or `w` (alpha) → otherwise `BLEND mask must be a
   sample alpha (RGB mask space unknown)`. Unswizzled masks read `.x` (red).
5. The mask comes from the port-0/1 sample → `BLEND mask must reuse the port 0/1 texture sample`.
6. Parameter words 0/1 equal f32 bits `3e19999a`/`3e23d70a` (0.15/0.16) →
   `BLEND thresholds other than 0.15/0.16`.
7. One COLOR and one sample → `BLEND phase 1 needs COLOR and one texture sample`
   (two- and three-sampler BLENDs are not implemented).
8. COLOR static, unwired, finite 0..1; the sampler passes the existing static UV0/UV1
   scale-offset/matrix rules.

Output = port0 + (port1 − port0) · f, f = clamp(100·m − 15, 0, 1), m = sample alpha
(byte/255, linear). Baked per texel: RGB lerp in linear space (sRGB-decoded sample,
COLOR as linear values), alpha linear, re-encoded to 8 bits; source pixels untouched;
4 Mpixel budget and cancellation like the alpha MIX bake. Displayed with nearest
sampling (`Asset::nearest_sampling`). BORDER wrap stays unsupported (only alpha MIX
may use the interior-BORDER path). Label: `BLEND mask alpha of sample, thresholds
0.15/0.16, color … on port n • nearest filtering`.

## Evidence

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (MIT), restated independently,
no code copied/adapted/translated. Paths under `lib/cwlib/src/main/java/cwlib/`.

| Rule | Label | Source |
|---|---|---|
| Type 16 = BLEND; three inputs by port; lerp port0→port1 by saturate(100·mask.x − 15) | `CONFIRMED_FROM_COMMUNITY_CODE` | `util/gfx/GfxAssembler.java` `resolve`, case `BoxType.BLEND`; `getWithSwizzle` (D76 E1–E3) |
| Literals 100/15 correspond to params 0.15/0.16 only; params not read by the generator | `CONFIRMED_FROM_COMMUNITY_CODE` (comment) / `STRONG_INFERENCE` | same case, author comment (D76 E5–E6) |
| Alpha mask is linear; RGB mask space unknown | `UNKNOWN` for RGB | D76 E10. Textures carry a `noSRGB` marker (`resources/RTexture.java` `process`, BUMP footer / `isBumpTexture`; `structs/texture/CellGcmTexture.java` flags bit 0); the flag name suggests other textures are sRGB-sampled, but the game sampler state is not shown → not used here. |
| Per-texel bake, nearest filtering, linear-space RGB lerp | authored preview policy | same as alpha MIX ([alpha-mix-preview.md](alpha-mix-preview.md)) |

Applicability: Toolkit LBP2 shader-generator path; no revision gate. Conflict: the
generator hardcodes one material's thresholds while boxes carry other values (below).

## Corpus facts (01.33, material locations with MAP rows, not asset counts)

Local probe over the 14 BLEND-without-THING_COLOR materials (counts only):
- Shapes: 4 have three direct texture samples (3 with a distinct mask sampler, 1 with
  the mask sampler reused on port 0); 10 have MULTIPLY products under a BLEND input.
- Mask swizzle: 14/14 unswizzled (red channel); 0 alpha masks.
- Threshold params (word0/word1): 13/14 = 0.0/1.0, 1/14 = 0.67/0.8; 0/14 = 0.15/0.16.
- Mask textures: all decoded `no_srgb = false` (BC1/BC3).

So the community literals 100/15 do not match any corpus parameter pair. With 0/1 the
parameter-curve reading (D76 E6) would give f = m (a plain mask MIX), while the
literals give a step at 0.15; the evidence does not decide between them, so these
stay unsupported. A future phase needs evidence for parameter use and RGB mask space.

## Counts

Census `diffuse_route_census` on 01.33 (8,094 material locations, same scope as D78):
before (d78) and after (d84) identical class counts — texture 6,938, solid 603,
unwired 10, animation-base-only 106, instance-only 228, unsupported 209, no MAP row 65.
Class changes: 0. Reason changes: 50 BLEND-root materials —
14 without THING_COLOR: 10 `inputs must be COLOR or texture samples`, 4 `mask must be
a sample alpha (RGB mask space unknown)`; 36 with THING_COLOR: `inputs must be COLOR or
texture samples`. The 2 materials with BLEND below a MULTIPLY are unchanged.
Newly viewable: +0. Newly Complete: +0 (no cohort changed; tested scope = the census).

## Verification

- `cargo fmt --all --check` — pass.
- `cargo test -q -p lbp-preview` — 27 unit tests pass (new: `blend_phase_one_selects_color_and_one_sample_stepped_by_its_alpha`
  covering D76 T1, T4–T8, T10; `blend_mask` factor/bake/threshold tests covering E-a–E-f).
- `cargo clippy -q -p lbp-preview --all-targets -- -D warnings` — pass.
- `cargo test -q --workspace` — pass (two `lbp-viewer` diagnostics tests failed on the
  first run in a fresh worktree because `local-analysis/` did not exist yet; pass on rerun).
- `python -m unittest discover -s tests` — 34 tests OK (1 skipped).
- `diffuse_route_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json local-analysis/d84-diffuse-route-shapes-0133.json`
  — counts above.
- Oracle/`diffuse_parity`: not run; no material changed class, so there is nothing to compare.
  `diffuse_parity` has no BLEND branch yet; add one when a corpus BLEND first selects.

## Not tested

No real material reaches the new route, so the load/bake path is covered only by
synthetic unit tests; no viewer QA or capture. Two/three-sampler BLEND grids, RGB
masks, other thresholds, BORDER with BLEND, and game-side BLEND math remain unsupported/`UNKNOWN`.
