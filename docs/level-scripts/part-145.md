# Level scripts, part 145: the native intro's voice-over (2026-10-19, D398)

Part 144 left the native Pod introduction without its voice-over request, so its wait
passed at once. This part reads the request and ports it; the intro now lasts as long
as its line.

### Found (the 01.33 scripts, `PodIntroductionThread.Run__`, research only)

- Before its zoom loop the thread asks the pod computer to start the video GUID 139230
  (`StartVideo__g`, answer unused), then requests `pod_1st_time`, or `pod_1st_time_alt`
  when `Profile.HasPlayedLBP1AndUnlockedPod__` is true, with interrupt and replay both
  set (`RequestVoiceOver__Q7StringAbb`, answer unused). Nothing else is conditional.
- The disc bank (part 144) has both lines: `pod_1st_time` 1,411,200 samples (960 ticks)
  and `pod_1st_time_alt` 1,215,360 (827 ticks), both at 44100 Hz.
- After the 451-tick zoom it yields while `IsVoiceOverPlaying__` or the pod computer's
  `IsPlayingVideo__` is true, then sets the flags (part 126).

### Inferred

- With the disc's 32-second line the intro ends on the frame after its 960 ticks, about
  17 seconds after the zoom; without the bank it ends after the zoom, as before.

### Code

- The native `PlanetsWorld` makes the request when the intro starts (runtime
  `native_planets_world.rs`); the video request is still not made, since no video plays.

### Next

The video request, once a video can play.
