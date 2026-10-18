# Level scripts, part 130: the level unlocking sequence (2026-10-18, D383)

The Pod's state 2 now plays `LevelUnlockingThread` and its `PendingLinkCluster`s (read
in part 127) natively, instead of clearing the pending links at once (parts 128, 129).

### What changed

- lbp-runtime `level_unlocking.rs`: the thread and the clusters on `LevelWorld`. The
  links are grouped as in part 127 (same source badge; same group, or within 30 degrees
  of the cluster's `AverageDir`). A cluster moves the cursor to its source badge, then
  to `AverageDir`, grows each link's progress from tick 10*(j+1) with the beeps at 5,
  20, 35, ..., makes its group the default result for one tick, unlocks each target
  (`unlock_level` or `unlock_theme`, the link cleared, the badge unlocked), holds 30
  ticks and moves the cursor to the last target. Cursor mode 2 while it runs, 4 after.
- The native `PlanetsWorld` runs it one tick per update in state 2 (a new
  `Host::level_unlocking_tick`); `ClearPendingLinks__` stays its `OnFinish__`, which
  unlocks any link the thread did not play (PLACEHOLDER).
- The return to the Pod with pending links asks for the Story search, and the thread
  waits for it: the native pod computer opens it; with the scripts loaded, the return
  presses in use (tick 1) and Story (tick 41) as the start's QA presses do (INFERRED:
  the game is back at its search).
- The viewer prints each step; `--native-scripts` (D417) is now accepted on the
  command line.

Run on the disc with `--native-scripts` (Pod menu, Cross on the selected level at tick
720, finish at 30): back in the Pod, the Story search was up at tick 41 with one link in
one cluster; the cursor went to (0, 127489) and to the links' direction, (0, 125365)
unlocked at tick 84 and the cursor went to it at tick 114.

INFERRED: the link progress step (1/30 a tick) and the group's result index. Not made:
the sounds and `PlayVO__` (no audio), the link drawing, the Tarsier DLC pop and the
group push after the clusters. With the scripts' own `PlanetsWorld` (no
`--native-scripts`), its `LevelUnlockingThread` still returns at once (no search on its
first tick, and the pending-link natives it reads are not bound).

### Next

Draw the links' progress; bind the pending-link natives for the scripted thread.
