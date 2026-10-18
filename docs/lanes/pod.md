# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-103.md` (parts 100-102 for the Story stand-in and planets)
2. `crates/lbp-runtime/src/native_script.rs`, `native_planets_world.rs` (the ports)

## State
- PRs 153 (D353), 156 (D354) and 158 (D355) merged. PR for D356 (branch `d356-story`,
  worktree `E:\Projects\wt-d324`): `PlanetsWorld`/`PlanetsView` as native ports; handed to
  the coordinator for merge once rust and python are green.
- The lane owns the Pod/Popit native scripts too (Popit lane stopped, 2026-10-18).
  Decision numbers: D348-D399, next free D357.

## Facts
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`,
  `CARGO_TARGET_DIR=E:/Projects/wt-d324/target-d320`; viewer tests
  `cargo test -p lbp-bevy-viewer --bins`.
- Scripted run (from the canonical checkout): `--snapshot
  local-analysis/d133-inventory-0133-edat.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"
  --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --window-16-9
  --preview-lighting --qa-hide-ui --story` (scripted: in use 600, choice 640). Add
  `--level-scripts-none` for the shipped case, `--level-scripts-trace PATH` for a trace.
  Disc + update only: `--snapshot local-analysis/d79-inventory-0133.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133"` (also for the `level_scripts` example).
- Traces and captures: `local-analysis/d353/` (trace-vm3.jsonl the scripted golden run;
  trace-native7.jsonl the all-native run; vm-900.png vs native7-90.png, mean abs 0.36).
- Research only (never commit): listings in `local-analysis/d353/listings/` (lbp-level
  example `script_census SNAPSHOT GAME-DIRS NEW-JSON NEW-DIR`), PlanetsWorld/PlanetsView
  pseudocode `local-analysis/d353/planetsworld-pseudocode.md`.
- Script GUIDs: PlanetsWorld g00016e0a, PlanetThing g00016e09, Earth g000053d0,
  OrbitingMoon g000053ce, Pod g00004905, Trigger_Pod g00002d07.
- Native ports run when a Thing's class is not loaded (`Runner`), `--native-scripts`
  forces them; the viewer prints "scripted Things and their native ports".
- The decrypted overlay holds only scripts (.ff) and SPU code; every Story data file is on
  the disc or update.

## Next
1. The pod computer's states (main menu `PodStateMainMenu`-style states and
   `PodStateSearch`) as ports, so the menu and Story are reached by the player rather than
   `--story`; `PodStateSearchStory` then becomes the search state's port.
2. The cutscene threads (`PodIntroductionThread`, `LevelUnlockingThread`) and
   `OnEnterPod__`'s voice-over; the interactive music.
3. Leaving Story (back to the main menu); more searches.
