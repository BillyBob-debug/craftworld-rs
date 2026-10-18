# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-100.md`
2. `crates/lbp-runtime/src/native_script.rs` (the Popit lane's shared trait, PR 152)

## State
- PR for D353 (branch `d353-story`, worktree `E:\Projects\wt-d324`): the Story screen
  without the game's scripts; handed to the coordinator for merge once rust and python
  are green.
- The lane also owns the Pod/Popit native scripts from here (Popit lane stopped,
  2026-10-18). Decision numbers: D348-D399, next free D354.

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
- Measured stand-ins in `story_fallback.rs` (pod open, mesh 0x23959 on shell Thing 2,
  planets Things 0-4, in-use camera) belong to pod.bin and the planets world script.

## Next
1. Move `PodStateSearchStory` behind `NativeScript` (PR 152) and replace the measured
   pod/planets stand-ins with native pod.bin (already in PR 152) and planets world scripts.
2. Play the moves (pod change-over, world Thing move in, Earth's turn, frame slide) from
   the trace instead of snapping to the end state.
3. Leaving Story (back to the main menu); more searches.
