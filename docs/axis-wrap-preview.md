# D54 independent U/V addressing — 2026-10-06

Static single-texture diffuse bindings now preserve each axis independently:
Repeat/Repeat, ClampToEdge/ClampToEdge, Repeat/ClampToEdge and ClampToEdge/Repeat.
The shared CPU IR stores two typed modes instead of one repeat boolean. Bevy maps
them to separate sampler address_mode_u/address_mode_v; raw/transformed UV arrays
and original texture pixels remain intact. No UV folding or invented default for
missing/unknown wrapping. Static texture-tint routes use the same policy.

## Sources, applicability, conflicts and tests

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [RGfxMaterial::serialize wrapS/wrapT enum arrays](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java)
  and [TextureWrap::getValue/fromValue](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/TextureWrap.java):
  independent S/T arrays, WRAP=1 and CLAMP_TO_EDGE=3. Existing disk parser retains
  both raw arrays; no new disk-layout rule. Prior matching-axis restriction was
  authored preview policy, not a format requirement. No reader source reused.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear supported GMT920–951/branch0.
  Target material00001d92 sack_boy_stitching_decal,7865stored bytes,hash
  02a5c9ea46d5479b73c7664f0f3effe12e4e346e. Pinned headless metadata oracle confirms
  slot0 direct static identity UV1,wrapS[0]=3,wrapT[0]=1,unflagged GUID texture.
  Both complete raw arrays/root/selector compare independently,1record0mismatches.
  Existing48/32metadata sets also compare both arrays,0mismatches; route eligibility
  remains45/26, because graph selection itself did not gate addressing.
- `CONFIRMED_FROM_DOCUMENTATION`: pinned Bevy0.19.1
  [ImageSamplerDescriptor/ImageAddressMode](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_image/src/image.rs)
  permits distinct U/V modes. Headless descriptor checks cover both mixed cases,
  prior uniform cases,sRGB format and linear filters. Pinned epaint0.36.2
  [TextureOptions::wrap_mode](https://github.com/emilk/egui/blob/0.36.2/crates/epaint/src/textures.rs)
  has one shared wrap mode. Legacy eframe explicitly retains geometry/reports mixed
  addressing unsupported before creating a texture; no silently substituted axis.
- `UNKNOWN`: native GPU result on this new route,original game pixels/pose/filter/
  mip/alpha/full-shader fidelity and corpus totals. No image or native window needed
  for the bounded descriptor mapping check; this milestone is headless preparation.

Synthetic tests preserve all four raw U/V combinations,return no uniform mode for
mixed pairs,and reject absent modes/0/2/4/5/6/7/8/255 on either axis. Mirror,border,
legacy CLAMP and mirror-once modes remain unsupported. Bevy adapter keeps existing
extent/pixel budgets and uses linear filters. No shader edits,extra source loads,
coordinate/pixel allocations or change to D53 four-source/cache/cancel ceilings.
Two/three/four-texture products still require Repeat on both axes for every source;
mixed product addressing is explicitly unsupported. No parser UI dependency.

App labels U/V modes independently and keeps Viewable/Complete confirmations
separate. Census records raw U/V modes and addressing frontend scope; route oracle
diagnostic schema6 retains both complete source arrays and comparison result.
Same37hash/37distinctGUID beforecode baseline,unchanged geometry/GUID counts:
37→37viewable,31→33all-diffuse,0verifiedComplete. **+0newly viewable/+0newly
Complete**. Three assets gain3groups/81triangles:0001da30 gains38triangles3/6→4/6;
000201dc gains8triangles7/8→8/8;000156e7 gains35triangles5/6→6/6. The last two
gain full diffuse preparation; that does not certify all textures/correct geometry.
Corpus totals unknown;Bevy preparation scope. Exact commands/limitations:STATUS.md.
No original writes,proprietary assets/code/keys,recovered source or publication.

D60 adds MIRROR2 and independent two-source axes; earlier unsupported statements
above describe D54. [Current scope](mirror-preview.md).
