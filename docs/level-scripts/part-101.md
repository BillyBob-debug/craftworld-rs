# Level scripts, part 101: the Story screen's pod opens through pod.bin's native port (2026-10-18, D354)

Part 100 opened the Story screen without the game's scripts by snapping the pod open (the
change-over value set to 1 and the pod mesh swapped on the first tick, values measured
from the scripted run). D417 (Popit lane) has since ported pod.bin's two scripts (`Pod`,
`Trigger_Pod`) to native Rust behind the shared `NativeScript` trait.

### What changed

- The Story stand-in no longer touches the shell level. It sets the pod computer in use
  on its first tick, as before, and pod.bin's native port answers it the way the script
  does: the change-over value goes up 0.05 a tick from tick 2 and the pod is open on tick
  21 with the mesh swapped to 0x23959. The scripted run does the same from tick 601 to 620.
- `story_fallback::STORY_SHELL_POD` and `LevelWorld::set_thing_mesh` are gone.

### Left out

- The planets level's world script still has no port: the world Thing, the Moon, Earth's
  turn and the in-use camera stay the measured end state of part 100 (no move in).

### Verification

`--level-scripts-none --story` with the d133 snapshot: "native script ports for 2 Things
(Pod, Trigger_Pod; D417)", change-over 0.05 on tick 2, open (1.00) on tick 21, the trace's
shell mesh 12140 then 145753 on tick 21. Held at tick 60 against the scripted run at tick
900: mean abs 0.36 levels, the same as part 100. `cargo fmt --all --check`; clippy on
lbp-runtime and lbp-bevy-viewer with `-D warnings`; lbp-runtime and viewer tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
