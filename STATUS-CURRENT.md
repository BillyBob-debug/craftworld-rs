# Current state (overwrite each milestone) — after D63, 2026-10-07

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
products; constant and alpha MIX. Opt-in: animation base values (matrix two-curve
and, since D62, subtype-0 one-curve scale/offset); user-supplied instance color
(ThingColor, D61). Unwired diffuse previews as labelled black (D63,
Toolkit generator evidence; game default unknown). Groups whose other channels need unrendered textures are flagged
and cannot be confirmed Complete.

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- D60 39-GUID cohort: 39 viewable / 35 full diffuse / 0 Complete.
- D60 40-GUID cohort: 40 / 36 / 0.
- D61–D63 48-GUID cohort (d61-mesh-samples.json): default 48/43/0; animation base
  48/45/0; supplied color + animation base 48/48/0. Corpus totals unknown.
- D63: +0 newly viewable, +0 newly Complete.

## Open blockers
- Bevy exe not relinked/captured for D63 (user's viewer window held it open).
- Glow, reflection and other non-diffuse channels are not rendered.
- ThingColor assets need a supplied color; real instance values unknown.
- BORDER color, CLAMP5, mirror-once 6–8, motion/time, filtering fidelity unknown.

## Next
Relink and capture 00020c92; then run a corpus-wide census of unsupported diffuse
routes and pick the next blocker by asset count.

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu).
- Git: local repo at project root, branch `main`, default-deny `.gitignore`.
  Baseline d7d33b2 (D60 + unrecorded D61 code); D62/D63 committed after it. No remote.
- Native QA windows on non-primary monitor 165v0, verified in log before showing.

## Where detail lives
- Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`.
- Milestone docs: `docs/instance-preview.md` (D61), `docs/animation-base-preview.md` (D56, D62),
  `docs/unwired-diffuse-preview.md` (D63).
- Format evidence: `docs/formats.md` (grep headings). Rules: `AGENTS.md`.
