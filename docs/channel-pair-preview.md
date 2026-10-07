# D51 independent UV0/UV1 texture products — 2026-10-06

D52 adds a bounded three-leaf nested product; nested rejection and two-source
budgets below describe D51. Current policy: [nested-preview.md](nested-preview.md).

Bevy's two-texture diffuse projection now accepts different requested channels on
the two static samplers. Each channel is selected from the decoded mesh explicitly,
transformed independently and supplied to its own GPU sampling attribute. Baked
nearest grids require matching original channels, even with identical transforms.
Pairs on different channels always use independent linear samples. Raw UV arrays,
triangle indices, original channel identity and transform bits remain unchanged.
UI labels both original channels; private census diagnostics include the second
channel, source texture and frontend scope. Legacy eframe retains geometry and
explicitly reports independent sampling unsupported.

## Rule sources, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [MaterialBox sampler parameters](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialBox.java)
  and [GfxAssembler::resolve TEXTURE_SAMPLE/non-BUMP MULTIPLY](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  each sampler parameter4 independently selects a UV pair; diffuse multiplication
  combines sampled results. D49's same-channel guard was authored grid policy.
  Existing affine/matrix/interleaved UV evidence applies; no new disk-layout rule.
  Coverage: both selected channels/raw coefficients compared with pinned external
  metadata; CPU no-channel-fallback tests and separate GPU attribute checks.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00, supported clear GMT heads920–951/
  branch0. Two existing48material records multiply UV1 by UV0, previously blocked.
  Same48/32external datasets:0mismatches,42/26eligible versus40/26; material counts
  are not asset totals. Same37hash/37distinctGUID mesh sample:37→37viewable,
  all-diffuse26→30,0verifiedComplete. Four assets0000ffff/0000ef9d/00013051/0000ffa9
  gain four textured groups/1577triangles; vertex/triangle/GUID counts unchanged.
  Native circuit board00013051 shows its892triangle UV1×UV0 group, plus existing
  texture and two solid groups,4/4supported. Manual completeness stays unset.
- `UNKNOWN`: original game pixel/pose/filter/full shader fidelity. Native pattern
  availability and successful metadata/UV preparation are not evidence of Complete.
  Existing sRGB documentation and authored opaque/unlit/top-mip policies apply;
  no new color-space or alpha specification inferred.

Missing requested UV1 never falls back to UV0. Unsupported channel>1, nonfinite
coordinates, wrong vertex correspondence, bounded derived-coordinate failures,
selected animation/wired inputs, malformed/ambiguous graph routes, nested products
and descriptor/wrap failures remain explicit. Two-source read/pixel/coordinate/
cache/cancellation bounds remain D50's. No extra textures or sampling allocations
introduced; the same two-channel decode supplies both samplers. `uses_grid` has a
regression proving integer transforms on different channels cannot enter baking.
GPU adapter regression retains distinct coordinate bit arrays (including signed
zero), preserves triangle order and rejects short/nonfinite secondary arrays.
No shader edits or parser/UI dependencies added. No reference code translated.

Native check selected the non-primary monitor before showing and used one image
to resolve combined circuit pattern display. App shows1viewable/0complete,
1ready for review. Complete means all textures showing and correct geometry,
independently of renderer completion. Full diffuse support alone does not certify
that. Corpus totals unknown; measurements concern Bevy preparation for the same
selected37GUIDs. Exact commands, ignored evidence and untested areas: STATUS.md.
No original writes, proprietary assets/code/keys or recovered source bundled,
publication or strict clean-room-process claim.
