# Handoff: level1 lane (Rookie Test played with scripts), part 2 (2026-10-17)

## Read first
- docs/level1-play.md, section "Part 2" (why the first line stays open; the wall is real)
- crates/lbp-runtime/src/switches.rs `compute_output` (counter type 15, timer type 16) and
  `update_switch` (deactivated counters/timers)

## Facts
- Worktree E:\Projects\wt-level1, branch level1-2 (docs only). D numbers from D501 on (none used).
- Debug patch with the switch/message/Thing dumps (env `LBP_DBG_SW=1`):
  local-analysis/level1/p2-switch-debug.patch (apply with `git apply`, never commit it).
- Chain on board 1048 (Da Vinci 1029's chip): timer 1059 (59) -> counter 1060 (target 1) ->
  bubble 1064 and timer 1061 (44) -> AND 1062 (port 1 from 2997, outside) -> chip output 1063.
  1064 -> counter 1068 and camera tweak 1069. AND 1052 (1068, board out 0) -> timer 1053 (299)
  -> bubble 1054 (line 2) and the board's port 0. Camera tweak 1050 from counter 1049.
- RPCS3: launched lane A with SW_SHOWNOACTIVATE; the window and the screen region stayed white
  (bgcap and ImageGrab), guest tick 0xe3aaec stayed 0. Closed it. A normal launch (as in the
  2026-10-12 threads) may be needed; it takes focus once, so check with the coordinator first.
- Ghidra bridge on :8765 was running and answered (slow first calls; Git Bash needs
  MSYS_NO_PATHCONV=1 for gq.py).

## Next
1. Observe the level start in the game: does line 1 close by itself, after how long, and does
   line 2 follow; read the messaging list ([0xe40f10]+0x10, max 8) and counter 1060's PSwitch
   state if possible. Then fix the rule that turns 1060/1064's input off.
2. Candidates to check in the EBOOT: a counter reset rule (0x11ca04), chip deactivation of an
   NPC chip, the part update at 0x6a3264 (event 3 on its active byte 1 -> 0).
3. The wall is not a bug (Thing 1028, piston-hung post); the gate is opened by level logic later.
