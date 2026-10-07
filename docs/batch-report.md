# Batch report — 2026-10-07d (Integrator)

## Merged
- D79 `origin/cloud/D79-fntb` (PR #11): FNTb font header reader. First check failed
  Clippy 1.99 (`chunks_exact_to_as_chunks`, font.rs 165/177/189); fixed on the branch
  by the Integrator (e328bf1), then merged. 01.33 scan: unsupported resources 1 -> 0.
- D80 `origin/cloud/D80-thing-color-research`: THING_COLOR research doc (docs only).
- D78 `origin/claude/d78-animated-product-7erb3i`: animation-base opt-in for 2+ texture
  products. Route census unsupported 258 -> 209 (+46 animation-base-only, +3
  instance-only). Toolkit parity on the 49 changed materials: 0 mismatches. Merged
  with CRLF kept in linked.rs and compatibility_census.rs (branch had converted to LF).

Checks on merged main: fmt, workspace Clippy -D warnings, tests (lbp-resource,
lbp-cli, lbp-preview), both viewer builds, diffuse parity regressions unchanged.

## Failed
None after the D79 fix.

## Counts
+0 newly viewable / +0 newly Complete. Cohorts unchanged: d68-new 48/45/0, D61
48/43/0, D75 48/44/0 (distinct GUID; viewable/all-diffuse/Complete).

## Backlog
Refreshed: animated-sampler products mostly moved to the animation-base opt-in; next
MIX (14), BLEND (14).
