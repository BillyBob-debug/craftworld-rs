# D114 Shape extrusion v1 (plan item Lx)

2026-10-08 America/Chicago. Implements the "proposed minimal first implementation" of
[d108-shape-geometry.md](d108-shape-geometry.md) section 2 as a pure, renderer-independent
module: `crates/lbp-level/src/shape_mesh.rs` (no Bevy/GPU dependency, AGENTS.md rule 5).
It reads the D107 structs (`parts::Shape`, `parts::GeneratedMesh`, `parts::Pos`).

**Label shown with every result** (`shape_mesh::PREVIEW_LABEL`): "preview approximation:
straight extrusion, no bevel, UV scale uncalibrated". Shapes stay non-Complete.

## API (for the Bevy level scene, D113)

```rust
lbp_level::shape_mesh::extrude(&Shape, &GeneratedMesh, &[f32; 16] /* Pos::world */)
    -> Result<ShapeMesh, ShapeReject>
lbp_level::shape_mesh::extrude_thing(&ThingRecord) -> Result<ShapeMesh, ShapeReject>
```

`ShapeMesh` holds world-space `positions`, `normals`, `uv0`, `indices` (u32, CCW seen from
outside), index ranges `front` / `back` / `sides`, `material` (GENERATED_MESH gfxMaterial
descriptor; `material_guid()`), `bevel_unused` (recorded, not applied), `uv_offset`,
`mirrored`, loop/rewind/duplicate counters, `local_area`, `thickness`, `world_z` and
`label`. Pass `shape_mesh::IDENTITY` as the matrix to get local-space geometry (e.g. for
caching by polygon and applying the transform on the GPU; then the renderer must handle
mirrored transforms itself). `ShapeReject::reason()` is a stable census key;
`is_selection()` separates the four selection outcomes from geometry failures.

## Rules

