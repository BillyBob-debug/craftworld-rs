# Level scripts, part 116: the menus slide in (2026-10-18, D369)

The scripted Pod menus slide in as they open: the main menu from tick 601, Story from
641 and a group from 681. The native ports drew each menu where it comes to rest. They
now slide in the same way.

### What the game does (FOUND, scripts)

- No script animates the frame itself. The Pod states set a 15-tick fade
  (`SetFrameTransitionTime(15,15)`, already ported) and, every tick,
  `PodComputerState.SetupFrameMatrix` sets the frame's matrix to the world Thing's
  position (`GetWorld().GetWorldPosition()`) times the inverse of the planets view's
  target transform, a scale and a centring translation.
- So the frame moves with the planets level's world Thing as the planets world script
  carries it into the new view. The view's transform is fixed while it moves, so the
  frame is `W(t) * inverse(W_rest) * rest`.

### What changed

- `lbp-runtime`: `sliding_frame_matrix` (`story_native.rs`) gives that matrix from the
  world Thing now. `MAIN_MENU_WORLD_REST`, `STORY_WORLD_REST` and `GROUP_WORLD_REST`
  hold the world Thing where each frame's rest matrix was measured (ticks 620, 659 and
  695 of the d362 trace).
- The main menu (`main_menu_ui`) and the search frames (`PodStateSearch::update`) use
  it for 30 ticks after the frame is first built (`FRAME_SLIDE_TICKS`, INFERRED: the
  slides settle in 15 to 19), then the rest matrix as before.

MEASURED (run369, the d368 inputs): the native frames follow the scripted path. The
group slides from (-2091, 2593, -4025) to (-2091, 1179, 161) over 681-695 and matches at
every tick. The main menu (601-621) and Story (641-660) follow the same path one tick
later, because the native world Thing moves a tick after the scripted one; at most 1534
units apart mid-slide (tick 610), equal once settled.

Not done: Story built again after a group (721-730), where the view itself moves; it is
drawn at rest. The native world Thing's tick of lag.

FOUND (other searches, for the record): offline the main menu has only Story open;
Community, Recent Activity, Store and Me are locked (state 6, MEASURED). Every other
search needs the network, and My Levels is reached only through the locked Me menu. No
other search can be ported offline yet.

### Next

The default result before the search opens.
