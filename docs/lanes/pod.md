# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-129.md` (128 for the finish, 127 for the unlock sequence, 126
   for the pod introduction, 125 for the warning popup, 124 for the start checkpoint, 123
   for the spawn hold, 122 for the player texts, 121 for the cursor, 120 for Back, 119 for
   the pushes, 118 for Story back, 117 for closed menus, 116 for the slide-in, 115 for the
   cursor's flight, 114 for the launch warnings, 113 for Sackboy, 112 for the entrance
   camera, 111 for the switch, 110 for the hand-off, 109 for Cross on a level, 108 for the
   cursor, 107 for the group zoom, 106 for choosing a group, 105 for Back, 104 for the pod
   computer's stack and main menu)
2. `crates/lbp-runtime/src/story_native.rs` (`PodStateSearch`),
   `pod_computer_native.rs`, `native_planets_world.rs` (the ports)

## State
- PRs 153 (D353) through 185 (D381) merged. PR for D382 (branch `pod-next`,
  worktree `E:\Projects\wt-d324`): back to the Pod after a finish;
  handed to the coordinator for merge once rust and python are green.
- The lane owns the Pod/Popit native scripts too (Popit lane stopped, 2026-10-18).
  Decision numbers: D348-D399, next free D383.

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
- D373 runs: `local-analysis/d370/*-e1*` (`run370b.ps1`): native world Thing and drawn
  menu equal scripted at every tick (0.001 float rounding at 741, 759).
- D372 runs: `local-analysis/d370/*-d2*` (`run370b.ps1`): the native world Thing equals
  scripted at every tick; menus differ only at 740-741 and 759 (Back's pop).
- D371 runs: `local-analysis/d370/*-c2*` (`run370b.ps1`): native Story slides back
  exactly as scripted; menus differ only at 601-620, 640-659 (lag) and 740.
- D370 runs: `local-analysis/d370/*-b2*` (`run370b.ps1`, group at 680, Back at 700
  and 740): scripted frames go 15 ticks after native's; the drawn menu is unchanged.
- D369 runs: `local-analysis/d369/*-a1*` (`run369.ps1`): native menus follow the
  scripted path (group exact, main menu and Story a tick later).
- D368 runs: `local-analysis/d368/*-a1*`, `*-a2*` (`run368.ps1`): moves at 700, 720,
  740 land at 707, 726, 743; a move mid-flight is refused.
- D367 runs: `local-analysis/d367/*-a1*` (`run367.ps1`, as D366's): no warnings for
  Trash Valley, switch at tick 761. Dispatcher disassembly:
  `local-analysis/d367/disp*.asm`.
- D366 runs: `local-analysis/d366/*-a*` (`run366.ps1`, as D365's): both read Trash
  Valley's physics after the switch and spawn Sackboy at Thing 33, (-11560, 19306, 200).
- D365 runs: `local-analysis/d365/*-a*` (`run365.ps1`, as D364's): both start
  0001f333 at checkpoint Thing 33, spawn (-11560, 19306, 200), the entrance pipe.
- D364 runs: `local-analysis/d364/` (`run364.ps1` in the session scratchpad: run 2 with
  hold at 900): both switch after tick 761 to 0001f333. With `--preview-lighting` the
  level's overview frame is fog colour only (also when loaded alone); without it, it
  draws.
- D363 runs: `local-analysis/d362/*-d363a*` (as run 2): both hand slot (0, 127795) to
  the game after tick 761, root GUID 0001f333, which loads alone with `--level-guid
  0001f333`. Notes `local-analysis/d363/level-start-notes.md`. The viewer has no runtime
  level switch: `level_play::install` builds `Play` once; `load()` in main.rs clears and
  respawns only the display entities.
- D362 runs: `local-analysis/d362/` (run 2: group at 680, right 700 and 720, left 740,
  Cross 760, Back 775: both launch slot (0, 127795) and stay on the group; run 3 without
  Cross: both pop; notes in story-level-notes.md). D361 runs in `local-analysis/d361/`.
  The native menus follow the world Thing every tick (D369, D371), Story back from a
  group included.
- Other searches (D369): offline only Story is open; Community, Recent Activity, Store
  and Me are locked (state 6), the rest need the network, My Levels sits under Me.
- Group chain (FOUND): Cross runs `TriggerOnPlanetSearchResultSelected` 0x615664, then the
  script's `OnPlanetSearchResultSelected` via 0x4037bc; when false, jump table 0x4038ac
  (0x3fa448, 0x6589f8) calls `OnPlanetGroupSelected(SlotID,true,true)`. Left/Right between
  results is engine-only (D361, `move_result_cursor`; pick 0x42d584, cursor Planet+0x98).
  `GetPlanetsViewType`: planet 1 in Story is 7 with `GetWantZoom` (group types 4, 8, 9),
  else 6. Group extra zoom 500 for types 4 and 9.
- Result kinds (vtable+0x50, 0xd83ea0 -> 0x452324): group types 3 and 4 are kind 1
  (`OnPlanetGroupSelected`); 8 and 9 go to `OnTweak`; a story level (type 0) is kind 0 if
  it is the pod's pending or current slot (0x3fce40), else kind 4:
  `TriggerOnLevelLaunch(slot, 0)` (0x4033b4), which stores the pending slot at pod+0x954
  and sets pod+0x9c4 to 1; `OnPlanetCancel` returns while it is pending. `OnTweak` pushes
  `PodStateTweakLevel`, whose Play calls `TriggerOnLevelLaunch(slot,1)`.
- Back in the scripts: `PopState` calls `OnLoseFocus`/`OnDeactivate(false)` on the popped
  state and keeps it drawing in `OldStates` until `HasTransitionedOut`; the state below
  gets `OnGainFocus` only. `GooeyState::drawn_menu` breaks same-frame ties toward the
  frame built last. A menu frame left out of a tick's builds goes 15 ticks later (D370,
  INFERRED); `HasTransitionedOut` is `!IsValidNode(1)` once inactive (FOUND).
- Research only (never commit): listings in `local-analysis/d353/listings/`, pseudocode
  `local-analysis/d353/planetsworld-pseudocode.md`,
  `local-analysis/d357/podmenu-pseudocode.md`.
- Script GUIDs: PlanetsWorld g00016e0a, PlanetThing g00016e09, Earth g000053d0,
  OrbitingMoon g000053ce, Pod g00004905, Trigger_Pod g00002d07.
- Native ports run when a Thing's class is not loaded (`Runner`), `--native-scripts`
  forces them. The decrypted overlay holds only scripts (.ff) and SPU code; every Story
  data file is on the disc or update.

## Next
1. Done: the launch path (spawn hold D376, start checkpoint D377, warning popup D378;
   the state 6/7/8 load is a handshake the viewer's switch matches, D379).
   (The native and scripted menus match through Back, D373; the default result 23
   before tick 640 is scripted mode's own seeding at load, native's none is kept, D369.
   The cursor's height and blend have no reader and its drawing is script-side, D374.)
2. The other entries' states as they unlock (the other searches need the network).
3. A finished level leaves its pending link (D381) and returns to the Pod (D382);
   next, `LevelUnlockingThread` (read, D380). It, the intro and `OnEnterPod__`
   also wait on audio (none plays yet). The interactive music needs the .fev.
