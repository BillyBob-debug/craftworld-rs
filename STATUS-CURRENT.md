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

## Latest: D506 the music triggers in native Rust (2026-10-19)
- level1 Part 7 (`docs/level1-play.md`): `TriggerMusic` is a native port; without the
  overlay Rookie Test's 9 music triggers start the interactive music as the script does.

## D505 the launch pads in native Rust (2026-10-19)
- level1 Part 6 (`docs/level1-play.md`): `Launcher` is a native port; without the
  overlay Rookie Test's 19 launch pads set their sprite lights as the script does.

## D1018 the native pod computer enters and leaves the Pod (2026-10-19)
- Part 165 (`docs/level-scripts/part-165.md`): without the game's scripts the native
  planets world gets `OnEnterPod__` and `OnExitPod__`; the Pod's view type is kept.

## D1017 the pod computer leaves the Pod (2026-10-19)
- Part 164 (`docs/level-scripts/part-164.md`): launching a level calls the pod computer's
  `OnExitPod__`; the Pod's view type is kept for the return.

## D1016 the pod computer enters the Pod (2026-10-19)
- Part 163 (`docs/level-scripts/part-163.md`): the pod computer's `OnEnterPod__` runs on
  each entry to the Pod; no script restores the launching group's search.

## D1015 no push without links (2026-10-19)
- Part 162 (`docs/level-scripts/part-162.md`): with no pending link the unlock thread
  ends at once and pushes no group (corrects D1014).

## D1014 the unlock thread's group push (2026-10-19)
- Part 161 (`docs/level-scripts/part-161.md`): the unlock thread ends by pushing the
  group under the cursor and moving to its default level.

## D1013 the level-created dialogue (2026-10-19)
- Part 160 (`docs/level-scripts/part-160.md`): the planets world's state update clears
  the level-created dialogue's want; its line needs a My Levels search, not ported.

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
