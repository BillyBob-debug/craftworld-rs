# D56 local animation base snapshots — 2026-10-06

Bevy can opt into the stored base values of two named XYZ matrix-row animations
on a direct diffuse sampler. The default remains static-only. This is a local
base-value snapshot: no game time, key interpolation or motion is inferred.
The UI labels the snapshot and excludes it from Complete confirmation because
the texture placement has not been established against the game's animation
resolver. Complete still means all textures showing and correct geometry,
independently of renderer completion. Changing this option reloads the selection
and clears source-session verification and measured counts.

## Evidence and limits

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`,
  [MaterialParameterAnimation::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/structs/gmat/MaterialParameterAnimation.java)
  stores a base vector, component mask, component-major value arrays and name.
  [GfxAssembler::resolve, TEXTURE_SAMPLE subtype 1](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/util/gfx/GfxAssembler.java)
  associates the first named animation with matrix row 0 and the second with row 1,
  using their first three components. Existing independently implemented metadata
  parsing is reused; no curve payload or recovered shader code is added.
- Conflict: `GfxAssembler::generateBakedShaderSource` takes uniform base values
  from the global material animation list, serialized by
  [RGfxMaterial::serialize](https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/resources/RGfxMaterial.java).
  Local sampler base rows are not established as equivalent to that list or the
  game's live uniform resolver. The opt-in policy explicitly uses local rows;
  neither a game-time-zero frame nor reader baked-output parity is claimed.
- `CONFIRMED_FROM_LOCAL_DATA`: BCUS98245 01.00 clear GMT920–951, branch 0.
  Materials 0001d959/to_glow (2fb2c5ce264ec100e017f7674393235047ab9e8c)
  and 0001d958/lg_glow (94aedad0b1e7260fd3590c35aa7d01165a6c3010)
  have direct subtype-1 UV1 slot-0 samplers, names b1/c1, XYZ mask 7,
  603 component values / 201 keys per row and identity local base rows.
  Two pinned metadata records, 23682 stored bytes, compare with zero mismatches.
  Original static matrix bits, all four base components, key counts, derived
  coefficients, sampler slot/channel and wrap metadata are retained or checked.
- `UNKNOWN`: key rate, interpolation, wrapping, global name resolution, game time,
  original game pixels and texture placement, other revisions/platforms, and
  corpus-wide asset totals. No executable investigation or decryption was needed.

The independently authored selector is deliberately narrow: one static output,
one unswizzled direct diffuse edge, one unwired subtype-1 UV0/UV1 sampler and two
distinct two-byte alphanumeric names followed by NUL. Both curves require exactly
XYZ, positive component counts divisible by three and at most 4096 values each,
with matching derived key counts. All graph endpoints are validated within
1024 nodes / 4096 wires. Missing UVs, nonfinite or oversized derived coordinates,
other masks/names, wired samplers, products, ambiguous outputs and other routes
remain explicit unsupported outcomes. Existing descriptor/wrap/read/decompression/
pixel/cache/cancellation limits apply. No new key arrays, image buffers or shader
resources are introduced. The parser remains independent of UI/rendering.

`AnimationBaseSnapshot` retains six original static parameter bits separately
from both four-component local base rows and key counts. Only the six XYZ values
derive sampling coordinates; original UVs and texture pixels remain intact.
Static callers and the legacy viewer retain the static-only default. Bevy exposes
the option in-app and as `--animation-base`. Versioned census schema 2 and route
oracle schema 8 retain the mode and snapshot metadata, with time explicitly null.

Synthetic tests use asymmetric rows, signed zero and a preserved static identity
matrix, and reject nonfinite rows, bad names/masks, wired samplers and invalid
endpoints. Existing UV bounds, missing-channel, cancellation and source tests
remain applicable. App tests ensure snapshot coverage cannot become Complete
even with both manual checks set. Exact verification commands are in STATUS.md.

The unchanged 37-hash / 37-distinct-GUID mesh cohort has 37 viewable and 0 verified
Complete assets before and after: **+0 newly viewable / +0 newly Complete**.
Default static full diffuse preparation remains 34; opt-in snapshots raise it to
35. Geometry boy 0001da30 gains two groups / 223 triangles, from 4/6 to 6/6 groups
prepared. This measures preparation coverage, not visual correctness. Corpus
totals are unknown. One native Bevy check is scoped to this snapshot and its UI;
it cannot establish game animation fidelity. Research files remain ignored;
original game inputs are read-only. No proprietary product files or publication.
