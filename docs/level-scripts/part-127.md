# Level scripts, part 127: the level unlocking sequence (2026-10-18, D380)

What the Pod does after a finished story level unlocks new badges, read from the 01.33
scripts (research only) so a native port is ready once the viewer can finish a level.
Nothing in the viewer changed: no pending link can exist yet (below).

### Why it is not ported yet (FOUND, this repo)

- `Game.GetNumPendingLinks__` is answered 0 by `LevelWorld::state_query`; no other
  pending-link native (`GetPendingLink__i`, `ClearPendingLinks__`, `ClearPendingLink`,
  `IsLinkInCurrentSearch`, `CanStartEndOfLevelSequence__`) is registered, and nothing
  adds a link. The native `PlanetsWorld` already enters state 2 when the count is above 0.
- The viewer cannot finish a story level and return to the Pod, which is what creates
  links in the game.
- No audio plays anywhere in the viewer (no audio crate; the Audio natives are recorded,
  D271), so the thread's sounds and voice-over lines would be silent.

### LevelUnlockingThread (FOUND, script ec688550...; branch order partly INFERRED)

- Returns at once unless the pod computer's state is a `PodStateSearch`. Sets the cursor
  mode 2 and waits while its details are busy obtaining results.
- Groups the pending links that are in the current search into `PendingLinkCluster`s: a
  link joins the first cluster that accepts it, else starts a new one. With no cluster
  it sets cursor mode 4 and ends.
- For each cluster, while `PlanetsWorld.CanStartEndOfLevelSequence__`: if the details
  are a `SearchDetailsGroup` whose slot is a group (type 4) other than the cluster's,
  pops the state and, unless the details are now story levels, pushes a
  `SearchDetails(CreateStoryLevels)` state with cursor mode 2, then waits until
  `ViewTransition` >= 1, the results are in and the decorations are loaded. Then it
  runs the cluster's `Update__` one tick per yield until it finishes, calls `PlayVO__`
  on the story-levels state, and moves to the next cluster.
- After the clusters, with `IsTarsierDLCEnabled__` it pops back; then cursor mode 4. If
  the focus planet's current result is a group (result type 1, slot type 4) it pushes
  `SearchDetails(CreateGroup(slot, false))`, waits as above, then moves the cursor to the
  planet's `PickDefaultResult__` and sets cursor mode 4.
- `OnFinish__`: `Game.ClearPendingLinks__`, then
  `SearchDetails.UpdateAllSearchDetailsCachedInfo__`.

### PendingLinkCluster (FOUND, script aff692cc...)

- A link joins when it comes from the same badge as the first link and, for a cluster
  with a group, goes to the same group; for one without, its badge direction is within
  30 degrees (dot3 > cos 0.5235988) of `AverageDir`, the normalised centre of the
  min/max box of the target badges' directions.
- `Update__` states: 0 marks every target badge unlocking and moves the cursor to the
  source badge; 1 waits for the cursor, then moves it to `AverageDir`; 2 updates link j
  from tick 10*(j+1), plays `gameplay/unlocking/unlock_beeps` at ticks 5, 20, 35, ...
  until the last link's progress reaches 1, makes the cluster's group the story results'
  default (one tick), then once the planets are not loading unlocks: per link
  `unlock_level` (same group) or `unlock_theme`, `Game.ClearPendingLink`,
  `UnlockBadge`, the cached-info update; 3 holds 30 ticks, then moves the cursor to the
  last target; 4 is finished.
- `PlayVO__`: the first level-slot target among slot numbers 124819, 124808, 124821,
  124814 starts `pod_unlock_level_2` (`_alt` with LBP1's Moon) to `_5`; 127134 requests
  `pod_complete_theme_1` (`_alt` likewise). INFERRED: those slots are story levels 2-5
  and the first theme's last level.

### Audio (FOUND, this repo)

- `docs/local-data.md` lists 259 loose .fsb banks, 382 .mp3 files and one .fev; the FSB
  version is unknown. Bevy is built without `bevy_audio`. A first audio step would decode
  one bank, play `LevelWorld.media.voice_overs` and answer `IsVoiceOverPlaying__`; the
  interactive music needs the .fev as well.

### Next

A level-finish path (the scoreboard, then back to the Pod with pending links) before
this thread; or audio, which `OnEnterPod__`, the intro and this thread all wait on.
