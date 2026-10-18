# Level scripts, part 129: back to the Pod after a finish (2026-10-18, D382)

The second half of a level's end: once a launched story level finishes (part 128),
the viewer goes back to the Pod it was launched from. The Pod's state 2 then clears
the pending link the finish left and marks its target unlocked.

### What changed (lbp-bevy-viewer)

- The session keeps its home: the Pod level, its shell level and the viewer's Pod
  framing (camera, focus, shell, script camera), taken at the launch.
- A finish (`Game.GotoScoreboard__b` or `--level-scripts-finish-at N`) starts a switch
  back to the Pod on a worker thread, the same way the launch reads the level (D364).
  The switch now reads the shell level too, and the shell's start (our Pod, the
  session's hand-over) is shared by the start and the return.
- On the switch, the Pod's scripts are read fresh, as the game loads the Pod again;
  the profile's flags and story progress go with the session; the display is rebuilt
  with the Pod framing kept at the launch.
- The Pod's Sackboy is read again from the shell level's physics and arrives by the
  Pod's checkpoint spawn rule, as at the start (no spawn hold).

Run on the disc (Pod menu, Cross on the selected level at tick 720, finish at 30): the
cutscene (0, 127489) ended, the viewer was back in the Pod at tick 751, and on the
Pod's first tick the link to (0, 125365) was cleared with its target unlocked. Sackboy
stood in the Pod again.

Not made: the scoreboard and its choices; the pod computer stays closed on the return
(INFERRED: the game opens the story search for `LevelUnlockingThread`, not ported).

### Next

`LevelUnlockingThread` (read, D380), which now has a pending link to show.
