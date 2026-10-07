# D48 static UV matrix sampling — 2026-10-06

Static subtype-one TEXTURE_SAMPLE now supports a bounded 2×3 matrix on UV0 or
UV1. Row-major coefficients `[a,b,c,d,e,f]` retain raw float bits from material
parameters `[0,1,2,3,6,7]`; parameters4/5 still select channel/texture slot.
Derived sampling coordinates are `(a*u+b*v)+c, (d*u+e*v)+f`. Raw mesh UVs remain
unchanged. Identity rows preserve raw signed zero. SamplingTransform separates
scale/offset from matrix behavior; both frontends consume its derived array and
display the selected transform. No disk layout or parser/UI dependency added.

## Evidence, applicability and limits

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [GfxAssembler::resolve TEXTURE_SAMPLE subtype-one static branch](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java).
  Static row coefficient positions and row dot products are behavior evidence;
  implementation independently expresses arithmetic. No shader assembly copied,
  translated or executed. Coverage: asymmetric cross terms/rotation and exact
  raw coefficient/channel/subtype comparison against pinned metadata records.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT heads920–951, branch0.
  Three selected subtype-one roots observed in the existing80 records: one static,
  two with named/keyed animation. Only the static route becomes eligible. Reused
 48/32 pinned material records: zero mismatches,36/24 eligible respectively
  (previous36/23). These overlapping material sets are not asset totals.
- `CONFIRMED_FROM_LOCAL_DATA`: tutorial floor GUID0001a560 gains its968-triangle
  stitching group with128×256 BC3 Repeat on transformed UV0. Native capture shows
  patterned edge geometry; other four groups stay neutral. Same37hash/37GUID
  before/after cohort:37→37 viewable,23→23 all-diffuse,0 verified complete.
  Exactly one asset gains one texture group. Original36mesh oracle cohort unchanged.
  Added mesh selected through catalog material linkage; no external geometry oracle
  was run for it. Its5928vertices/7783triangles remain unchanged between runs.
- `UNKNOWN`: original GPU rounding, pose, alpha, lighting and full shader fidelity.
  Community assembler formats floating-point shader text; preview retains decoded
  f32 values and separate multiply/add operations. No original-game pixel parity
  or visual completeness claim. Existing block-compression documentation applies.

Bounds are authored preview policy: finite coefficients with magnitude≤1,000,000,
finite raw coordinates, nonidentity derived coordinates magnitude≤1,000,000,
at most100,000pairs, cancellation every256vertices and before return. No fused
multiply-add assumption. Missing channels, unknown subtypes, named/keyed animation,
wired sampler inputs and malformed/ambiguous graph routes remain unsupported.
Only direct sampler or existing texture×static COLOR diffuse routes are accepted.
Tests cover nonfinite/overflow/budget/cancellation, signed zero/raw preservation,
unknown subtype and animated matrix rejection; prior channel/graph tests retained.
Existing eight-binding/read/pixel budgets remain unchanged.

App status distinguishes Viewable geometry from Complete (all textures showing
and correct geometry). This matrix sample remains Viewable with one supported
group out of five; session reports1viewable/0complete. Completeness confirmations
stay unset. Full diffuse support alone never certifies Complete.

Verification commands/results and private filenames: latest STATUS.md. One native
image inspected, startup logs non-primary placement before visibility. No original
input writes, recovered code/decryption, proprietary data/keys bundled, publication
or strict clean-room-process claim. Animated matrix sampling remains deferred.
