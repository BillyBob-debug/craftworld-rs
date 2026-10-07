# D49 bounded two-texture diffuse products — 2026-10-06

D50 adds independently sampled static matrix/affine pairs in Bevy; references below
to unsupported matrices/offsets describe D49's grid scope. Legacy eframe retains
that grid scope. See [dual-preview.md](dual-preview.md).

Two direct static samplers feeding one MULTIPLY→DIFFUSE can now display a texture
product. Both use the same explicit UV0/UV1 channel, subtype-zero positive integer
scales1..16, zero offsets (signed zero accepted), Repeat on both axes, present
unflagged descriptors. Distinct sampler nodes, one output, unique unswizzled
source-port-zero edges, multiply input ports0/1 and no selected animation/wired
sampler inputs are required. Nested products, channel mismatch, matrix/offset/
fractional/negative scales and Clamp remain explicitly unsupported.

## Evidence and applicability

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve MULTIPLY, non-BUMP branch](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  inputs0/1 multiply component-wise for diffuse. BUMP has different behavior and
  is outside this projection. Sampler scale/channel/wrap rules retain earlier
  MaterialBox/GfxAssembler evidence in affine-preview.md. No reference shader
  source copied, translated or executed; independently expressed CPU policy.
  Coverage: both source nodes/slots/channels/raw transform bits and root indices
  compared with existing pinned external metadata; authored malformed route tests.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT heads920–951, branch0.
  Reused48/32material records compare with zero mismatches,40/26eligible versus
  36/24before. These overlapping material samples are not asset totals. Same37
  mesh hashes/37distinct GUIDs:37→37viewable,23→25all-diffuse,0verifiedComplete.
  Seven assets gain13textured groups/5733triangles. Additional all-diffuse GUIDs
  00002635 and00010d89. Tutorial floor0001a560 advances1/5→4/5supported groups;
  one matrix-sampler product stays neutral. Mesh vertex/triangle counts and
  cohort identities unchanged. Added floor still lacks external geometry oracle.
- `CONFIRMED_FROM_DOCUMENTATION`: existing sRGB transfer specification,
  [Khronos glTexImage2D](https://wikis.khronos.org/opengl/GLAPI/glTexImage2D), retained
  from tinted-preview.md. RGB is decoded to linear, multiplied, encoded/rounded
  to8bits; alpha multiplies linearly. No encoded-byte RGB product assumption.
- `UNKNOWN`: original game filtering/mips/color calibration/pose/alpha/full
  shader and GPU rounding. Nearest-grid rendering is authored preview policy,
  not evidence of original-game pixel parity or Complete.

## Grid, budgets and representation

For each axis, a source has `texture_extent * integer_scale` texel intervals per
raw-UV period. Their least common multiple forms the product grid (checked LCM).
Each output texel reads the corresponding periodic source texel, multiplies RGB
in linear space and alpha linearly. Grid≤4096peraxis/4Mtotalpixels. Both frontends
use raw UVs and **nearest** filtering for this product, so grid sampling reproduces
the two nearest samples apart from GPU boundary rounding/8bitquantization.
Linear filtering a baked product does not equal multiplying two linearly filtered
samples; consequently this route explicitly selects nearest. Original one-texture
routes retain linear filtering. No claims about original game filter selection.

Asset retains primary/secondary decoded pixels, keys, raw sampler bits, selected
raw UVs and separate product pixels/extents. Original texture/mesh exports unchanged.
Each source decoder retains16MiBinput/32MiBexpansion/4Mpixels bounds. Eight cached
bindings now have ceilings256MiBrawpixels+128MiBderivedpixels,266MiBcumulative
input/720MiBcumulativeexpansionwork (including repeated UV decode). Extra65,536byte
lookup table is temporary. No unbounded recursion; source reads/cancellation and
stability checks retained. Cancellation before/while table construction, every4096
output pixels and before return; coordinate products are bounded by validated
source extents/scales and useu64. Oversized LCM fails explicitly, with geometry
still viewable. Decoded original images never altered.

Tests cover asymmetric extents/scales, periodic texel ownership, linearRGB product
versus encoded multiplication, alpha, invalid extent/scale, LCM/pixel limits and
cancellation, plus selection channel mismatch/animation/fractional/nested rejection.
Native tutorial-floor capture resolves whether both texture patterns appear and
the neutral group remains. App labels two-texture product/nearest grid, keeps
Viewable versus Complete and measured source-session counts visible. Full diffuse
coverage enables manual review; it does not certify all textures/geometry correct.
Exact commands, counts, private evidence and untested areas: latest STATUS.md.

No original input writes/decryption/recovered-code use, proprietary data/keys
bundled, publication or strict clean-room-process claim.
