# D65 three-texture product with an independent third transform — 2026-10-07

Before D65 the nested product `A × (B × C)` / `(A × B) × C` was supported only when
the third sample had exactly the second sample's channel and transform. The D64
route census found 114 materials (about 132 dependent meshes, 84 blocked by nothing
else) where the third sample has its own scale/offset. These now preview in Bevy.

## Evidence

- `CONFIRMED_FROM_COMMUNITY_CODE`: Toolkit pin
  `d6a2ef80b5a09999f08f23a803b516096055e5c9`, GfxAssembler TEXTURE_SAMPLE
  (lines 197–208, static subtype 0: `uv * scale + offset` on UV0 or UV1 per
  parameter 4) and MULTIPLY (lines 328–347). Each sampler has its own transform;
  nothing requires samplers in one product to share coordinates.
- `CONFIRMED_FROM_LOCAL_DATA`: 48 most-referenced materials
  (d65-third-material-samples.json): all three samplers subtype 0, Repeat/Repeat
  except one; channel pairings vary (UV0/UV0/UV1, UV1/UV0/UV0, all UV0 with three
  different scales, ...). Pinned oracle parity 48 compared / 0 mismatches / 48 projected.
- `UNKNOWN`: game filtering, mip selection and rounding of the product.

## Rule and rendering

Only two raw UV channels exist, so the third sample always shares a raw channel
with the primary or second sample. `relative_third` re-derives its coordinates from
that displayed set: with base transform `(bs, bo)` and target `(ts, to)`,
`third = base × (ts / bs) + (to − bo × ts / bs)`, computed in f64 and rounded once
to f32. This is exact in real arithmetic; there is no bit-exact or GPU rounding
claim. Requirements: all three samplers finite scale/offset with nonzero scale; the
base and third are not instance-UV-subtracted (D61); existing Repeat and
unflagged-descriptor rules for the second/third textures still apply. Exact
sharing keeps the old identity-from-second behaviour.

The Bevy extension shader gains uniform 107 (`third_transform`); `enabled.y` = 1
samples from second coordinates (`uv_b`), 2 from primary (`uv`). Legacy viewer
already refuses all independent-sampling products. Labels show the base and the
relative transform. Tests cover identity, scaled base, primary base on the other
channel, instance subtraction, zero base scale and a matrix third.

## Counts (distinct catalog GUID)

Cohort d65-third-mesh-samples-pinned.json: first 48 (by hash) of the 85 catalog
meshes blocked only by these materials. Before 48 viewable / 0 full diffuse /
0 Complete; after 48 / 29 / 0; supported groups 92 → 134 of 235; geometry identical.
Remaining groups are mostly the separate 8-distinct-material mesh budget (78
groups), 11 groups whose third channel's base is a matrix, plus texture-size,
non-Repeat and descriptor limits. D61 48 cohort 48/43/0 and D64 squared cohort
26/23/0 unchanged. **+0 newly viewable / +0 newly Complete.**
