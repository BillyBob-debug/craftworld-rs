# Level scripts, part 147: the native intro's video request (2026-10-19, D1000)

Part 145 ported the native Pod introduction's voice-over but not the video request
before it. This part reads the pod computer's side of that request and ports it.

### Found (the 01.33 scripts, research only)

- `PodIntroductionThread.Run__` asks the pod computer to start the video GUID 139230
  (`StartVideo__g`) before it asks for its line (part 145).
- The pod computer's `StartVideo__g` (script 16e6e8c9) is
  `Video.StartVideo__Q5Thinggb(null, guid, false)`, its answer kept as `VideoHandle`.
  `IsPlayingVideo__` is `VideoHandle != -1`, and `PlayVideoAndWait__g` does the same
  request under the "Voice Over" mix overlay (0.2), then yields while the handle is set.

### Code

- The native `PlanetsWorld` makes the video request when the intro starts, before the
  voice-over (runtime `native_planets_world.rs`, `INTRO_VIDEO`). No video starts, so
  `StartVideo` answers -1 and the intro's wait sees no video, as before.

### Next

A video player, once the game's video files can be read.
