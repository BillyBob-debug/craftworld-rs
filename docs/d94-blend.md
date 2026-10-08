# D94 BLEND (box 16) parameters and mask space — 2026-10-07

Follows [d76-blend.md](d76-blend.md) (research) and [d84-blend-route.md](d84-blend-route.md)
(phase 1, 0 users). Backlog row [blend](backlog.md#blend). Question: how does BLEND use
its two parameters and an unswizzled (red) mask, and can the real 01.33 shapes route?

Result: **the parameter rule is resolved; the RGB mask space is not.** BLEND now accepts
any finite threshold pair p0 < p1. All 14 BLEND-without-THING_COLOR materials (and the 36
with THING_COLOR) stay unsupported because every corpus mask is an unswizzled RGB
channel. Classes unchanged; all 50 BLEND-root locations now report the real blocker.

## Rule (implemented)

factor f = clamp((m − p0) / (p1 − p0), 0, 1), output = port0 + (port1 − port0) · f, where
p0/p1 are box parameter words 0/1 read as f32 and m is the first mask component.
Accepted when p0, p1 are finite, p0 < p1 and 1/(p1 − p0) is finite in f32; otherwise
`Unsupported: BLEND thresholds must be finite with param0 < param1`.
`blend_thresholds`/`blend_factor` in `crates/lbp-preview/src/blend_mask.rs`;
`select_blend` in `crates/lbp-preview/src/linked.rs`. Everything else in the D84 route
(COLOR + one sample, alpha mask of that sample, per-texel bake, nearest filtering) is
unchanged. The label now prints the thresholds (`thresholds 0/1`, `0.66999996/0.79999995`).

Check order change: the mask-swizzle check (RGB unknown) and the threshold check now run
before the input-kind check, so a BLEND with an RGB mask reports the mask first.

## Evidence

| # | Finding | Label | Source |
|---|---|---|---|
| F1 | Community generator: lerp port0→port1 by saturate(100·mask.x − 15); author comment notes params 0.15/0.16 on that material. | `CONFIRMED_FROM_COMMUNITY_CODE` | Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (MIT), `lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java` `resolve`, case `BoxType.BLEND` (D76 E3/E5). Restated, not copied. |
| F2 | Each 01.33 GfxMaterial carries its compiled PS3 fragment programs (shader blob, the 10 `CGB` headers plus a trailing ucode region). In the one BLEND material with params 0.67/0.8 (bits `3f2b851e`/`3f4ccccc`), the ucode region holds an inline constant pair 7.692310 / −5.153850 (halfword-swapped f32 `40f62767`/`c0a4ec57`) = 1/(p1 − p0) and −p0/(p1 − p0) within 2e-6 relative (4 and 9 f32 ulps from the f32 quotient; the generator's exact arithmetic is not known). In the one material with 0.3/0.7 (`3e99999a`/`3f333333`, a THING_COLOR BLEND) the pair is 2.5 / −0.75 = 1/0.4 and −0.3/0.4. Neither pair occurs in any other of the 46 distinct BLEND-material blobs; both sit adjacent in one 16-byte constant block. | `CONFIRMED_FROM_LOCAL_DATA` | Read-only scan of the user's 01.33 game data (disc DATA.FARC + patch FARCs), byte-pattern search for f32 constants only; no disassembly or decompilation. |
| F3 | F1's literals are the same scale/bias form for p0/p1 = 0.15/0.16 (1/0.01 = 100, 0.15/0.01 = 15), so community code and local data agree on f = saturate(m/(p1−p0) − p0/(p1−p0)). | `STRONG_INFERENCE` for pairs never observed compiled (incl. 0/1) | Arithmetic on F1/F2. For 0/1 the scale/bias is 1/0, which a compiler folds away, so no distinguishing constant can exist; the rule there is extrapolated from two observed non-trivial pairs. |
| F4 | Mask component: the first (`.x`, red) of the port-2 value; all 14 corpus masks are unswizzled. | `CONFIRMED_FROM_COMMUNITY_CODE` | F1 (`input3.x`). |
| F5 | Mask color space for RGB channels (raw encoded byte/255 or sRGB-decoded by the sampler) | `UNKNOWN` | See below. |

Corpus parameter distribution (62 BLEND boxes in the 46 distinct BLEND materials,
01.33): 59 at 0/1, 2 at 0.67/0.8 (one material, two boxes), 1 at 0.3/0.7 (THING_COLOR).
Texture descriptor flags in all 11 distinct plain-BLEND materials are 0 (no
`NOSRGB_TEX` 0x400 on any reference), so the material does not mark masks as linear.

Applicability: LBP2 PS3 GfxMaterial, branch 0, revisions in the 01.33 census; the
compiled constants are per material, so F2 is direct evidence only for its two
materials. Conflict: none; F1's hardcoded literals are F3's special case.

## What is still missing (RGB mask space)

With thresholds 0/1 the factor is the mask value itself, so the result depends directly
on whether m is the raw byte/255 or the sRGB-decoded value. The shader constants cannot
answer this: sRGB decode on RSX is texture-unit state set by the engine at bind time, not
fragment-program code. Missing evidence, in AGENTS.md order:

1. Documentation/community code stating how LBP2 sets the RSX texture gamma (sRGB) bits
   for textures without the `noSRGB` marker (Toolkit names the marker —
   `resources/RTexture.java`, `structs/texture/CellGcmTexture.java` flags bit 0,
   `enums/ResourceFlags.java` `NOSRGB_TEX` — but shows no sampler setup). Not found in
   `references/` (Toolkit, cwlib-rs, lbp-documentation-wiki, lbptoolspy-lite, farctool2).
2. A local experiment that separates the spaces (e.g. a reference capture of a 0/1 BLEND
   material with known mask bytes) — not available without running the game.
3. Targeted executable investigation (permitted, **not done** here): locate where the
   renderer builds texture control words from a texture's `noSRGB` flag and check whether
   the per-channel gamma bits cover R/G/B. Question to answer: are diffuse/mask textures
   without `noSRGB` sampled with sRGB decode on R/G/B? Record only the finding.

Until one of these answers it, RGB masks stay `Unsupported: BLEND mask must be a sample
alpha (RGB mask space unknown)`. Products under a BLEND input and two/three-sampler
BLENDs were not implemented: no corpus material could use them before the mask space is
known (10 of 14 have MULTIPLY products under an input; 4 have three direct samples).

## Counts

`diffuse_route_census` on 01.33 (8,094 material locations with MAP rows; not asset
counts): b93 → d94 class counts identical — texture 6,953, solid 619, unwired 10,
animation-base-only 121, instance-only 247, unsupported 144, no MAP row 65. Class
changes 0. Reason changes 46 locations (all BLEND roots that previously reported
`inputs must be COLOR or texture samples`): 10 without THING_COLOR and 36 with THING_COLOR
(projected reason too) now report `RGB mask space unknown`; with the 4 already there,
all 50 BLEND roots share that reason. Newly viewable +0, newly Complete +0 (no class
changed; tested scope = the census). Corpus mesh census not rerun (selection classes
unchanged, so baseline all-diffuse 3,669 is unaffected).

## Verification

- `cargo fmt --all --check` — pass.
- `cargo test -q -p lbp-preview` — 50 unit tests pass (updated: threshold generalisation,
  community literal equivalence over m = 0..1, 0/1 alpha-lerp bake, 0.67/0.8 and 0.3/0.7
  scale/bias, invalid pairs, RGB-mask-first ordering).
- `cargo clippy -q -p lbp-preview --all-targets -- -D warnings` — pass.
- `cargo test -q --workspace --no-fail-fast` — all pass except two `lbp-viewer`
  diagnostics export tests in the workspace run (fresh worktree, as noted in D84);
  `cargo test -q -p lbp-viewer` then passes 65/65; `lbp-viewer` is unchanged by D94.
- `python -m unittest discover -s tests` — 34 OK (1 skipped).
- `diffuse_route_census local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d68-material-census-0133b.json local-analysis/d94-diffuse-route-shapes-0133.json`
  (release, run from the main checkout) — counts above.
- Constant scan: a local, uncommitted probe dumped the 46 BLEND materials' decompressed
  shader blobs to the session scratch directory (not `local-analysis/`, not committed);
  a Python search for halfword-swapped f32 values gave F2.
- Oracle / `diffuse_parity` / `corpus_mesh_census`: not run — no material changed class.

## Not tested

No corpus material reaches the BLEND route, so the generalised bake is covered only by
synthetic tests; no viewer QA. RGB mask space, BLEND filtering order in the game, product
or multi-sampler BLEND inputs and THING_COLOR BLEND under the instance-color opt-in remain
unsupported/`UNKNOWN`.
