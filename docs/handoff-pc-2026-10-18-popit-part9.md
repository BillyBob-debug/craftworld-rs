# Handoff: Popit lane, part 9 (PC, 2026-10-18)

## Read first
1. `docs/d409-popit-worn-colour-and-pod-square.md` (D409 / D410, and its Open list).
2. `crates/lbp-bevy-viewer/src/popit_play.rs`: the tooltip `place` / `pill` block in the
   card UI (search `Place::Above`), `popit_input`, `swatch_sections`.
3. Part 8 handoff "Facts" (`docs/handoff-pc-2026-10-17-popit-part8.md`).

## State
- PR #134 (D408) merged. PR #136 (branch `popit-10`, worktree `E:\Projects\wt-popit`):
  D409 / D410 plus this handoff; the coordinator merges once `rust` + `python` are green.
- RPCS3 is closed.

## Facts
- RPCS3 never-focus launch: canonical `local-analysis/pc-helpers/rpcs3_launch.py`
  (`launch()`, `place()`, `close()`): starts rpcs3.exe through WMI Win32_Process.Create
  (no foreground rights), moves the window with SWP_NOACTIVATE, and a watchdog kills
  RPCS3 if it ever becomes foreground. Tested 90 s from boot: never foreground. Use it
  instead of `subprocess.Popen` in boot scripts; keys via `bgpad.py`.
- Pod Popit capture (frames in ignored canonical `local-analysis/d409/`; game frame
  `local-analysis/pc-helpers/pod-popit3.png`): the command in the D409 doc's MEASURED.
  The planets camera is at rest from tick 452; earlier frames look from inside the shell.
- Standing rule (owner, 2026-10-18): the shipped app must not need decrypted files; only
  the updated USA ISO plus RPCS3's installed update. The Poppet script (card, pages) is in
  the decrypted patch.edat overlay; `fallback_cards()` / `FALLBACK_OPENS` are the built-in
  fallback, so check the Popit still works without `local-analysis/d133-edat-overlay`.
- Use decision numbers D411 and up. `scripts/use-local-rust.ps1` is only in the canonical
  repo; dot-source it from there.

## Next
1. Tooltip pill placement rule (D409 Open): Pod top-left focus puts the pill above-right,
   Moon top-right focus puts it right; find the game's rule (EBOOT gooey tooltip).
2. Check the Popit without the decrypted overlay (standing rule above).
3. Highlight sizing 264 / style 4 vs the 68 px focus tile (D400 Open).
