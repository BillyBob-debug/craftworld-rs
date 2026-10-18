# Level scripts, part 128: finishing a story level (2026-10-18, D381)

The first half of a level's end: the game's level-complete path now runs on the
session's profile, so a finished story level leaves a pending link to the next one,
which the Pod's state 2 then clears. The return to the Pod is the next step.

### The profile's pending links (FOUND, 01.33)

- The vector at profile+0x2b0 (count +0x2b4) holds 16-byte {From, To} SlotIDs.
  Natives: `GetNumPendingLinks` 0x5fab40 -> 0x5e8a30, `ClearPendingLinks` 0x5f4e80 ->
  0x19ac2c, `GetPendingLink__i` 0x5f5f3c, `AddPendingLink` 0x6008ac -> 0x1a8764,
  `ClearPendingLink` 0x60d1b8 -> 0x19ab60.
- The level-complete path 0x1ad654 returns at once for slot types 5, 6 and 7, then calls
  0x1aa43c, which adds {slot, PrimaryLinkLevel} when the link is set and the slot number
  is below 0x506ca. The add 0x1a8764 checks the target, tests its lock (0x1a7efc,
  group-aware) and skips a link the list already holds.

### The disc's links (MEASURED, lbp2story_slots.slt, 82 slots)

- Level N's `PrimaryLinkLevel` is level N+1: 124806 -> 124819 -> 124808 -> 124821 ->
  124814 -> 127111 (the first theme's outro) -> 127134 (the second theme's intro, group
  (4,23)) -> 124823. All of these are `InitiallyLocked`. No slot is numbered 570.

### What changed

- `lbp-runtime`: `SearchSlot.link` and `initially_locked`; `StoryProgress` (finished
  levels, unlocked slots, pending links) on `LevelWorld`, handed on with the progression
  flags; `finish_story_level` (the path above) and `slot_unlocked`;
  `GetNumPendingLinks__` reads the list; the new native `Game.ClearPendingLinks__`
  empties it, and the native `PlanetsWorld` calls it in state 2.
- PLACEHOLDER: `LevelUnlockingThread` (D380) is not ported; its badge unlocks are folded
  into the clear, so each link's target is marked unlocked there.
- `lbp-bevy-viewer`: the launched level's SlotID is kept across the switch, with the
  profile's flags and progress. The level ends on its `Game.GotoScoreboard__b` or the QA
  option `--level-scripts-finish-at N` (the launched level's tick N); the scoreboard is
  not shown.
- Test `d381_a_finished_level_links_to_the_next_and_the_pod_unlocks_it`.

Run on the disc (Pod menu, Cross on the selected level at tick 720, finish at 30): the
launched cutscene (0, 127489) left the link to (0, 125365), one pending.

Not ported: the add's case for a target numbered 570, the branch for slot numbers from
0x506cc (not 0x54747), and 0x1a88f4's tables of a level's own unlocks (loader not found).

### Next

Back to the Pod after the finish, with the links pending, so its state 2 runs there.
