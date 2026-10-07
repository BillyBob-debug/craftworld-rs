# Batch report — 2026-10-07c (Integrator)

## Merged
- D77 `origin/cloud/D77-branch-4431` (PR #8): Vita 0x4431 header gate. 01.33 scan:
  116/116 now `metadata_parsed`; unsupported 117 -> 1.
- D76 `origin/cloud/D76-blend-research` (PR #7): BLEND research doc (docs only).
- D75 `origin/claude/d75-two-tex-color-erbljt` (PR #9): two textures x static COLOR.
  Route census unsupported 386 -> 258 (texture +110, instance-only +18). Toolkit
  diffuse parity on all 128 changed materials: 0 mismatches. Regressions unchanged.

Checks on merged main: fmt, workspace Clippy -D warnings, tests (lbp-resource,
lbp-mesh, lbp-cli, lbp-preview), both viewer builds.

## Failed
None.

## Counts
+0 newly viewable / +0 newly Complete. D75 cohort (48 meshes, distinct GUID): all-diffuse
0 -> 44. A second 26-mesh part could not be censused (meshes without MAP rows).

## Backlog
Refreshed (docs/backlog.md): rank 1 done; next ranks 2 (65), 4 (14), 5 BLEND (14, D76 rule ready).
