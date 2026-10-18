# Level scripts, part 102: the Pod planets' scripts as native ports (2026-10-18, D355)

The planets level's Earth and Moon run the scripts `Earth` and `OrbitingMoon` (g000053d0,
g000053ce), both `PlanetThing` (g00016e09) with only `GetPosition__` of their own. Without
the decrypted patch.edat nothing placed or turned them, so the Story screen (parts 100 and
101) put them where the scripted run leaves them. They now run as native ports behind the
shared `NativeScript` trait (D417).

### What was found

- FOUND (the 01.33 scripts, read from the decrypted overlay for research only; nothing of
  them is in the repository): `OnCreate__` zeroes `Pitch` and `Spin`, keeps the world
  matrix as `OriginalPos`, clears `Updated`, `MaxVel` 10 and `MaxAcc` 0.01.
  `Update__rb(point, arrived)` builds `scale(axis lengths of OriginalPos x Scale)` at the
  class's place, takes the point into it, aims for spin `-atan2(x, z)` (within half a turn
  of `Spin`) and pitch `atan2(y, |xz|)` (clamped to a quarter turn), eases there with
  `Math.Smooth2D` after its first update while not arrived and snaps otherwise, then sets
  the world matrix `rotateY(SpinOffset) * rotateX(Pitch) * rotateY(Spin) * scale` and
  answers whether both angles are within 0.01.
- FOUND (`PlanetsView.SetPlanetsView__i`): the views that focus Earth set `Yaw` 0.7 and
  `EarthSpinOffset` to minus `Yaw`; `PlanetsWorld.UpdatePlanetsView` gives it to Earth's
  `SpinOffset`. MEASURED: the scripted Story Earth is turned -0.70 against the port's
  without it.
- MEASURED places (the subclasses' `GetPosition__`): Earth (-1250, 0, 0), the Moon
  (2000, 0, 0).

### What changed

- `native_script.rs`: `PlanetScript` (`EARTH`, `ORBITING_MOON`) in `NATIVE_SCRIPTS`;
  `instance_class` / `instance_frame_entry` give a Thing that runs a port its class's frame
  entry (the planets' `Update__rb` with the point and "arrived"), and Earth its Story point.
- The Story stand-in now places only the world Thing (the planets world script is not
  ported) and gives Earth the Story view's `SpinOffset` (`STORY_EARTH_SPIN_OFFSET`).

### Left out

- `PlanetsWorld` and `PlanetsView` (the camera dolly, the world Thing's move, the views'
  offsets and zoom): the world Thing and the in-use camera stay part 100's measured end
  state.

### Verification

No scripts, `--story`, held at tick 60: Earth at rest on tick 14 and the Moon on tick 1,
both equal to the scripted run's settled matrices (largest difference 0.0); the frame
against the scripted one: mean abs 0.36 levels, as before. Tests (`tests/native_planet.rs`):
`d355_earth_turns_to_the_story_group_as_the_script_does` (snap, the first eased tick and
the rest matrix against the traces), `d355_moon_is_placed_on_its_first_update`.

Counts: +0 newly viewable / +0 newly Complete (viewer feature).
