# Level scripts, part 120: Back's main menu (2026-10-18, D373)

Back from Story now shows the main menu on the tick after Back in native mode, as the
scripts do. Before, the native pop built the main menu on Back's own tick, one tick
early.

### What the game does (MEASURED, scripted runs)

Back reaches the scripts as `PodComputer.OnPlanetCancel`, after the frame's `UpdateUI`
(part 105). The scripted run still draws Story on Back's tick 740 and the main menu from
741, when the main menu's own `UpdateUI` first runs again.

### What changed

- `lbp-runtime` (`pod_computer_native.rs`): `NativePodComputer::pop` no longer builds the
  main menu; the next `after_tick` does, from the top state's update. A Story resumed
  under a group was already built only on the next tick.

MEASURED (`run370b`, tag e1, group at 680, Back at 700 and 740): the native world Thing
and drawn menu equal the scripted ones at every tick from 590, to within 0.001 (float
rounding at 741 and 759). Parity 5 pass.

### Next

The lane's open items (the cursor's height and drawing, the player-count texts, the
warning popup's drawing, the entrance animation).
