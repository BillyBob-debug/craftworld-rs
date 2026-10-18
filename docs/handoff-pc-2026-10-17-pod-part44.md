# Handoff: PC, Pod scripts part 44 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-98.md`, this note.

## Facts

- PR 130 (D351, a left overlay shrinks out; the cards go when Story closes), worktree
  `E:\Projects\wt-d324`, branch `d351-story`; rust and python green, handed to the
  coordinator for merge. This note is in its own PR.
- The Story lane keeps D348-D399; next free decision number: D352.
- Owner rule (2026-10-18): the shipped app must never need a decrypted copy or keys; only
  the fully updated USA ISO plus the RPCS3 update folder. Data only in encrypted files
  (patch.edat scripts) needs a built-in fallback. Decrypted copies are fine for research.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`,
  `CARGO_TARGET_DIR=E:/Projects/wt-d324/target-d320`; viewer tests
  `cargo test -p lbp-bevy-viewer --bins`.
- Ghidra bridge restarted this part (the old one was hung with the port closed): kill the
  bridge pids, delete `project-nidfix/LBP2_0133.lock*`, run `tools/start-bridge.ps1`.
  Query helper `local-analysis/d349/g.py`; dumps for this part in `local-analysis/d351/`.
- Open: the card's own scale spring on Story open and close (0x42a92c, card+0x210; target
  = level scale times planet+0x114/+0x118), the dispatcher above CPlanets slot 0x1c
  (0x43d900, vtable word 0xd7489c), the generator state (seed 1 INFERRED), the pie's zero
  axis (shader 0x109ee), the Gooey scale 1.42 and the bubble's line metrics (fitted).
