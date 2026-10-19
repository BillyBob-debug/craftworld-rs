# Level scripts, part 152: entering the Pod (2026-10-19, D1005)

`PlanetsWorld.OnEnterPod__` and `ResetPodInteractiveMusic__` had no native port, so the
native world answered them with "no function". This part ports both.

### Found

- `OnEnterPod__` (script g0000248b), in order:
  - `State` = 0.
  - `ResetPodInteractiveMusic__`: `IMusicInit` = true, `IMusicEnabled` and
    `IMusicPlaying` = false, so the next frame sets the music and its pair volumes
    again.
  - `WantLevelCreatedDialogue` = the type (`SlotID.GetType__`) of
    `Game.GetPreviousLevelSlotID__()` is 2, and `Game.IsPreviousLevelValid__()`
    (short-circuit).
  - `ViewType` = `LocalViewType` (kept by `OnExitPod__`), `ViewTransition` = 1.
  - `DesiredView.SetPlanetsView__i(ViewType)`, then `CurrentView` and `OldView` copy
    it, so the view does not blend.

### Inferred

- The world keeps no previous level yet (there are no natives for
  `GetPreviousLevelSlotID__` or `IsPreviousLevelValid__`), so the native port clears
  `WantLevelCreatedDialogue`.

### Code

- `PlanetsWorldScript::on_enter_pod` and `reset_interactive_music`, dispatched by name.
- Open: the previous level's slot and the level-created dialogue it asks for; nothing
  in the crates calls `OnEnterPod__` yet (the pod computer does on entering the Pod).
