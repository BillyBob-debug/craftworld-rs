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

## Latest: D1001 where a key's kind 2 comes from (2026-10-19)
- Part 148 (`docs/level-scripts/part-148.md`): a search key of kind 2 is a user's
  result; Story slots only give kind 1, so the Pod is unchanged.

## D1000 the native intro's video request (2026-10-19)
- Part 147 (`docs/level-scripts/part-147.md`): the native Pod intro asks for its video
  as the game's script does; no video plays yet, so its wait is unchanged.

## D399 the unlock voice-overs (2026-10-19)
- Part 146 (`docs/level-scripts/part-146.md`): a finished unlock cluster in the native
  Pod asks for the game's line for the level or theme it unlocked.

## D398 the native intro's voice-over (2026-10-19)
- Part 145 (`docs/level-scripts/part-145.md`): the native Pod intro asks for its
  voice-over line and waits for it, as the game's script does.

## D397 voice-over timing (2026-10-19)
- Part 144 (`docs/level-scripts/part-144.md`): voice-over lines play silently for
  their length from the disc's bank, so the Pod's scripts wait as the game does.

## D396 a group's lock (2026-10-19)
- Part 143 (`docs/level-scripts/part-143.md`): a developer group or DLC pack is locked
  while all its slots are, as 0x1a7efc gives; the runtime's lock test follows.

## D395 the active link look (2026-10-19)
- Part 142 (`docs/level-scripts/part-142.md`): a link to a locked level is drawn white
  with the active texture band; pending (green) once the level unlocks.

## D394 the line shader's constant and the active test (2026-10-19)
- Part 141 (`docs/level-scripts/part-141.md`): the link shader's constant is read and
  only one badge class can make a link active; links stay drawn pending.

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
