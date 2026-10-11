# Current state (overwrite each milestone) — after D233, 2026-10-11

## Product
Independent LBP2 Asset Explorer (Rust): Bevy viewer (main) and legacy viewer read the
user's own extracted game data. Parsing crates stay independent of UI/rendering; gameplay
runtime (physics, scripts, playable Sackboy) lives in its own crates (AGENTS.md).

## Inputs (read-only), working version 01.33
- `game-data/a1-disc` (BCUS98245 01.00) plus `game-data/a1-update-0133`, passed as
  `--game-dir "game-data/a1-disc;game-data/a1-update-0133"`.
- Snapshot `local-analysis/d68-inventory-0133-verified.json` (~166 MB; summarize by script).
- Toolkit oracle pin d6a2ef80b5a09999f08f23a803b516096055e5c9 under `references/`.

## Latest: D234 animation fitted to game vertices (2026-10-11)
- D233 moods/emotes/head look merged (#130, compiled, tests pass).
- D234: RPCS3's skinned Sackboy vertices read from memory and fitted to our poses
  (`crates/lbp-preview/examples/pose_fit.rs`, `scripts/rpcs3_vdump.py`, viewer env
  `LBP_DUMP_VERTS`). Walk right 0.8, walk left 1.5 (mirror OK), jump now 1.3-1.5 after the
  air weight fix (0.7 -> 0.9, was 1.0). `docs/d234-anim-vertex-fit.md`.
- Emote inputs confirmed in play mode (memory reads): Up/Down/Right/Left = happy/sad/angry/scared,
  levels 1-3-off, replace and opposite-cancel rules, no timeout (docs/d234-anim-vertex-fit.md).

## Before that (2026-10-11)
D231 play mode draws Sackboy (0000043f) with his own clips; D232 blends them by the EBOOT
layer rules with eased facing, mirror and the QA fixes; D232 pass 2: idle roll 0..249, lane
after jump, idle after 300 quiet ticks, seed 42 (`docs/d232-sackboy-anim-rules.md`).
Movement: D230 fall drag / gravity; Moon handoff `docs/handoff-pc-2026-10-11*.md`.

## Measured counts (distinct catalog GUID; Complete = manual, all textures + geometry)
- Mesh census 01.33 (4,261 GUIDs, after D188-D190): 4,259 viewable, 4,239 all-diffuse by
  default, 0 Complete. Files `local-analysis/b177-{default,legacy}.json`.
- Texture census 01.33: all 23,247 hashes decode (top mip, decodability only).
- Levels 01.33: 398 resources; lbp-level walks 236 in-band, Toolkit parity 236/236.
- D233: +0 newly viewable / +0 newly Complete (runtime feature).

## Open blockers
- Stop/turn poses fit 3-5 (need tick-aligned captures); head-top bones 22/23/24/27/28.
- Head-look inputs (right stick, creature+0xc20 target, tilt term, roll 0x808/0x810): OPEN.
- Camera lead while walking; mirror flip types; arms mapping; face morph readers.
- Non-diffuse channels (glow, reflection) only partly rendered; ThingColor needs a supplied
  color; Vita (0x4431) payloads refused.

## Next
1. Arms / stop / turn poses (tick-aligned vertex captures); emote clip frames vs game.
2. Popit open sequence in play mode (Z/Square): card, cord, cursor, Sackboy turns to camera.
3. Camera lead, slopes (paused by the owner).

## Environment
- Toolchain: `. ./scripts/use-local-rust.ps1` (project-local, windows-gnu). Build trees are
  ~11 GB: reuse `target-pr*` through `CARGO_TARGET_DIR`.
- Git: remote origin github.com/chilladept/lbp2-asset-explorer (private), default-deny
  `.gitignore`, CI `.github/workflows/ci.yml`. Standing rules: CLAUDE.md.
- Native QA windows hidden or on the non-primary monitor; never steal focus.

## Where detail lives
Latest full entry: end of `STATUS.md`. Short log: top of `MODLOG.md`. Earlier milestone
docs under `docs/` (grep headings); format evidence `docs/formats.md`; rules `AGENTS.md`.
