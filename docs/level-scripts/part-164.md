# Level scripts, part 164: the pod computer leaves the Pod (2026-10-19, D1017)

Part 163 called the pod computer's `OnEnterPod__` on each entry to the Pod. Its pair,
`OnExitPod__`, was not called, so the planets world's `OnExitPod__` (ported in D1005)
never kept its view type, and the entry always restored view type 0.

### Found

- `PodComputer.OnExitPod__` (script 16e6e8c9) calls `AbortLevelOperation__`, then the
  planets world's `OnExitPod__` through `GetWorld__`. `AbortLevelOperation__` asks the
  computer's `LevelOperationThread` to abort with `RequestAbort__` when it has one, clears
  it and sets `NeedUpdate`.
- The planets world's `OnExitPod__` copies `ViewType` into `LocalViewType`, and its
  `OnEnterPod__` sets `ViewType` and the planets view from `LocalViewType` (part 152).
- 01.33 executable: the export's wrapper 0x624bfc is reached through 0x504b1c and
  0x8946ec from 0x403f0c, slot 4 of the pod computer's vtable 0xd73d40. The pod mode
  switch 0x401c30 calls it on the old object when the mode changes (part 163).

### Inferred

- A level launched from the Pod is that mode change.
- The game keeps the Pod's world while a level plays, since the member is written on
  exit to be read on entry. The viewer reads the Pod again on a return, so the session
  carries the member across the switch.

### Code

- `LevelScripts::pod_computer_exit` calls `OnExitPod__` in the planets level and keeps
  the world's `LocalViewType` in `LevelWorld::pod_view_type`. The viewer calls it when a
  level switch leaves the Pod and carries the value to the next level.
  `pod_computer_enter` sets it back on the world before `OnEnterPod__`.
