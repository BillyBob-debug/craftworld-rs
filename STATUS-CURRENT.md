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

## Latest: D358 the Pod search state as a native port (2026-10-18)
- Part 105 (`docs/level-scripts/part-105.md`): `PodStateSearch` replaces the Story
  stand-in; with no scripts Back from Story (`--level-scripts-back-at`) returns to the main
  menu with the scripted search state on every tick 640-780, and the scripted run's
  main menu now draws after Back too (a draw tie went to the popped Story frame).

## D357 the pod computer's menu states as native ports (2026-10-18)
- Part 104 (`docs/level-scripts/part-104.md`): with no scripts, engaging the pod computer
  gives the main menu (the scripted frame exactly, tick 601), Story is chosen from it
  (tick 640) and Back pops each state; `--story` is no longer needed to reach Story.

## D356 the Pod planets' world script as a native port (2026-10-18)
- Part 103 (`docs/level-scripts/part-103.md`): `PlanetsWorld`/`PlanetsView` run natively;
  with no scripts the Story camera dolly, the level's view blend, Earth and the Moon match
  the scripted run exactly, and the plain Pod shows its planets. No measured stand-ins left.

## D354 the Story screen's pod opens through pod.bin's native port (2026-10-18)
- Part 101 (`docs/level-scripts/part-101.md`): the Story stand-in sets the pod computer in
  use and the D417 port of pod.bin opens the pod (0.05 a tick, open on tick 21, mesh
  swapped), as the scripts do; the measured pod-open values are gone.

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
