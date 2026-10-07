# D57 constant COLOR / single-texture MIX — 2026-10-06

The static diffuse selector now accepts a constant MIX between one unwired COLOR
and one static texture sampler. It preserves input direction and exact stored
weight/color bits, sampler slot, original UV channel/transform and independent
wrapping axes. The existing derived diffuse image buffer stores bounded linear
RGB / linear alpha mixing; original texture pixels and raw UVs stay intact.
No new disk parser, shader or instance default is introduced.

## Evidence, applicability and conflicts

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve, MIX and COLOR](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java).
  MIX combines ports 0/1 with parameter 0 when factor port 2 is unwired; COLOR
  uses four stored floats if unanimated. The same established graph fields and
  independent standard interpolation used by D55 apply to this simpler route.
  No reader shader or recovered game code was copied, adapted or translated.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, ordinary clear GMT936, branch 0.
  Material 00012fd2, gamedata/gmat/bounce_pad_casing.gmat,
  hash ba976413affbf809f2d65301f711d725642be82a, 8742 stored bytes.
  Diffuse output 0 takes MIX3 with black COLOR1/alpha1 on port0 and identity UV0
  texture sampler2/slot0 on port1; weight bits1056964608 (0.5). Descriptor is
  present/unflagged with Repeat on both axes; decoded texture is512×512.
  One targeted pinned record compares root, input edges/direction, weight/color
  bits, slot/channel/transform and wrapping with zero mismatches. Existing
  32 diffuse records now have27 eligible versus26, all0mismatches;48 linked
  records remain46 eligible, all0mismatches. Material records are not asset counts.
- `CONFIRMED_FROM_DOCUMENTATION`: sRGB transfer specification linked in
  tinted-preview.md/product-preview.md. RGB mixes after sRGB decoding and encodes
  once; alpha is linear. Constant interpolation commutes with linear filtering
  before 8-bit quantization. This is authored preview policy, not established
  original game pixel calibration. No alpha-weighted MIX is accepted.
- `UNKNOWN`: game pixels/pose/full shader/alpha/mip/filter fidelity, native display
  of this new route, other GPUs/platforms/revisions and corpus asset totals.
  Floating arithmetic uses f64 after exact f32 decoding; game GPU rounding remains
  unverified. The pinned reader supports more graph shapes than this product.

One static subtype0 MIX must have exactly two unswizzled source-port0 inputs on
unique ports0/1 and no factor input. One leaf must be an unwired unanimated subtype0
COLOR with finite0..1 components. The other must be a static unwired UV0/UV1
sampler, accepted through the existing bounded scale/offset or matrix selector.
The output, graph endpoints and1024node/4096wire limits are validated first.
These four distinct node types require three selected edges; other branches remain
ignored and labeled. Missing UVs/textures, named animation, ThingColor, swizzles,
cycles, wired weight or sampler, bad color/weight and other graph shapes remain
explicitly unsupported. Existing source stability/cache/read/expansion limits apply.

`DiffuseBlend` already retains MIX index/color/weight/input direction. A single
MIX has no multiply index or second sampler. A1024-byte lookup table produces at
most4Mpixels/16MiB in the existing derived buffer, matching the tint budget.
Texture-only weight endpoints avoid a second image allocation. Cancellation is
checked before bounded table work, every4096output pixels and at completion.
No product/grid restrictions are imposed on this single sampler; existing
per-axis wrapping and derived UV rules still apply. Both frontends consume the
existing display pixel path and show the static MIX label. Bevy's source-session
distinct-GUID Viewable/Complete ledger stays separate from diffuse coverage.
Complete requires all textures showing and correct geometry, independent of
renderer completion; supported groups alone cannot certify it.

Synthetic tests cover linear RGB/alpha (including sRGB128), source preservation,
weight endpoints, both input directions, invalid pixel length and early/mid-output
cancellation. Selector tests use an asymmetric matrix on UV1, reversed wire storage,
both port orders, and reject alpha-weighted factor, named sampler, wired sampler
and ThingColor. Existing invalid weights/color, missing UV, bounds, cycles and
source/cache/cancellation checks remain. Exact commands: STATUS.md.

The before-code cohort is the prior37mesh hashes plus catalog-linked bounce pad
00012fd4 (b9955d73fa4789190a56ec0640c6adfe50573d22),38distinctGUIDs total.
The added mesh has no external geometry oracle. Same38GUIDs/geometry before and
after:38→38viewable,34→34all-diffuse,0→0verifiedComplete.
**+0 newly viewable / +0 newly Complete**. Bounce pad improves0/2→1/2groups,
one232triangle casing group. Its95triangle surface sampler still has unsupported
wiring. No automatic full-asset claim. Corpus totals unknown. Checks are headless;
zero native windows or images. Original inputs remain read-only, local evidence
ignored, no proprietary bundling, publication or strict clean-room claim.
