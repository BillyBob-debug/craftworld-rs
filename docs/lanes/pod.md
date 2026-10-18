# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-105.md` (part 104 for the pod computer's stack and main menu)
2. `crates/lbp-runtime/src/story_native.rs` (`PodStateSearch`),
   `pod_computer_native.rs`, `native_planets_world.rs` (the ports)

## State
- PRs 153 (D353) through 160 (D357) merged. PR for D358 (branch `pod-search-native`,
  worktree `E:\Projects\wt-d324`): the search state as a native port, Back from Story,
  and the draw tie after Back; handed to the coordinator for merge once rust and python
  are green.
- The lane owns the Pod/Popit native scripts too (Popit lane stopped, 2026-10-18).
  Decision numbers: D348-D399, next free D359.

## Facts
- Build (PowerShell): `cd E:\Projects\wt-d324; . ./scripts/use-local-rust.ps1 | Out-Null;
  cargo build -p lbp-bevy-viewer` (the worktree's own `target/`).
- Scripted run (from the canonical checkout): `--snapshot
  local-analysis/d133-inventory-0133-edat.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"
  --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --window-16-9
  --preview-lighting --qa-hide-ui`, then `--level-scripts-in-use-at 600
  --level-scripts-choose-at 640 --level-scripts-back-at 700 --level-scripts-hold-at 780
  --level-scripts-trace PATH --qa-capture PNG` (each capture needs a new name).
  Disc + update only: `--snapshot local-analysis/d79-inventory-0133.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133"`. A run takes several minutes.
- D358 runs: `local-analysis/d358/` (trace-vm-back4/6.jsonl scripted, trace-native-back1
  native; vm-back6-780 and native-back1-780 captures, both the main menu).
- Back in the scripts: `PopState` calls `OnLoseFocus`/`OnDeactivate(false)` on the popped
  state and keeps it drawing in `OldStates` until `HasTransitionedOut`; the state below
  gets `OnGainFocus` only. `GooeyState::drawn_menu` breaks same-frame ties toward the
  frame built last; old frames are not pruned.
- Research only (never commit): listings in `local-analysis/d353/listings/`, pseudocode
  `local-analysis/d353/planetsworld-pseudocode.md`, `local-analysis/d357/podmenu-pseudocode.md`.
- Script GUIDs: PlanetsWorld g00016e0a, PlanetThing g00016e09, Earth g000053d0,
  OrbitingMoon g000053ce, Pod g00004905, Trigger_Pod g00002d07.
- Native ports run when a Thing's class is not loaded (`Runner`), `--native-scripts`
  forces them. The decrypted overlay holds only scripts (.ff) and SPU code; every Story
  data file is on the disc or update.

## Next
1. The search state's Story browsing: moving between the Story planets and choosing a
   level (`PodStateSearch`'s cursor modes), then the other searches.
2. The default result before the search opens (scripts 23 from tick 600, native none) and
   pruning `OldStates` frames.
3. The main menu's move in and the other entries' states as they unlock.
4. The cutscene threads (`PodIntroductionThread`, `LevelUnlockingThread`) and
   `OnEnterPod__`'s voice-over; the interactive music.
