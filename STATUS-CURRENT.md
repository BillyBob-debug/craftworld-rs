# Current state (overwrite each milestone) — after D352

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D361 moving between results on the planet (2026-10-18)
- Part 108 (`docs/level-scripts/part-108.md`): the stick moves the planet's cursor to the
  result that way, scored on Earth's surface as the engine does (0x42d584); the QA option
  `--level-scripts-move-at N:DIR` gives the same picks and an identical frame with and
  without the scripts.

## D360 a Story group zooms in on its planet (2026-10-18)
- Part 107 (`docs/level-scripts/part-107.md`): with no scripts, a chosen Story group
  selects its default level, wants the zoom view (7) and the 500 extra planet zoom, so the
  planets zoom in as the scripted run does (planets level identical on every tick 680-760).

## D359 choosing a Story group, and Back to Story (2026-10-18)
- Part 106 (`docs/level-scripts/part-106.md`): with no scripts, choosing a Story group
  (`--level-scripts-group-at`) pushes the group's search state over Story and Back brings
  Story back on top, with the scripted search state on every tick 640-760.

## D358 the Pod search state as a native port (2026-10-18)
- Part 105 (`docs/level-scripts/part-105.md`): `PodStateSearch` replaces the Story
  stand-in; with no scripts Back from Story (`--level-scripts-back-at`) returns to the main
  menu with the scripted search state on every tick 640-780, and the scripted run's
  main menu now draws after Back too (a draw tie went to the popped Story frame).

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Foot placement on slopes and steps (the standing spot is fixed in model space, not the contact point).
- Landing: only a standing jump is captured (ledge fall, running jump: same tables, unchecked). Upper arms
  2 to 3 degrees off late in the flight, unattributed (the arm terms are 0 there).
- Head-top bones 22/23/24/27/28.
- After a landing the body rests 0.5 to 1.5 above the spawn height (shape cast distances).
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; arm buttons (the arm clips); face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Emote clip frames vs game.
2. Popit: page titles per collection, pod button condition, focus tile from highlight sizing.
3. Camera lead, slopes (paused by the owner).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu). Build trees are
  ~11 GB: reuse `target-pr*` through `CARGO_TARGET_DIR`.
- Git: remote origin github.com/chilladept/craftworld-rs (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
