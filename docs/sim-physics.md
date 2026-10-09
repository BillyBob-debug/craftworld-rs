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
gravity 3000 units/s² (until the S9 jump measurement); 6 substeps (our choice). The
solver works in units of 100 world units (`UNIT_SCALE` 0.01, part 2); read positions
back with `to_level`. `PBody.frozen` is kept but unused (meaning UNVERIFIED). Not modelled yet:
PWorld maxVel/maxAVel clamps, materials, movers, lethal shapes.

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

## Part 2: joints (2026-10-09)

`lbp-level` keeps the PJoint drive fields as `Joint::params` (`JointParams`: offsetTime,
invertAngle, strength, stiff, slideDir, animationPattern, the ten motion floats, tweak
target min/max length); the read order is unchanged. `build_scene` now adds a
`SceneJoint` per PJoint whose ends resolve to scene entities: each end Thing belongs to
its body (nearest BODY on the parent chain) or is a static shape. Contact points are in
the end Thing's POS frame. Rejected joints are counted by reason (legacy or unknown
type, missing endpoint, endpoint without a collider, both ends on one body, both ends
static, endpoint without POS). `spawn_scene` returns `Spawned { bodies, colliders,
joints }`.

| PJoint type | avian2d | Notes |
| --- | --- | --- |
| BOLT 7 | `RevoluteJoint` at a's contact | |
| MOTOR 9 | revolute + velocity motor, target = motion speed × `MOTOR_SPEED_SCALE` | speed unit UNVERIFIED (PLACEHOLDER scale 1 = rad/s) |
| SPRING_ANGULAR 8, QUANTIZED 10 | revolute + spring motor to the stored pose | `SPRING_FREQUENCY` 2 Hz, damping 0.5: PLACEHOLDER; patterns not animated |
| PISTON 4 | `PrismaticJoint` along the contact line, travel = tweak target min..max (else the rest length) | not animated |
| ROD 6, CHAIN 3 | `DistanceJoint` min = max = length | winch not animated |
| STRING 5 | `DistanceJoint` 0..length | |
| ELASTIC 1 | `DistanceJoint` 0..length, compliant | `SPRING_COMPLIANCE`: PLACEHOLDER |
| SPRING 2 | `DistanceJoint` length, compliant | same |
| LEGACY 0 | not made | counted |

Rest length = PJoint.length when positive and finite, else the contact distance
(INFERRED). Jointed pieces do not collide with each other (INFERRED). Angular bases are
global at spawn, so a spring's rest angle is the stored pose. Motors use avian's
`AccelerationBased` model with `MOTOR_DAMPING` 0.5 (the share of the speed error
removed per substep; avian adds it unscaled, so above 1 it overshoots). Motor strength
is not modelled.

Failed experiment: in world units a 100-unit box has an angular inertia near 1e8, and
avian skips a motor whose inverse inertia is below f32 epsilon; the wheel never turned.
Everything spawned is now scaled by `UNIT_SCALE` (0.01). A first motor gain of 20 spun
the wheel at -132 rad/s; 0.5 holds the target.

Tests: `joint_scene_rules` (endpoints, world and local anchors on a turned body, rest
length, same-body and legacy rejections); `rod_pendulum_keeps_its_length`;
`string_stops_a_falling_box_at_its_length`; `motor_turns_a_bolted_wheel_in_place`
(3 rad/s, centre within 2 units); `piston_slides_to_its_limit_and_holds_its_line`.
`cargo +1.99.0 clippy -p lbp-sim -p lbp-level -p lbp-preview --all-targets -- -D warnings`
clean; `cargo test -p lbp-level -p lbp-sim -p lbp-preview` pass.

PC check: `level_physics` now also reports joints and rejected joints by reason; same
command as part 1. A level with bolted wheels or pistons shows whether the frames and
endpoints resolve (rejections) and whether jointed bodies stay put (moves).

## Next

- Joint animation (motion patterns, winch and piston targets), strength and breaking.
- RMaterial reads (density, friction, restitution, collide masks, gravity multiplier).
- Dev "play" toggle in the level scene of the viewer.
