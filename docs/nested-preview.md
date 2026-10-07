# D52 bounded three-texture diffuse product — 2026-10-06

D53 additionally permits a fixed four-texture chain with two coordinate classes.
Three-source ceilings and rejection of the handkerchief's final group below are
D52 history; current four-source policy: [four-product-preview.md](four-product-preview.md).

Bevy accepts exactly `(texture × texture) × texture` with two static MULTIPLY
nodes and three distinct static sampler leaves. Either outer input order is
accepted; inner wire ordering does not change eligibility. The third sampler must
share the original channel and exact transform bits with one inner sampler. That
pair uses the same derived secondary coordinates; the other inner sampler retains
its independently selected/transformed UV0/UV1 coordinates. No third coordinate
array, generalized graph evaluator, recursive traversal or nearest-grid baking.

## Sources, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve TEXTURE_SAMPLE/non-BUMP MULTIPLY](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  establishes sampled component-wise products. Existing MaterialBox/MaterialWire
  layout, channel selector and static affine/matrix rules apply; no new disk rule.
  No community shader copied, translated or executed in the product.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 ordinary clear GMT920–951,
  branch0. Two existing pinned external material records have a UV1 leaf and two
  identity UV0 leaves under two diffuse multiplies. Independent root/inner edges,
  three sampler indices/slots/channels and transform bits compare with those
  records. Same48/32records give0mismatches,44/26eligible versus42/26. Eligibility
  alone is not an asset count. Mesh00007a6d gains two groups/160triangles,0/3→2/3;
  the remaining four-texture route is explicitly unsupported.
- `CONFIRMED_FROM_DOCUMENTATION`: existing sRGB image/linear sample product policy
  in product-preview.md and pinned Bevy0.19.1 ExtendedMaterial/render interfaces in
  dual-preview.md. Authored shader adds one optional texture/sampler and multiplies
  the third filtered sample after the first two. Disabled extension keeps prior
  behavior. This is a diagnostic opaque/unlit/top-mip policy.
- `UNKNOWN`: original game pixels, pose, alpha, mip/filter/full-shader fidelity,
  other platforms/GPUs and corpus-wide counts. A successful render is not Complete.

All endpoints are checked before indexing, within1024boxes/4096wires. Each selected
multiply must be subtype0/unanimated with exactly two incoming edges on unique
ports0/1, source port0 and zero swizzle. Exactly three distinct sampler nodes are
required; selected samplers are unwired/unanimated. Depth is structurally limited
to two multiplies, so cycles and deeper branches fail without recursion. Unknown
subtypes, missing UV1, nonfinite/out-of-bound transforms, ambiguous inputs and
nonmatching secondary channel/transform bits remain explicit failures. Three
present unflagged descriptors require Repeat on both axes; ambiguous resolution,
source stability and cancellation guards remain. Signed zero in transform bits is
not silently equated with positive zero for sharing.

Each of three textures retains its original pixels/key and existing16MiB input,
32MiB expanded and4Mpixels caps. Eight cached material loads bound decoded raw
pixels at384MiB plus up to128MiB derived tint/grid pixels for other bindings;
stored input reads≤394MiB and cumulative expansion≤976MiB including UV decoding.
No third derived sampling array is allocated. GPU images remain separately owned
and all three are removed with scene replacement; cleanup regression covers this.
Legacy eframe retains geometry and explicitly rejects independent sampling.

Synthetic guards cover the valid three-leaf route, reversed wire ordering, exact
coefficient sharing, duplicate ports/leaves, animation, cycle/deeper nesting and
exclusion from grid baking. Existing channel absence/nonfinite/bounds/cancellation
tests apply. App labels all three original channels/transforms; census diagnostics
retain third source/extent/frontend. Viewable and Complete session counts remain
separate, with manual texture/geometry confirmations required for Complete.

Same37hash/37distinctGUID baseline captured before changes:37→37viewable,
30→30all-diffuse,0verifiedComplete. **+0newly viewable / +0newly Complete**, one
asset improved. These are bounded Bevy preparation counts; corpus totals unknown.
Exact commands/native check/limitations: STATUS.md. Original inputs unchanged;
no proprietary assets/code/keys, recovered source or publication involved.
