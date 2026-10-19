# Level scripts, part 150: the intro's video length (2026-10-19, D1003)

Part 147 had the native intro request the pod computer's video 139230 and keep the
game's -1 handle, so the intro waited only on its voice-over. This part times the video.

### Found

- GUID 139230 is `gamedata/videos/lbp2/lbp2_podintro.bik`, a loose, unencrypted file on
  the USA disc (`/PS3_GAME/USRDIR/GAMEDATA/VIDEOS/LBP2/LBP2_PODINTRO.BIK`, 11,140,084
  bytes); the update does not replace it.
- Its header is `BIKi`, then little-endian u32s: the file size less 8, 378 frames, the
  largest frame, 378 again, 640, 360, and the frame rate 2997/100 at 0x1c and 0x20. At
  30 Hz that is ceil(378 * 30 * 100 / 2997) = 379 ticks, about 12.6 seconds.
- `StartVideo` (core 0x67c620, via 0x67c45c) returns the video's handle (obj+0x318), or
  -1, and keeps the GUID at `PTR_DAT_00d9ef34+0x414`.
- `UpdateVideo` (0x67c22c) looks the handle up (0x679438): none, false; true while
  0x67bc44 is non-zero; then it finishes the video (0x67a25c) and answers false.
- The pod computer's script keeps the handle in `VideoHandle`: `IsPlayingVideo__` is
  `VideoHandle != -1`, and `UpdateVideo__` sets it to -1 when the native answers false.

### Inferred

- One video plays at a time (the pod computer keeps one handle), so a new one replaces
  the last.
- `Video.IsVideoPlaying__` is left false: nothing on the intro's path reads it.

### Code

- `lbp_runtime::bink_ticks` reads the header; `VIDEO_FILES` lists the video. The viewer
  reads it beside `voiceover.fsb` (USRDIR or PS3_GAME/USRDIR) into
  `MediaState::video_lengths`. Without the file no video starts: the handle is -1 and
  the intro waits only on its line, as before.
- `StartVideo__Q5Thinggb` and `UpdateVideo__ib` ask the world for the handle and the
  answer; the native intro keeps the handle (`IntroVideo`), calls `UpdateVideo__ib` each
  tick after the zoom and waits while the handle is not -1 or the line plays.
- The launcher keeps the 11 MB video in its disc copy (`iso::KEPT_VIDEO`); an older
  copy without it is copied again.
