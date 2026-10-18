# Level scripts, part 104: the pod computer's menu states as native ports (2026-10-18, D357)

Without the decrypted patch.edat no pod computer state ran: engaging the pod computer
opened the pod and nothing more, and the Story screen was reached only with `--story`. The
pod computer's state stack (`PodComputer`), its main menu (`PodStateMainMenu`) and the
search state's Story entry now run natively, so a player with the disc and the update
alone engages the pod computer, gets the main menu, chooses Story and backs out again.

### What was found

FOUND (the 01.33 scripts, read from the decrypted overlay for research only; nothing of
them is in the repository):

- `PodComputer.PerFrameUpdate` pushes a new main menu when the pod computer is in use and
  its stack is empty, then updates the top state.
- A push deactivates the states under it and asks the planets world for the new top
  state's view (`GetPlanetsViewType`: 1 for the main menu, the search state's 6 for
  Story). A pop invalidates the state's frame; the last pop asks the host to stop using the
  pod computer (`RequestHostUsePod(false)`) and the planets world for view 0.
- The main menu focuses Story (`EnsureNodeOrDescendantHasFocus(6)`) on its first build;
  choosing Story (its button answers 256) pushes the Story search; the root frame's
  `EndFrameInput(512)` answering 512 pops the state.

MEASURED (the scripted golden trace, in use from tick 600, Story chosen on tick 640): the
main menu is first built on tick 601, its matrix settles on tick 620, Story's search opens
on tick 640 and its frame is built from tick 641.

### What changed

- `lbp-runtime/src/pod_computer_native.rs`: `NativePodComputer`, the state stack
  (`PuterState::MainMenu`, `PuterState::StorySearch`), run after each tick of the planets
  level on the gooey input (choose, back). It answers `PodComputerEvent`s: a state pushed,
  popped, or the last one popped (`Released`, the session's `ask_to_close`). Views go to
  the planets world's native port (`SetViewFromRemotePodComputer__i`). `main_menu_frame()`
  is the trace's frame (Story open; Community, Recent Activity, Store and Me locked; the
  player's card; the friends frame).
- `PodStateSearchStory::close`: what opening and updating Story set is cleared, the
  counts kept (INFERRED: the search state's `OnDeactivate` was not traced).
- The viewer's Story fallback is the pod computer: `--story` pushes the search over the
  main menu on the first tick as before; otherwise engaging the pod computer gives the
  menu, Enter or Cross chooses, and the Puter key's Back pops.

### Left out

- The main menu's move in (ticks 601-619): the frame is drawn where it comes to rest.
- The other main menu entries (all locked in the trace), the friends list, and the search
  state's other searches.
- `PodComputer`'s cutscene and loading states.

### Verification

Disc and update only, in use at 600 (`--level-scripts-in-use-at 600`): the main menu is
pushed on tick 601 with planets view 1; its frame equals the scripted trace's tick-601
frame string exactly. With `--level-scripts-choose-at 640` the Story search is pushed on
tick 640 with view 6 and its frame built on tick 641, as in the trace. Tests
(`pod_computer_native.rs`): `d357_main_menu_when_in_use`, `d357_story_and_back_out`,
`d357_open_story`, `d357_choose_elsewhere_stays`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
