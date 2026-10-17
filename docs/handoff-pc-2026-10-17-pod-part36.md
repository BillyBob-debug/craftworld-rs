# Handoff: PC, Pod scripts part 36 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-90.md`, this note.

## Facts

- PR 111 (`d343-badge-mesh`, base main, worktree `E:\Projects\wt-d324`): the Story view's
  badge card (D343). When rust and python are green, send the coordinator its number.
- Captures: `cap.ps1` (in the main checkout's scratchpad copies; release target
  `target-d320`) with `-Records local-analysis/d341-profile/records.txt -Light
  "--lbp-lighting" -Out local-analysis\d343`. Bash builds source a copy of `env.sh`
  (CARGO_HOME/RUSTUP_HOME under `local-analysis/toolchain`, the gnu self-contained linker).
- The badge's place: `story_world::add_badge` = T(Earth model centre) x R(Earth Thing POS)
  x S(Earth model scale 1.35) x `badge_matrix` (FOUND 0x6ffff4). Using the Earth model's
  rotation put it behind the machine; the planet's turn works in the Thing's frame.
- Open, in order of size: the card draws 1.18x the game's height (194 vs 164 px; 0.66x
  without the Earth scale); the frame should take the profile yellow (INFERRED instance
  tint; trace 0x6fe974 / the polaroid's draw for a colour); the goodies icons are overlay
  slots 1..12 (0x6fca08, offsets TOC 0x6d4c..0x6d74); the info bubble's measured place
  (part 86) overlaps the card, the game's tail points at the card's right edge (x 715).
- Set aside: `local-analysis/d342/slot-icons.patch` (menu slot icons; the badge now
  decodes its own icon in the worker).
- Next free decision number: D344.
