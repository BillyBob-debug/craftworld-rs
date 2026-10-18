# Level scripts, part 131: the game's own unlock sequence (2026-10-18, D384)

The scripted `PlanetsWorld` (without `--native-scripts`) now plays its own
`LevelUnlockingThread` and `PendingLinkCluster`s on the return to the Pod, instead of
returning at once (part 130). It needed the natives below, the Story search up before the
thread's first look, and the pending links as script objects.

### What changed

- lbp-runtime `level_unlocking.rs`: `unlock_query` binds the calls the thread and the
  clusters make: `Game.GetPendingLink__i` and `ClearPendingLink__Q8SlotLink`;
  `PodComputer.IsLinkInCurrentSearch__Q8SlotLink` (the search is up and the link's
  target is a Story slot), `GetBadgeDir__Q6SlotID`, `SetBadgeUnlocking__Q6SlotID`,
  `UnlockBadge__Q6SlotID`, `MoveCursorToPos__rb` and `MoveCursorToResult__Q6SlotIDb`;
  `PlanetsWorld.CanStartEndOfLevelSequence__` (true, INFERRED) and
  `SearchDetails.UpdateAllSearchDetailsCachedInfo__` (no effect). Each prints as in
  part 130.
- `Game.GetNumPendingLinks__` answers 0 while the Story search is asked for and not yet
  up, so the thread's first look waits for it (INFERRED: the game returns to a search
  that is already up).
- Each pending link is a `SlotLink` object (`From`, `To`: the Story results' SlotIDs, or
  a new SlotID for an end outside them), made with the Story results and again when the
  profile's progress is handed to the Pod after the switch.
- lbp-script-vm: `Math.dot3__rr`, `minPerElem__rr` and `maxPerElem__rr`.
- `GetFocusPlanet__` on the pod computer as on `Planets`; `SetDefaultResult` with a
  handle it does not know keeps the default (INFERRED).

Run on the disc with the scripts (Pod menu, Cross on the selected level at tick 720,
finish at 30): back in the Pod, the scripted thread found one link at tick 41, flew the
cursor to it, cleared it and unlocked (0, 125365) at tick 53, and moved the cursor to it
at tick 83.

INFERRED: the answers above where the game's code was not read. Not made: the sounds,
the link drawing and the Tarsier DLC pop.

### Next

Draw the links' progress.
