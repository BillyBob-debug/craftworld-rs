# Batch report — 2026-10-07f (Integrator)

## Merged
- D85 `cloud/D85-optin-corpus-census`: corpus census opt-in modes.
- D86 `cloud/D86-thing-color`: constant THING_COLOR products fold under a supplied
  instance color (8 materials; default unchanged). Parity 8/0 with --instance-white.
- D87 `cloud/D87-small-families`: CONST4 alone, COLOR x COLOR, TEX with CONST UV scale
  (25 materials). Parity 25/0. Merge conflict in diffuse_parity's report line resolved
  (schema 16).

Checks on merged main: fmt, workspace Clippy -D warnings, tests (lbp-preview 42 and
four other crates), both viewer builds, Python OK, parity regressions unchanged.

## Failed
None.

## Counts
Route census: unsupported 207 -> 174. Corpus (4,261 mesh GUIDs): viewable 4,257,
all-diffuse 3,657 -> 3,665 (default), 3,709 -> 3,717 (animation base), 3,932 -> 3,954
(both opt-ins). +0 newly viewable / +0 newly Complete.

## Backlog
Refreshed; see docs/backlog.md.
