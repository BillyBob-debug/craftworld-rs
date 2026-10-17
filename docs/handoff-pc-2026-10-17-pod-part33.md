# Handoff: Story info bubble and goodies row, part 33 (2026-10-17, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-86.md`, `docs/level-scripts/part-87.md`,
this note.

## State

- PRs 103 to 107 are merged (Story zoom, backdrop, jelly glass route, AO overlays, the
  info bubble D339).
- PR 108 (`d340-goodies-row`, base main): the info bubble's goodies row. It is built but
  hidden, because its total comes from the player profile, which isn't read yet. Merging now
  goes through the coordinator: when a PR is green, send it the number.
- Worktree `E:\Projects\wt-d324`, branch `d340-goodies-row`.

## Next

1. Read the profile's per-slot records (part 87: 0x80-byte entries keyed by SlotID at
   profile+0x280, list at +0x38) so `profile_goodies` fills in and the row shows.
2. The picture card beside the bubble (game frame `local-analysis/d337-copper/game-story.jpg`).
   The EBOOT notes in `local-analysis/d339-info-bubble-notes.md` found no card in the bubble
   code. INFERRED: it is the globe badge (slot icon) under the script-driven tether
   (`Planets.ActivateBadgeTether`).
3. The "Story" title bubble is green in that game frame. It is probably the player colour
   too; the viewer draws it yellow.
4. The bubble's placement is measured, not decoded (0x701780).

## Notes

- Capture: `cap.ps1` (copy from part 32's scratchpad) with `-Light "--lbp-lighting"
  -Out "local-analysis\d339-bubble"`. It runs the viewer hidden, so it takes no focus.
- Next free decision number: D341.
