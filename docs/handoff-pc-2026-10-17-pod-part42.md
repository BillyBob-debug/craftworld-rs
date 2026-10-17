# Handoff: PC, Pod scripts part 42 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-96.md`, this note.

## Facts

- PR 124 (D349, the badge overlays' spin and spring-in, plus this note) was green
  (`rust`, `python`) and handed to the coordinator to merge. Worktree `E:\Projects\wt-d324`,
  branch `d349-story`.
- The Story lane keeps D348-D399; Popit uses D400s, level1 D500s.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`. Viewer tests:
  `cargo test -p lbp-bevy-viewer --bins`.
- Ghidra bridge (:8765): client Python is
  `local-analysis/ghidra/tools/pyghidra-mcp-venv/Scripts/python.exe -I` (the system Python
  has no `mcp`); clients `local-analysis/d349/g.py`, `local-analysis/d254-mirror/gdis.py`.
  The TOC r2 is 0xdaa0c0 (Ghidra's assumed 0xd9a0f8 gives wrong constants).
- Story capture: a copy of `cap.ps1` (PowerShell, `-Name X -Build -Out local-analysis\d349
  -Records local-analysis\d346\records.txt`) builds into `target-d320` from the worktree
  and runs the viewer hidden with `--level-guid 00018fdb --pod-shell 000027b9
  --level-scripts --level-scripts-in-use-at 600 --level-scripts-choose-at 640
  --level-scripts-hold-at 760`. Last frame `local-analysis/d349/spin1.png`.
- Open: the generator state at the overlay draw (seed 1 INFERRED) and whether 0x6fe974
  runs per frame or per tick; who sets card+0x22c; the overlay rotation axis (Y from part
  93, a second read suggested the card-plane Z: check 0x6fefb0-0x6ff0c8); the pie's zero
  axis (shader 0x109ee); the Gooey scale 1.42 and the bubble's line metrics (fitted); the
  light mat inside the card rim.
- Next free decision number: D350.
