# Handoff: profile goodies records, part 34 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-88.md`, this note.

## State

- PR 109 (`d341-profile-slots`, base main): `--profile-records PATH` and
  `scripts/rpcs3_profile_records.py`; the Story bubble's goodies row shows (The Cosmos 100%).
  When it is green, send the coordinator its number; the coordinator merges.
- Worktree `E:\Projects\wt-d324`, branch `d341-profile-slots`. Release target for captures:
  `target-d320`.
- Local, ignored: `local-analysis/d341-profile/` holds `records.txt` (the owner's records, the
  viewer input), `profile.json` (full dump), `dump.py` / `dump2.py` (boot RPCS3 minimised,
  X until the profile loads, read, close). They flip RPCS3's "Start games in fullscreen mode"
  to false; `config.yml.bak` there is the original, already restored.
- Capture: `cap.ps1` in this session's scratchpad (copy to the next one) takes `-Records
  local-analysis/d341-profile/records.txt -Light "--lbp-lighting"`; output there too.

## Next

1. Badge picture card beside the bubble (game frame `local-analysis/d337-copper/game-story.jpg`:
   the level icon in a white card). INFERRED: the globe badge under
   `Planets.ActivateBadgeTether`.
2. The bubble's green: the player's colours in memory (`*0xDC7B00` +0x1e4, player +0x16c,
   +0x4c..) are yellow first, so the Gooey's colour comes from elsewhere (part 88).
3. The viewer's default Story result is The Cosmos; the game frame has Da Vinci's Hideout.
4. Items without a GUID (`*(profile+0x518)` check) are counted as not owned.

## Notes

- Next free decision number: D342.
