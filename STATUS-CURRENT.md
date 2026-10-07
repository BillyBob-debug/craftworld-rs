# Current state (overwrite each milestone) — after batch D72/D71/D73, 2026-10-07

## Product
Independent LBP2 Asset Explorer (Rust). Bevy viewer (main) and legacy viewer read
the user's own extracted game data and preview meshes with diffuse materials.
Parsing crates stay independent of UI/rendering.

## Inputs (read-only) — working version is 01.33 since D69
- `game-data/a1-disc` BCUS98245 01.00 plus `game-data/a1-update-0133` (official
  updates 01.01..01.33, extracted in order; packages in `local-analysis/lbp2-update-pkgs/`).
- Pass both as one list: `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by
  script). Old 01.00 snapshot `a2-inventory-verified.json` still works with the disc alone.
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`;
  oracle runners take `-Archive` (patch FARCs live under the update dir).

## Supported now (diffuse preview routes)
Static textures with scale/offset, matrix UVs, UV0/UV1; Repeat, Clamp, Mirror per
axis; interior-only BORDER; COLOR solids; texture × COLOR; two- to four-sample
products, one sample squared (D64), independent third transform (D65/D67, Bevy only);
constant and alpha MIX. Opt-in: animation base values; supplied instance color.
Unwired diffuse previews as labelled black (D63). Up to 64 materials per mesh within a
640 MiB retained budget (D66). Resource heads accepted through LBP2_MAX 0x3f8 (D69).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- 01.33 (D69): D61 cohort 48/43/0; D65 48/38/0; D66 22/7/0; D64 26/23/0 (= D67 on 01.00).
- D69 new cohort d68-new-mesh-samples.json (48 post-951 meshes): 48/45/0.
- Corpus selection census on 01.33 (8,094 material locations with MAP rows): 6,828
  texture, 603 solid, 10 unwired, 60 animation-base-only, 207 instance-only, 386
  unsupported (01.00: 5,163 / 4,306 / 459 / 7 / 34 / 122 / 235). Not asset counts.
- D69: +48 newly viewable in the tested cohort, +0 newly Complete; corpus totals unknown.
- Batch D72/D71/D73: tests and CI only, +0 viewable / +0 Complete.

## Open blockers
- Glow, reflection and other non-diffuse channels are not rendered.
- ThingColor assets need a supplied color; real instance values unknown.
- BORDER color, CLAMP5, mirror-once 6–8, motion/time, filtering fidelity unknown.
- Branch 0x4431 resources (116) and FNTb fonts unsupported; patch.edat not decrypted.

## Next
Two-texture products that also multiply a static COLOR, re-ranked on 01.33 with
`d68-diffuse-route-shapes-0133.json` (386 unsupported). The census does not run Bevy
draw grouping.

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu).
- Git: branch `main`, remote origin github.com/chilladept/lbp2-asset-explorer (private),
  default-deny `.gitignore`. CI: `.github/workflows/ci.yml` (D72).
- Workflow: `docs/workflow.md`. On demand: cloud agents push `cloud/*` or `claude/*`
  branches; the PC Integrator verifies and merges passing ones into main without asking
  (owner approval 2026-10-07). Daily task LBP2-DailyBatch exists but is disabled.
- Native QA windows on non-primary monitor 165v0, verified in log before showing.

## Where detail lives
- Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`.
- Last batch: `docs/batch-report.md`; ranked backlog: `docs/backlog.md`.
- Update/overlay: `docs/update-overlay.md` (D68 data, D69 code). Earlier milestone docs:
  `docs/third-sample-preview.md` (D65, D67), `docs/material-budget-preview.md` (D66),
  `docs/route-census.md` (D64), `docs/unwired-diffuse-preview.md` (D63).
- Format evidence: `docs/formats.md` (grep headings). Rules: `AGENTS.md`.
