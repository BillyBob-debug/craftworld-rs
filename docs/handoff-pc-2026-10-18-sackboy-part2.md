# Handoff: Sackboy model lane, part 2 (PC, 2026-10-18)

## Read first
1. `docs/d700-sackboy-model.md`: D701 and the Open list.
2. `crates/lbp-preview/src/surface.rs`: `same_value` and the port 2 match in `select_routes`.
3. This file's Facts.

## State
- PR #144 (branch `sackboy-model-2`, worktree `E:\Projects\wt-sackboy-model`): D701, the zip
  pull's gloss takes its own diffuse colour. Waiting on the rust and python checks; the coordinator merges.
- The eyes are waiting on the owner's answer about where they look wrong (the coordinator relays it).

## Facts
- Pod capture command and crops: see part 1's Facts. Ours pull pixels (635,570)-(640,588); game (635,545)-(640,562).
- Pull median: ours 64/70/64, game 86/97/86, while the knit matches. The unlit pull is light (about 165/172/188).
- Ruled out: normals, zip morphs (`zip04_morph` 28 opens the teeth), the gloss `w`, and the textures' sRGB flags.
- Program listings: `python -I scripts/cgb.py blobs zip_pull_steel` from `local-analysis/shading`.
  The gmat names come from `d68-inventory-0133-verified.json`.
- Eyes: at morph weights 0 they look about 1.2x the game's size relative to the head. `eyelid_*_up` (5, 6) at 1
  half-closes them. To test morphs, add a temporary env override in `morphed_bind_weights`.

## Next
1. The pull's brightness gap: try per-sampler gamma (does the game sample the UV1 AO raw?), then the pull's skinning joint.
2. Eyes: act on the owner's answer.
