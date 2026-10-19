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

pub mod character;
pub mod sackboy;
pub mod sackboy_anim;
pub mod scene;

pub use avian2d;
pub use scene::{build_scene, build_scene_with, build_spawn_scene_with, PhysicsScene};

/// Game ticks per second (X1, CONFIRMED_FROM_LOCAL_DATA, emulated; docs/runtime-research.md).
pub const TICK_HZ: f64 = 30.0;
/// Solver substeps per tick: our choice (the game's solver is not public).
pub const SUBSTEPS: u32 = 6;
/// Gravity in world units per second² along -y: 2,424 (2.693 per tick²). S9 jump
/// measurement in RPCS3 (Rookie Test, three runs, Sackboy's descent fits one parabola with
/// error under 0.1%): CONFIRMED_FROM_LOCAL_DATA (emulated) for Sackboy falling; using it for
/// every body is INFERRED (material gravity multipliers are not read yet).
pub const GRAVITY: f32 = 2424.0;
/// Physics units per LBP world unit: the solver works in "metres" of 100 world units
/// (our choice). In world units a 100-unit box has an angular inertia near 1e8, and
/// avian skips motors whose inverse inertia is below f32 epsilon. Everything spawned is
/// scaled by this; read positions back with [`to_level`].
pub const UNIT_SCALE: f32 = 0.01;

/// D330: the world's gravity per tick² at the normal gravity setting (1.0), the value
/// [`gravity_per_tick`] gives there. FOUND (01.33): the game's gravity-ratio getter (0xce6f0)
/// divides the world gravity by -2.7.
pub const NORMAL_GRAVITY_PER_TICK: f32 = 2.7;

/// D330: the world's gravity per tick² (downwards) for the level's gravity global setting
/// (`Game.SetGlobalSettingsGravityTo`, the PWorld's blended type-1 value): `2.3 x setting +
/// 0.4`, so 2.7 at the normal 1.0 and 0.4 at 0.0. FOUND (01.33): the gravity setter (0xcef3c)
/// and the blend-factor setter (0xd623c) write `setting x -2.3 - 0.4` into the y lane of the
/// physics world's gravity vector each time they run, with the setting as now blended; the
/// world gravity native (0x5fd4c8) reads that vector's length.
pub fn gravity_per_tick(setting: f32) -> f32 {
    2.3 * setting + 0.4
}

/// D330: the level's gravity global setting in force, which scales the world's gravity
/// ([`SimPlugin`]) and Sackboy's ([`character`]) against the normal setting. A display that
/// runs the level's scripts writes it; without them it stays at the normal 1.0.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WorldGravity {
    pub setting: f32,
}

impl Default for WorldGravity {
    fn default() -> Self {
        Self { setting: 1.0 }
    }
}

impl WorldGravity {
    /// The world's gravity over the normal one: 1.0 at the normal setting. A setting that is
    /// not a number counts as the normal one (our choice: the game would carry the NaN).
    pub fn scale(&self) -> f32 {
        if self.setting.is_finite() {
            gravity_per_tick(self.setting) / NORMAL_GRAVITY_PER_TICK
        } else {
            1.0
        }
    }
}

/// A physics position in LBP world units.
pub fn to_level(p: Vector) -> [f32; 2] {
    [p.x / UNIT_SCALE, p.y / UNIT_SCALE]
}

fn sv(p: [f32; 2]) -> Vector {
    Vector::new(p[0] * UNIT_SCALE, p[1] * UNIT_SCALE)
}

