# Handoff: PC, Pod scripts part 38 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-92.md`, this note.

## Facts

- PR 112 (D344, badge size): green, waits on the owner's merge OK (the coordinator asked).
- PR 113 (`d345-badge-tint`, base main, stacked on 112, worktree `E:\Projects\wt-d324`):
  the badge frame in the profile's first colour (D345). When rust and python are green,
  send the coordinator its number. Once 112 merges the diff shrinks to D345 alone.
- D345 facts: `cosmos_polaroid.mol` group 0 (gmat 0x6a16) is the picture (icon mixed over
  grey 0.30 by alpha), group 1 (gmat 0x6a17) the frame (solid grey 0.486); no ThingColor
  box in either, so `lbp_preview::tint_solid_group` tints group 1 directly.
- Open: the game's card has a light mat between the gold rim and the photo; the viewer's
  frame is gold up to the photo (picture group's grey vs an untinted frame part, unchecked).
- Next: the goodies icons (overlay slots 1..12, 0x6fca08, TOC 0x6d4c..0x6d74), then the
  info bubble overlapping the card (the game's tail points at the card's right edge, x 715).
- RPCS3 memory tools: `local-analysis/d344/` (`boot.py`, `scan5.py`, `dump.py ADDR LEN`).
  Story view: after boot, bgpad enter, x, z, then x.
- Ghidra bridge from Git Bash needs `MSYS_NO_PATHCONV=1`; TOC r2 0xdaa0c0 for 0x6ffff4/0x4226b0.
- Captures: scratchpad `cap.ps1` (copy from the newest session scratchpad; -Build,
  `-Records local-analysis/d341-profile/records.txt -Light "--lbp-lighting" -Out
  local-analysis\d345`); crops of the badge at (440,240,840,460); game frame
  `local-analysis/d344/story-full.png`; comparison `local-analysis/d345/compare.jpg`.
- Next free decision number: D346.
