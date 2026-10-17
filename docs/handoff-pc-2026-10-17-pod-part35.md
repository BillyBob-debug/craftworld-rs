# Handoff: PC, Pod scripts part 35 (2026-10-17)

Read first: STATUS-CURRENT.md, `docs/level-scripts/part-89.md`, this note.

## State

- PR 110 (`d342-badge-card`, base main): the profile's colour (D342). When rust and python are
  green, send the coordinator its number; the coordinator merges.
- Worktree `E:\Projects\wt-d324`. Release target for captures: `target-d320`.
- Local, ignored (main checkout's `local-analysis/`):
  - `d341-profile/records.txt` now has the owner's `colours` line (the viewer input).
  - `d342-green/dump3.py` boots RPCS3 minimised, reads `*(profile+0x514)` colours, closes,
    restores config.yml (fullscreen flag). `d342-badge/notes.txt`: the badge trace notes.
  - `d342/slot-icons.patch`: viewer change set aside: `MenuAssets::slot_icons` decoded from
    `SearchSlot::icon` in `load_menu_assets` (82 icons decode). Re-apply when it gets a use.
- Capture: `cap.ps1` (copy from this session's scratchpad) with `-Records
  local-analysis/d341-profile/records.txt -Light "--lbp-lighting" -Out local-analysis\d342`.
  Bash builds: source a copy of the toolchain env (CARGO_HOME/RUSTUP_HOME under the main
  checkout's `local-analysis/toolchain`, the gnu self-contained linker); PowerShell needs
  `-ExecutionPolicy Bypass` for `use-local-rust.ps1`.

## Next: the badge mesh (D343)

1. FOUND (part 89): the Story badge is `cosmos_polaroid.mol` (0x6a1a) for a group/level card.
   MEASURED (lbp-preview load): 2 primitives. 0: textured, 32 tris, UV 0..1, a placeholder
   256x256 picture (lips) with alpha mix to 0.30 grey: the game swaps the slot icon into this
   texture (0x6fe974). 1: Solid 0.486/0.486/0.510, 73 tris: INFERRED the back/frame; the owner's
   frame's border (about 100, 88, 18) fits the profile yellow x 0.49 (an instance tint; check).
2. Placement (part 81 FOUND): slot `location` x Earth radius 1750 in Earth space, axes from
   (0,1,0) and that direction (0x6ffff4), scale radius x angle / 240, angle 0.18 group, 0.12
   level. The globe centre comes from the Pod Earth item's world (`story_world::earth_item`).
3. Route (Explore): load the mesh on the worker beside main.rs ~494 with
   `lbp_preview::load_mesh_groups_with_overrides`; swap primitive 0's `Asset.texture` for the
   default group's icon and recompute `tinted_rgba` (linked.rs ~1786 chain; add a pub helper);
   push one SceneMesh + SceneItem { thing: globe, alternate: true, .. } in `story_world::merge`
   so `follow_story_world` shows it only while Story is up.
4. The owner's frame (`d330-earth-badges/game/b-story-settled.jpg`, 1280x720): card's coloured
   square x 564..715, y 266..430; three gold goodies icons along its bottom (not traced).

## Notes

- Next free decision number: D343.
