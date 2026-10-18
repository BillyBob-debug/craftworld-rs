# Handoff: PC, Pod scripts part 45 (2026-10-18)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-99.md`, this note.

## Facts

- PR 137 (D352, the Story card opens at full size and shrinks out on closing, with this
  note), worktree `E:\Projects\wt-d324`, branch `d352-story`; handed to the coordinator
  for merge once rust and python are green.
- The Story lane keeps D348-D399; next free decision number: D353.
- Owner rule (2026-10-18): the shipped app needs only the updated USA ISO plus the RPCS3
  update folder, never a decrypted copy or keys. Cross-lane finding (Popit lane): all 426
  game scripts live only in patch.edat, so with disc and update alone the Pod and Story
  load no scripts. Story needs a built-in fallback; the launcher thread planned a built-in
  Story screen as its part two, and the Popit lane owns the Pod/Popit fallback.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh`,
  `CARGO_TARGET_DIR=E:/Projects/wt-d324/target-d320`; viewer tests
  `cargo test -p lbp-bevy-viewer --bins`. Ghidra helper `local-analysis/d349/g.py`; dumps
  for this part (full `.text` disassembly `full.txt`) in `local-analysis/d352/`.
- Next (owner-approved, `docs/project-boost.md` item 1): Story's script fallback as native
  Rust, one script per task, the RScript VM kept as a dev oracle. Plan: list the scripts
  the Story search runs (from `native_coverage`, starting with the one that opens it and
  sets `story_open`, then the planets/badge fill), record a VM golden trace per script
  with decrypted data (inputs, native calls, globals per tick; derived data, kept in
  ignored local-analysis/), write each native entry behind the shared trait, and test it
  against its trace. Coordinate with the launcher thread's built-in Story screen. Do not
  edit STATUS-CURRENT.md's structure or .github/workflows (another thread owns them).
- Open: the card's snap flag `*(planets+0x20)+0xb0` and the 0.05 speed kick (0x4279a8),
  card+0x214/+0x218 (0x89568c), whether the rest of the Story world stays up while the
  cards shrink, the dispatcher above CPlanets slot 0x1c (0x43d900), the generator seed
  (1 INFERRED), the pie's zero axis (shader 0x109ee), the Gooey scale 1.42 and the
  bubble's line metrics (fitted).
