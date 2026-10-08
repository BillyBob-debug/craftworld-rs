# Batch report — 2026-10-08d (Integrator)

## Merged
- D103 `cloud/D103-offgrid-bake`: off-grid graph bakes, labelled preview approximation
  (15 hashes; parity 15/0).
- D104 `cloud/D104-uv-channel-2`: UV channel word 2 samples UV0, from the materials'
  compiled fragment programs (4 locations; parity 4/0). Research output in ignored
  local-analysis/d104-uv-channel/.
- D105 `cloud/D105-instance-graphs`: evaluator on the original graph under a supplied
  instance color (2; parity 2/0).

Merge conflicts D103 vs D104/D105 resolved by keeping both; one D105 test updated to
expect D103's resampling of an off-grid graph.

## Failed
None.

## Counts
Route census: unsupported 53 -> 34. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,695 -> 3,700, animation base 3,762 -> 3,767, instance color
3,962 -> 3,971, both 4,046 -> 4,055. Textures 23,247 of 23,247 decode.
+0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
