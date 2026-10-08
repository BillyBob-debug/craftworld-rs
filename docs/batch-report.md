# Batch report — 2026-10-08a (Integrator)

## Merged
- D94 `cloud/D94-blend-more`: general BLEND threshold rule; RGB mask colour space still
  unknown, so 0 materials change class. Its shader byte dumps were moved into ignored
  local-analysis/d94-blend-shader-blobs/ (they had been left in a scratch folder).
- D95 `cloud/D95-texture-rest`: B8 up to 16 Mi pixels, BC1 cube faces (layered API).
  Textures 23,215 -> 23,244 of 23,247. Texture parity 7/0.
- D96 `cloud/D96-thing-sampler-ports`: sampler-port folds under the instance color
  (6 materials). Parity 6/0.

Checks on merged main: fmt, workspace Clippy -D warnings, tests (7 crates), both viewer
builds, Python OK, parity regressions unchanged, cohorts unchanged.

## Failed
None.

## Counts
Route census: unsupported 144 -> 138. Corpus meshes (4,261 GUIDs): viewable 4,257;
all-diffuse default 3,669, animation base 3,733, instance color 3,899 -> 3,902, both
3,980 -> 3,983. Textures 23,244 of 23,247 decode. +0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
