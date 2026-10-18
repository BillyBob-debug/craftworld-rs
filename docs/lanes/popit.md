# Lane state: popit (2026-10-18, seeded from handoff-pc-2026-10-18-popit-part11.md)

## Read first
1. `docs/d414-popit-tooltip-side-rule.md` (the full 0x417b78 rule and its Open list).
2. `docs/d415-popit-head-look-at-cursor.md` (head look, body-facing finding, Open).
3. Research notes (canonical, ignored): `local-analysis/d415/tooltip-rule.md`,
   `local-analysis/d414/popit-facing-headlook.md`.

## State
- PR #143 (branch `popit-13`, worktree `E:\Projects\wt-popit`): D414 + D415; the
  coordinator merges once `rust` + `python` are green. RPCS3 not used. Nothing running.
- Decision numbers: D416 and up.

## Facts
- Toolchain: `. "E:\Projects\LBP2 Rust Rewrite\scripts\use-local-rust.ps1"` (the worktree's
  copy throws: no local Rust there).
- No-decrypted run (absolute canonical paths): `--snapshot ...\local-analysis\d79-inventory-0133.json
  --game-dir "...\game-data\a1-disc;...\game-data\a1-update-0133"`. Blank level for quick
  checks: `--level-path gamedata/levels/palettes/borders/blank_level_large.bin --play-sackboy`.
- `--popit-cursor` starts in cursor mode; the Pod capture holds ticks, so the pose freezes.
- EBOOT: no Popit body-turn rule (0x2002ac); pi only for the pod computer's user. Head look
  target branches at 0x20e28c; arms toward the cursor at 0x2097e0 (not ported).
- Tooltip y side is still the MEASURED row rule (the y test is too close to call with the
  port's pill size).

## Next
1. Arms toward the cursor (0x2097e0 / 0x20936c, clips anim+0x4e0..0x4f8).
2. Which Popit screens are cursor-active; the Pod's look up at the card (branches 2-3).
3. Compare D414's outward lean with the Moon create frame side by side.
4. Pod planets without the scripts (D411 Open); highlight sizing (D400 Open).
