# Current state (overwrite each milestone) — after D64, 2026-10-07

## Product
Independent LBP2 Asset Explorer (Rust). Bevy viewer (main) and legacy viewer read
the user's own extracted game data and preview meshes with diffuse materials.
Parsing crates stay independent of UI/rendering.

## Inputs (read-only)
- `game-data/a1-disc` extracted BCUS98245 01.00; `USRDIR/DATA.FARC` 1023990298 bytes.
- Snapshot `local-analysis/a2-inventory-verified.json` (~119 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Supported now (diffuse preview routes)
Static textures with scale/offset, matrix UVs, UV0/UV1; Repeat, Clamp, Mirror per
axis; interior-only BORDER; COLOR solids; texture × COLOR; two- to four-sample
products, including one sample squared (D64); constant and alpha MIX. Opt-in: animation base values (matrix two-curve
and, since D62, subtype-0 one-curve scale/offset); user-supplied instance color
(ThingColor, D61). Unwired diffuse previews as labelled black (D63,
Toolkit generator evidence; game default unknown). Groups whose other channels need unrendered textures are flagged
and cannot be confirmed Complete.

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- D60 39-GUID cohort: 39 viewable / 35 full diffuse / 0 Complete.
- D60 40-GUID cohort: 40 / 36 / 0.
- D61–D63 48-GUID cohort (d61-mesh-samples.json): default 48/43/0; animation base
  48/45/0; supplied color + animation base 48/48/0. Corpus totals unknown.
- D64 squared cohort (d64-square-mesh-samples.json, 26 hashes): 26/23/0.
- Corpus selection census (5163 materials): 4175 texture, 459 solid, 7 unwired,
  34 animation-base-only, 122 instance-only, 366 unsupported (not asset counts).
- D64: +0 newly viewable, +0 newly Complete.

## Open blockers
- Glow, reflection and other non-diffuse channels are not rendered.
- ThingColor assets need a supplied color; real instance values unknown.
- BORDER color, CLAMP5, mirror-once 6–8, motion/time, filtering fidelity unknown.

## Next
Three-texture product with an independent third transform: largest blocker
(84 meshes blocked by nothing else). Needs a three-sampler Bevy path or a bounded bake.

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu).
- Git: local repo at project root, branch `main`, default-deny `.gitignore`.
  Baseline d7d33b2 (D60 + unrecorded D61 code); D62-D64 committed after it. No remote.
- Native QA windows on non-primary monitor 165v0, verified in log before showing.

## Where detail lives
- Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`.
- Milestone docs: `docs/instance-preview.md` (D61), `docs/animation-base-preview.md` (D56, D62),
  `docs/unwired-diffuse-preview.md` (D63), `docs/route-census.md` (D64).
- Format evidence: `docs/formats.md` (grep headings). Rules: `AGENTS.md`.
