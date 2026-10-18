# Level scripts, part 114: the launch warnings (2026-10-18, D367)

When the Pod launches a story level, the game's launch (FOUND 01.33, 0x402b24, called
by 0x4033b4) first checks the party against the slot and may raise warning popups (type
0x4b, buttons Continue 0x119fbf3b and Cancel 0x5162e2b2). The port now raises the same
warnings for a Play launch, and the hand-off to the level waits until they are answered.

### What the game does (mode Play)

- The world's warning flags (world+0x3b0) are cleared.
- If the slot's `MoveRecommended` (slot+0xc7) is set, the first player without a Motion
  Controller raises one warning: the primary user gets text 0x5d46982a (the level
  needs a Motion Controller) and flag 1; another player gets text 0xae61784b (not every
  player has one), no flag.
- The player count (world+0x1e8) below `MinPlayers` (slot+0xc5) raises a warning with
  flag 0x10 (text built by 0x16e3b0); else above `MaxPlayers` (slot+0xc6), flag 0x20
  (text by 0x16de78).
- The pending launch is then stored as before (pod+0x954 on).

Mode Create/Tweak checks Wireless Controllers instead (keys 0x5e507c0e and 0x66658d07,
flag 8); not ported, the Pod's story launch is mode Play.

### What changed

- `lbp-runtime` (`pod.rs`): `SearchSlot` carries `min_players`, `max_players` and
  `move_recommended` from the slot; `PartyPad`, `LaunchWarning` (text key, flag) and
  `launch_warnings` follow 0x402b24. `PodState` holds the party (default: one primary
  player on a Wireless Controller), the warnings still up and the flags.
- `cross_on_level` raises the warnings; `pod_launch_step` waits while one is up;
  `answer_launch_warning` takes one down (Continue) or drops the launch (Cancel,
  INFERRED: the popup's result handler was not traced).
- The viewer logs the warnings after a Cross.

FOUND: there is no select sound. The dispatcher's Cross case (0x403b3c) only looks up the
resource `pod/pod_select_badge` (0x2d37c4) and drops the handle at once; the same lookup
sits in cases 1, 5 and 6. Nothing to port.

MEASURED (the part 109 runs, scripted and native): Trash Valley (slot players 1-4, no
Move) raises no warning for the one local player (`Launch warnings: [] (flags 0x0; ...)`),
and the hand-off and switch go ahead at tick 761 as before.

Not done: the player-count texts (0x16e3b0, 0x16de78) and drawing the popup.

### Next

The cursor's flight and sound, then the other searches.
