# Level scripts, part 118: Story slides back (2026-10-18, D371)

Back from a group shows Story again. The scripted Story menu slides back in with the
world Thing and settles where the world Thing comes to rest, which is not where Story
first settled. The native port drew it at its first rest.

### What the game does (FOUND, scripts; part 116)

`PodComputerState.SetupFrameMatrix` sets every drawn state's frame matrix every tick
from the world Thing's position and the planets view's fixed target transform. So the
frame follows the world Thing all the time, not only as it first opens.

### What changed

- `lbp-runtime`: `sliding_frame_matrix` (`story_native.rs`) follows the world Thing every
  tick; the D369 window after the first build (`FRAME_SLIDE_TICKS`) is gone. The main
  menu and the search frames use it as before.

MEASURED (`run370b`, group at 680, Back at 700 and 740, tag c2): native Story slides
from (-8095, 2220, 214) at 701 to (-8095, 3386, -2981) at 710 and stays there, as the
scripted one does, at every tick; before, native drew it at its first rest (-8095, 3634,
-3971) from tick 732. The drawn menu now differs only at 601-620 and 640-659 (the native
world Thing's tick of lag, D369) and at 740, where native shows the main menu one tick
before the scripts do. Parity 5 pass.

### Next

The native world Thing's tick of lag.
