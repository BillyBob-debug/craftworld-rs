# Level scripts, part 110: starting a chosen story level (2026-10-18, D363)

After Cross on a story level the launch stayed pending and nothing else happened. The pod
now hands the chosen slot's root level to the game's level load, in both modes, and the
trace records it. The viewer does not switch levels yet.

### What was found

FOUND (the 01.33 EBOOT, read from local research copies; nothing of it is in the
repository; notes in `local-analysis/d363/level-start-notes.md`):

- The pending launch is read every frame by FUN_00402560, called from the pod's update
  (0x40663c). pod+0x9c4 is 1 while the launch waits on its level request (id pod+0x9c8,
  state from FUN_0014e374: 1 queued, 3 done, 4 failed, 5 none) and 3 while the launch
  warning (popup 0x4b, "Continue" or "Cancel") is up.
- On a done request with no warning up, FUN_004022fc hands the level to the game:
  FUN_002467b8 copies the slot's root level to game+0x2f0, its SlotID to game+0x328 and
  the mode (0, Play) to game+0x120, then FUN_002258bc moves the game to its level load
  (state 7, or 6 or 8 by its flags). The pod then clears its wait (FUN_003f9794). A
  failed request shows error popup 0x13 and drops the launch.
- No script event fires on the hand-off; `IsLaunchingLevel` stays true until the level
  takes over. The pod draws nothing of its own for it.

INFERRED: a story level on the disc has nothing to fetch, so its request is done on the
first frame after the Cross. The online fetch, the warning and the error popup are left
out.

MEASURED (the part 109 runs, scripted against native, Cross at 760): both hand slot
(0, 127795) to the game after tick 761, with root level GUID 0001f333; the trace is
identical in both modes on every tick 680-790. That GUID resolves in the catalog and
loads in the viewer on its own (`--level-guid 0001f333`).

### What changed

- `lbp-runtime`: `SearchSlot::root` keeps the slot's root level; `LevelStart` (slot,
  root, mode) and `PodState::level_start`; `LevelWorld::pod_launch_step` makes the
  hand-off once per launch, and a new launch clears it.
- `lbp-bevy-viewer`: the launch step runs every tick before the input steps and prints
  the hand-off; the story trace writes `launch` and `level_start`.
- Test `d363_a_done_launch_hands_the_root_level_to_the_game`.

Left out: loading the level in the running viewer (the scripted world is built once at
start), the game's load timers, fade and loading screen, the warnings at 0x402b24 and
the select sound.

### Next

Switch the running viewer to the handed-off level, then the cursor's flight and sound and
the other searches.
