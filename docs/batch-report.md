# Batch report — 2026-10-07g (Integrator)

## Merged
- D88 `cloud/D88-thing-color-textured`: textured THING_COLOR shapes under the instance
  color (8). Parity 8/0 with --animation-base --instance-white; default unchanged.
- D89 `cloud/D89-add-uv2`: ADD of two textures and TEX x COLOR + COLOR (4). Parity 7/0
  (4 projected). UV channel raw 2 documented unsupported.
- D90 `cloud/D90-texture-census`: corpus texture census; 22,973 of 23,247 hashes decode.

Merge conflicts D88 vs D89 (linked.rs binding call, route-census instance check)
resolved by keeping both; diffuse_parity schema 18.

## Failed
None.

## Counts
Route census: unsupported 174 -> 162. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,665 -> 3,669, animation base 3,717 -> 3,721, instance color
3,894 -> 3,899, both 3,954 -> 3,967. +0 newly viewable / +0 newly Complete.
Textures: 22,973 of 23,247 hashes decode.

## Backlog
Refreshed; texture decode gap added (GTF 0x85/0x81).
