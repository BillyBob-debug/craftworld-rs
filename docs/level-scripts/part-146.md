# Level scripts, part 146: the unlock voice-overs (2026-10-19, D399)

Part 130 ported `LevelUnlockingThread` without its voice-overs. This part reads
`PendingLinkCluster.PlayVO__` and ports it; a finished cluster now asks for its line.

### Found (the 01.33 scripts, research only)

- `LevelUnlockingThread` calls the cluster's `PlayVO__` once the cluster's update ends,
  then moves to the next cluster (script ec688550). It does not wait on the line.
- `PlayVO__` (script aff692cc) walks the cluster's links in order and skips any whose
  target's `Type` is not 0 (a level). The first link whose `SlotNumber` has a line
  decides: 124819, 124808, 124821 and 124814 ask the pod computer's story state to start
  `pod_unlock_level_2` to `_5`; 127134 asks `Audio.RequestVoiceOver` for
  `pod_complete_theme_1`. Level 2 and the theme line take their `_alt` name when
  `Profile.HasPlayedLBP1AndUnlockedMoon__` is true. Other level slots are passed over.
- The story state's `StartVO__` (script 110fc2e8) requests the line with interrupt set
  and replay clear, as the theme line is requested, and keeps the answer; closing the
  state stops a line it started.

### Code

- `UNLOCK_VOICE_OVERS` and `play_unlock_voice_over` (runtime `level_unlocking.rs`): the
  request goes through the media state (part 144), so with the bank the line plays for
  its length and is heard once.

### Not modelled

- (Corrected in part 159: the kept answer and the stop on closing are ported.)
