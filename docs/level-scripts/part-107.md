# Level scripts, part 107: a Story group zooms in on its planet (2026-10-18, D360)

A chosen Story group kept Story's view (6) without the scripts, so the planets stayed where
Story left them. The group now selects its default level, asks for the zoom view (7), and
gives the planets world the extra zoom its script state gives, so the planets zoom in on the
group exactly as in the scripted run.

### What was found

FOUND (the 01.33 scripts and EBOOT, read from local research copies; nothing of them is in
the repository):

- `SearchResults.GetWantZoom` (0x6057bc) reads a flag the group results constructor
  (0x44dfc0) sets for slot types 4, 8 and 9. `GetPlanetsViewType` gives 7 when it is true.
- A group search sets no explicit default result. `SetWantSearchDetails`' fourth argument
  (stored at Planets+0x5d8) makes the planet update (0x42f43c) pick one (0x4290b8) and snap
  the cursor to it. The comparator (0x4285d8) orders selectable, then unlocked, then lowest
  state, then the last result, then nearest the cursor, then list order; type 4 items are
  not selectable in a group search.
- `PodStateSearch.GetExtraPlanetZoom` asks its details; `SearchDetailsGroup` answers 500
  for a group of slot type 4 or 9, every other state 0. `PlanetsWorld.UpdatePlanetsView`
  smooths towards it while the pod computer is in use.
- Cross on a story level is a UI event (kind 4, 0x4033b4) unless it is the pod's current
  slot; who pushes `PodStateTweakLevel` for it is not found yet.

INFERRED: the default level is the group's first non-group level in list order (no lock or
play state is read offline).

MEASURED (scripted against native run, in use 600, Story 640, group 680, Back 720): the
planets level, camera, story point and search state are identical on every tick 640-760;
the group capture at tick 715 is identical. The remaining differences are the known menu
slide-ins and the default result before the search opens (ticks 600-639, both pre-D360).

### What changed

- `lbp-runtime/src/pod.rs`: `PlanetsSearch::group`, the open group's slot ID.
- `lbp-runtime/src/lib.rs`: `selected_result` (the group's cursor, else the default) for
  the story point and info slot, `group_default_result`, and the host's
  `extra_planet_zoom`. Choosing a group sets it; Back from a group clears it.
- `lbp-runtime/src/gooey.rs`: `GetWantZoom__` is true in a group search;
  `SetWantSearchDetails` with its fourth argument selects the group's default level.
- `lbp-runtime/src/story_native.rs`, `pod_computer_native.rs`: the native group search sets
  and clears the group and uses view 7.
- `lbp-runtime/src/native_planets_world.rs`, `lbp-script-vm/src/natives.rs`: the native
  planets world asks the host for the extra zoom (`PodComputer.HostExtraPlanetZoom__`, a
  host-only native).
- Test `d360_group_zoom_and_default_level`.

### Next

Left/Right between results, Cross on a story level, and the group's title.
