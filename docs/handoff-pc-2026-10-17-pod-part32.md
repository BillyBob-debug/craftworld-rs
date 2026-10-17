# Handoff: Story globe jelly and AO overlays, part 32 (2026-10-17, PC)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-85.md`, this note.

## State

- PR 103 (`d331-story-zoom`, base main): green, waiting on the owner's merge (`gh pr merge`
  is blocked by the auto-mode check; the owner was asked in the thread).
- PR 104 (`d336-story-backdrop`, draft, base `d331-story-zoom`).
- PR 105 (`d337-copper-glassy`, draft, base `d336-story-backdrop`): the jelly
  (0x1a6f5) on the glass route; a static gloss colour above 1 no longer drops a surface.
- PR 106 (`d338-ao-overlay`, draft, base `d337-copper-glassy`): a solid material with its
  alpha port wired blends by it (`alpha_overlay`, `Asset::overlay_alpha`); the Story globe's
  continents now show copper. All in worktree `E:\Projects\wt-d324`.
- Local checks pass on 106 (fmt, workspace clippy, viewer and lbp-preview tests).

## Next

1. When 103 merges: rebase each stacked PR onto its merged base (`git rebase --onto
   origin/main d331-story-zoom` on 104, then 105 onto 104, 106 onto 105), retarget, mark
   ready, and ask the owner to merge each on green.
2. The group's badge card and its info bubble (the game frame
   `local-analysis/d156-rpcs3-pod-story.png` shows it: picture card and a green bubble with
   the level name, author and goodies percentage).

## Notes

- Capture script: copy part 31's `cap.ps1` (scratchpad), `-Light "--lbp-lighting"`, output
  `local-analysis\d337-copper` (before/after JPEGs there).
- Next free decision number: D339.
