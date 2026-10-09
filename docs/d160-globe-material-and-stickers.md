# D160: Pod globe sea material and sticker decals

Date: 2026-10-08 (America/Chicago). Two parts, committed separately.

## Part 1: the Pod globe's grey sea

### Finding

New read-only probe `crates/lbp-preview/examples/mesh_group_probe.rs` (prints each
primitive's material GUID, index range and preview outcome; for neutral groups also the
material's texture slots, boxes and wires as numbers) on `cloth_earth` 000027bd:

| Prim | Material | Indices | Before | After |
|---|---|---|---|---|
| 0, 2 | 0001f14e, 0001d256 | 30,507 / 6,794 | texture x11 | unchanged |
| 1, 3 | 0001d254 | 12,050 / 4,766 | independent two-texture sampling (x11 / x4.4) | unchanged |
| 4-6, 8 | 000204ed..f0 | 6,351 total | texture (1, 2) | unchanged |
| **7** | **0001d257** | **4,832** | **neutral: "Unsupported: texture product pixel budget"** | **independent two-texture sampling (x12 / identity)** |

Material 0001d257 graph (numbers from the probe): TEX slot 0 (scale 12, 12) and TEX slot 1
(scale 1, 1) into MULTIPLY (type 10), MULTIPLY into output port 0 (diffuse); TEX slot 2
(scale 12) into output port 3 (not diffuse; stays unrendered and listed). All slots Repeat.

Both diffuse samples have integer scales, so the D50 nearest-grid product was selected;
its LCM grid is 512 x 12 = 6,144 texels wide, over the 4,096 per-axis / pixel budget, and
the route failed instead of falling back. Non-integer pairs (prim 1, x4.4) already used
the independent dual path (D50/D99).

### Rule and change

`lbp-preview` `linked.rs`: a **plain** two-texture product (no MIX, BLEND, ADD or
constant BLEND) whose grid bake would exceed the product grid or pixel budget now uses the
independent dual path (both textures sampled with their own transform on the GPU and
multiplied), the same output function as the grid bake per texel; only filtering differs
(linear rather than nearest-grid). Malformed extents still fail; MIX/BLEND/ADD products
still require the grid (unchanged "requires Repeat grid" errors).

| Rule | Evidence label | Source |
|---|---|---|
| MULTIPLY of two TEX samples is the per-texel product | CONFIRMED_FROM_COMMUNITY_CODE (existing D50 rule) | Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, material box types as recorded in docs/dual-preview.md; no code copied |
| Over-budget grid -> dual sampling is display-equivalent up to filtering | STRONG_INFERENCE | product is commutative per texel; both samplers Repeat |

Unit test `product_grid_over_budget_falls_back_only_on_budget` (synthetic extents: 512 x 12
vs 512 x 1 falls back; x2 does not; malformed extent and x4.4 do not).

### Counts (unit: primitive groups / meshes; scope: mesh 000027bd and level 00018fdb)

- 000027bd: neutral groups 1 -> 0 (9 of 9 groups select a diffuse route).
- Newly viewable: 0 meshes (000027bd already drew since D156, with one grey group).
- Newly Complete: 0. 000027bd still has "Other required texture slots unrendered"
  (non-diffuse channels such as port 3), so it cannot be called Complete.
- Corpus effect not measured (no census rerun); other meshes whose plain integer product
  exceeded the budget would also move from neutral to the dual route.

Capture: `local-analysis/d164-pod-menu.png` (`--level-guid 00018fdb --pod-camera 1
--pod-menu PodStateMainMenu --qa-hide-ui`; log "Preview window positioned on non-primary
monitor 214v0 before showing"; "Drawn with neutral (unsupported material) groups: 0
Things"). Visual check: the globe's oceans now show a blue woven fabric with the land
patches, no grey band. Not compared against the game.

## Part 2: sticker decals projected onto shapes

### What changed

- `lbp-level` `parts.rs`: `Decal` keeps every serialized field; `PartData::stickers` holds
  the main STICKERS array (costume arrays stay counted in `sticker_decals` only).
- `lbp-level` `shape_mesh.rs`: `front_face` returns a drawn shape's validated polygon,
  triangles, front-cap z (+thickness) and POS matrix (same selection as `extrude_thing`).
- `lbp-level` `decal_mesh.rs` (new): `project` maps a decal into the owner's local frame
  (below), clips every front-cap triangle to the decal parallelogram (Sutherland-Hodgman
  against the unit square in decal space), lifts it 0.5 local units above the cap (+0.02
  per later decal of the same owner, so later decals draw on top) and returns world
  positions, decal texture coordinates 0..1 and indices. Rejects with reasons:
  non-finite, degenerate axes/scale, off the owner's face, output budget.
- `lbp-preview` `level_scene.rs`: `add_sticker_decals` (runs after shapes in
  `load_level_scene_with_options`) projects decals, merges them per owner Thing and
  decal texture into `ScenePayload::Decals` (one scene item each, so
  `--level-focus-thing` on the owner frames them), and decodes each texture once as a
  plain clamped texture material. `SceneCounts` replaces `sticker_decals_not_drawn` with
  `sticker_decals_total` / `_drawn` / `_skipped` (by reason); the panel and
  `--level-report` print "Sticker decals drawn: N / M" plus one line per reason.
- `lbp-bevy-viewer`: decal draws use alpha blending and a depth bias; geometry and
  textures go through the existing bucket/upload path.
- Read-only probe `crates/lbp-preview/examples/sticker_probe.rs` (owner kinds, per-owner
  numbers, frame-candidate check).

### Decal frame (inferred)

| Rule | Evidence label | Source |
|---|---|---|
| Decal serialization order (texture, u, v, xvecu, xvecv, yvecu, yvecv, color i16, type i8, metadataIndex i16, numMetadata i16 for head <= 0x3ba, placedBy i16, playModeFrame i32, scorchMark bool, plan) | CONFIRMED_FROM_COMMUNITY_CODE (in the `parts.rs` header since D152) | Toolkit pin d6a2ef80b5a09999f08f23a803b516096055e5c9, `Decal` serializer (field order only) |
| Quad = centre -/+ xvec -/+ yvec; texture (0,0) at centre - xvec - yvec, (1,0) at + xvec - yvec, (1,1) at + xvec + yvec | COMMUNITY_BEHAVIOUR | Toolkit pin, `cwlib-gl` `DecalBaker` vertex layout (read for behaviour; no code copied). Toolkit bakes decals only for RENDER_MESH owners into the mesh's last UV set and subtracts about 0.0044 from v (not applied here) |
| SHAPE owner: `u = x * abs(sx) / 500`, `v = 1 - y * abs(sy) / 500` (x, y local polygon coordinates; sx, sy POS column lengths; 500 = D108 planar constant); uvOffset not involved | STRONG_INFERENCE (local data) | `sticker_probe` on the three levels: decal centres inside the owner's local bbox (+5%) under `v = 1 - y`: 502/641, 355/374, 83/85; under `v = 1 + y`: 339/240/59; under `v = -y`: 228/132/28. Worked cases: Rookie Thing 34 (one decal spans the shape: u -0.66..0.64 vs x*sx/500 -0.60..0.65) and Things 43-50 (pieces sharing one POS frame and one 11-decal list; union u -1.97..0.31 vs -1.88..0.31, v 0.61..2.89 vs 0.61..2.88). Misses are mostly decals of a list shared by several pieces that lie on another piece |
| Decals clip to the owner's front cap | Preview choice | Shared lists then draw each decal only on the piece(s) it covers |

Not applied (explicit): colour (0xad55 on most decals, about 0.68 grey if read as RGB565;
meaning unverified), type, scorch marks, placedBy / playModeFrame, eyetoy data, the
Toolkit v offset, bevels (`--shape-bevels` moves the cap; decals stay at the straight
cap), filtering fidelity.

### Counts (unit: STICKERS decals, main plus costume arrays; scope: one level each, 01.33 + EDAT overlay; baseline D152: 0 drawn)

| Level | GUID | Drawn / total | Not drawn by reason |
|---|---|---|---|
| Rookie Test | 0001e786 | 550 / 910 | 256 owner has no surface (microchip / switch / camera Things), 84 outside the owner's front face, 13 owner is a mesh, 7 owner shape not drawn (no_generated_mesh) |
| Grab and Swing | 0001e793 | 368 / 444 | 56 no surface, 14 owner is a mesh, 6 outside the face |
| Hedge Hopping | 0001e7f4 | 85 / 118 | 22 no surface, 11 owner is a mesh |

Unsupported cases: decals on RENDER_MESH owners (38 across the three levels; would need
projection in mesh UV space), decals on logic Things (334; microchip / switch boards, not
level surfaces), costume decal arrays (0 here), shapes that are not drawn.

Newly viewable: 1,003 sticker decals drawn across the three levels (0 before). Newly
Complete: 0 claimed; the frame is inferred, colour is not applied and nothing was
compared with the game.

### Verification (part 2)

- Unit tests: `decal_mesh::tests` (centre/axes/corner mapping and texture area 1; clipping
  at a face edge; off-face, degenerate, non-finite rejects) and
  `level_scene::tests::sticker_decals_project_per_texture_and_count_reasons` (texture
  failure, off-face, costume, edit-only owner, logic owner, report line).
- `--level-report` on the three levels: counts above; "Level reader: complete".
- Capture `local-analysis/d164-rookie-stickers.png` (`--level-guid 0001e786
  --level-focus-thing 50 --qa-hide-ui`; log "Preview window positioned on non-primary
  monitor 213v0 before showing"). Visual check: sticker pictures (a rat with cheese, a
  scarfed character, a circuit board, round labels) sit upright on shape fronts with
  transparent surroundings. Some large black silhouettes in the same view were not
  identified (decal with opaque black pixels, or shape material); not resolved.

## Checks (both parts)

`cargo fmt --all --check` (ok); `cargo test -q -p lbp-level -p lbp-preview -p
lbp-bevy-viewer` (ok); `cargo clippy -q --workspace --all-targets -- -D warnings` (ok);
`cargo build -q -p lbp-bevy-viewer -p lbp-viewer` (ok). Python tests not run (no Python
changes). Not tested: other levels, legacy `lbp-viewer` (decals are Bevy only), the
corpus census after part 1.
