# D47 static COLOR-only diffuse materials — 2026-10-06

A clear ordinary mesh can now display a static COLOR directly wired to its material's
diffuse output. This is an explicitly supported material with no diffuse texture.
It does not need UV decoding, texture resolution, an image allocation or a fallback
image. Unsupported material bindings remain distinct and neutral.

## Evidence, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: pinned Toolkit
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [BoxType](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java)
  and [BrdfPort](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BrdfPort.java):
  OUTPUT=0, COLOR=3, DIFFUSE output port=0. Existing independent graph layout is
  unchanged. Coverage: selected indices/route kind match the established external
  metadata oracle; duplicate, swizzle, wiring and unsupported ThingColor tests.
- `CONFIRMED_FROM_COMMUNITY_CODE`: same pin,
  [GfxAssembler.resolve, COLOR case](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  four raw IEEE float parameters at indices 0–3 supply an unanimated constant
  vector; a named animation instead references instance parameters. Product accepts
  only unanimated subtype-zero COLOR and OUTPUT with no incoming color-node wires.
  Coverage: exact raw bits for all newly eligible color nodes; finite/animation
  guards. No shader/template execution, copying or translation.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, existing clear ordinary GMT heads
  920–951/branch 0. Reused source-bound D46 pinned oracle records: 48 material
  samples project 18→36, 32 project 18→23, zero route/color-bit mismatches. These
  23 additional material hashes do not alone establish asset completeness.
- `CONFIRMED_FROM_LOCAL_DATA`: same D42 36-mesh asset sample, before and after D47:
  13 distinct catalog GUIDs gain 23 supported nonempty color groups. Eight additional
  GUIDs now have every nonempty diffuse group supported. Geometry/alias set is unchanged.
- `CONFIRMED_FROM_LOCAL_DATA`: native fez `0001f915` shows four supported solid
  groups, 512 vertices/666 triangles, with red body and black tassel. The app reports
  zero textured/four solid groups and one ready for completeness review. One image
  inspected; app logged non-primary monitor selection before becoming visible.
- `UNKNOWN`: exact original game color calibration, final alpha/shader/lighting,
  original pose and visual completeness. Bevy uses linear unlit constant RGB with
  opaque display policy. Eframe encodes that RGB through the existing D46 sRGB
  transfer policy and uses constant vertex color with approximate painter ordering.
  Raw alpha is retained; blending is deferred. No game-pixel parity claimed.

These are independently expressed behavior/preview policies using the unchanged
MIT reader as evidence/oracle. No recovered source/decryption or strict clean-room
claim. New serialized layouts, proprietary assets or game-specific keys are absent.

## Contract and representation

The graph budget remains 1024 nodes/4096 wires, with strict endpoint bounds.
One output, one unique diffuse wire, source port zero, zero swizzle, COLOR source.
Selected output and color nodes are subtype zero and have no named/keyed animation;
any incoming color wire rejects. Unrelated branches are visibly counted and ignored.
Colors must be finite in [0,1], including alpha. Negative/HDR values are unsupported.
Cycles, ThingColor, animated colors and broader graph evaluation remain unsupported.
Existing material flags/alternative descriptors, read limits, cancellation and
stable-source checks still apply.

`MaterialAsset` explicitly distinguishes boxed textured `Asset` from `SolidAsset`.
SolidAsset contains only its material location and SolidSelection (raw color bits,
indices and ignored-branch counts). Its texture and sampling-UV accessors return
None. The loader returns it before UV decoding or texture reads. Shared caches
retain at most eight distinct bindings; solid materials add no pixel/UV buffers.
Both UI/GPU adapters handle that variant explicitly, preserving source geometry.
The eframe solid path uses constant vertex color rather than diagnostic gray shading.

The Bevy compatibility panel now distinguishes textured, solid and neutral groups.
Supported solid groups satisfy the diffuse-coverage prerequisite for visual review;
they are not counted as textured. The session summary shows how many assets are
ready for completeness review. Complete still requires both explicit texture/geometry
confirmations; the texture checkbox permits a verified no-texture requirement.
Ready does not certify all material outputs, original geometry or game fidelity.

## Measured counts

| Same 36 mesh hashes / distinct catalog GUIDs | Before | After |
|---|---:|---:|
| Geometrically viewable | 36 | 36 |
| All nonempty diffuse groups supported | 15 | 23 |
| Newly visually verified complete | 0 | 0 |

**+0 newly viewable, +0 newly complete; 13 improved assets / 23 supported color groups.**
Eight new all-diffuse candidates: `0001354e`, `00013edb`, `00014cc4`, `00015daf`,
`0001f732`, `0001f915`, `0001f972`, `000279ca`. Other improved assets retain explicit
unsupported groups. Whole-corpus totals remain unknown. Solid support is not texture
loading, and full diffuse coverage is not automatic completeness certification.

The before census was run before implementation. Reports and comparison remain
ignored: d47-compatibility-baseline.json, d47-compatibility-after.json and
d47-compatibility-delta.json. Comparison checks identical hashes/GUID sets and
unchanged vertex/triangle counts. D46 baseline/texture routes remain the prior
reference. Native eframe interaction and other GPUs/platforms were not tested.
Exact commands, tests, build results and remaining work: latest STATUS.md.
