# Handoff: Moon movement, part 2 (2026-10-11, PC thread, after #130)

Read CLAUDE.md, STATUS-CURRENT.md, docs/handoff-pc-2026-10-11-moon-anim.md, then this file.

## Done
- #130 (merged): moods/emotes + head-look layers, D233 (`docs/d233-sackboy-moods-headlook.md`). Ghidra pass 3:
  `local-analysis/results/2026-10-11-ghidra-sackboy-moods-headlook.md` (not committed). Mood input = D-pad press
  edges (up happy, down sad, right angry, left scared, INFERRED from mask bits), levels 1/2/3/off, no timeout,
  opposite cancels. Head look has no camera term (stick + tilt + c20 target only).
- Viewer: I/K/L/J moods; `--play-sackboy-input` letters U D R L. Clip bank 54 clips.

## Verified vs not
- RPCS3 D-pad Up on the Moon: happy pose ~1 s then the held last frame (matches 6 + 16 frames). D-pad Right (angry)
  showed nothing in 10 F12 frames (inconclusive; F12 is ~0.45 s apart, emote ~0.7 s). Retry with a held Right or
  memory read of the mood fields once the anim object address is known (this+0x894[i] levels).
- Left (scared) and Down (sad, create mode uses Down for hover/Popit) not tried.

## Open (priority order)
1. Slopes and walls: needs a level with sloped geometry loaded in RPCS3; identify it from unique data (name/GUID
   from the level inventory) before measuring, never list position. Ghidra slope limit ~55 deg (cos 0.5735),
   move-and-stop, no slide. Compare Sackboy y/x per tick (guest 0xe2b83c / 0xe2b840) against our controller.
2. Camera lead in walk direction: no static reader found; do a live memory diff of the follow camera vs vx.
3. Crouch spring, mirror maths, GroundTurnSpeed readers, right-stick head look (D233 OPEN list).

## Tooling notes
- Play run (hidden, no focus): `target-pr83\debug\lbp-bevy-viewer.exe --snapshot local-analysis/d68-inventory-0133-verified.json
  --game-dir "game-data/a1-disc;game-data/a1-update-0133" --level-path gamedata/levels/palettes/borders/blank_level_large.bin
  --play-sackboy --no-activate --play-sackboy-input "<script>"` (do not pass --select-guid; it drops the clip bank).
- RPCS3 key helper: local-analysis/pc-helpers/pad.py (hold(key, ms)); D-pad = arrow keys; F12 screenshots land in
  RPCS3\screenshots\BCUS98245. Focus use was granted for these jobs; restore focus after.
- Implementer workers: `claude -p` in a worktree cannot run cargo under its permission gate; build and test from
  the main session with CARGO_TARGET_DIR=target-pr83.
