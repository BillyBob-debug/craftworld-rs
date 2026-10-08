# D152: level decorations in the Bevy level mode

Date: 2026-10-08 (America/Chicago). Scope: DECORATIONS part entries of level Things,
drawn in `--level-guid` / `--level-path` mode. STICKERS decals are counted, not drawn.

## What changed

- `lbp-level` (`parts.rs`): `Decoration` now keeps every serialized field (parentBone,
  parentTriVert, baryU/V, angle, onCostumePiece, earthDecoration, scale, placedBy,
  reversed, playModeFrame) instead of skipping them. `PartData::sticker_decals` counts
  STICKERS decals (main array plus costume arrays); decal bodies are still skipped.
- `lbp-preview` (`level_scene.rs`): `add_decorations` adds one scene item per DECORATIONS
  entry with a mesh reference, world = owner POS x offset (then x inverse bind of the
  decoration mesh's bone 0 under the default inverse-bind placement, as POS Things).
  Meshes load once per reference through the shared level mesh cache and draw through
  the existing textured mesh/material pipeline (same as RENDER_MESH Things).
  `SceneCounts` replaces `decorations_not_drawn` with `decorations_total`,
  `decorations_drawn`, `decorations_drawn_neutral`, `decorations_skipped` (by reason) and
  `sticker_decals_not_drawn`; the panel / `--level-report` print them.
- Read-only probe `crates/lbp-preview/examples/decoration_probe.rs` prints per-entry
  numbers (owner POS translation, offset translation/column norms, fields).

## Format and placement evidence

| Rule | Evidence label | Source |
|---|---|---|
| PDecorations field order (renderMesh ref, offset m44, parentBone i32, parentTriVert i32, baryU, baryV, angle f32, onCostumePiece s32, earthDecoration s32, scale f32, placedBy i16, reversed bool, playModeFrame i32, planGUID guid) | Community reader (already in `parts.rs` header since D107) | Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, `Decoration` / `PDecorations` serializers (read for field order only, no code copied) |
| `offset` is in the owner Thing's POS frame (world = POS x offset) | Local experiment, inferred | `decoration_probe` on Rookie Test: offset translations are tens to ~1,600 units while owner POS translations are ~3,000-29,000; offset z is 10/90/190/290 (front faces of shapes at z -500/-400/-200/-100 with the usual thickness steps). Across all 150 entries of the three levels no offset |x|/|y| translation exceeds 2,000 units and every owner has POS. Toolkit does not render decorations, so no oracle placement exists |
| Inverse bind of decoration mesh bone 0 applied after the offset | Inferred (consistency with D120 POS Things) | D120 rule; not separately verified for decorations |
| `scale` / `angle` fields are not applied on top of the offset | Inferred | probe: `scale` usually equals the offset's y-column norm (e.g. 2.685 with columns 0.14/2.685/0.14) but not always (one Rookie entry: columns 0.535/0.382/0.513, scale 0.745); the matrix is taken as the full transform |

Applicability: 01.33 levels in band (heads read by `lbp-level`). Tested on the three
levels below only.

## Counts (counting unit: DECORATIONS entries; scope: one level each, 01.33 + EDAT overlay)

| Level | GUID | Decorations drawn / total (baseline D149: 0 drawn) | Sticker decals not drawn |
|---|---|---|---|
| Rookie Test | 0001e786 | 51 / 51 | 910 |
| Grab and Swing | 0001e793 | 84 / 84 | 444 |
| Hedge Hopping | 0001e7f4 | 15 / 15 | 118 |

Newly viewable: 150 decoration entries across the three levels (0 before). Newly
Complete: not claimed; placement is inferred and no oracle comparison exists.

## Unsupported / not done (explicit)

- STICKERS decals (1,472 across the three levels) are not drawn. Decals are stored in the
  owner's surface UV space (u, v, xvec, yvec, texture, colour); projecting them needs the
  shape/mesh UV mapping and a decal pass. Counted and listed in the report as not drawn.
- Decoration fields not applied: parentBone / parentTriVert / bary (all 150 entries are
  bone 0, triVert -1, so the offset alone places them), angle (52 entries non-zero;
  assumed baked into offset), reversed (20 entries true; possibly a mirror not in the
  matrix, unverified), onCostumePiece / earthDecoration (costume and planet decorations;
  0 entries here), playModeFrame.
- Decorations on Things without POS (JOINT-placed) are skipped by reason; none occur in
  the three levels.
- Back-referenced RENDER_MESH bodies that the reader did not keep are skipped as
  "render mesh not read"; none occur in the three levels.

## Verification

Run from the main checkout with the worktree build:

- `lbp-bevy-viewer --snapshot local-analysis/d133-inventory-0133-edat.json --game-dir "game-data/a1-disc;game-data/a1-update-0133;local-analysis/d133-edat-overlay" --level-guid <guid> --level-report`: counts above; "Level reader: complete" on all three.
- `decoration_probe <snapshot> <game-dirs> 0001e786 25`: "decorations 51, sticker decals 910".
- Unit test `decorations_draw_at_owner_pos_times_offset` (synthetic; POS x offset,
  skip reasons, sticker count).
- `cargo fmt --all --check`, `cargo test -q -p lbp-preview -p lbp-level`, `cargo clippy -q
  --workspace --all-targets -- -D warnings`, `cargo build -q -p lbp-bevy-viewer -p lbp-viewer`.
- `--level-report` also reports 0 drawn decorations with neutral material groups on all
  three levels (all decoration meshes take the textured material route).
- Captures (ignored, not committed; window logged "positioned on non-primary monitor"):
  `--level-guid 0001e786 --level-focus-thing 404 --qa-capture
  local-analysis/d152-decorations-rookie.png` (panels cover most of the view) and the
  same with `--qa-hide-ui` to `local-analysis/d152-decorations-rookie-noui.png`. Visual
  check: textured red drapes sit on the wall arches and greenery on the shelf front, on
  their owner surfaces with no stray floating copies. Not compared against the game.

Not tested: other levels, the legacy `lbp-viewer` level path (not affected; Bevy only),
`--level-placement pos-only` visuals.
