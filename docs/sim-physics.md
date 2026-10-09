# Physics sandbox, part 1 (runtime plan item Rg, phase R2)

New crate `lbp-sim` (runtime only; the parsing crates do not depend on it): a read level's
shapes as one avian2d 0.7.0 world (Bevy 0.19.1, `enhanced-determinism`, no `parallel`),
stepped in `FixedPostUpdate` at the game's 30 Hz tick, with a z-slab pair filter for
layers. Written in the cloud with synthetic shapes only; the level check needs the PC.

## What it adds

- **`lbp-level`**: `PartData::body` keeps PBody (`velocity` v3, `angular_velocity`,
  `frozen`); the read order is unchanged. `shape_mesh::outline_triangles(shape)` returns
  the validated polygon in local xy and its triangles (the same triangulator as the D114
  front cap) as an `Outline`.
- **`lbp-sim::scene`** (no Bevy): `build_scene(things)` gives bodies, colliders and a
  report (shapes, dynamic, static, not collidable, rejected by reason).
- **`lbp-sim`**: `SimPlugin` (avian2d + `SlabHooks`, `Time<Fixed>` at 30 Hz, 6 substeps,
  gravity), `ZSlab`, `ThingIndex`, `spawn_scene(commands, scene)`: one dynamic entity
  per body with its stored velocities, one collider entity per shape (child of its body,
  or a static body). A convex shape is one hull; a concave one is a compound of its
  triangles.
- **Example `level_physics`** for the PC (below).

## Rules

| Rule | Evidence | Applicability |
| --- | --- | --- |
| A Thing with BODY is a rigid body at its POS frame | INFERRED (PBody carries the velocities; runtime-evidence/physics.md A2) | LBP2 levels |
| A SHAPE joins the topmost BODY Thing on its parent chain (itself included); none = static. BODY Things under another BODY are merged and counted as `nested_bodies` | INFERRED (A1: glued pieces in one parent tree are one body). Changed from "nearest" after the first PC run (Rookie: 0 static, every body fell) | Joints not handled yet |
| A body is static when its owner Thing has WORLD, or any of its shapes uses a fixed physical material. `level_physics` marks a material fixed when its MAP path names dark matter (`dark_matter`/`darkmatter` `.mat`) | PUBLIC behaviour (dark matter is immovable, glued things stay put; physics evidence B); path match INFERRED until the PC report's `fixed_material_paths` and `shape_materials_top40` confirm it | Test `dark_matter_and_world_bodies_are_static` |
| Collision is 2D; a shape's slab is world z ± thickness; slabs that only touch do not collide | INFERRED (A3/A4) | Layer spacing UNKNOWN (check P5) |
| No COLLIDABLE_GAME flag (0x1) = no collider | COMMUNITY (ShapeFlags) | — |
| Mass = area × massDepth × density | INFERRED (A2); avian derives it from `ColliderDensity` = massDepth × density | — |
| Frame = POS translation xy and the x-axis angle; scale and mirroring stay in the points (winding fixed) | Same matrix convention as D114 | — |
| 30 ticks per second | CONFIRMED_FROM_LOCAL_DATA (X1, emulated) | — |

Placeholders (each a named constant): density 20, sliding friction 0.8, restitution
0.3 (Toolkit RMaterial defaults, used for every shape until RMaterial is read);
gravity 3000 units/s² (until the S9 jump measurement); avian length unit 100; 6 substeps
(our choice). `PBody.frozen` is kept but unused (meaning UNVERIFIED). Not modelled yet:
joints, PWorld maxVel/maxAVel clamps, materials, movers, lethal shapes.

## Verification (cloud)

- `cargo +1.99.0 clippy -p lbp-sim -p lbp-level -p lbp-runtime --all-targets -- -D warnings`: clean.
- `cargo test -p lbp-sim -p lbp-level`: pass. `lbp-sim` tests: scene rules (glued piece,
  mirrored winding, not collidable, body without POS, areas, frames); a box dropped
  onto a floor rests on top (the bounce slides it about 6 units; none with restitution
  0); a box one layer behind the floor falls through; a stack of four boxes stays put
  for 300 ticks.

Failed experiment: spawning the collider and adding its `Transform` afterwards left
avian's initial `Position` at the origin (the box landed on a floor 50 units too high).
The placement now goes in the spawn bundle.

## PC check

```
cargo run -q -p lbp-sim --example level_physics -- <snapshot> "<game dirs>" <Rookie Test level path> local-analysis/<new>.json --ticks=90
```

(Snapshot and game dirs as for `level_scripts`.) Report the summary line: shapes (dynamic, static, not
collidable, rejected by reason), and after 90 ticks the bodies moved over 1 and over 100
units, non-finite positions and the largest move. Success for R2 part 1 = a story level
with few large moves; each large move says which rule is wrong (glue, slabs, mass).
Then `--all`.

Counts: +0 newly viewable, +0 newly Complete (no asset change).

## Next

- PJoint mapping (bolt, motor, piston, winch, rod, string, springs) from the table in
  docs/runtime-research.md §4.
- RMaterial reads (density, friction, restitution, collide masks, gravity multiplier).
- Dev "play" toggle in the level scene of the viewer.
