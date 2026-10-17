# Handoff: level1 lane (Rookie Test played with scripts), part 3 (2026-10-17)

## Read first
- docs/level1-play.md, section "Part 3" (Circle closes a line; it corrects Part 2)
- crates/lbp-runtime/src/lib.rs `LevelScripts::script_activations` and
  `LevelScripts::player_close_message`

## Facts
- PR 129 (branch level1-3, D502): C/Circle runs the bubble's `OnMagicMouthRemoteStop__b(true)`.
  Scripted Things with `SwitchGetNewActivation__i` now set their outputs' stored signal before
  the switch pass. Viewer flag `--level-scripts-close-at N`. Next D number: D503.
- Game (RPCS3, 2026-10-17): line 1 opens in a letterboxed cutscene with a Circle "Close"
  prompt and waits for Circle. Line 2 opens about 10 s after Circle. The level then goes
  right (arrow sign). The post left of the spawn (Thing 1028, on piston 1026) is not on the
  route, so driving pistons is not needed to progress.
- Pistons: nothing drives a joint's length from its input (lbp-sim lib.rs `spawn_joint`
  sets fixed limits; sackboy_play.rs `pod_scene` clears joints in --play-sackboy).
- RPCS3 in the background: PrintWindow/bgcap gives white or black frames (Vulkan). F12
  through rpcs3_bgpad.py saves a real frame to rpcs3/screenshots/BCUS98245 (delete it after
  use). Navigation that worked: X (skip intro), X (OK on the profile warning), Z (Use Pod
  Computer), X (Story), A/D (cycle worlds; matched "Da Vinci's Hideout" by name), X, A
  (Rookie Test, matched by name), X (Play). Level tick 0xe3aaec counts only in a level. The
  messaging pointer [0xe40f10] did not change while bubbles opened, so it is the wrong
  address for the list.
- Toolchain: `local-analysis/toolchain`. PATH needs cargo\bin plus the gnu toolchain's
  `lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained` (dlltool). CARGO_TARGET_DIR is
  E:\Projects\wt-level1\target.

## Next
1. Close line 2 and see what follows: run with two closes and check the next chain on board
   1048 and beyond.
2. The cutscene camera while line 1 is open (camera tweak 1069 from counter 1068 / 1050).
3. Draw the Close prompt on the no-prompt bubble (the game shows it for 1064).
