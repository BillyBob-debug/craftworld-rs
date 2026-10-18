# Level scripts, part 126: the level load and the pod introduction (2026-10-18, D379)

On a fresh profile the Pod now opens with the game's introduction camera without the
encrypted scripts: the camera starts far below the planets and zooms straight in to the
Pod's rest framing over 451 ticks, then the progression flags 3 are set and the intro
does not run again. Before, the scriptless Pod world ended the intro state on the next
frame. The level load after a launch needed no change (below).

### The level load after a launch (FOUND, 01.33 EBOOT)

- 0x2467b8(gs, mode, key) sets gs+0x120 = mode once 0x223e74(gs) is 0 and 0x240db0
  agrees. The state: 7 while game+0x164 is 0 (the boot sequencer 0x228594 sets it to 1);
  else 8 when key+0x38 is not 2, mode is not 1 and 0x246784(0x228818(game, 0), 0) is not
  4; else 6. 0x2258bc applies it only from the idle state 0x14, else holds it at gs+0x258.
- States 6 and 7 share their handlers (table 0xc24944, state*26+substate): 35
  (0x25a62c: resets each player, 0x228328, stride 0x6b0; loader+0x2d = 1; the key kept
  at global+0x4c2c), 37 (0x25b288) and 0 (0x25bf40, back to 0x14). State 8 adds the peer
  list (9, 0x25ab50) and a popup (10, 0x25a838, 0x3e995c type 0xa).
- The level swap is 0x25fdb0 (via 0x261028 from the native at 0x987554); the loader
  ticks in 0x2537e8 while game+0x124 is 3.
- No loading art, fade, minimum time or music stop was found in these states.
  INFERRED: they are a handshake, not a visible phase, and a solo disc story level takes
  state 8; the viewer's switch once the level is loaded (D364) matches.

### The pod introduction (FOUND, the 01.33 scripts, research only)

- `PlanetsWorld.UpdateState__` starts it (state 1) while progression flag 2 is clear and
  the game is interactive, and goes back to state 0 once the thread has finished.
- `PodIntroductionThread` asks the pod computer for a video and for a voice-over line,
  then sets its `Zoom` to i/450 for i = 0..450, one yield a tick (451 ticks). It waits
  while the line or the video plays, sets the flags 3, also 8188 when LBP1's Moon is
  unlocked and this is not a demo or kiosk build, and 8064 in the PSN demo.
- `UpdatePodCamera` in state 1 sets the camera from (-500, -2000, 2000) to the rest eye
  (0, -250, 1130) and the focus from (-100, -800, 1000) to (0, 100, -1500) by the zoom,
  up (0, 1, 0), without easing.

### What changed

- `lbp-runtime`: the native `PlanetsWorld` runs the introduction as port-only members
  (`IntroStep`, `IntroZoom`, `IntroRunning`): a step before each frame's update, the
  camera above, then the flags. Its video and voice-over requests are not made (nothing
  would play). `LevelUnlockingThread` stays unported (state 2 ends on the next frame).

Parity 5 pass.

### Next

The level unlocking thread and `OnEnterPod__`'s voice-over.