/// Joint motors: share of the speed error removed per substep (avian `AccelerationBased`
/// damping; above 1 it overshoots). Our choice; the motor strength is not modelled.
pub const MOTOR_DAMPING: f32 = 0.5;
/// Angular springs (SPRING_ANGULAR, wobble): avian spring-damper. PLACEHOLDER until
/// PJoint.strength is calibrated.
pub const SPRING_FREQUENCY: f32 = 2.0;
pub const SPRING_DAMPING_RATIO: f32 = 0.5;
/// ELASTIC and SPRING joint compliance (inverse stiffness). PLACEHOLDER.
pub const SPRING_COMPLIANCE: f32 = 1e-4;

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
        let base = self.gravity;
        // D330: the world's gravity follows the level's gravity setting.
        let follow = move |g: Res<WorldGravity>, mut gravity: ResMut<Gravity>| {
            if g.is_changed() {
                gravity.0 = Vector::new(0., -base * g.scale() * UNIT_SCALE);
            }
        };
        app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<SlabHooks>())
            .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
            .insert_resource(SubstepCount(SUBSTEPS))
            .insert_resource(Gravity(Vector::new(0., -self.gravity * UNIT_SCALE)))
            .init_resource::<WorldGravity>()
            .add_systems(FixedPreUpdate, follow)
            .add_systems(
                FixedPostUpdate,
                put_to_sleep
                    .after(PhysicsSystems::StepSimulation)
                    .before(PhysicsSystems::Writeback),
            );
    }
}

/// D1101: a body's anti-gravity tweakers, for [`follow_tweaks`].
#[derive(Component, Debug, Clone)]
pub struct AntiGravityTweaks(pub scene::AntiGravity);

