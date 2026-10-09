//! R2 physics sandbox (docs/runtime-research.md §4 and §5; docs/sim-physics.md): a read
//! level's shapes as one avian2d world stepped at the game's 30 Hz tick, with a z-slab
//! pair filter standing in for layers. Runtime only; the parsing crates never depend on it.
//!
//! - [`scene`] (no Bevy) decides bodies, colliders, frames, triangles and slabs.
//! - [`SimPlugin`] adds avian2d with [`SlabHooks`], the fixed tick, substeps and gravity.
//! - [`spawn_scene`] spawns one entity per body and one collider entity per shape.
//!
//! Constants marked PLACEHOLDER wait on RPCS3 measurements or RMaterial reads.
use avian2d::math::Vector;
use avian2d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub mod scene;

pub use scene::{build_scene, PhysicsScene};

/// Game ticks per second (X1, CONFIRMED_FROM_LOCAL_DATA, emulated; docs/runtime-research.md).
pub const TICK_HZ: f64 = 30.0;
/// Solver substeps per tick: our choice (the game's solver is not public).
pub const SUBSTEPS: u32 = 6;
/// Gravity in world units per second² along -y. PLACEHOLDER until the S9 jump measurement.
pub const GRAVITY: f32 = 3000.0;
/// avian2d length unit (scales its tolerances). PLACEHOLDER: a guess at a "typical object"
/// size in LBP world units.
pub const LENGTH_UNIT: f32 = 100.0;

/// Thing index of the entity in the level.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThingIndex(pub usize);

/// A collider's layer slab: world z centre and half depth.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ZSlab {
    pub z: f32,
    pub half_thickness: f32,
}

impl ZSlab {
    /// Slabs collide when their depth ranges overlap (touching is not overlapping).
    pub fn overlaps(&self, other: &ZSlab) -> bool {
        (self.z - other.z).abs() < self.half_thickness + other.half_thickness
    }
}

/// Collision hooks: shapes in non-overlapping slabs never touch.
#[derive(SystemParam)]
pub struct SlabHooks<'w, 's> {
    slabs: Query<'w, 's, &'static ZSlab>,
}

impl CollisionHooks for SlabHooks<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _: &mut Commands) -> bool {
        match self.slabs.get_many([collider1, collider2]) {
            Ok([a, b]) => a.overlaps(b),
            Err(_) => true,
        }
    }
}

/// avian2d with [`SlabHooks`], stepped in `FixedPostUpdate` at [`TICK_HZ`].
pub struct SimPlugin {
    pub gravity: f32,
}

impl Default for SimPlugin {
    fn default() -> Self {
        Self { gravity: GRAVITY }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            PhysicsPlugins::default()
                .with_length_unit(LENGTH_UNIT)
                .with_collision_hooks::<SlabHooks>(),
        )
        .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
        .insert_resource(SubstepCount(SUBSTEPS))
        .insert_resource(Gravity(Vector::new(0., -self.gravity)));
    }
}

fn transform(frame: &scene::Frame) -> Transform {
    Transform::from_xyz(frame.position[0], frame.position[1], 0.)
        .with_rotation(Quat::from_rotation_z(frame.angle))
}

/// One convex hull when the shape is convex (hull area equals the triangle area), else a
/// compound of its triangles. A single hull has no inner edges for contacts to catch on.
fn collider(c: &scene::SceneCollider) -> Collider {
    let area: f32 = c.triangles.iter().map(scene::area).sum();
    let points: Vec<Vector> = c
        .triangles
        .iter()
        .flatten()
        .map(|p| Vector::from(*p))
        .collect();
    if let Some(hull) = Collider::convex_hull(points) {
        let hull_area = hull.mass_properties(1.).mass;
        if (hull_area - area).abs() <= 1e-3 * area {
            return hull;
        }
    }
    Collider::compound(
        c.triangles
            .iter()
            .map(|[a, b, t]| {
                (
                    Position::default(),
                    Rotation::default(),
                    Collider::triangle(Vector::from(*a), Vector::from(*b), Vector::from(*t)),
                )
            })
            .collect(),
    )
}

/// Spawns the scene: a dynamic body per [`scene::SceneBody`] with its stored velocities,
/// a collider entity per shape (child of its body, or a static body of its own). Returns
/// the body entities in scene order.
pub fn spawn_scene(commands: &mut Commands, scene: &PhysicsScene) -> Vec<Entity> {
    let bodies: Vec<Entity> = scene
        .bodies
        .iter()
        .map(|b| {
            commands
                .spawn((
                    ThingIndex(b.thing),
                    RigidBody::Dynamic,
                    transform(&b.frame),
                    LinearVelocity(Vector::from(b.velocity)),
                    AngularVelocity(b.angular_velocity),
                ))
                .id()
        })
        .collect();
    for c in &scene.colliders {
        let shape = (
            ThingIndex(c.thing),
            collider(c),
            ColliderDensity(c.density),
            Friction::new(scene::FRICTION),
            Restitution::new(scene::RESTITUTION),
            ZSlab {
                z: c.z,
                half_thickness: c.half_thickness,
            },
            ActiveCollisionHooks::FILTER_PAIRS,
        );
        // Placement goes in the spawn bundle: avian takes the initial position from it.
        match c.body {
            Some(b) => commands.spawn((shape, Transform::IDENTITY, ChildOf(bodies[b]))),
            None => commands.spawn((shape, RigidBody::Static, transform(&c.frame))),
        };
    }
    bodies
}
