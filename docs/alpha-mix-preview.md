# D58 same-sample alpha MIX — 2026-10-06

The bounded diffuse selector accepts one static COLOR / texture MIX whose factor
is exactly the same texture sample's alpha. Original color bits, input direction,
sampler slot/channel/transform and overridden stored weight bits remain intact.
RGB mixes in linear space, alpha linearly. Baking happens per texel and both
frontends require nearest minification/magnification for this route. This is an
authored diagnostic policy: nonlinear alpha mixing does not commute with linear
filtering. Original game filtered output is not claimed.

## Evidence and applicability

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::getWithSwizzle and ::resolve MIX](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java).
  Wire swizzle `a` selects alpha; factor port2 overrides parameter0, and MIX
  interpolates ports0/1. Existing independently implemented raw metadata is used;
  no reader shader or recovered code is copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT945, branch0.
  cosmos_polaroid material00006a16, hash05e7d877fa19251c37c9563143f95c036c2bd667,
  6804stored bytes. COLOR2 is on MIX3 port0, identity UV0 sampler1/slot0 on port1,
  same sampler on port2 with swizzle bytes[97,0,0,0,0]. One targeted pinned record
  compares root, all source edges/ports/swizzles, color/input direction/overridden
  weight and existing sampler coefficients/wrap arrays:0mismatches,1projected.
  Existing32diffuse records:0mismatches,28projected versus27;48linked:0mismatches,
  46projected unchanged. These are graph counts, not asset compatibility counts.
- Blocking local evidence: selected texture uses raw wrapping[4,4]. Pinned
  [TextureWrap::getValue/fromValue](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java)
  calls4 BORDER. Actual border color/game edge behavior remain `UNKNOWN`; neither
  Repeat nor ClampToEdge is substituted. Local polaroid payload preview stays
  unsupported. A bounded structural search of48material samples with4nodes/
  4wires and no tail animation (376627stored bytes) found this one same-sample
  alpha-factor shape; no additional supported-wrapping asset was established.
- `CONFIRMED_FROM_DOCUMENTATION`: sRGB transfer references in tinted-preview.md
  and product-preview.md. Existing nearest policy/frontends are referenced in
  product-preview.md. Standard interpolation is independently expressed in f64
  after exact f32 color decoding. Filtering/quantization policy is authored;
  original game rounding, pixels, pose, alpha/full shader/mips are `UNKNOWN`.

The fixed route requires one static output, a direct unswizzled diffuse wire,
one subtype0 unanimated MIX and exactly three incoming wires with source port0
and unique ports0/1/2. Ports0/1 must be unswizzled static COLOR and sampler leaves;
port2 must reuse that sampler with exactly `a` and four NUL bytes. COLOR must be
unwired/unanimated subtype0 and finite0..1. Sampler reuses the existing unwired/
unanimated UV0/UV1 scale-offset or matrix rules. Endpoints and1024node/4096wire
limits are validated first. Four distinct nodes/four selected edges, no recursive
evaluation. Other factor samples/channels/swizzles/weights, ThingColor, cycles,
named animation, wired COLOR/sampler, missing UV/texture and unknown wrapping
stay explicit unsupported outcomes. Constant MIX/product paths remain unchanged.

`AlphaDiffuseMix` is separate from `DiffuseBlend`; its overridden weight bits are
retained without treating them as an active finite weight. Existing original
texture/raw UVs stay separate from derived display pixels/UVs. A192KiB heap LUT
and2KiB transfer table bound nonlinear work; output uses the existing4Mpixels/
16MiB derived buffer. No extra source/image/UV arrays or shader bindings. Existing
four-source read/expansion/cache ceilings apply, with194KiB temporary table maximum.
Cancellation precedes LUT work, every16alpha rows, every4096output pixels and at
completion. Frontends use `nearest_sampling()` for products and alpha MIX; UI
labels sample-alpha factor, color port, UV/wrapping and nearest policy. Diagnostic
oracle schema9/census3 retain alpha metadata and filtering. Bevy still shows
source-session distinct-GUID Viewable/Complete counts; Complete means all textures
showing and correct geometry, separately from renderer completion.

Synthetic tests cover alpha0/255 and128, sRGB/linear alpha, both input directions,
original pixels unchanged, invalid color/pixel length, early/table/mid-output
cancellation. Selector tests cover asymmetric matrix UV1, reversed wire order,
overridden nonfinite parameter0 retention and wrong factor swizzle/source rejection.
Existing animation/wired sampler/ThingColor/output/UV bounds/cycle/source guards
remain. Initial misplaced UI label match arm caused a compile error and was
corrected before successful tests. Exact commands and remaining limitations:
STATUS.md.

Before-code sample adds catalog-linked polaroid00006a1a to the prior38hash cohort,
39distinctGUIDs. Added mesh has no external geometry oracle. Identical GUIDs/
geometry/group support before/after:39→39viewable,34→34all-diffuse,
0→0verifiedComplete. **+0newly viewable / +0newly Complete;0improved sampled assets**.
The polaroid retains1/2supported groups,32triangles blocked by BORDER wrapping.
Corpus totals unknown; successful metadata selection is not payload compatibility
or visual correctness. Headless checks,0native windows/0images. Original inputs
read-only; derived research ignored; no proprietary bundling or publication.
