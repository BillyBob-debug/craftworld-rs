# Level scripts, part 160: the level-created dialogue (2026-10-19, D1013)

Entering the Pod after a valid locally stored user level sets `WantLevelCreatedDialogue`
(part 154). The native planets world set it and never read it; its state update
fetched the pod computer and dropped it.

### Found

- The end of `UpdateState__` (script bc1d8f3a, `PlanetsWorld`), reached in state 0 when it
  starts neither the intro nor the unlock thread: with `WantLevelCreatedDialogue` set
  and `CanStartEndOfLevelSequence__` true, it asks the pod computer it fetched first for
  `GetMyLevelsState__`. If that is not null it calls the state's `StartVO__` with
  `pod_after_saved_1st_level` (part 159: interrupt set, replay clear, kept in
  `_StartedVO`). Either way it clears `WantLevelCreatedDialogue`. While either test
  fails the want stays.
- `GetMyLevelsState__` (script 16e6e8c9, `PodComputer`) walks the state stack from the
  top and returns the first `PodStateSearch` whose details are `SearchDetailsMyLevels`,
  else null.
- `SetState__` fetches the pod computer too and does not use it.

### Code

- The native `UpdateState__` clears the want once `CanStartEndOfLevelSequence__` is
  true (always, D383). The port's stack has no My Levels search (main menu, Story and a
  group only), so `GetMyLevelsState__` is taken as null and no line is asked for.

### Next

The line, once a My Levels search is on the stack.
