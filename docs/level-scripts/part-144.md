# Level scripts, part 144: voice-over timing (2026-10-19, D397)

Parts 126, 127 and 131 left the intro, `OnEnterPod__` and the unlock thread waiting on
voice-over that never played, so each wait passed at once. This part reads the game's
voice-over bank and the request's rules. A line now "plays" silently for its real
length, so the scripts wait as long as the game does.

### Found (the disc and EBOOT)

- The bank is `PS3_GAME/USRDIR/gamedata/audio/voiceover.fsb`, a loose file on the disc
  (18,926,496 bytes), not in `DATA.FARC`; its map GUID 0xebc4 does not resolve there.
- It is an FMOD FSB4 bank, little-endian and not encrypted: `FSB4`, 45 samples, sample
  headers of 3,600 bytes (80 each), data of 18,922,848 bytes, version 0x40000, mode 0x20.
  The header is 48 bytes; each sample header holds a u16 size, a 30-byte name ending
  `.wav`, the length in samples (+30) and the rate (+50). Every line is 44100 Hz, mode
  0x80220.
- Lengths include `pod_1st_time` 32.0 s, `pod_select_story` 19.72 s,
  `pod_unlock_level_4` 14.58 s and `pod_complete_theme_1` 1,315,584 samples (29.8 s).
- `RequestVoiceOver`'s core (0x2de6b8) starts a line unless voice-over is off, the
  profile has heard it and the second flag (replay) is clear, or a line is playing and
  the first flag (interrupt) is clear. The native returns whether the line started.

### Inferred

- A line counts as heard once it starts. The heard set lives on the runtime for the
  session and is carried across level switches; the profile's own copy is not read.
- The native intro's own voice-over request is ported in part 145 (D398).

### Code

- `voice_over_lengths` (runtime `media.rs`) reads the sample headers into ticks at 30 Hz;
  `MediaState` plays a line for that long (`voice_over_playing`, ticked each frame), and
  `IsVoiceOverPlaying` and `RequestVoiceOver` answer from it.
- The viewer reads the bank's first 64 KiB from any game dir (`USRDIR` or
  `PS3_GAME/USRDIR`). Without it no line starts, as before. The launcher does not copy
  `.fsb` files yet, so the bank only reaches the viewer from a dir that holds it.

### Next

The launcher to copy `voiceover.fsb` (or its header); the native intro's requests.
