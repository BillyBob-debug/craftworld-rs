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

## Latest: D1107 winches run the piston rule (2026-10-19)
- Logic lane: winches share the piston length function (0xc9790), so they wind and
  pay out on the piston clock; the rope only pulls (INFERRED, 0x960988 flag; P9).

## D1106 speed pistons servo to their signal (2026-10-19)
- Logic lane: a mode 4 piston runs its clock toward the analog signal's fraction of
  its range, full speed until within a tenth (0xc9e80; docs/logic.md P8).

## D1105 a piston's ends are a joint limit (2026-10-19)
- Logic lane: the game never clamps a piston's clock; the solver holds its length
  between base and base + range, so a long-on piston waits before it returns (P7).

## D1104 pistons follow their saved mode (2026-10-19)
- Logic lane: each piston saves its mode; the 27 wired ones are direction pistons that
  ramp out while on and back while off; route reaches x 25,657 (docs/logic.md P6).

## D1103 switched pistons run while their signal is on (2026-10-19)
- Logic lane: each piston keeps the game clock (modStart/Delta/Scale); a switch sets
  its rate, so an on/off piston runs while on and holds while off (docs/logic.md P5).

## D1102 pistons with no input run the game's wave (2026-10-19)
- Logic lane: unwired pistons move between base and base + range on the EBOOT's
  eased wave (0xc95ec); the 27 wired ones wait on the switch clock (docs/logic.md P4).

## D1101 anti-gravity tweakers follow their switch signals (2026-10-19)
- Logic lane: a tweak reads its wired switch (digital, or analog magnitude) as the
  EBOOT does; the 3 wired tweakers in the Rookie Test now switch (docs/logic.md P3).

## D1100 anti-gravity tweakers hold the Rookie Test walkway (2026-10-19)
- Logic lane: tweakers are kept and configuration 0 cancels gravity as the EBOOT does;
  the walkway stays up and the route reaches x 22,785 (docs/logic.md P1-P2).

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