/// D1101: sets each tweaked body's gravity scale from the tweaks' current activations and
/// wakes a body whose scale changed (a sleeping body would not feel the new gravity).
pub fn follow_tweaks(
    commands: &mut Commands,
    q: &mut Query<(Entity, &AntiGravityTweaks, &mut GravityScale)>,
    activation: impl Fn(&scene::AntiGravityTweak) -> f32,
) {
    for (e, tweaks, mut scale) in q.iter_mut() {
        let s = tweaks.0.gravity_scale(&activation);
        if s.to_bits() != scale.0.to_bits() {
            scale.0 = s;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1109: a body's gyroscopes and rotators, for [`drive_spins`].
#[derive(Component, Debug, Clone)]
pub struct SpinTweaks(pub Vec<scene::SpinTweak>);

/// D1109: each gyroscope and rotator moves its body's turn speed towards its target
/// ([`scene::SpinTweak::turn`], per tick, in order) with the tweak's current activation,
/// and wakes a body whose speed changed.
pub fn drive_spins(
    commands: &mut Commands,
    q: &mut Query<(Entity, &SpinTweaks, &Rotation, &mut AngularVelocity)>,
    activation: impl Fn(&scene::SpinTweak) -> f32,
) {
    let hz = TICK_HZ as f32;
    for (e, tweaks, rot, mut w) in q.iter_mut() {
        let angle = rot.as_radians();
        let before = w.0 / hz;
        let after = tweaks
            .0
            .iter()
            .fold(before, |v, t| t.turn(angle, v, activation(t)));
        if (after - before).abs() > 1e-6 {
            w.0 = after * hz;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1110: a body's movers and followers, for [`drive_moves`].
#[derive(Component, Debug, Clone)]
pub struct MoveTweaks(pub Vec<scene::MoveTweak>);

/// D1110: each mover and follower moves its body's velocity towards its rows
/// ([`scene::MoveTweak::push`], per tick, in order) with the tweak's current activation; a
/// follower (configuration 11 without followKey) follows `player` when it sees it. Wakes a
/// body whose velocity changed.
pub fn drive_moves(
    commands: &mut Commands,
    q: &mut Query<(
        Entity,
        &MoveTweaks,
        &Position,
        &Rotation,
        &mut LinearVelocity,
    )>,
    player: Option<Vec2>,
    activation: impl Fn(&scene::MoveTweak) -> f32,
) {
    // The rows are in game units per tick; the bodies move in sim units per second.
    let hz = TICK_HZ as f32;
    let per_tick = 1. / (hz * UNIT_SCALE);
    for (e, tweaks, pos, rot, mut v) in q.iter_mut() {
        let up = [-rot.sin, rot.cos];
        let before = [v.0.x * per_tick, v.0.y * per_tick];
        let after = tweaks.0.iter().fold(before, |vel, t| {
            let d = player
                .map(|p| [(p.x - pos.0.x) / UNIT_SCALE, (p.y - pos.0.y) / UNIT_SCALE])
                .filter(|&d| t.configuration == 11 && t.sees(d, up));
            t.push(vel, up, d, activation(t))
        });
        if (after[0] - before[0]).abs() + (after[1] - before[1]).abs() > 1e-4 {
            v.0 = Vec2::new(after[0], after[1]) / per_tick;
            commands.queue(avian2d::dynamics::solver::islands::WakeBody(e));
        }
    }
}

/// D1102: a piston joint, kept so its length can follow the game's piston rule, with its
/// clock (D1103).
#[derive(Component, Debug, Clone)]
pub struct PistonDrive {
    pub joint: scene::SceneJoint,
    pub clock: scene::PistonClock,
}

impl PistonDrive {
    /// D1103: at tick `now` sets the clock rate, 1 with no input, else from the `input`
    /// signal (ternary, activation) by mode ([`scene::piston_scale`], D1104; mode 4
    /// [`scene::SceneJoint::servo_scale`], D1106), and returns the length (in sim units)
    /// from the clock.
    pub fn step(&mut self, now: f32, input: Option<(i32, f32)>) -> f32 {
        let p = self.joint.params;
        let s = match input {
            None => 1.,
            Some(_) if p.held => 0.,
            Some((_, a)) if p.mode == 4 => {
                let l = self.joint.length(self.clock.frame(now), true);
                self.joint.servo_scale(a, l)
            }
            Some((t, a)) => scene::piston_scale(p.mode, p.held, t, a),
        };
        self.clock.set_scale(now, s);
        self.joint.length(self.clock.frame(now), input.is_some()) * UNIT_SCALE
    }

    /// D1108: steps the clock as [`Self::step`] does and returns the motor's turn rate in
    /// rad/s: the change of its angle ([`scene::SceneJoint::motor_angle`]) over the next
    /// frame of its clock, as the motor row builder 0x95f494 takes it (0xc99d0 at frame and
    /// frame + 1), times [`TICK_HZ`] (one game frame per tick). Pattern 1 gives speed (+0xd4,
    /// rad per frame) × clock rate, as 0xc7370 returns.
    pub fn turn(&mut self, now: f32, input: Option<(i32, f32)>) -> f32 {
        self.step(now, input);
        let wired = input.is_some();
        let f = self.clock.frame(now);
        let a = |f| self.joint.motor_angle(f, wired);
        (a(f + self.clock.scale) - a(f)) * TICK_HZ as f32
    }
}

/// D1103: at tick `now` steps each piston ([`PistonDrive::step`]) with the `signal` on its
/// input and holds its slider at the length, waking both ends when it changed.
pub fn drive_pistons(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut PrismaticJoint)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        let l = drive.step(now, input);
        if joint.limits.is_some_and(|d| d.min == l && d.max == l) {
            continue;
        }
        joint.limits = Some(DistanceLimit::new(l, l));
        wake_ends(commands, joint.body1, joint.body2);
    }
}

/// D1107: winches (CHAIN, type 3) run the piston's clock and length rule (0xc95ec sends
/// types 3, 4 and 9 to 0xc9790, the solver gives types 3 and 4 the same limit rows), so
/// at tick `now` each one is stepped like a piston and its rope held to at most the length.
/// The solver builds the winch row (0x960988) with a flag (set at 0x1c9964) that the piston
/// and rod rows clear; INFERRED from that, the rope only pulls and goes slack when shorter.
pub fn drive_winches(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut DistanceJoint)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        let l = drive.step(now, input);
        if joint.limits.min == 0. && joint.limits.max == l {
            continue;
        }
        joint.limits = DistanceLimit::new(0., l);
        wake_ends(commands, joint.body1, joint.body2);
    }
}

/// D1108: at tick `now` sets each motor's (MOTOR, type 9) target turn rate from its clock
/// and angle rule ([`PistonDrive::turn`]), waking both ends when it changed. INFERRED: the
/// row's position term (error to the measure 0xc9250) and its strength cap are left out.
pub fn drive_motors(
    commands: &mut Commands,
    q: &mut Query<(&mut PistonDrive, &mut RevoluteJoint)>,
    now: f32,
    signal: impl Fn(&scene::SceneJoint) -> Option<(i32, f32)>,
) {
    for (mut drive, mut joint) in q.iter_mut() {
        let input = signal(&drive.joint);
        let w = drive.turn(now, input);
        if joint.motor.target_velocity == w {
            continue;
        }
        joint.motor.target_velocity = w;
        wake_ends(commands, joint.body1, joint.body2);
    }
}

fn wake_ends(commands: &mut Commands, a: Entity, b: Entity) {
    for e in [a, b] {
        commands.queue(move |world: &mut World| wake_if_island(world, e));
    }
}

/// Wakes a body's island; a static anchor (no island) is left alone.
fn wake_if_island(world: &mut World, e: Entity) {
    use avian2d::dynamics::solver::islands::{BodyIslandNode, WakeBody};
    if world.get::<BodyIslandNode>(e).is_some() {
        let _ = WakeBody(e).apply(world);
    }
}

/// A body that starts asleep (PBody.frozen != 0, INFERRED: a saved level is at rest). avian
/// builds islands during the first step, so after it the body goes back to its stored pose
/// with no velocity and its island is put to sleep; contacts and joints wake it.
#[derive(Component, Debug, Clone, Copy)]
pub struct StartAsleep {
    position: Vector,
    angle: f32,
}

fn put_to_sleep(
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &StartAsleep,
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    for (e, start, mut p, mut r, mut v, mut w) in &mut q {
        // Without change detection: a changed pose or velocity would wake the island again.
        p.bypass_change_detection().0 = start.position;
        *r.bypass_change_detection() = Rotation::radians(start.angle);
        v.bypass_change_detection().0 = Vector::ZERO;
        w.bypass_change_detection().0 = 0.;
        commands.entity(e).remove::<StartAsleep>();
        commands.queue(avian2d::dynamics::solver::islands::SleepBody(e));
    }
}

fn transform(frame: &scene::Frame) -> Transform {
    let p = sv(frame.position);
    Transform::from_xyz(p.x, p.y, 0.).with_rotation(Quat::from_rotation_z(frame.angle))
}

/// One convex hull when the shape is convex (hull area equals the triangle area), else a
/// compound of its triangles. A single hull has no inner edges for contacts to catch on.
fn collider(c: &scene::SceneCollider) -> Collider {
    let area: f32 = c.triangles.iter().map(scene::area).sum::<f32>() * UNIT_SCALE * UNIT_SCALE;
    let points: Vec<Vector> = c.triangles.iter().flatten().map(|p| sv(*p)).collect();
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
                    Collider::triangle(sv(*a), sv(*b), sv(*t)),
                )
            })
            .collect(),
    )
}

