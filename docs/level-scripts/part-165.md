# Level scripts, part 165: the native pod computer and the Pod (2026-10-19, D1018)

Parts 163 and 164 call the pod computer's `OnEnterPod__` and `OnExitPod__`. Both need
the computer's script object, which exists only when the game's scripts are loaded. With
the disc and update alone the planets world runs its native port and there is no pod
computer object, so neither call was made and the view type was never kept.

### Found

- The script computer's two calls reach the planets world's `OnEnterPod__` and
  `OnExitPod__` (parts 163, 164); its own `AbortLevelOperation__` has work only when a
  `LevelOperationThread` is running, which the scripts start only for My Levels' delete,
  copy and move (`OnLevelDelete__`, `OnLevelCopy__`, `OnLevelMove__`).

### Inferred

- The native pod computer has no My Levels operations, so its exit has nothing to abort
  and goes straight to the world's `OnExitPod__`.
- Without the scripts there is no `SlotID` object, and only story levels are played, so
  the native `OnEnterPod__` takes the previous level as not a user level and does not ask
  for the level-created dialogue.

### Code

- `LevelScripts::pod_computer_enter` and `pod_computer_exit` call the native port's
  `OnEnterPod__` and `OnExitPod__` on the world Thing when it runs the planets world's
  port (its class not loaded, or `--native-scripts`). A fault or a yield is reported in
  `StepReport::thread_faults`, as for the scripted computer.
- `NativePlanetsWorld::on_enter_pod` reads the previous slot's type as 0 when no
  `SlotID` object exists.
