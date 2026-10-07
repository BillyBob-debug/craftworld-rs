# D55 static COLOR / two-texture product MIX — 2026-10-06

The diffuse slice accepts one static MIX node with an unwired static COLOR on one
input and one two-texture MULTIPLY on the other. Weight is the finite stored f32
parameter0 in0..1; input port order is retained, so exchanging inputs changes the
blend direction. Both textures must fit the existing same-channel integer Repeat
grid. Bounded CPU baking multiplies linear RGB, applies the constant mix and encodes
sRGB once. Alpha follows the same mix in linear space. Original texture pixels,
raw UVs, color/weight bits and graph indices remain intact. No shader changes.

## Sources, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [BoxType.MIX=12](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java)
  and [GfxAssembler::resolve MIX/non-BUMP MULTIPLY](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java):
  MIX combines ports0/1 using stored parameter0 unless weight port2 is wired.
  Existing MaterialBox/MaterialWire layout retained; no new disk rule. Independent
  bounded standard interpolation, no reader shader copied, translated or executed.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 ordinary clear GMT920–951/branch0.
  Lamp material0001dc46 vic_lab_wall_lamp_glassy,hash
  3b974d545f61c79b424213c6e10737dfad1b7c2a,10175stored bytes. MIX5 combines
  COLOR1 on port0 with MULTIPLY4 on port1,weight raw1058642329. Product samplers2/3
  use UV0,Repeat slots0/1,identity and3×scale; selected nodes unanimated. Targeted
  pinned reader compares root/color/product edges,indices,input order,color/weight
  bits,slots/channels/transforms and full wrap arrays:1record0mismatches. Existing
  48/32records0mismatches,46/26eligible versus45/26. Metadata counts are not assets.
- `CONFIRMED_FROM_DOCUMENTATION`: established sRGB/linear filtering rules in
  product-preview.md and tinted-preview.md. Product and mix occur before output
  8-bit quantization, avoiding an intermediate encoded product buffer. Grid display
  retains authored nearest filtering; original game filtering is not inferred.
- `UNKNOWN`: original game pixels/pose/full shader/alpha/mips/filter fidelity,
  native GPU display for this new route,other platforms/GPUs and corpus totals.
  Color/weight arithmetic uses f64 after exact f32 decoding; GPU rounding parity
  and the reader's formatted coefficient text are not adopted as a specification.

Endpoints/output validated before fixed-depth selection,within1024nodes/4096wires.
Exactly six selected nodes/five selected edges:one MIX,one COLOR,one MULTIPLY,two
distinct samplers and output. MIX/product need subtype0,unanimated,exactly two
incoming edges with unique ports0/1,source port0 and zero swizzle. Weight port2 is
not allowed. COLOR must be unwired/unanimated,finite0..1. Samplers remain unwired/
unanimated,bounded UV0/UV1 subtype0 positive integer scales1..16/zerooffsets and
matching original channel. Present unflagged descriptors require Repeat both axes.
Missing channel/unknown type/deeper or cyclic routes/static weight outside0..1,
named/keyed animation,swizzle,ambiguous sources or grid budget failure remain
explicitly unsupported. Source/cancel/cache/read/pixel limits unchanged.

DiffuseBlend retains MIX index,color bits,weight bits and color-first flag; original
COLOR index/product indices remain in DiffuseSelection. Shared product baker uses
three bounded65536-byte lookup tables for differing RGB constant channels,192KiB
temporary table versus64KiB for plain products. Output≤4Mpixels/16MiB,LCM axes≤4096.
No extra source or derived image buffer; D53 four-source ceilings remain unchanged
apart from this explicit temporary table allowance. Cancellation before table work,
every16table rows,every4096output pixels and at completion. Both frontends already
display bounded nearest grids; parser remains UI-free. App labels MIX/color/weight
and existing separate Viewable/Complete confirmations; census records static mix.
Oracle diagnostic schema7 retains mix index/color bits/weight bits/input direction.

Synthetic tests cover linear RGB/linear alpha,asymmetric input direction,weight
endpoints,signed-zero raw retention,invalid weights,mid-output cancellation and
original pixels retained. Selector tests cover valid route/reversed wire storage,
both input orders,selected/ignored indices,wired weight,animated color,swizzle,
different channels and cycle rejection. Existing LCM/pixel/source guards retained.

Initial neon target0000be15 is instance-dependent ThingColor multiplied into
diffuse and swizzled into sampler input; no static color invented. Pinned target
1record0mismatches,0eligible. Other remaining research shows two glow samplers
with named b1/c1 animations,201keys each; custom LCD has missing diffuse wire,
other LCD route uses ThingColor. These remain explicit unsupported/unresolved
outcomes. Raw research metadata stays ignored; no speculative runtime support.

Same37hash/37distinctGUID beforecode baseline:37→37viewable,33→34all-diffuse,
0verifiedComplete. **+0newly viewable/+0newly Complete**. Lamp mesh0001e458 gains
one190triangle group,3/4→4/4supported;geometry/GUID counts unchanged. Full diffuse
preparation only enables review;Complete requires all textures showing and correct
geometry,independently of renderer completion. Corpus totals unknown. Checks were
headless;0native windows/0images. Exact commands and limits:STATUS.md.
No original writes,proprietary assets/code/keys,recovered source or publication.
