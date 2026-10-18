# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-108.md` (part 107 for the group zoom, part 106 for choosing a
   group, part 105 for Back, part 104 for the pod computer's stack and main menu)
2. `crates/lbp-runtime/src/story_native.rs` (`PodStateSearch`),
   `pod_computer_native.rs`, `native_planets_world.rs` (the ports)

## State
- PRs 153 (D353) through 163 (D360) merged. PR for D361 (branch `pod-story-cursor`,
  worktree `E:\Projects\wt-d324`): the stick moves the planet's cursor between results
  (`--level-scripts-move-at`); handed to the coordinator for merge once rust and python
  are green.
- The lane owns the Pod/Popit native scripts too (Popit lane stopped, 2026-10-18).
  Decision numbers: D348-D399, next free D362.

## Facts
- Build (PowerShell): `cd E:\Projects\wt-d324; . ./scripts/use-local-rust.ps1 | Out-Null;
  cargo build -p lbp-bevy-viewer` (the worktree's own `target/`).
- Scripted run (from the canonical checkout): `--snapshot
  local-analysis/d133-inventory-0133-edat.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay"
  --level-guid 00018fdb --pod-camera 1 --pod-shell 000027b9 --level-scripts --window-16-9
  --preview-lighting --qa-hide-ui`, then `--level-scripts-in-use-at 600
  --level-scripts-choose-at 640 --level-scripts-group-at 680 --level-scripts-back-at 720
  --level-scripts-hold-at 760
  --level-scripts-trace PATH --qa-capture PNG` (each capture needs a new name).
  Disc + update only: `--snapshot local-analysis/d79-inventory-0133.json --game-dir
  "game-data/a1-disc;game-data/a1-update-0133"`. A run takes several minutes.
- D361 runs: `local-analysis/d361/` (run 1: group at 680, right 700 and 720, left 740;
  trace-vm-1/trace-nat-1 identical 680-760; cursor notes in cursor-notes.md). The native
  menus are drawn at rest while the scripted ones slide in (Story 640-659, the group
  680-695, Story again from 721); not ported yet.
- Group chain (FOUND): Cross runs `TriggerOnPlanetSearchResultSelected` 0x615664, then the
  script's `OnPlanetSearchResultSelected` via 0x4037bc; when false, jump table 0x4038ac
  (0x3fa448, 0x6589f8) calls `OnPlanetGroupSelected(SlotID,true,true)`. Left/Right between
  results is engine-only (D361, `move_result_cursor`; pick 0x42d584, cursor Planet+0x98).
  `GetPlanetsViewType`: planet 1 in Story is 7 with `GetWantZoom` (group types 4, 8, 9),
  else 6. Group extra zoom 500 for types 4 and 9.
- Result kinds (vtable+0x50, 0xd83ea0 -> 0x452324): group types 3 and 4 are kind 1
  (`OnPlanetGroupSelected`); 8 and 9 go to `OnTweak`; a story level (type 0) is kind 0 if
  it is the pod's current slot, else kind 4 (UI event, 0x4033b4). `OnTweak` pushes
  `PodStateTweakLevel`, whose Play calls `TriggerOnLevelLaunch(slot,1)`.
- Back in the scripts: `PopState` calls `OnLoseFocus`/`OnDeactivate(false)` on the popped
  state and keeps it drawing in `OldStates` until `HasTransitionedOut`; the state below
  gets `OnGainFocus` only. `GooeyState::drawn_menu` breaks same-frame ties toward the
  frame built last; old frames are not pruned.
- Research only (never commit): listings in `local-analysis/d353/listings/`, pseudocode
  `local-analysis/d353/planetsworld-pseudocode.md`,
  `local-analysis/d357/podmenu-pseudocode.md`.
- Script GUIDs: PlanetsWorld g00016e0a, PlanetThing g00016e09, Earth g000053d0,
  OrbitingMoon g000053ce, Pod g00004905, Trigger_Pod g00002d07.
- Native ports run when a Thing's class is not loaded (`Runner`), `--native-scripts`
  forces them. The decrypted overlay holds only scripts (.ff) and SPU code; every Story
  data file is on the disc or update.

## Next
1. Story browsing: Cross on a story level (kind 4; who pushes `PodStateTweakLevel` is
   unknown) and the group's title; then the cursor's flight and sound, and the other
   searches.
2. The default result before the search opens (scripts 23 from tick 600, native none),
   the menus' slide-in for the native ports, and pruning `OldStates` frames.
3. The main menu's move in and the other entries' states as they unlock.
4. The cutscene threads (`PodIntroductionThread`, `LevelUnlockingThread`) and
   `OnEnterPod__`'s voice-over; the interactive music.
