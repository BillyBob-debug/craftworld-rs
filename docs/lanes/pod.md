# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-100.md`
2. `crates/lbp-runtime/src/native_script.rs` (the Popit lane's shared trait, PR 152)

## State
- PR 153 (D353) merged. PR for D354 (branch `d354-story`, worktree
  `E:\Projects\wt-d324`): the Story screen's pod opens through pod.bin's native port;
  handed to the coordinator for merge once rust and python are green.
- The lane also owns the Pod/Popit native scripts from here (Popit lane stopped,
  2026-10-18). Decision numbers: D348-D399, next free D355.

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
- Traces and captures: `local-analysis/d353/` (trace-vm3.jsonl the scripted golden run,
  trace-native.jsonl the stand-in; vm-900.png vs native-120.png, mean abs 0.36).
- The decrypted overlay holds only scripts (.ff) and SPU code; every Story data file is on
  the disc or update (slot list in DATA.FARC, strings in patch_0133.farc).
- Measured stand-ins left in `story_fallback.rs` (planets Things 0-4, in-use camera)
  belong to the planets world script; pod.bin runs as its D417 native port.

## Next
1. Port the planets level's world script (camera dolly, world Thing, Moon) and Earth's
   planet script behind `NativeScript`, checked against `trace-vm3.jsonl`; then drop
   `STORY_LEVEL_THINGS` and `POD_IN_USE_CAMERA`. `PodStateSearch` is a pod computer state,
   not a Thing script, so it stays a plain native entry until the state machine is ported.
2. Play the moves (pod change-over, world Thing move in, Earth's turn, frame slide) from
   the trace instead of snapping to the end state.
3. Leaving Story (back to the main menu); more searches.
