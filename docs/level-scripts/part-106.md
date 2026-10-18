# Level scripts, part 106: choosing a Story group, and Back to Story (2026-10-18, D359)

Story's planets held its groups, but Cross on one did nothing without the scripts. Choosing
a group now pushes a search state for that group over Story, as the game's pod computer
does, and Back pops it and brings Story back on top with the values the scripts leave.

### What was found

FOUND (the 01.33 scripts and EBOOT, read from local research copies; nothing of them is in
the repository):

- Cross runs the native `TriggerOnPlanetSearchResultSelected` (0x615664), then the
  script's `OnPlanetSearchResultSelected` (through 0x4037bc). When that returns false the
  engine (jump table 0x4038ac, then 0x3fa448 and 0x6589f8) sends a group result to
  `PodComputer.OnPlanetGroupSelected(SlotID, true, true)`.
- That pushes `PodStateSearch(SearchDetails.CreateGroup(slot, true))`, carrying the cursor
  mode over and asking for the default result.
- The Story state under it gets `OnDeactivate(true)`: the selected result is saved as its
  default result, the browse mode (`Planets.GetBrowseMode__`) is saved in its member, then
  the same clearing as Back (part 105).
- `GetPlanetsViewType` on planet 1 in Story is 7 when the results want a zoom and 6
  otherwise. Offline `GetWantZoom` is false, so the group keeps view 6.
- Left and Right between results are engine-only and are not ported yet.

MEASURED (scripted trace: in use 600, Story 640, group 680, Back 720):

- Tick 680: planet 1, cursor 1, browse 0, allowed 0, loader 0, Earth not in control, search
  details asked 3 times, default result 23 kept, info bubble from Story's push (219906).
- Tick 681: cursor 4, badge 2, browse 1, Earth in control. The group frame (2400x1250, a
  64-wide border, a 700-wide curved panel with an empty title) slides in and settles on
  tick 695.
- Tick 720: the group closes and Story's `OnActivate(false)` runs. Planet 1, cursor 1,
  search details asked 5 times, default result and info bubble unchanged.
- Tick 721: cursor 4, badge 3, browse 1, Earth in control. Story's frame keeps its first
  build (219906) and slides in again.

### What changed

- `lbp-runtime/src/story_native.rs`: `SearchDetails::Group(slot)` with the group frame and
  matrix (frame key (3, 1)). `PodStateSearch::suspend` (`OnDeactivate(true)`) and
  `resume` (`OnActivate(false)`).
- `lbp-runtime/src/pod_computer_native.rs`: `PuterState::GroupSearch` and `choose_group`.
  A state pushed over another suspends it and keeps its first build, and popping the group
  resumes Story.
- `lbp-runtime/src/gooey.rs`: the natives the group push calls in the scripted run
  (`SearchDetailsGroup.GetSlotID__`, `IsOnlyOwnedDLC__`,
  `Planets.GetCurrentSelectedResult__`, `Planets.GetBrowseMode__`).
- The viewer: `--level-scripts-group-at N` chooses the selected group after tick N, with
  the scripts or the native pod computer.

INFERRED: `GetBrowseMode__` answers the mode last set, and the group view stays 6 until
the results' zoom is ported.

### Checks

- `story_native::tests::d359_*` and `pod_computer_native::tests::d359_group_and_back_to_story`
  hold the measured values at ticks 680, 681, 720 and 721.
- Native run (disc and update only) against the scripted trace: the search state is the
  same on every tick 640-760. The native menus are drawn at rest while the scripted ones
  slide in (the group 680-695, Story 640-659 and again from 721).

### Next

Moving between results (Left/Right), choosing a level from a group, and the group's zoom
view (7).
