# Batch report — 2026-10-08c (Integrator)

## Merged
- D100 `cloud/D100-grid-graph-bake`: general static graph evaluator on the Repeat grid
  (36 materials; parity 36/0).
- D101 `cloud/D101-multiply-rest`: four-leaf trees without shared pairs, COLOR-only
  trees, CONST leaves (8 materials; parity 8/0).
- D102 `cloud/D102-sampler-rest`: any-mask matrix rows and animated x COLOR under the
  animation-base opt-in (4 materials; parity 4/0).

Merge conflicts D100 vs D101/D102 resolved by keeping both. Integrator decision: the
overbright CONST material (tint 4.89) refused by D101 is now evaluated by D100 (baked
and clamped); D101's test updated to expect that.

## Failed
None.

## Counts
Route census: unsupported 102 -> 53. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,683 -> 3,695, animation base 3,747 -> 3,762, instance color
3,929 -> 3,962, both 4,010 -> 4,046. Textures 23,247 of 23,247 decode.
+0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
