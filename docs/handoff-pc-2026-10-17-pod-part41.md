# Handoff: PC, Pod scripts part 41 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-95.md`, this note.

## Facts

- PR 117 (D348, the goodies disc's pie) was rebased onto main after PRs 118 to 121; the
  coordinator merges it once `rust` and `python` are green. Worktree `E:\Projects\wt-d324`.
- Other lanes now use D400s (Popit) and D500s (level1); the Story lane keeps D3xx.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`. Viewer tests are in the
  binary: `cargo test -p lbp-bevy-viewer --bins`.
- Ghidra bridge (:8765): from Git Bash set `MSYS_NO_PATHCONV=1` or `/EBOOT.elf` is mangled.
  0x4f5abc is not a defined function in the bridge's program; the D348 evidence was read
  with headless Ghidra on the untouched `project`, read-only (`local-analysis/d348/`).
- Open: the pie's zero axis and turn are in shader 0x109ee (INFERRED straight up,
  clockwise); no game frame of a part-filled disc yet (needs a Story level below 100%);
  the Gooey scale 1.42 and the bubble's line height / ink raise / break gap are fitted, not
  read (the scale is at the Gooey +4: a memory read would settle it); the badge overlays'
  random spin and grow-in; the light mat inside the card rim.
- Next free decision number: D349.
