# Level scripts, part 109: choosing a story level (2026-10-18, D362)

Cross on a story level did nothing in either mode: the script declines it, because asking
for the level's launch is the engine's job. Cross now asks for that launch in both modes,
Back waits while one is pending, and a group search's title follows the script's rule.

### What was found

FOUND (the 01.33 EBOOT, read from local research copies; nothing of it is in the
repository; notes in `local-analysis/d362/story-level-notes.md`):

- In story play `OnPlanetSearchResultSelected` declines, so the engine's dispatcher
  (0x4037bc) acts on the result kind from 0x452324. A level slot (type 0 or 1) equal to
  the pod's pending or current slot (0x3fce40) is kind 0 and does nothing; any other is
  kind 4, `TriggerOnLevelLaunch(slot, Play)` (0x4033b4, mode 0).
- That call cancels an older pending launch (0x402478), stores the slot at pod+0x954 and
  sets pod+0x9c4 to 1. No state is pushed. `IsLaunchingLevel` (0x615858 through 0x3fce40)
  reads the pending slot. The button prompt is key 0x964b09a1, "PLAY".
- `PodComputer.OnPlanetCancel` returns without popping while a launch is pending (pc
  0019-0021). The search state's own `IsLaunchingLevel` gate (`PerFrameUpdate` ins
  0025-0032) covers only its page prompts and debug options.
- `SearchDetailsGroup.DoTitle`: for a group (type 4 or 9) with completable levels
  (`GetNumCompletableLevelsInGroup`, pc 0076) the title is the group's translated name in
  text style 1, a break and a bubble counting completed levels ("of", "Levels"). None of
  the natives is bound, so the scripted count is 0 and the title is empty.

MEASURED (scripted against native, Story at 640, the default group at 680, moves at
700-740, Cross at 760, Back at 775): both runs ask for the launch of result 43, slot
(0, 127795), and both keep the group search on top after Back; the search state, planets
level and story point are identical on every tick 680-790 and so is the frame at 790.
Run without Cross, both pop on Back.

### What changed

- `lbp-runtime`: `PodState::launch` (pod+0x954), `current_level`, `group_levels`;
  `LevelWorld::cross_on_level` returns `LevelCross` (not a level, the current one, or a
  launch); `IsLaunchingLevel` reads the pending launch; the native pod computer ignores
  Back while it is set, and the scripted host keeps the group's cursor then;
  `group_title` draws the group's name when its count is known.
- `lbp-bevy-viewer`: the QA option `--level-scripts-cross-at N` presses Cross on the
  cursor's result after tick N, in both modes.
- Tests `d362_cross_on_a_story_level_asks_for_its_launch` and
  `d362_back_waits_for_a_pending_launch`.

Left out: the level's load and start (whatever reads pod+0x9c4), the warnings at
0x402b24, the select sound (string 0xda18fc unresolved) and the title's count bubble.

### Next

The level's load and start, then the cursor's flight and sound and the other searches.
