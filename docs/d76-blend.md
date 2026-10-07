# D76 BLEND (box 16) semantics research — 2026-10-07

Research only. No route, parser or renderer code changes. Backlog rank 5
([backlog.md#blend](backlog.md#blend)): 14 materials on the 01.33 census contain a
BLEND node and no THING_COLOR. Counts are material locations with MAP rows, not
asset counts. Cloud session: no game data was read; nothing here is verified
against disc data.

## Question

What does a GfxMaterial box of type 16 compute on the diffuse path, which of its
ports/parameters matter, and what is the smallest preview route that could express
it without claiming game fidelity?

## Evidence

Toolkit pin `d6a2ef80b5a09999f08f23a803b516096055e5c9` (MIT). Paths under
`lib/cwlib/src/main/java/cwlib/`. Findings are restated independently; no Toolkit
code is copied, adapted or translated.

| # | Finding | Label | Source |
|---|---|---|---|
| E1 | Box type 16 is named BLEND; 12 is MIX, 1 TEXTURE_SAMPLE, 3 COLOR, 2 THING_COLOR. The enum is marked "incomplete list". | `CONFIRMED_FROM_COMMUNITY_CODE` | [enums/BoxType.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java) |
| E2 | The shader generator reads three inputs, by destination port 0, 1, 2, each through the same wire lookup and swizzle handling used for MIX/MULTIPLY. | `CONFIRMED_FROM_COMMUNITY_CODE` | [util/gfx/GfxAssembler.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java) `resolve`, case `BoxType.BLEND` (lines 438–473); `getWithSwizzle` (lines 83–96) |
| E3 | Output = in0 + (in1 − in0) · f, where f = clamp(100 · m − 15, 0, 1) and m is the **first component** of the (possibly swizzled) port-2 value. Same lerp direction as MIX (port 0 at f=0, port 1 at f=1). | `CONFIRMED_FROM_COMMUNITY_CODE` | same, lines 452–453 |
| E4 | Result width is the widest of the three input widths (MIX/MULTIPLY use the same rule). | `CONFIRMED_FROM_COMMUNITY_CODE` | same, lines 444–448 |
| E5 | The generator ignores box parameters for BLEND; 100 and 15 are literals. An author comment records one observed material with param0 = 0.15, param1 = 0.16 and asks whether they are lower/upper curve bounds; it labels port 0 "MAT 1", port 1 "MAT 2", port 2 "MASK". | `CONFIRMED_FROM_COMMUNITY_CODE` (comment only; not a rule) | same, lines 458–472 |
| E6 | 1/(0.16 − 0.15) = 100 and 0.15/(0.16 − 0.15) = 15, so the literals equal f = clamp((m − p0)/(p1 − p0), 0, 1) for p0 = 0.15, p1 = 0.16. In f32 (p0 bits `3e19999a`, p1 bits `3e23d70a`) the derived scale/bias are 100.0001/15.00001; both forms agree at the threshold ends. Whether the game evaluates params generally is not shown. | `STRONG_INFERENCE` | arithmetic on E3/E5 |
| E7 | No unwired-input handling: a missing port makes the generator fail (null dereference), unlike MIX which raises an explicit "two inputs" error. A BLEND with fewer than three wired inputs therefore has no community-code meaning. | `CONFIRMED_FROM_COMMUNITY_CODE` | same, lines 440–448 |
| E8 | MaterialBox has 8 parameters (6 before revision 0x2a4), stored as raw 32-bit words; float use is per box type. | `CONFIRMED_FROM_COMMUNITY_CODE` | [structs/gmat/MaterialBox.java](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java) `PARAMETER_COUNT`, `LEGACY_PARAMETER_COUNT` |
| E9 | Unrelated "blend" hits are not box 16: `CgAssembler.BLEND` is a Vita shader flag (alpha mode), `MeshExporter` sets glTF alpha mode, `BL_*` (128+) / `isBlenderMaterial` are Blender-family nodes (see [unwired-diffuse-preview.md](unwired-diffuse-preview.md)). | `CONFIRMED_FROM_COMMUNITY_CODE` | `util/gfx/CgAssembler.java` 101–172; `io/exports/MeshExporter.java` 1065; `resources/RGfxMaterial.java` 130–160 |
| E10 | Game shader behavior, mask color space (sRGB-decoded or raw), param distribution in the 14 materials, their input shapes, and filtering order (step after or before filtering) | `UNKNOWN` | — |

Applicability: E1–E9 come from Toolkit's LBP2 shader generator path; the pin does not
gate BLEND by revision. No other public reader with BLEND semantics was found in the
pinned tree (`grep -ri blend`; `brdf*.cg` templates have no BLEND box code).

Conflicts: E5 vs E3 — the parameters exist but the community generator hardcodes one
material's values. Treat parameter-general evaluation as unproven (E6).

## Consequences for a preview

- BLEND is a per-pixel **hard step** of width 0.01 in mask units. It is a MIX whose
  weight is a steep nonlinear function of a sample, so it does not commute with
  linear filtering: like D58 alpha MIX it must bake per texel and use nearest
  filtering, and that is authored policy.
- The mask space matters far more than for MIX. On 8-bit masks:
  - raw encoded value m = byte/255: bytes ≤ 38 → 0, 39 → 0.294, 40 → 0.686, ≥ 41 → 1;
  - sRGB-decoded m (existing RGB policy, tinted-preview.md): bytes ≤ 108 → 0,
    109 → 0.293, 110 → 0.593, 111 → 0.896, ≥ 112 → 1.
  These pick visibly different regions. Alpha (`a` swizzle) is linear under both
  existing policies, so an alpha-channel mask has no ambiguity.

## Proposed route rule (not implemented)

`select_blend_mask` (next to `select_alpha_mix` in `crates/lbp-preview/src/linked.rs`),
a separate `BlendDiffuseMask` record, never folded into `DiffuseBlend`/`AlphaDiffuseMix`:

1. Output diffuse wire: static output, direct unswizzled wire to one BLEND root.
   Existing endpoint, 1024-node/4096-wire limits validated first.
2. Root: type 16, subtype 0, unanimated (same `static_node` test as alpha MIX).
3. Exactly three incoming wires, unique destination ports 0/1/2, source port 0.
   Fewer or more → `Unsupported: BLEND needs exactly ports 0/1/2` (E7).
4. Ports 0 and 1: each an unswizzled static COLOR leaf (unwired, finite 0..1) or a
   static sampler accepted by the existing UV0/UV1 scale-offset/matrix rules. Phase 1:
   at least one texture; at most two distinct samplers on the existing same-channel
   integer Repeat grid (the static-MIX grid rule). Products/MIX/ThingColor under a
   BLEND input → unsupported.
5. Port 2 (mask): a static sampler already used on port 0 or 1, or a third sampler
   on the same grid. Swizzle: none (→ `.x`, red) or exactly one of `r/g/b/a`
   (`x/y/z/w`) followed by NULs. Multi-component swizzles → unsupported, since only
   the first component is used and the rule should stay explicit.
6. Parameters: accept only p0 bits `3e19999a` and p1 bits `3e23d70a` (the one
   community-observed pair, E5); use f = clamp(100·m − 15, 0, 1) exactly as E3.
   Any other pair → `Unsupported: BLEND thresholds other than 0.15/0.16` until the
   PC census shows the distribution (then decide on E6 under its own milestone).
7. Mask space: alpha → linear. RGB channel → **unsupported in phase 1** (E10 space
   unknown). Optional phase 2, opt-in and labelled: raw-encoded mask, recorded as
   authored policy, with the UI showing which space was used.
8. Bake per texel on the existing grid; RGB lerp in linear space, alpha linear
   (D58 policy); nearest filtering in both frontends; existing 4 Mpixel/16 MiB
   derived-buffer ceilings and cancellation points. Original texture/raw UVs kept.
9. UI label: `BLEND mask <channel> of sample <n>, thresholds 0.15/0.16, nearest`.
   Census reason strings for each rejection above so the 14 split by cause.

Expected count change is unknown until the PC census reports the 14 shapes
(rank 5 notes say nothing about inputs). Phase 1 may accept zero of them if all
masks are RGB.

## Synthetic test plan (for the implementing milestone)

All fixtures are graphs built in test code from `GraphMetadata`/`BoxMetadata`
literals and authored RGBA arrays; no game bytes.

Selection (`linked.rs` tests):
- T1 accept: COLOR(port0) + sample A(port1) + sample A `a`(port2), p0/p1 exact bits.
- T2 accept: sample A(port0) + sample B(port1) + sample A `a`(port2), same grid.
- T3 accept: two samples + third mask sampler on same grid.
- T4 reject: 2 wires (port 2 missing); 4 wires; duplicate port 1; source port 1.
- T5 reject: params 0.15/0.17; params differing by one ulp from `3e19999a`.
- T6 reject: mask swizzle `xy`, `r` (phase 1 RGB), unknown byte `q`.
- T7 reject: animated/subtype-1 BLEND root; wired or animated COLOR; COLOR 1.5/NaN.
- T8 reject: MULTIPLY or MIX under port 0; THING_COLOR anywhere; cycle through root.
- T9 reject: samplers on different channels/non-integer Repeat; BORDER wrap.
- T10 order: wires listed out of port order select identically.

Evaluation (`blend.rs`-style unit tests on a pure `factor(m)` and `pixels`):
- E-a factor: m = 0.15 → 0; 0.155 → 0.5 (±1e-6); 0.16 → 1; 0, 1, below/above clamp.
- E-b alpha bytes 38/39/40/41 → factors 0 / 0.294 / 0.686 / 1 (exact quantized output).
- E-c direction: factor 0 returns port-0 pixel bytes unchanged; factor 1 port-1.
- E-d linear-space RGB lerp: half-factor of sRGB bytes 0 and 255 yields 188
  (linear 0.5 re-encoded), not the encoded-byte midpoint 128.
- E-e budget: >4 Mpixel grid → Invalid; cancellation honoured before LUT and per row.
- E-f determinism: same input twice → identical bytes; census JSON ordering stable.

Census (`diffuse_route_census.rs`): one synthetic graph per rejection reason, assert
reason string and count bucket.

## PC verify (Integrator)

Docs only, so no code check. To size the implementing milestone, extend the D64
census output (local JSON, counts only) for the 14 BLEND materials: input box types
per port, mask swizzle, p0/p1 bit pairs, sampler wrap/grid compatibility. Report
counts per bucket in the batch report; no GUIDs or bytes in the repo.

## Not tested

No Rust code changed; `cargo` checks unaffected. No game data, oracle run or viewer
QA in this session. Game-side BLEND math, parameter role and mask color space remain
`UNKNOWN`.
