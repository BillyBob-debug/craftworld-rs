# Current state (overwrite each milestone) — after batch D115-D123, 2026-10-08

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
Two textures x static COLOR under two multiplies (D75). Vita branch 0x4431 headers
parse; payloads refused (D77). FNTb font headers and glyph tables (D79).
Animation-base opt-in also covers 2+ texture products with one animated leaf (D78).
Two-texture alpha MIX on the Repeat grid (D83). Static BLEND phase 1, 0 users (D84).
Legacy CLAMP 5 edge-clamped (D82). CONST4 and COLOR x COLOR solids; TEX with a CONST
UV scale (D87). Constant THING_COLOR products fold under a supplied color (D86);
textured THING_COLOR shapes too (D88). ADD of two textures / TEX x COLOR + COLOR (D89).
Animation base for 2+ animated leaves (D92). GTF A8R8G8B8 and B8 textures (D91).
Both viewers show census counts: --mesh-census/--texture-census/--census-baseline (D93).
B8 up to 16 Mi pixels and BC1 cube faces decode (D95). Sampler-port constants and
ThingColor fold under the instance color (D96). BLEND threshold rule general (D94);
masks red sRGB / alpha linear, two-sample BLEND on the grid (D97). MULTIPLY trees and
off-grid alpha MIX via Bevy dual path (D99). All texture formats on 01.33 decode (D98).
General static graph evaluator on the Repeat grid as fallback (D100). Four-leaf trees
without shared pairs, COLOR-only trees (D101). Matrix masks, animated x COLOR (D102).
Off-grid graphs resampled (labelled preview approximation, D103). UV word 2 = UV0 (D104).
Evaluator on instance-color graphs (D105). Asset work paused 2026-10-08.
Levels: lbp-level crate reads 236 in-band 01.33 levels (D107); Toolkit LevelOracle (D106).
Also: .trans string tables (D109), mesh bones (D110), font glyphs (D112).
Bevy level mode --level-path/--level-guid (D113) with straight-extruded shapes (D114);
tutorial_23 renders with its backdrop (D119), inverse-bind/joint placement (D120),
--preview-lighting from level settings (D121), --shape-bevels (D122, inferred).
Runtime: skin/morphs (D115), animation keys (D116), --play-animation GPU/CPU skinning
(D118), lbp-gooey UI + --ui-demo (D117, no per-frame rebuild since D124), slot/pin/
palette readers (D123; 15/16 complete, 1 excluded test file). RScript reader (D126; 0
scripts readable on 01.33, likely in encrypted patch.edat). Runtime scope per AGENTS.md (D125).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- 01.33 (D69): D61 cohort 48/43/0; D65 48/38/0; D66 22/7/0; D64 26/23/0 (= D67 on 01.00).
- D69 new cohort d68-new-mesh-samples.json (48 post-951 meshes): 48/45/0.
- Corpus mesh census on 01.33 after D105 (all 4,261 mesh GUIDs): 4,257 viewable, 3,700
  all-diffuse, 0 Complete (D81 baseline 3,657). Opt-ins: --animation-base 3,767;
  --instance-rgba 3,971; both 4,055. Files b105-corpus-*.json.
- Corpus texture census on 01.33 after D98: all 23,247 texture hashes decode (23,477
  GUIDs; top mip, decodability only). b99-texture-census-0133.json.
- Corpus selection census on 01.33 after D105 (8,094 material locations with MAP rows):
  7,003 texture, 620 solid, 10 unwired, 126 animation-base-only, 301 instance-only, 34
  unsupported (after D102: 6,989 / 296 / 53) (after D75: 60 / 225 / 258; before D75: 6,828 texture, 386 unsupported) (01.00: 5,163 / 4,306 / 459 / 7 / 34 / 122 / 235). Not asset counts.
- D69: +48 newly viewable in the tested cohort, +0 newly Complete; corpus totals unknown.
- Batch D72/D71/D73: tests and CI only, +0 viewable / +0 Complete.
- Batch D77/D76/D75: +0 viewable / +0 Complete; D75 cohort (48 meshes) 48/0/0 -> 48/44/0.
- Batch D79/D80/D78: +0 viewable / +0 Complete; cohorts unchanged.
- Batch D81-D84: +0 viewable / +0 Complete vs the D81 corpus baseline.
- Batch D85-D87: +0 viewable / +0 Complete; corpus all-diffuse +8.
- Batch D88-D90: +0 viewable / +0 Complete; corpus all-diffuse +4.
- Batch D91-D93: +0 viewable / +0 Complete; textures +242 hashes decode.
- Batch D94-D96: +0 viewable / +0 Complete; textures +29 hashes decode.
- Batch D97-D99: +0 viewable / +0 Complete; all-diffuse +14 default; textures +3 (all decode).
- Batch D100-D102: +0 viewable / +0 Complete; all-diffuse +12 default.
- Batch D103-D105: +0 viewable / +0 Complete; all-diffuse +5 default.
- Levels on 01.33: 398 resources; Toolkit parses 385; lbp-level walks 236 (all in-band),
  parity 236/236. 76 LBP2 story levels (inferred). Batch D106-D112: +0 / +0.

## Open blockers
- Glow, reflection and other non-diffuse channels are not rendered.
- ThingColor assets need a supplied color; real instance values unknown.
- BORDER color, mirror-once 6–8 (0 users on 01.33), motion/time, filtering fidelity unknown.
- No unsupported resource headers left on 01.33. Vita (0x4431) payloads refused;
  FNTb +8 word meaning unknown; patch.edat not decrypted.

## Next
RPCS3 measurements (docs/rpcs3-measurement.md) once the owner OKs the downloads; script
bytes for G10 (patch.edat).
Level visual QA against the game; joint skinning; bevel/UV calibration; Rookie Test and
more story levels on screen; Toolkit spot-check for U4 slot lists (not done).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu).
- Git: branch `main`, remote origin github.com/chilladept/lbp2-asset-explorer (private),
  default-deny `.gitignore`. CI: `.github/workflows/ci.yml` (D72).
- Workflow: `docs/workflow.md`. On demand; from 2026-10-07 batches run as local agents
  on plan usage (no new cloud sessions). Agents push `cloud/*` or `claude/*` branches; the PC Integrator verifies and merges passing ones into main without asking
  (owner approval 2026-10-07). Daily task LBP2-DailyBatch is disabled; its script was
  removed (PR #13). CI pinned to Rust 1.99.0.
- Native QA windows on non-primary monitor 165v0, verified in log before showing.

## Where detail lives
- Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`.
- Last batch: `docs/batch-report.md`; ranked backlog: `docs/backlog.md`.
- Update/overlay: `docs/update-overlay.md` (D68 data, D69 code). Earlier milestone docs:
  `docs/third-sample-preview.md` (D65, D67), `docs/material-budget-preview.md` (D66),
  `docs/route-census.md` (D64), `docs/unwired-diffuse-preview.md` (D63).
- Format evidence: `docs/formats.md` (grep headings). Rules: `AGENTS.md`.
