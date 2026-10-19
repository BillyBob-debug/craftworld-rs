# Lane state: level1 (Rookie Test played with scripts) (2026-10-19)

## Read first
1. docs/level1-play.md, sections "Part 5" (native speech bubble, the script trace) and
   "Part 6" (native Launcher); Part 7 is native TriggerMusic
2. crates/lbp-runtime/src/native_script.rs (the D417 registry), native_bubble.rs and
   native_launcher.rs (the smallest port, a template for the next ones)

## State
- PRs 155 (D504) and 228 (D505, native `Launcher`) merged. PR for D506 (branch
  `level1-music`, worktree `E:\Projects\wt-d324`): native `TriggerMusic`; handed to the
  coordinator for merge once rust and python are green.
- Decision numbers: D504-599, next free D507.

## Facts
- Route run: `--level-guid 0001e786 --level-scripts --play-sackboy --qa-hide-ui
  --level-scripts-close-at 120,600,1150 --play-sackboy-input
  "-:620,r:160,rj:20,r:330,f:15,-:5,r:400,rj:20,r:400" --level-scripts-hold-at 2000
  --qa-capture local-analysis/<new>.png`, run from the canonical repo folder (capture under
  its local-analysis, a new name each run).
- No decrypted files: `--snapshot local-analysis/d68-inventory-0133-verified.json
  --game-dir "game-data/a1-disc;game-data/a1-update-0133"` (the d133 edat snapshot needs
  the overlay). Lines open 63/419/882, close 120/600/1150, same as with the overlay.
- Trace check: the route with the overlay, once plain and once with `--native-scripts`,
  traced with the D417 trace (lbp_script_vm::trace). Known difference: 21
  `OnCreate__ set HasUpdateWorkToDo` (VM skips `.init__` on Thing scripts).
- Rookie Test classes: PostItNote 165, Trigger_Speech_Bubble 21, Launcher 19,
  TriggerMusic 9, TutorialMonitor 3, PostItNoteUnlockStickers 1, PostItNoteUnlockSuicide 1,
  Score_Board 1, WorldMooCamera 1 (`level_scripts` example, `instances_by_class`).
- Script listings (research only): local-analysis/d133-script-listings-0133/<sha1>.txt;
  bubble spec local-analysis/level1/p5/tsb-spec.md.
- Line 3 opens on tick 882 at bubble 1485. Sackboy stops at x -938 (a jump clears it) and
  at x -385; with no input after tick 1965 he falls from x -385.
- Toolchain: `. ./scripts/use-local-rust.ps1` from the canonical folder, then cd to the
  worktree; each worktree builds into its own `target/`.

## Next
1. Native classes for the other Rookie Test scripts, one per PR, each checked with the
   trace: Launcher (done, D505), TriggerMusic (done, D506), PostItNote(+Unlock*),
   TutorialMonitor, WorldMooCamera
   (Score_Board may be shared with other lanes; ask the coordinator).
2. The route past x -385: a game reference (RPCS3 F12 frames past line 2) or jumps there.
3. The camera at x -938 (a frame at tick 1600 looks through a front wall).
4. Sackboy pushed into a wall turns to the camera and plays `stand`, not a push.
