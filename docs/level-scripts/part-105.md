# Level scripts, part 105: the Pod search state as a native port, and Back from Story (2026-10-18, D358)

The Story screen ran on a stand-in (`PodStateSearchStory`) that opened and drew Story but
had no Back: the stand-in's close was inferred, and no flag could press Back for a check.
The search state (`PodStateSearch`) now runs natively in its place, opened with the Story
details as the Story button opens it, and Back from Story returns to the main menu with
the same state values the game's scripts leave.

### What was found

FOUND (the 01.33 scripts, read from the decrypted overlay for research only; nothing of
them is in the repository):

- The Story button pushes `PodStateSearch(SearchDetails.CreateStoryLevels())`: planet 1,
  default browse mode 1, no other browse modes allowed.
- `OnActivate` takes the browse mode and planet from the details, asks for the search
  details, sets cursor mode 1, the default result and the info bubble time. The first
  update sets cursor mode 4, the badge, Earth in control and the allowed browse modes.
- `OnDeactivate(false)` clears the cursor, planet (0), browse modes and loader, hands
  control back from Earth and asks for the search details again; the default result is
  kept. Back also calls `PodComputer.OnPlanetCancel__`, which asks
  `Network.GetPartyOperation` and `PodComputer.CancelDiveIn`.
- `PopState` does not reactivate the state below: it calls `OnLoseFocus` and
  `OnDeactivate(false)` on the popped state and keeps it in `OldStates` (still drawn)
  until `HasTransitionedOut`; the new top gets `OnGainFocus`.

MEASURED (scripted trace, in use 600, Story chosen 640, Back on 700): tick 640 planet 1,
cursor 1; tick 641 cursor 4, browse 1, allowed 0, loader 0, Earth in control; tick 700
planet 0, cursor 0, browse 0, Earth not in control, search details asked twice, default
result kept; the main menu is rebuilt from tick 701.

### What changed

- `lbp-runtime/src/story_native.rs`: `PodStateSearch` with `SearchDetails::StoryLevels`
  replaces `PodStateSearchStory`; `open`, `update` and `close` follow `OnActivate`, the
  first update and `OnDeactivate(false)`.
- `lbp-runtime/src/gooey.rs`: `drawn_menu` breaks a tie between frames whose matrix was set
  on the same frame toward the one built last. After Back the scripted run drew the popped
  Story frame from `OldStates` on the same frame as the main menu, and the tie picked
  Story, which flies away; the main menu now shows. The natives
  `PodComputer.TriggerOnPlanetCancel__` (stored), `Network.GetPartyOperation__` (20, no
  party operation; INFERRED) and `PodComputer.CancelDiveIn__` are answered.
- The viewer: `--level-scripts-back-at N` presses Back on tick N (scripted runs also call
  `OnPlanetCancel__` after that tick).

### Left out

- Pruning `OldStates` frames once they have transitioned out: the old Story frame stays in
  the gooey's frames, but no longer wins the draw.
- The search state's other searches, browsing the Story planets and choosing a level.
- The default result before the search opens (ticks 600-639: the scripts show 23, native
  none).

### Verification

Disc and update only, in use 600, Story 640, Back 700: the native trace's search state
equals the scripted trace's on every tick from 640 to 780, and the pod computer pops to the
main menu on tick 700. Captures at tick 780 of the scripted and the native run both show the
main menu. Tests: `d358_back_matches_the_scripted_trace` (`story_native.rs`),
`d358_a_tie_goes_to_the_frame_built_last` (`gooey.rs`), and the `d357_*` tests.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
