# Level scripts, part 159: the unlock line's stop (2026-10-19, D1012)

Part 146 left one thing of the Story state's unlock line unported: the state keeps
whether its line started and stops it when it closes. The native Story state now
closes mid-line, so the stop is ported.

### Found

- `StartVO__` and `StartVOMovePack__` (script 110fc2e8, `ModalOverlayState`) keep the
  bool `Audio.RequestVoiceOver` returns in the private `_StartedVO`. Nothing sets it
  back to false.
- `OnDeactivate__` is the only reader besides an unused getter (`StartedVO__`): with
  the flag set it calls `Audio.StopVoiceOver__`, then marks the state inactive and
  invalidates its frame.
- `OnActivate__` starts a line only when the virtual `WantVO__` names one; of the Pod's
  search states only the Story state plays the unlock lines (part 146). The theme line
  goes to `Audio` directly and does not touch the flag.

### Code

- The pod search keeps `story_vo_started`, set from the media state's answer when a
  level line is requested (`play_unlock_voice_over`), never cleared.
- The native Story state's close and suspend (`deactivate`) stop the voice-over when the
  flag is set; a group's state does not.
