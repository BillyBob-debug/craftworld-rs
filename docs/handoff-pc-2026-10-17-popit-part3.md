# Handoff: Popit lane, part 3 (PC, 2026-10-17)

## Read first
1. `docs/d402-popit-titles-and-pod-button.md` (titles, default pages per collection, Pod rule).
2. `crates/lbp-ui-script/src/popit.rs`: `answers(edit, our_pod)`, `run_card`, `run_responses`.
3. `crates/lbp-bevy-viewer/src/popit_play.rs`: `PAGES` (title keys), `SOUNDS_TITLE`,
   `HEART_TITLE`, `PLAY_ROWS_POD`, `script_cards`.
4. Part 1 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part1.md`) for inputs and captures.

## State
- PR #121 (branch `popit-3`, worktree `E:\Projects\wt-popit`): D402 plus this handoff.
  The coordinator merges once `rust` + `python` are green.
- Scratch (ignored): `local-analysis/d402/` (viewer logs and captures). RPCS3 was not started.

## Facts
- Build env: copy `E:\Projects\d321-env.sh` with `CARGO_TARGET_DIR=/e/Projects/wt-popit/target`
  and `source` it in bash (cargo is not on PATH otherwise).
- Ghidra bridge: from `local-analysis/ghidra`, `export MSYS_NO_PATHCONV=1` before
  `tools/pyghidra-mcp-venv/Scripts/python.exe tools/gq.py ... binary_name=/EBOOT.elf` (Git Bash
  otherwise rewrites `/EBOOT.elf`). `native_GetTitleKey__` is at 0x605364; the default
  collection/page builder is around 0x465688 (evidence only, never copied).
- English table with keys: `local-analysis/d109-oracle-0000799b-english.json` (`result.rows`).
- The viewer has no Popit in the Pod (D281), so `--pod-shell` captures show no card; `--qa-capture`
  refuses an existing PNG path.

## Next
1. Popit in the viewer's Pod (Square away from the computer), then capture the Pod card.
2. Page switching within a collection (default pages listed in D402).
3. Highlight sizing 264 / style 4 vs the 68 px focus tile (D400).
4. Use decision numbers D403 and up.
