# Level scripts, part 115: the cursor's flight (2026-10-18, D368)

Part 108 moved the Earth planet's cursor to the next result at once. The game flies it
there over a few frames, plays a sound, and takes no new move until it lands. The port
now does the same.

### What the game does (FOUND 01.33)

- `MoveCursorToResult` (0x429554; the script native 0x618a3c passes the script's bool as
  `snap`) sets the current result (Planet+0x98) at once. Without snap it starts a
  flight: Planet+0x140 = 1, progress +0x144 = 0, step +0x148 = 0, target +0x150 = the
  result's direction, start +0x160 = the cursor's direction (+0x190). With snap the
  cursor direction is the target at once.
- The pad pick (0x42df50) plays `pod/pod_cursor_move` and makes no move while +0x140 is
  set; `MoveCursorToResult` itself plays nothing.
- The flight (0x4298e8, last in the planet update 0x42f43c), each frame: `d` = the angle
  from start to target; progress += `((d / pi) * 240 + 60) / (Planet+0x1c * 0.5 +
  2500)`. It lands when progress reaches `d`, `d` is under 0.001 or the planets' byte
  +0xb0 is set: step 1, direction the target, +0x140 cleared. Else `s` = 3t^2 - 2t^3
  (t = progress / d, clamped), step = `s * s`, direction = slerp(start, target, s) (a
  normalized lerp above a dot of 0.999).
- Script natives: `IsCursorMoving` (0x610a3c) answers +0x140 != 0 and
  `GetMoveCursorStep` (0x6109a8) answers +0x148.

### What changed

- `lbp-runtime` (`pod.rs`): `PlanetsSearch` holds the cursor direction, the flight
  (`CursorFlight`) and the step; `move_cursor_to` and `cursor_flight_step` follow
  0x429554 and 0x4298e8.
- `move_result_cursor` returns nothing while the cursor flies; a pick counts the sound
  `pod/pod_cursor_move` and starts the flight. `cursor_flight_step` runs a frame.
- The natives `Planet.IsCursorMoving__` and `GetMoveCursorStep__` answer the flight;
  the script's `MoveCursorToResult` passes its snap.
- The viewer steps the flight after each tick and logs the landing.

INFERRED: Planet+0x1c is unidentified and taken as 0, the fastest flight the formula
gives. A cursor that has never moved starts its flight from the result it was on.

MEASURED (run368, scripted and native alike): moves at ticks 700, 720 and 740 land at
ticks 707, 726 and 743 (results 42, 81, 43); the Cross at 760 launches Trash Valley as
before. A second move 3 ticks into a flight is refused (run a2): the move at
tick 703 answers none, and the cursor still lands on result 42 at 707.

Not done: the cursor's height and blend (+0x1a0, +0x1b0), the planets' land-at-once
flag (+0xb0), `HasCursorArrived` (0x615998) and drawing the cursor. The camera or
planet turn that follows the cursor direction is UNVERIFIED.

### Next

The other searches.
