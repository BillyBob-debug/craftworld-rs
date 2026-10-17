# Handoff: PC, Pod scripts part 43 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-97.md`, this note.

## Facts

- PR 128 (D350, the overlays' turn axis and the badge's first fill, plus this note),
  worktree `E:\Projects\wt-d324`, branch `d350-story`. Merge route: when `rust` and
  `python` are green, the coordinator merges.
- The Story lane keeps D348-D399; next free decision number: D351.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`,
  `CARGO_TARGET_DIR=E:/Projects/wt-d324/target-d320`. Viewer tests:
  `cargo test -p lbp-bevy-viewer --bins`.
- Ghidra bridge (:8765) was down at the end of this part. Offline fallback: capstone over
  `local-analysis/ghidra/eboot-0133/EBOOT.elf` (venv Python
  `local-analysis/ghidra/tools/pyghidra-mcp-venv/Scripts/python.exe -I`); TOC r2 0xdaa0c0.
- The Story capture script (`cap.ps1`, part 42's note) was not found under E:\Projects at
  depth 4; look for it before the next capture.
- Open: the generator state at the overlay draw (seed 1 INFERRED); the dispatcher above
  CPlanets slot 0x1c (0x43d900), so per tick vs per frame; whether the rebuild 0x42324c
  (from 0x4235fc, 0x436cc4) runs on each Story opening; whether a hidden slot shrinks out;
  the pie's zero axis (shader 0x109ee); the Gooey scale 1.42 and the bubble's line metrics
  (fitted); the light mat inside the card rim.