| # | Rule | Label | Source |
|---|---|---|---|
| X1 | Select: SHAPE + GENERATED_MESH + POS, GENERATED_MESH visible in play mode; otherwise `no_shape` / `no_generated_mesh` / `no_pos` / `edit_only`. A missing gfxMaterial is not a reject (65 such shapes are extruded, `material = None`). | STRONG_INFERENCE | D108 T4/T5 |
| X2 | `loops` are vertex counts per loop, summed with checked u64 arithmetic and required to equal the vertex count; loop 0 is the outer boundary, later loops are holes. | CONFIRMED_FROM_COMMUNITY_CODE / STRONG_INFERENCE | D108 P2/P3 |
| X3 | Winding is computed per loop (shoelace, f64) and normalised to outer CCW / holes CW in local xy. Stored winding is never trusted; same-winding holes are counted and rewound. | CONFIRMED_FROM_LOCAL_DATA | D108 P4/P5 |
| X4 | Polygon z is ignored; the slab spans local z in [-thickness, +thickness] (thickness = half depth) and the front cap faces +z. | STRONG_INFERENCE | D108 P6/D3/D4; tiling check below |
| X5 | Front cap = ear clipping of the outer loop with holes merged by bridge edges (holes by decreasing max x; bridge from the hole's rightmost vertex to the nearest +x ray hit, or to the blocking vertex with the smallest ray angle; duplicated bridge vertices pick the occurrence whose interior cone contains the hole vertex). Written here from the published method (D. Eberly, "Triangulation by Ear Clipping"); no crate added, no code copied. | Implementation choice | |
| X6 | Back cap = front triangulation reversed at -thickness. Sides = one flat-shaded quad per loop edge, normal to the right of the edge (outward for CCW outer and CW holes). | Preview choice | D108 Lx step 5 |
| X7 | World transform = `Pos::world` (column-major, translation 12..14). Positions by the matrix, normals by the inverse transpose; triangle order flipped when det(3x3) < 0. Thickness is not divided by the z-scale. | CONFIRMED_FROM_COMMUNITY_CODE (layout) / UNKNOWN (z-scale behaviour, D108 D5) | serial.rs m44 note |
| X8 | Render material = `GeneratedMesh.gfx_material` for all three face groups. `Shape.material` is the physics material and is not used; `GeneratedMesh.bevel` is recorded as `bevel_unused`. | CONFIRMED_FROM_COMMUNITY_CODE | D108 M1 |
| X9 | UV0 front/back: (world 3x3 applied to local (x, y, 0)) * K + uvOffset.xy, v flipped; sides: u = cumulative world edge length * K, v = world depth * K. K = `UV_SCALE` = 1/500 (uncalibrated). | COMMUNITY_BEHAVIOUR shape / UNKNOWN constant | D108 U2/U3 |

## Validation and budgets (explicit reject reasons)

Input: at most 4,096 vertices (`vertex_budget`) and 64 loops (`loop_budget`); `no_loops`;
`loop_sum_mismatch` (including overflow); non-finite coordinates (`non_finite`); |x|, |y|
above 1e6 (`oversized`); thickness not finite, <= 0 or above 1e5 (`bad_thickness`);
non-finite or singular matrix (`degenerate_transform`). Consecutive duplicate vertices
(and a closing duplicate) are dropped and counted; a loop with fewer than 3 left is
`loop_too_small`. A loop with |area| <= 1e-9 * extent^2 is `zero_area_loop`. Edges of one
loop that cross, touch or fold back are `self_intersecting`; edges of two loops that meet
are `loops_intersect` (D108 P3: intersecting loops may mean a cut in game; not modelled).
A hole not inside the outer loop is `hole_outside_outer`; a hole inside another hole is
`nested_holes`. Triangulation that finds no ear is `triangulation_failed`; a triangle area
sum that differs from outer minus holes by more than 1e-6 relative is `area_mismatch`.
Output is capped at 65,536 vertices (`output_budget`).

## Tests (synthetic recipes only)

`crates/lbp-level/tests/shape_mesh.rs` (10 tests): square, clockwise square, square with
hole (opposite and same winding), three holes, concave L, 40-point star with a hole,
mirrored + scaled + translated transform (and clockwise L under it), rotated transform,
closing duplicate vertex, and the degenerate set (collinear, bow tie, too few vertices,
hole outside, crossing hole, nested holes, loop sums incl. overflow, empty loops, NaN,
oversized, zero/infinite thickness, singular matrix, vertex and loop budgets).
Invariants checked on every accepted case: edge-manifold and consistently oriented after
welding equal positions (every directed edge once, its reverse once), each triangle's
geometric normal agrees with its unit vertex normals, signed volume = area * 2 * thickness
* |det| (so outward winding, also when mirrored), front-cap area = polygon area.

## Census (01.33, `shape_census` example)

Command (release, from the worktree; paths of the main checkout):
`cargo run -q --release -p lbp-level --example shape_census -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" local-analysis/d114-shape-census-0133-r2.json`.
Output files (ignored): `d114-shape-census-0133.json` (first run, no touching metric) and
`d114-shape-census-0133-r2.json` (same counts plus the touching metric).
Unit: one Thing with SHAPE data per containing level content hash. Scope: 398 level
hashes; 236 read completely by lbp-level, 142 out of band, 20 wrapper errors (no shapes
counted from those).

| | all 236 levels | tutorial_23.bin |
|---|---|---|
| Things with SHAPE | 166,801 | 36 |
| skipped: no_generated_mesh | 45,419 | 27 |
| skipped: edit_only | 96 | 0 |
| selected | 121,286 | 9 |
| **extruded OK** | **121,167** | **9** |
| rejected | 119 | 0 |
| triangles (all faces) | 4,270,716 | 456 |
| front-cap triangles | 958,543 | 105 |
| output vertices | 7,060,890 | 738 |
| mirrored (det < 0) | 21,539 | 1 |
| with holes | 6,505 | 0 |
| distinct gfx materials | 318 | 4 |

Rejects by reason (all levels): `loop_too_small` 58 (single-vertex polygons), `self_intersecting`
33, `no_loops` 26 (empty polygons), `hole_outside_outer` 2. No triangulation failures,
area mismatches, budget or transform rejects. Loops rewound to the normal form: 109,591;
same-winding holes: 5; duplicate vertices removed: 0.

**Layer tiling check (D108 D3).** For each level, the distinct world z-ranges of extruded
root shapes (no parent) with a z-aligned transform, in hundredths of a unit: 220 levels,
2,936 distinct intervals. Under half-depth (slab = 2 * thickness), 2,367 intervals have an
end that meets another interval's opposite end; under the full-depth alternative (slab =
thickness), 0 do. Neither hypothesis makes every level tile (half: 16 levels with no
partial overlap and no gap, 21,242 overlapping pairs, 21 gaps; full: 1 level, 13,595
pairs, 491 gaps) because objects placed at arbitrary z overlap the layer slabs. Result:
the touching counts support half-depth (STRONG_INFERENCE, still to confirm visually).
tutorial_23 intervals: [-510, -490], [-490, -310], [-290, -110] (thin layer at -500,
thick layers at -400 and -200; one gap where no -300 thin layer exists).

## What is approximate or not done

- No bevel profile, inset front face, slot materials 1-3, smoothing, relax or subdivision
  (D108 Ly). Hard 90-degree walls.
- UV scale K and the uvOffset meaning are uncalibrated; side UV mapping is a preview choice.
- Thickness is not compensated for z-scale (D5). PShape.color/THING_COLOR not applied here.
- Intersecting loops (game "cuts") and touching loops are rejected, not modelled.
- Bridge/ear clipping uses exact f64 predicates without robust arithmetic; very thin or
  nearly degenerate outlines could fail (0 of 121,286 selected in the census).
- Not tested: rendering (no viewer change; D113 owns the Bevy scene), visual correctness
  against the game, plans (only levels), levels the reader stops on.
