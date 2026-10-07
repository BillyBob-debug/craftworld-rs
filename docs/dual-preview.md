# D50 independent static two-texture sampling — 2026-10-06

D52 additionally accepts a bounded three-texture nested route with exact
secondary/third coordinate sharing. D50 two-source budgets below are historical;
current three-source ceilings and shader ownership: [nested-preview.md](nested-preview.md).

D51 permits different original UV0/UV1 channels; the same-channel restriction
below describes D50. Grid baking still requires matching channels. See
[channel-pair-preview.md](channel-pair-preview.md).

Bevy now displays two static texture samples with independently transformed UVs
feeding a single diffuse MULTIPLY. Existing integer Repeat products retain D49's
nearest grid. Other finite bounded subtype-zero affine or subtype-one matrix pairs
use two independently linearly filtered textures in an authored unlit fragment
extension. Both samplers still require the same explicit UV0/UV1 channel, Repeat,
present unflagged descriptors, no wired inputs or named/keyed animation. Graph
endpoint/unique-port/swizzle/output guards remain; nested products unsupported.
Raw transform bits are never snapped to an axis or integer grid.

## Rule evidence, applicability and conflicts

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve TEXTURE_SAMPLE subtype-one static branch and non-BUMP MULTIPLY](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java).
  Coefficients `[0,1,2,3,6,7]` form two matrix rows; diffuse multiplies sampler
  outputs. No new disk layout. Existing independent graph parser/UV decoder are
  retained. Community shader output is evidence, never copied/translated/executed.
  Coverage: targeted external metadata compares root indices/both slots/channels/
  raw transform coefficients; authored asymmetric matrix route/nonfinite rejection.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT branch0 in supported
  heads920–951. Target floor_tut_purple0001a547 has static cross terms/offsets and
  an identity sampler feeding diffuse. Its matrix cannot be represented exactly by
  D49's axis-aligned periodic grid. Pinned reader run on one8732byte material gives
  zero raw-route mismatches. Native tutorial floor0001a560 gains its2000triangle
  group:4/5→5/5supported,5928vertices/7783triangles unchanged. Same37hash/37GUID
  sample remains37viewable,all-diffuse25→26,0verifiedComplete; one asset improved.
- `CONFIRMED_FROM_DOCUMENTATION`: existing sRGB transfer policy in
  product-preview.md. Bevy's sRGB images decode RGB before filtering/multiplication;
  alpha samples multiply linearly, with retained opaque display policy.
- `UNKNOWN`: original pose, lighting, alpha, filtering/mips/full shader fidelity and
  GPU boundary/rounding behavior. Toolkit formats float shader text; this viewer
  transforms vertices using full decoded f32 coefficients. No original-game pixel
  parity or Complete certification from this native capture.

## Architecture, bounds and coverage

DualAsset retains the secondary decoded texture/key, independently derived sampling
array and Repeat mode. Primary sampling uses its authored affine/matrix transform;
mesh/raw UVs/textures remain intact. Existing finite≤1Mcoefficient/derived-coordinate
policy,100kvertices,cancellation every256vertices and source stability checks apply
to both coordinate arrays. Two-source input/pixel bounds remain D49 ceilings; at
most another800kbytes of sampling coordinates per binding (6.4MB total
bytes across eight bindings). No extra product pixel buffer for DualAsset.

Authored `dual.rs` uses pinned Bevy0.19.1 API contracts from installed source:
[ExtendedMaterial/MaterialExtension](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_pbr/src/extended_material.rs),
[forward_io/pbr_fragment](https://github.com/bevyengine/bevy/tree/v0.19.1/crates/bevy_pbr/src/render).
Shader is embedded authored text, no reference templates/assets bundled. Primary
uses standard material UV0, secondary transformed coordinates use UV1 GPU attribute
(independent of original material channel). The unlit extension samples UV1 only
when enabled; disabled extension preserves prior solid/single/nearest-grid color
behavior. Both GPU images/materials/meshes/entities removed on scene replacement;
regression adds secondary images to the existing ownership cleanup test.

Legacy eframe explicitly reports this route unsupported and retains geometry;
independent sampling is currently a Bevy frontend feature. The census counts Bevy
preparation, not universal frontend compatibility. Diagnostics label this scope,
secondary source/extent and filtering. UI distinguishes Viewable from Complete,
shows5/5diffuse groups and1ready for review, but1viewable/0complete until manual
texture/geometry confirmations. All-diffuse support does not prove all game shader
channels/textures are visible or correct geometry.

Exact commands, one native image, baseline/delta and untested areas: STATUS.md.
Original files remain read-only; no proprietary assets/keys/recovered code, original
writes, publication or strict clean-room-process claim.


D60 supersedes two-source Repeat restrictions above with independent Repeat/
Mirror/Clamp axes. Grid/product MIX/shared chains stay guarded. [Scope](mirror-preview.md).