/// Entities [`spawn_scene`] made, in scene order.
#[derive(Debug, Clone, Default)]
pub struct Spawned {
    pub bodies: Vec<Entity>,
    pub colliders: Vec<Entity>,
    pub joints: Vec<Entity>,
}

/// Spawns the scene: a dynamic body per [`scene::SceneBody`] with its stored velocities,
/// a collider entity per shape (child of its body, or a static body of its own), and a
/// joint entity per [`scene::SceneJoint`].
pub fn spawn_scene(commands: &mut Commands, scene: &PhysicsScene) -> Spawned {
    let bodies: Vec<Entity> = scene
        .bodies
        .iter()
        .map(|b| {
            let mut e = commands.spawn((
                ThingIndex(b.thing),
                if b.fixed {
                    RigidBody::Static
                } else {
                    RigidBody::Dynamic
                },
                transform(&b.frame),
                LinearVelocity(sv(b.velocity)),
                AngularVelocity(b.angular_velocity),
            ));
            if let Some(a) = &b.anti_gravity {
                // FOUND (01.33, 0xc6308): gravity scale clamp(1 - Σ, 0, 1).
                e.insert((
                    GravityScale(a.gravity_scale(|t| t.saved)),
                    AntiGravityTweaks(a.clone()),
                ));
            }
            if !b.spins.is_empty() {
                e.insert(SpinTweaks(b.spins.clone()));
            }
            if !b.moves.is_empty() {
                e.insert(MoveTweaks(b.moves.clone()));
            }
            if !b.fixed && b.frozen != 0 {
                e.insert(StartAsleep {
                    position: sv(b.frame.position),
                    angle: b.frame.angle,
                });
            }
            e.id()
        })
        .collect();
    let mut colliders = Vec::with_capacity(scene.colliders.len());
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
        colliders.push(
            match c.body {
                Some(b) => commands.spawn((shape, Transform::IDENTITY, ChildOf(bodies[b]))),
                None => commands.spawn((shape, RigidBody::Static, transform(&c.frame))),
            }
            .id(),
        );
    }
    let joints = scene
        .joints
        .iter()
        .map(|j| spawn_joint(commands, j, &bodies, &colliders))
        .collect();
    Spawned {
        bodies,
        colliders,
        joints,
    }
}

