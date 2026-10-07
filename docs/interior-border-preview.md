# D59 interior-only BORDER preview — 2026-10-06

Nearest same-sample alpha MIX can preview source BORDER axes when every selected
mesh sampling UV stays inside the texture with a conservative margin. The
polaroid now prepares both diffuse groups. This does not implement out-of-range
BORDER behavior or infer a game border color. Source wrapping remains visible in
the app and diagnostics, separately from equivalent interior display addressing.

## Evidence, applicability and conflicts

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [TextureWrap::getValue/fromValue](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java)
  identifies BORDER4; RGfxMaterial::serialize retains separate S/T arrays.
  Existing RMesh::serialize/init/getUVs rules and independently implemented UV
  reader are described in textured-mesh.md. No new disk layout or reader source
  reuse. Original matrix/scale-offset derivation is unchanged.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT945, branch0 material
  cosmos_polaroid00006a16/05e7d877fa19251c37c9563143f95c036c2bd667 has wrapping[4,4]
  and D58's nearest alpha MIX. Mesh00006a1a/5c8cb186d0b5b87089f90d3cd863cc87eddc650c
  has72vertices/105triangles/two primitives. Referenced UV0 range on the32triangle
  textured group is[0.0004995167,0.0004994869]..[0.9995005,0.9995005]. Whole selected
  UV0 array passes the same guard; UV1 reaches1.4112475 and is not substituted.
  Texture256×256 decodes through the existing reader (24890stored bytes).
  Target1/previous32/48pinned material records compare0mismatches,1/28/46projected.
- `CONFIRMED_FROM_DOCUMENTATION`: Khronos
  [Vulkan Sampling, Texel Nearest Filtering and Wrapping Operation](https://docs.vulkan.org/spec/latest/chapters/textures.html)
  describes a single selected texel and addressing of integer coordinates.
  **Inference / authored policy:** interior nearest samples select in-range texels,
  so BORDER and ClampToEdge produce the same selected texel within this restricted
  domain. The1/4096normalized margin is a conservative preview limit, not a game
  format requirement or a formal cross-GPU rounding bound. Existing triangle
  interpolation retains the interior vertex domain; no shader modifies these UVs.
  Top mip only and no sampling offsets. Linear filtering would reach outside the
  image near an edge and is deliberately excluded from this path.
- Pinned Bevy0.19.1
  [ImageAddressMode::ClampToBorder](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_image/src/image.rs)
  requires the optional wgpu ADDRESS_MODE_CLAMP_TO_BORDER feature and separate
  border color. No feature is requested here: the guarded display uses existing
  ClampToEdge on validated source BORDER axes, retaining other axes. Existing
  legacy uniform-axis limitation remains explicit. No hardware border default.
- `UNKNOWN`: actual game border color/out-of-range edges/filtering/mips/full
  shader/pose/pixels, other GPUs/platforms/revisions and corpus totals. This does
  not claim original RSX behavior. The delivered code is independently expressed,
  not a strict clean-room claim.

Global TextureWrapping::from_raw still rejects4. A proposed display mapping may
accept4 only for the structurally validated same-sample alpha MIX, whose frontends
already force nearest. Before texture decode/baking or publication, BorderDomain
checks every derived coordinate in the entire selected mesh UV channel, not just
the first primitive. This makes shared material caching safe across groups.
Every BORDER axis must be within[1/4096,1−1/4096]; other valid Repeat/ClampToEdge
axes retain their own domain. All coordinates must be finite, nonempty and at most
100000vertices. Missing/unknown/mirror/legacy clamp modes remain unsupported.
Coordinates that can approach or cross the border explicitly fail; source UVs
are never clipped, folded, changed or replaced by another channel.

The guard scans with cancellation every256vertices and at completion, allocating
no coordinate/image buffer. Existing transform bounds/source/cache/read/expansion/
alpha-bake budgets remain. BorderDomain retains source raw wrapping and min/max
float bits. Asset.wrap describes effective display addressing; source_wrap_raw()
preserves the original modes. App labels source BORDER, validated range, nearest
policy and unused border color. Census schema4 records original and display axes,
range bits, margin and whole-channel scope. Legacy metadata also separates source
and display modes. No new shader or GPU resource. Viewable/Complete remain
distinct GUID counts and manual texture/geometry confirmation; full preparation
is not Complete. Complete remains independent of renderer completion.

Synthetic tests cover bounds including0/1/nonfinite/near-edge values, mixed axes,
missing/unknown addressing, non-nearest rejection, empty correspondence, original
UV retention and early/mid-scan cancellation. Existing alpha/transform/source/
complete-review tests remain. The bounded border_domain_probe example uses
existing readers for selected-triangle raw range research, with no color claim.
It initially called a nonexistent helper; corrected to validated geometry ranges.
A Bevy synthetic fixture initially lacked the new field; fixed before final
92tests/fmt/Clippy/both builds passed. Exact commands:STATUS.md.

Same before-code39hash/39distinctGUID cohort:39→39viewable,34→35all-diffuse,
0→0verifiedComplete. **+0newly viewable / +0newly Complete**. One polaroid group
gains32triangles,1/2→2/2supported. GUIDs/geometry unchanged; corpus totals unknown.
One native Bevy window was positioned on non-primary165v0 before showing. One
capture verified the formerly neutral group has its image,2/2groups,source BORDER
range label and source-session1viewable/0Complete. No manual Complete certification
or game comparison. Original data remains read-only; private research ignored;
no proprietary product files, commit or publication.
