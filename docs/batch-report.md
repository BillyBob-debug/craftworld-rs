# Batch report — 2026-10-07e (Integrator)

## Merged
- D81 `cloud/D81-corpus-census`: corpus_mesh_census. 01.33: 4,257 of 4,261 mesh GUIDs
  viewable, 3,657 all-diffuse, 0 Complete.
- D82 `cloud/D82-wrap-modes`: legacy CLAMP 5 edge-clamped; mirror-once refused. 0 users.
- D83 `cloud/D83-mix-shapes`: two-texture alpha MIX on the Repeat grid. Route
  unsupported 209 -> 207; parity 8/0 (2 projected).
- D84 `cloud/D84-blend-route`: static BLEND phase 1; 0 materials selected. Merge
  conflict in linked.rs tests resolved by keeping both tests.
- PR #13 `claude/audit-fixes`: CI Rust 1.99.0 + --locked, viewer check, workflow doc.

Checks on merged main: fmt, workspace Clippy -D warnings, tests for 7 crates, both
viewer builds, Python 34 OK, diffuse parity regressions unchanged, cohorts unchanged.

## Failed
None.

## Counts
+0 newly viewable / +0 newly Complete. Corpus (D81 tool, after batch): 4,257 viewable /
3,657 all-diffuse / 0 Complete of 4,261 mesh GUIDs.

## Backlog
MIX 14 -> 12, BLEND 14 unchanged (thresholds), see docs/backlog.md.
