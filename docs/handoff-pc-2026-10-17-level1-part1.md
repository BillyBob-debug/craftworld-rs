# Handoff: level1 lane (Rookie Test played with scripts), part 1 (2026-10-17)

## Read first
- docs/level1-play.md (part 1: the chain from player sensor 1019 to Da Vinci's bubble 1064)
- crates/lbp-runtime/src/switches.rs `board_port_offset`
- crates/lbp-bevy-viewer/src/level_play.rs header (how `--level-scripts` and `--play-sackboy` combine)

## Facts
- Worktree E:\Projects\wt-level1, branch level1-1, PR #120 (D500). D numbers from D501 on.
- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"` in PowerShell, then
  cargo in the worktree (own target dir). Run the viewer from the main repo folder (relative
  game-data paths); command in docs/level1-play.md.
- QA: `--play-sackboy-input "-:30,l:45,-:75" --qa-hide-ui --qa-capture local-analysis/level1/X.png`;
  the log prints `Messages at tick N` lines. Captures in local-analysis/level1 (ignored).
- Spawn and landing match the game (y 5975 vs 5974). The level's sensors and switch chain run.
- Speech bubbles' own trigger children have off-level matrices (radius -11 etc.): unused.
- `level_scripts` example: the level argument must be the MAP path, not the GUID; the output
  JSON must be a new file under local-analysis/.
- Script listings: local-analysis/d133-script-listings-0133 (Trigger_Speech_Bubble is
  a2fd9350...txt). The Ghidra bridge on :8765 was not running.

## Next
1. Why the first line never closes: the bubble's dialogue length / audio clock
   (DialogueLengthMs 3060 on 1054; `UpdateAudio__`, `Update__` in the listing). Then the
   second line (1054) follows via counter 1068 -> AND 1052 -> timer 1053 (299 ticks).
2. The wall left of the spawn at x -14348 that stops Sackboy before Da Vinci: check in RPCS3
   whether the game lets him through (do not make slopes/walls the slice; owner paused them).
3. RPCS3 capture of the level start to time the first line (bgpad/bgcap only, patches off,
   check RPCS3 is not running in another thread first).
