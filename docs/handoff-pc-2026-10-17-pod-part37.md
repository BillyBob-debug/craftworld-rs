# Handoff: PC, Pod scripts part 37 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-91.md`, this note.

## Facts

- PR 112 (`d344-badge-size`, base main, worktree `E:\Projects\wt-d324`): the badge card's size
  (D344). When rust and python are green, send the coordinator its number.
- Next, D345: tint the badge frame with the profile's first colour (0xffceb424, the
  `colours` line of `--profile-records`; MEASURED at the badge instance +0x10/+0x14). The viewer's
  tint path is in `thing_view.rs` (render mesh tint as an RGB multiplier).
- Then: the goodies icons (overlay slots 1..12, 0x6fca08, TOC 0x6d4c..0x6d74), and the info
  bubble overlapping the card (the game's tail points at the card's right edge, x 715).
- RPCS3 memory tools: `local-analysis/d344/` (`boot.py` boots minimised and idles until a
  `stop` file exists; `scan5.py` finds Earth's planet object by its 1750 radius; `dump.py ADDR LEN`).
  Story view: after boot, bgpad enter, x, z, then x.
- Ghidra bridge from Git Bash needs `MSYS_NO_PATHCONV=1`; TOC r2 0xdaa0c0 for 0x6ffff4/0x4226b0
  (0xd9a0f8 for 0x16afb0). Capstone is installed for raw disassembly where the bridge skips.
- Captures: scratchpad `cap.ps1` (-Build, `-Records local-analysis/d341-profile/records.txt
  -Light "--lbp-lighting" -Out local-analysis\d344`); game frame `local-analysis/d344/story-full.png`.
- Next free decision number: D345.
