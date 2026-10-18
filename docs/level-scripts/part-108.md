# Level scripts, part 108: moving between results on the planet (2026-10-18, D361)

The planet's cursor stayed on the default result: nothing moved it between results, in
either mode, because moving it is the engine's job, not a script's. The stick now moves it
to the result in the pushed direction the way the engine picks one, and the Story view
turns to that result.

### What was found

FOUND (the 01.33 EBOOT, read from local research copies; nothing of it is in the
repository; notes in `local-analysis/d361/cursor-notes.md`):

- The planet update (0x42f43c, Planet vtable 0xd745a8 +0x24) calls 0x42df50, which adds
  the stick (0x38b3ec, stick 0x12) and the held d-pad into an input vector at Planet+0x130,
  zeroed in the dead zone.
- Once the cursor has landed (Planet+0x140 0), 0x42d584 picks a result and
  `MoveCursorToResult` (0x429554) flies there. The current result is Planet+0x98.
- Each candidate scores `df * (cos + 1)`: `cos` is the angle between the push and the
  projected direction to the candidate (0x4245cc), which must be above 0 (above 0.866 with
  no current result), and `df = 1 - dist / pi`. The highest wins, a tie the later entry,
  and there is no wrap. In a Story group only the group's entries count, plus entries
  linked to the current one. A locked entry (entry+0x228 0, from result vt+0x5c 0x45024c)
  is skipped.
- There is no repeat delay: the next move waits for the flight to land, whose progress per
  frame is `((d / pi) * 240 + 60) / (Planet+0x1c * 0.5 + 2500)`, eased. Each pick plays
  `pod/pod_cursor_move`; scripts are not called, they poll `GetCurResult` and
  `IsCursorMoving`.

INFERRED: the projection is onto Earth's tangent plane at the current result (east from
the world up), the distance is the angle between the slots' `location`s, and with no lock
state read offline every level counts (groups are left out of a group search, as for its
default pick; the Story search takes the slots in no group).

MEASURED (scripted against native, Story at 640, the default group at 680, right at 700
and 720, left at 740): both runs pick results 42, 81 and 43; the planets level, story
point and search state are identical on every tick 680-760, and the frame at 760 is
identical.

### What changed

- `lbp-runtime/src/lib.rs`: `LevelWorld::move_result_cursor`, the engine's pick, and
  `selected_result` is the cursor result in any search.
- `lbp-bevy-viewer`: the QA option `--level-scripts-move-at N:DIR[,...]` (left, right, up,
  down) pushes the stick after tick N; a group chosen with `--level-scripts-group-at` is
  the cursor's result.
- Test `d361_cursor_moves_to_the_nearest_result_that_way`.

Left out: the cursor's flight and its sound, linked results, and the lock state.

### Next

Cross on a story level and the group's title.
