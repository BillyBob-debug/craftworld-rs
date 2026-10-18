# Level scripts, part 117: closed menus go (2026-10-18, D370)

A Pod menu that closes or is covered now goes 15 ticks later in scripted mode, as the
native ports' menus already did at once. Before, scripted mode kept every menu frame it
had built, so a popped state never left the pod computer's `OldStates`.

### What the game does (FOUND, scripts)

- `PodComputer.PopState__` calls `OnLoseFocus` and `OnDeactivate(false)` on the popped
  state, then `HasTransitionedOut`: if true the state is destroyed, else it goes into
  `OldStates`. `PodComputer`'s update erases the `OldStates` that have transitioned out.
- `PodComputerState.HasTransitionedOut` is false while the state is active, else
  `!IsValidNode(1)` under the state's frame high bits: the state is done once its frame
  is gone.
- `PodComputer.Draw__m` draws every `StateStack` and `OldStates` state, and each sets
  its frame matrix every tick. `UpdateUI` builds only the active states (from
  `FirstActiveState` to the top of the stack).

### What changed

- `lbp-runtime` (`gooey.rs`): `GooeyState::prune_transitioned_out`, called once a tick
  from `advance_frame`. INFERRED (the engine's Gooey code was not read): a menu frame
  (one with an item matrix) left out of a tick's builds was left at that tick, and goes
  with its matrix and build once its out time (`SetFrameTransitionTime`'s second value,
  15) has passed. Its first build is kept (D359). `IsValidNode(1)` then turns false, so
  the scripts erase the state from `OldStates` themselves.
- The viewer's level-scripts trace writes `menu_frames`, the menu frames held, when they
  change.

MEASURED (`run370b`, group at 680, Back at 700 and 740): scripted mode now holds the
main menu to 655, Story from 641 to 695 and again from 701 to 755, and the group from 681
to 715; native mode holds the same frames, each dropped 15 ticks sooner. The drawn menu
is unchanged in both modes at every tick.

Not done: whether the game keeps a covered state's frame (the main menu under Story); it
is dropped like a popped one.

### Next

Story's slide back from a group (721-730, where the view moves).
