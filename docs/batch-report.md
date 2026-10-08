# Batch report — 2026-10-08b (Integrator)

## Merged
- D97 `cloud/D97-blend-mask`: BLEND masks (red sRGB-decoded, alpha linear); 9 BLEND
  locations under the instance color. Parity 0 mismatches on all 50 BLEND locations.
- D98 `cloud/D98-last-textures`: GCM remap decode and volume slices; all 23,247 texture
  hashes decode. Volume layout from RPCS3 (GPL-2.0, read only, no code copied) and not
  confirmable on the noise-like local volume; no Toolkit oracle for these 3 textures.
- D99 `cloud/D99-next-family`: MULTIPLY trees (16) and off-grid alpha MIX (6). Parity
  27/0. Merge conflict with D97 resolved (diffuse_parity schema 22).

Checks on merged main: fmt, workspace Clippy -D warnings, tests (7 crates), both viewer
builds, Python OK, parity regressions unchanged, cohorts unchanged.

## Failed
None.

## Counts
Route census: unsupported 138 -> 102. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,669 -> 3,683, animation base 3,733 -> 3,747, instance color
3,902 -> 3,929, both 3,983 -> 4,010. Textures 23,247 of 23,247 decode.
+0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
