# Handoff: PC, Pod scripts part 39 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-93.md`, this note.

## Facts

- PR 112 (D344) and PR 113 (D345): green, waiting on the owner's merge in the project chat.
- PR 114 (`d346-badge-medals`, base main, stacked on 113, worktree `E:\Projects\wt-d324`):
  the badge's medals and the bubble pinned to the card (D346). When CI (`rust` + `python`)
  is green, send the coordinator its number. Once 112/113 merge, its diff shrinks to D346.
- Toolchain from Git Bash: `source E:/Projects/d321-env.sh` (cargo is not on PATH);
  cap.ps1 (scratchpad of session 99805691) uses `scripts/use-local-rust.ps1`.
- Records with overlays: `local-analysis/d346/records.txt` (Earth groups 4 22..27, mask 0x34).
- RPCS3: `local-analysis/d344/boot.py` (stop file closes it), `local-analysis/d346/nav.py
  enter x z x` reaches Story, `scan.py` dumps planet results. Patches stay off.
- Game vs viewer: `local-analysis/d346/g1.png` (game, 1280x720), `bubble1.png` (viewer),
  `compare2.jpg`.
- Open: the info bubble's own size and goodies row (pink dot, big 100%) differ from the
  game's; the overlay spin isn't the game's random draw; the light mat inside the card rim.
- Next free decision number: D347.
