# Current state (overwrite each milestone) — after D337

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D337 the Story globe's jelly on the glass route (2026-10-16)
- Part 84 (`docs/level-scripts/part-84.md`): `proc_victoria_jelly_glassy` (0x1a6f5) takes
  the glass route (opacity 1, rim in full); a static gloss colour above 1 no longer drops a
  material's whole surface in lbp-preview.
- FOUND: the globe's black continents are the AO overlays `cloth_earth_land_ao` /
  `cloth_earth_sea_ao` (black on port 0, texture x 0.8 alpha on port 1), drawn opaque.
- Next: D338 the AO overlays blended by their port-1 alpha; the badge card and bubble.

## D336 the Story backdrop and its lighting (2026-10-16)
- Part 83 (`docs/level-scripts/part-83.md`): while Story is up the viewer swaps the Pod's
  backdrop for The Cosmos's own (0xdb2c4, FOUND) and lights the Pod world with that
  backdrop plan's preset under The Cosmos's PWorld factors (0xda074, FOUND). With
  `--lbp-lighting` the room fades to dark with coloured dots as in the game.
- D331 (part 82): Story turns Earth to The Cosmos; Story never zooms (view 6).
- D330 (part 81): the Story title's speech bubble.

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
2. Popit: compare more frames, cursor mode, inventory grid pages (submode 11).
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
