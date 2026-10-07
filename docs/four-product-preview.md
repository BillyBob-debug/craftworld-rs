# D53 four-texture diffuse chain — 2026-10-06

Bevy supports exactly `((A × B) × C) × D` with three static MULTIPLY nodes and
four distinct static texture leaves. Either input order on each multiply and
reversed wire storage order are accepted. D must share A's original channel and
exact transform bits; C must share B's. The bottom pair can be exchanged to match
those requirements. Original sampler indices, slots, channels and coefficients
remain metadata. No coordinate fallback or coefficient snapping.

## Rule evidence, applicability, conflicts and coverage

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve TEXTURE_SAMPLE/non-BUMP MULTIPLY](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  establishes sampled component-wise products. Existing MaterialBox/MaterialWire
  parsing, channel selection, affine/matrix rules and D52 evidence apply. No new
  disk-layout rule. No reader shader copied, translated or executed in product.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT920–951/branch0. Existing
  pinned material record ce658d5650822969f1780545df8c61bdc1bb55d3 has multiply
  nodes3→5→7, leaves1/2/4/6, UV1/UV0/UV0/UV1 identity transforms, slots0/1/2/3.
  Independent oracle comparison checks all three roots/edges and four selectors
  and raw transform coefficients. Same48/32records:0mismatches,45/26eligible
  versus44/26. Same37GUID sample:handkerchief00007a6d gains98triangle group,
  2/3→3/3supported. Material eligibility is not an asset total.
- `CONFIRMED_FROM_DOCUMENTATION`: existing sRGB/linear sample policy and pinned
  Bevy0.19.1 material interfaces described in dual-preview.md. Installed
  [forward_io VertexOutput](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_pbr/src/render/forward_io.wgsl)
  declares primary `uv` under VERTEX_UVS_A and secondary `uv_b` under VERTEX_UVS_B.
  Authored optional fourth texture samples primary coordinates, after the existing
  primary/secondary/third product. Disabled fourth sample retains prior behavior.
- `UNKNOWN`: original game pixel/pose/filter/mip/alpha/full-shader fidelity,
  floating-point reassociation/rounding boundaries, other GPU/platform behavior
  and corpus totals. Existing opaque/unlit/top-mip rendering is diagnostic policy.

Graph endpoints are checked before indexing, within1024nodes/4096wires. Fixed chain
inspection follows at most three multiplies without recursion; distinct selected
multiply nodes and distinct sampler leaves prevent cycles/repeated nodes. Each
multiply needs subtype0/unanimated, exactly two unique incoming ports0/1, source
port0 and zero swizzle. Output is static/unswizzled. Selected samplers are static,
unwired UV0/UV1; subtype0 affine or subtype1 matrix retains finite/bounded rules.
Missing UV1 never substitutes UV0. Named/keyed animation, invalid descriptors,
non-Repeat axes, ambiguous sources, mismatched coordinate classes, balanced trees,
five-texture/deeper chains and non-texture leaves remain unsupported. Exact-bit
sharing distinguishes signed zero. Four-source routes never enter grid baking.

Fourth original pixels/key are a separate SharedSampleAsset; third uses secondary
sampling coordinates and fourth uses primary. No additional UV arrays allocated.
Each texture remains≤16MiB input/32MiB expansion/4Mpixels. Eight cached bindings
bound raw decoded pixels≤512MiB plus≤128MiB derived pixels for other routes;
cumulative stored reads≤522MiB and expansion≤1232MiB including UV decoding.
Source stability/cancellation guards remain, including checks before shared source
loads. GPU owns four images and removes all on scene replacement; cleanup regression
adds a fourth image. Legacy eframe explicitly rejects independent sampling and
retains geometry. Parser libraries do not acquire rendering dependencies.

Synthetic regression covers valid four leaves, all multiply input orders/reversed
wire list, selected indices, ignored counts, no grid, transform/signed-zero mismatch,
fourth animation, wired source port, duplicate leaf and cycle rejection. Existing
transform/UV/cancellation/budget/cache tests apply. Private diagnostic schema5
retains deepest multiply/fourth selector; census records fourth source/extent and
primary-coordinate sharing. App labels all four original channel/transforms and
keeps session Viewable/Complete confirmations separate.

Baseline captured before code, same37hash/37distinctGUID cohort and unchanged
geometry counts:37→37viewable,30→31all-diffuse,0verifiedComplete. **+0newly
viewable / +0newly Complete**, one asset/one group/98triangles improved. Full
diffuse coverage only enables review; Complete requires all textures showing and
correct geometry independently of renderer completion. Corpus totals unknown.
Exact commands/native evidence/untested areas:STATUS.md. No original input writes,
proprietary assets/code/keys, recovered source or publication involved.
