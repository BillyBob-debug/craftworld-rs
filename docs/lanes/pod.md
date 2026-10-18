# Lane state: pod / Story (2026-10-18)

## Read first
1. `docs/level-scripts/part-102.md` (and parts 100-101 for the Story stand-in)
2. `crates/lbp-runtime/src/native_script.rs` (ports: Pod, Trigger_Pod, Earth, OrbitingMoon)

## State
- PRs 153 (D353) and 156 (D354) merged. PR for D355 (branch `d355-story`, worktree
  `E:\Projects\wt-d324`): the Pod planets' scripts as native ports; handed to the
  coordinator for merge once rust and python are green.
- The lane owns the Pod/Popit native scripts too (Popit lane stopped, 2026-10-18).
  Decision numbers: D348-D399, next free D356.

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
- Traces and captures: `local-analysis/d353/` (trace-vm3.jsonl the scripted golden run;
  trace-native5.jsonl the no-scripts run with the Story point; vm-900.png vs
  native4-60.png, mean abs 0.36).
- Script listings for research (never commit): `script_census SNAPSHOT GAME-DIRS NEW-JSON
  NEW-DIR` (lbp-level example) wrote `local-analysis/d353/listings/`: PlanetsWorld
  bc1d8f3a..., PlanetsView 0975443c..., PlanetThing 0bc6f2fc....
- Script GUIDs: PlanetsWorld g00016e0a, PlanetThing g00016e09, Earth g000053d0,
  OrbitingMoon g000053ce.
- The decrypted overlay holds only scripts (.ff) and SPU code; every Story data file is on
  the disc or update.
- Measured stand-ins left in `story_fallback.rs`: world Thing 0, the in-use camera and
  Earth's Story `SpinOffset` -0.7; they belong to `PlanetsWorld`/`PlanetsView`.

## Next
1. Port `PlanetsWorld` (UpdatePlanetsView, UpdatePodCamera, UpdateState) and `PlanetsView`
   (SetPlanetsView__i, about 1100 instructions) behind `NativeScript`, checked against
   `trace-vm3.jsonl`; then drop `STORY_LEVEL_THINGS`, `POD_IN_USE_CAMERA` and
   `STORY_EARTH_SPIN_OFFSET`.
2. `PodStateSearch` is a pod computer state, not a Thing script: it stays a plain native
   entry until the pod computer's states are ported.
3. Leaving Story (back to the main menu); more searches.
