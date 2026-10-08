# Batch report — 2026-10-07h (Integrator)

## Merged
- D91 `cloud/D91-gtf-uncompressed`: GTF A8R8G8B8/B8 decode. Textures 22,973 -> 23,215
  of 23,247 hashes. Pixel parity 20/0 on new samples, 22/0 on the D90 samples.
- D92 `cloud/D92-multi-animated`: 2+ animated leaves under animation base. Route
  unsupported 162 -> 144. Parity 24/0.
- D93 `cloud/D93-counts-in-app`: census counts shown in both viewers; headless
  --print-census verified on the real census files.

Checks on merged main: fmt, workspace Clippy -D warnings, tests (7 crates), both viewer
builds, Python OK, parity regressions unchanged, cohorts unchanged.

## Failed
None.

## Counts
Route census: unsupported 162 -> 144. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,669, animation base 3,721 -> 3,733, instance color 3,899, both
3,967 -> 3,980. Textures 23,215 of 23,247 decode. +0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
