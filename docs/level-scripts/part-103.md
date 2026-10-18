# Level scripts, part 103: the Pod planets' world script as a native port (2026-10-18, D356)

The planets level's world Thing runs `PlanetsWorld` (g00016e0a), which owns the Pod's
camera, the level's place under its view and the planets' spin offsets, with its view
objects (`PlanetsView`). Without the decrypted patch.edat nothing ran it: the Pod drew with
the D411 resting camera and its planets were not placed, and the Story screen (parts
100-102) put the world Thing and the camera where the scripted run leaves them. It now runs
as a native port behind `NativeScript`.

### What was found

FOUND (the 01.33 scripts, read from the decrypted overlay for research only; nothing of
them is in the repository):

- `UpdatePlanetsView` (each frame, after the planets): snap on the first frame, else blend
  the current view from the old to the asked-for one (`GetTransitionSpeed` a frame:
  0.1 between views of the same focus planet, 0.02 across planets, 0.05 otherwise, faster
  for a few named pairs; `Math.SmoothStep`), ease the pod computer state's extra zoom,
  run the state, move the camera, give Earth and the Moon the view's spin offsets, put the
  level under `GetTransform(eye, zoom)`, and set the depth of field at twice the view's.
- `UpdatePodCamera`: toward (0, -300, 900) looking at (0, 500, -1500) while the pod
  computer is in use, else (0, -250, 1130) at (0, 100, -1500) framed on the players;
  `Math.Smooth3D` (10, 100) a frame, snapped on the first; a loading badge zoom while the
  pod computer crash-zooms; `Cam.SetCameraFromToUp` (10, 100000, 0, 1).
- `PlanetsView.GetTransform`: `translate(eye) scale(2) translate(-eye) translate(0,
  -zoom/4, zoom) rotateZ(roll) rotateX(pitch) translate(offset) rotateY(yaw)
  translate(-focus)`. `SetPlanetsView__i`: 46 view types (the Story view 6: offset (0, 0,
  -3750), yaw 0.7, pitch 0.35, focus Earth, DOF 3600-4400, Earth spin offset -0.7).

### What changed

- `lbp-runtime/src/native_planets_world.rs`: `PlanetsWorldScript` (`OnCreate__`,
  `UpdatePlanetsView`, `SetViewFromRemotePodComputer__i`, `GetCurrentViewType__`,
  `OnExitPod__`) and `PlanetsView` (the view table, blend, speeds). The views are kept as
  members of the world Thing (`CurrentView.Yaw`, ...). Without the crafteroid objects the
  views are made on the first frame that hands the world its planets.
- The Story stand-in asks the planets world for view 6 instead of placing the world Thing
  and the camera; `POD_IN_USE_CAMERA`, `STORY_LEVEL_THINGS`, `STORY_EARTH_SPIN_OFFSET`,
  `place_thing` and `set_script_member` are gone. The plain Pod without scripts now shows
  Earth and the Moon placed by view 0 (the launcher's blurb says so).
- The viewer prints the scripted Things with their script GUIDs and native ports.

### Left out

- The cutscene threads (the pod introduction and level unlocking): the state machine runs
  and the threads are null. The "level created" voice-over (`OnEnterPod__`), the
  interactive music and the pod computer state's extra zoom (a script method) are not
  ported.

### Verification

No scripts, `--story`, held at tick 90: the camera eases in from (0, -250, 1130) and stops
at (0, -299.48, 902.39) on tick 20, the world Thing settles on tick 21; camera, world
Thing, Earth and Moon equal the scripted run's (largest difference 0.0), frame mean abs
0.36 levels against the scripted one. The plain Pod without scripts at tick 120 shows Earth
and the Moon through the window. `level_scripts` on pod_planets.bin from the disc and update
alone: no faults, the camera written every tick. Tests (`tests/native_planets_world.rs`):
`d356_story_view_settles_as_the_script_does`, `d356_planets_views`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