/// One joint entity. Jointed pieces do not collide with each other (INFERRED: a bolted
/// wheel overlapping its frame turns freely). Angular bases are global so the rest angle
/// is the stored pose.
fn spawn_joint(
    commands: &mut Commands,
    j: &scene::SceneJoint,
    bodies: &[Entity],
    colliders: &[Entity],
) -> Entity {
    let mut entity = |e: scene::Endpoint| match e {
        scene::Endpoint::Body(i) => bodies[i],
        scene::Endpoint::Static(i) => colliders[i],
        scene::Endpoint::Anchor { thing, frame } => commands
            .spawn((ThingIndex(thing), RigidBody::Static, transform(&frame)))
            .id(),
    };
    let (a, b) = (entity(j.a), entity(j.b));
    let v = sv;
    let pivot = || {
        RevoluteJoint::new(a, b)
            .with_anchor(v(j.world_a))
            .with_basis(Rotation::IDENTITY)
    };
    let distance = |min: f32, max: f32, compliance: f32| {
        let (min, max) = (min * UNIT_SCALE, max * UNIT_SCALE);
        DistanceJoint::new(a, b)
            .with_local_anchor1(v(j.local_a))
            .with_local_anchor2(v(j.local_b))
            .with_limits(min, max)
            .with_compliance(compliance)
    };
    let mut e = commands.spawn((ThingIndex(j.thing), JointCollisionDisabled));
    match j.kind {
        scene::JointKind::Bolt => e.insert(pivot()),
        scene::JointKind::Motor => {
            let mut drive = PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
            };
            let w = drive.turn(0., None);
            e.insert(
                pivot().with_motor(
                    AngularMotor::new(MotorModel::AccelerationBased {
                        stiffness: 0.,
                        damping: MOTOR_DAMPING,
                    })
                    .with_target_velocity(w),
                ),
            )
            .insert(drive)
        }
        scene::JointKind::SpringAngular => e.insert(
            pivot().with_motor(
                AngularMotor::new(MotorModel::SpringDamper {
                    frequency: SPRING_FREQUENCY,
                    damping_ratio: SPRING_DAMPING_RATIO,
                })
                .with_target_position(0.),
            ),
        ),
        scene::JointKind::Piston => {
            let d = [j.world_b[0] - j.world_a[0], j.world_b[1] - j.world_a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let axis = if len > 1e-3 {
                Vector::new(d[0] / len, d[1] / len)
            } else {
                Vector::Y
            };
            e.insert(
                PrismaticJoint::new(a, b)
                    .with_local_anchor1(v(j.local_a))
                    .with_local_anchor2(v(j.local_b))
                    .with_basis(Rotation::IDENTITY)
                    .with_slider_axis(axis)
                    .with_limits(j.min_length * UNIT_SCALE, j.max_length * UNIT_SCALE),
            )
            .insert(PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
            })
        }
        scene::JointKind::Rod => e.insert(distance(j.length, j.length, 0.)),
        scene::JointKind::Chain => {
            let l = j.length(0., false);
            e.insert(distance(0., l, 0.)).insert(PistonDrive {
                joint: j.clone(),
                clock: scene::PistonClock::default(),
            })
        }
        scene::JointKind::String => e.insert(distance(0., j.length, 0.)),
        scene::JointKind::Elastic => e.insert(distance(0., j.length, SPRING_COMPLIANCE)),
        scene::JointKind::Spring => e.insert(distance(j.length, j.length, SPRING_COMPLIANCE)),
    };
    e.id()
}
