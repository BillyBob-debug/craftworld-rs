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

## Latest: D1119 the control pad switch (2026-10-19)
- Logic lane: control pads (535) give the driver's buttons, sticks and tilt on 18
  outputs with the game's dead zone and inversion (0x11d378; docs/logic.md Part 21).

## D1118 the grab switch (2026-10-19)
- Logic lane: grab switches (666) are on while a player grabs their object, by the
  game's three modes, PlayerMode and tag filter (0x10f508; docs/logic.md Part 20).

## D1117 the velocity switches (2026-10-19)
- Logic lane: linear (771) and angular (122) velocity sensors follow the EBOOT rule:
  body motion over the scale, seven modes, local frame (docs/logic.md Part 19).

## D1116 the projectile and paint switches (2026-10-19)
- Logic lane: projectile (816) and paint (51) sensors output hits / required with
  the game's held ternary and empty 6 ticks after full (docs/logic.md Part 18).

## D1115 the magic mouth switch (2026-10-19)
- Logic lane: magic mouths (1,426) read their script's OutputEndSignalFrames in the
  switch pass, as the game does, so inverted mouths work (docs/logic.md Part 17).

## D1114 the camera finished switch (2026-10-19)
- Logic lane: camera finished switches (2,760) follow the game's cutscene timer:
  hold time, the 3-tick ending and 9 ticks after (docs/logic.md Part 16).

## D1113 the impact switch (2026-10-19)
- Logic lane: impact switches (3,944) follow the EBOOT rule from this tick's physics
  contacts: player mode, touching and the tag filter (docs/logic.md Part 15).

## D1112 every switch type counted; the random switch (2026-10-19)
- Logic lane: a census of all 141,572 switches in 358 levels by type, and the random
  switch's dice, pick patterns and on / off times from the EBOOT (docs/logic.md P14).

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
