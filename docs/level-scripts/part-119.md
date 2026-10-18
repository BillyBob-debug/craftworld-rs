# Level scripts, part 119: the pod computer pushes first (2026-10-18, D372)

The native Pod world Thing now moves on the same tick as the scripted one when the pod
computer opens its main menu and when Story is chosen. Before, it started each of those
moves a tick late, and the native menus were drawn a tick behind.

### What the game does (FOUND, scripts; part 116)

`PodComputer.PerFrameUpdate` is the frame's first script call, before the planets world
blends toward its view (`UpdatePlanetsView`). A state pushed there sets the view the
world moves to on that same frame.

### What changed

- `lbp-runtime` (`pod_computer_native.rs`): `NativePodComputer::before_tick` pushes the
  main menu (in use, nothing on the stack) and Story (Cross on the main menu's Story
  button) before the tick; `after_tick` keeps Back and the top state's update. The main
  menu covered by Story that tick is still drawn on it (its matrix set after the world's
  move) and dropped on the next. INFERRED: Story's own frame is first built on the tick
  after its push (MEASURED in scripted mode, 641).
- The viewer calls `Fallback::before_tick` before the level's tick and handles its event
  with the after-tick one.

MEASURED (`run370b`, tag d2): the native world Thing equals the scripted one at every
tick from 590 (before, it lagged at 601-620 and 640-659), and the drawn menus match there
too. They still differ at 740 (native shows the main menu on Back's tick, the scripts a
tick later) and at 759. Parity 5 pass.

### Next

Back's pop timing at 740.
