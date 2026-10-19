# Level scripts, part 163: the pod computer enters the Pod (2026-10-19, D1016)

Part 152 ported `PlanetsWorld.OnEnterPod__`, but nothing called it: the pod computer's
own `OnEnterPod__` export, which the engine calls, was never run. The question behind it
was which search the Pod shows after a finished level.

### Found

- No script restores the group search a level was launched from. `PodComputer`
  (script 16e6e8c9) `OnDestroy__` clears the state stack; `PerFrameUpdate__b` 0208-0262
  pushes `PodStateMainMenu` when the computer is in use and the stack is empty (the
  Story search instead only in the E3 build). `LevelUnlockingThread` pushes the Story
  search only for pending links (part 127).
- `PodComputer.OnEnterPod__` 0002-0005 calls the planets world's `OnEnterPod__`; 0006-0020
  call `SelectMoonLevel__` with the previous level's slot when its type is 2 and
  `IsMoonSelected__`; the rest is the Move, PSN and kiosk demo builds.
  `SelectMoonLevel__` pushes a My Levels search (or the group of the level's parent) and
  moves the cursor onto the level.
- 01.33 executable: the export's wrapper 0x624c88 is reached through 0x5044a0 and
  0x89445c from 0x3fb800, slot 3 of the pod computer's vtable 0xd73d40; `OnExitPod__`
  (0x624bfc) through 0x504b1c and 0x8946ec from 0x403f0c, slot 4. 0x401c30 switches the
  pod mode (0 none, 1 the pod computer 0xda1870, 2 the other pod object 0xda1874): on a
  change it calls the old object's slot 4 and the new one's slot 3. Its callers, 0x1ab508
  (a level becomes current, in its pod branch), the level swap 0x25fdb0 and 0x1a7580, all
  test the level's "is a pod" byte (+0x3b5). The object's creation (0x3ff034) does not
  call it, so it runs on the first load and on every return.

### Inferred

- The level's first tick stands for its becoming current, before the first
  `PerFrameUpdate__b`. Mode 1 is the player's own Pod.
- The moon is never selected in the port (no My Levels search), so `IsMoonSelected__`
  answers false and the moon branch is not reached.

### Code

- `LevelScripts::pod_computer_enter` calls `OnEnterPod__` once on the planets level's
  first tick, before `PerFrameUpdate__b`; the viewer reloads the level on a return, so it
  runs again then. `IsMoonSelected__` is a host native answering false.
