# Level scripts, part 125: the launch warning popup (2026-10-18, D378)

A Cross on a story level that raises a launch warning now shows the warning in a box in
the middle of the screen with its two buttons. Enter or Cross carries on, C or Circle
cancels the launch. Before, the warning was only printed and nothing could answer it, so
the launch waited for good.

### What the game does (FOUND, 01.33 EBOOT)

- 0x3e995c packs its arguments and calls 0x3e8dc0, which queues an entry (0x98 bytes,
  at most 8) in the global popup manager. The launch check 0x402b24 queues type 0x4b
  with the warning text and the button keys 0x119fbf3b then 0x5162e2b2, then sets
  pod+0x9c4 to 1 (0x3f9808) without waiting.
- The type's config (0x3e3248, table record +0x1770): flags +0x48 = 10, bit 1 modal.
  While a modal entry is up the manager's update (0x3e99f4) masks the pad (0x893e3c).
  Trigger 5 runs handler 0x10, trigger 6 handler 0x3c (jump table 0x3e5308).
- Handler 0x10 (0x3e68e0) acts only for the speech-bubble types; for 0x4b it goes to the
  common tail 0x3e5878, which closes the entry. Handler 0x3c (0x3e7870) calls
  0x402478(pod, 0) -> 0x3f9794: pod+0x9c4 = 0, the pending slot cleared and every 0x4b
  entry closed (0x3e2f64).
- The pod's step (0x402560) holds while a 0x4b entry exists (0x3e2db4); once it is gone
  with the wait still set it commits the launch (0x4022fc). So the first button
  launches, the second cancels.
- The box itself is drawn by the script `gamedata/scripts/gamemessagingmanager.ff`
  (GUID 42623, classes GameMessagingManager and GameMessagingBox) through the fader
  "FADER_MESSAGE_BOX" (0x3ea128). Only the patch's encrypted archive holds that script.
- INFERRED: Cross is the first button and Circle the second; the script raises the
  triggers (`Messaging_NewGameEvent`) and was not read.

### What changed

- `lbp-runtime`: `LevelWorld::launch_warning_popup` gives the warning that is up, its
  text and the two buttons' labels from the disc's LAMS text (built-in labels where a
  key has none). The comment on `answer_launch_warning` now has the handlers.
- `lbp-bevy-viewer`: `launch_popup` draws the viewer's own box (INFERRED look: the
  game's script is encrypted-only, so this is the built-in fallback); `warning_keys`
  answers it, and the menu and the speech bubble keys wait while it is up.

Parity 5 pass; the Story run is unchanged.

### Next

The state 6/7 load.
