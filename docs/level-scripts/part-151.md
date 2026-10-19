# Level scripts, part 151: the Pod's music state (2026-10-19, D1004)

`PlanetsWorld.UpdatePlanetsView__` calls `UpdateInteractiveMusic__` every frame; the
native port skipped it, and the world answered `GetCurrMusicGuid` with 0 forever, so
the scripted run asked for the music again each frame. This part ports the function and
keeps the music the script sets. Nothing plays yet.

### Found

- The assets are plain on the USA disc: `GAMEDATA/AUDIO/PS3_MAIN.FEV` (1,031,978 bytes,
  `FEV1`, version 0x35) and `I_POD.FSB` (14,787,392 bytes, FSB4 version 2), the Pod's
  interactive music.
- `.init__` sets `FSB` = 42103 and `SettingsFile` = 42108.
- `UpdateInteractiveMusic__` (script g0000248b), in order:
  - In the attract loop (`Input.IsInAttractionLoop__i` of the local leader) it stops the
    music once (`StopInteractiveMusic__`, clearing `IMusicPlaying` and `IMusicInit`) and
    returns.
  - While `Audio.GetCurrMusicGuid__()` is not `FSB` it calls
    `SetInteractiveMusic__ggffiQ5Thing(FSB, SettingsFile, 1.0, delay, 0, null)`, the
    delay being `NonDeterministicRandomBetween(1.49, 8.49)` cut to whole seconds, and
    sets `IMusicPlaying` and `IMusicInit`.
  - A changed `ViewType`, or a changed "heard" (user music stopped or paused, and
    `IsInteractiveMusicAudible__`), sets `IMusicInit`.
  - With `IMusicInit` it sets the six stereo pairs' volumes
    (`SetInteractiveMusicStereoPairVolume__giff(FSB, pair, v, -1.0)`) and clears it.
    Pair 3 is always 1. Views 1, 2, 33-35 and 45 add pair 0; views 6 and 7 add pairs 0
    and 4; views 3-5, 8, 9, 12-20, 24-27, 29, 30, 36, 37 and 39-44 add pairs 0 and 2;
    views 10, 11, 21-23, 28, 31 and 32 add pairs 0 and 1. View 0, unlisted views and
    "not heard" set pair 3 only.

### Inferred

- `GetCurrMusicGuid` answers the bank the last `SetInteractiveMusic` set, and 0 with
  none or after `StopInteractiveMusic`; this is what makes the script set it once.
- A pair volume for another bank is ignored.

### Code

- `MediaState::music` (`InteractiveMusic`: the bank, settings, volume, delay and pair
  volumes) and `music_starts`, kept by `LevelWorld::music_call` from `audio_command`;
  the world answers `Audio.GetCurrMusicGuid__` from it.
- `PlanetsWorldScript::update_interactive_music` and `music_pairs` port the function;
  `create()` sets `FSB` and `SettingsFile`.
- Open: reading `PS3_MAIN.FEV` and `I_POD.FSB` and playing the music;
  `ResetPodInteractiveMusic__` from the unported `OnEnterPod__`.
