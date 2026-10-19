//! Bevy-free physics description of a read level: which SHAPE Things collide, which move,
//! their 2D frames, triangles and z-slabs. [`crate::spawn_scene`] turns it into avian2d
//! entities.
//!
//! Rules (labels from docs/formats.md; evidence in docs/runtime-evidence/physics.md and
//! docs/sim-physics.md):
//! - A Thing with BODY is a rigid body at its POS frame (INFERRED: PBody holds the
//!   velocities, so it is the moving piece).
//! - A SHAPE Thing belongs to the topmost Thing with BODY on its parent chain, itself
//!   included (INFERRED, A1: pieces glued in one parent tree form one body; a BODY under
//!   another BODY is counted as `nested_bodies`). A SHAPE with no BODY on the chain is
//!   static.
//! - A body is fixed (static) when its owner is the level's WORLD Thing, or when any of its
//!   shapes uses a physical material the caller marks fixed (PUBLIC behaviour: dark matter
//!   is immovable and anything glued to it stays put; which descriptor is dark matter comes
//!   from the caller, see [`build_scene_with`]). INFERRED until a PC run confirms it.
//! - A shape whose material rule is [`MaterialRule::NoCollide`] (hologram) is not a collider
//!   (PUBLIC behaviour: hologram has no physical presence; INFERRED mapping).
//! - A body with GAMEPLAY_DATA (score bubbles, pick-ups) is fixed: it is collected through
//!   its trigger, not pushed (INFERRED; the Rookie run had 71 of them falling).
//! - A dynamic body with PBody.frozen != 0 starts asleep and wakes when touched or pulled
//!   by a joint (INFERRED: every dynamic body in the Rookie run has frozen 1, so a saved
//!   level is at rest; see `crate::spawn_scene`).
//! - Collision is 2D; a shape's slab is world z ± thickness (INFERRED, A3/A4).
//! - A shape without the COLLIDABLE_GAME flag (0x1) does not collide (COMMUNITY, ShapeFlags).
//! - PJoints join the entities their endpoint Things belong to (a body, or a static
//!   shape); contact points are in each endpoint Thing's POS frame (COMMUNITY field
//!   names; frame INFERRED). The type mapping is [`JointKind`].
//! - Mass = area × massDepth × density (INFERRED, A2); density is [`DENSITY`] until RMaterial
//!   is read (PLACEHOLDER).
use lbp_level::parts::Shape;
use lbp_level::serial::Descriptor;
use lbp_level::shape_mesh::outline_triangles_lenient;
use lbp_level::thing::ThingRecord;
use std::collections::BTreeMap;

/// RMaterial default density (Toolkit RMaterial field default, COMMUNITY). Every shape uses
/// it until physical materials are read: PLACEHOLDER.
pub const DENSITY: f32 = 20.0;
/// RMaterial default sliding friction (COMMUNITY default). PLACEHOLDER for every shape.
pub const FRICTION: f32 = 0.8;
/// RMaterial default restitution (COMMUNITY default). PLACEHOLDER for every shape.
pub const RESTITUTION: f32 = 0.3;
/// ShapeFlags.COLLIDABLE_GAME.
pub const COLLIDABLE_GAME: i16 = 0x1;
/// Parent-chain depth limit (untrusted references may cycle).
pub const MAX_PARENT_DEPTH: usize = 256;
/// Triangles below this area (world units²) after the transform are dropped.
pub const MIN_TRIANGLE_AREA: f32 = 1e-4;
/// Flat-transform samples kept per scene (diagnostics).
pub const FLAT_SAMPLES: usize = 4;

/// Signed area of a triangle (positive when counter-clockwise).
pub fn area([a, b, c]: &[[f32; 2]; 3]) -> f32 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
}

/// A 2D frame: world position and rotation about z (radians).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub position: [f32; 2],
    pub angle: f32,
}

impl Frame {
    /// Frame of a POS world matrix (column-major, translation at 12..14): translation xy
    /// and the angle of the x axis. Scale and shear stay in the collider points.
    pub fn of_matrix(m: &[f32; 16]) -> Option<Self> {
        if m.iter().any(|v| !v.is_finite()) || (m[0] == 0. && m[1] == 0.) {
            return None;
        }
        Some(Self {
            position: [m[12], m[13]],
            angle: m[1].atan2(m[0]),
        })
    }

    /// World point to this frame's local coordinates.
    pub fn to_local(&self, p: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        let d = [p[0] - self.position[0], p[1] - self.position[1]];
        [c * d[0] + s * d[1], -s * d[0] + c * d[1]]
    }
}

/// A rigid body: the BODY Thing's frame and stored velocities.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneBody {
    pub thing: usize,
    pub frame: Frame,
    pub velocity: [f32; 2],
    pub angular_velocity: f32,
    /// PBody.frozen as stored (meaning UNVERIFIED; not used yet).
    pub frozen: i32,
    /// Static body: WORLD owner or a fixed material in the glue tree (see module rules).
    pub fixed: bool,
    /// D1100: an anti-gravity tweaker glued to the body, if any.
    pub anti_gravity: Option<AntiGravity>,
}

/// D1100: the PPhysicsTweaks with configuration 0 (anti-gravity tweakers) on a body's
/// hierarchy, summed with the activation each was saved with.
#[derive(Debug, Clone, PartialEq)]
pub struct AntiGravity {
    /// Σ PPhysicsTweak.gravity × lastKnownActivation: the share of gravity cancelled.
    pub cancelled: f32,
    /// Σ dampening × activation; kept, not applied yet (the SPU side is unread).
    pub dampening: [f32; 3],
    /// D1101: each tweak, so a live switch signal can replace the saved activation.
    pub tweaks: Vec<AntiGravityTweak>,
}

/// D1101: one anti-gravity tweaker on a body's hierarchy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AntiGravityTweak {
    /// The PPhysicsTweak's Thing index.
    pub thing: usize,
    pub gravity: f32,
    /// PPhysicsTweak.behavior, read as the input mode (0 digital, 1 analog; INFERRED).
    pub behavior: i32,
    /// lastKnownActivation, the activation until a signal says otherwise.
    pub saved: f32,
}

impl AntiGravity {
    /// FOUND (01.33, 0xc6308): the body's gravity scale clamp(1 - Σ gravity × activation,
    /// 0, 1), with `activation` giving each tweak's current activation.
    pub fn gravity_scale(&self, activation: impl Fn(&AntiGravityTweak) -> f32) -> f32 {
        let cancelled: f32 = self.tweaks.iter().map(|t| t.gravity * activation(t)).sum();
        (1. - cancelled).clamp(0., 1.)
    }
}

/// One SHAPE Thing as a collider.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCollider {
    pub thing: usize,
    /// Index in [`PhysicsScene::bodies`]; None = static.
    pub body: Option<usize>,
    /// Frame the triangles are in: the body's frame, or the shape's own for a static one.
    pub frame: Frame,
    /// Counter-clockwise triangles in `frame` coordinates.
    pub triangles: Vec<[[f32; 2]; 3]>,
    /// Slab centre (world z) and half depth (PShape.thickness).
    pub z: f32,
    pub half_thickness: f32,
    /// Mass per unit area: massDepth × [`DENSITY`].
    pub density: f32,
}

/// Counts for the PC report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneReport {
    pub shapes: usize,
    pub dynamic: usize,
    pub static_: usize,
    pub not_collidable: usize,
    /// Bodies made static (counted in `static_` by shape, here by body).
    pub fixed_bodies: usize,
    /// BODY Things with another BODY above them on the parent chain (merged into it).
    pub nested_bodies: usize,
    /// Shapes left out, by reason.
    pub rejected: BTreeMap<&'static str, usize>,
    /// Shapes kept by the even-odd fill, by the reason the strict outline refused them.
    pub filled: BTreeMap<&'static str, usize>,
    /// Up to [`FLAT_SAMPLES`] flat-transform shapes: Thing and the 3x3 linear part of the
    /// world matrix (columns x, y, z).
    pub flat_samples: Vec<(usize, [f32; 9])>,
    pub joints: usize,
    /// PJoints left out, by reason.
    pub rejected_joints: BTreeMap<&'static str, usize>,
    /// Joint ends fixed in the world because their Thing has no collider.
    pub anchored_joint_ends: usize,
}

/// Material rule from a material's MAP path (INFERRED from names and public material
/// behaviour until RMaterial is read): dark matter and infinite-mass materials are fixed,
/// hologram does not collide.
pub fn material_rule_for_path(path: &str) -> MaterialRule {
    let p = path.to_ascii_lowercase().replace(' ', "_");
    if !p.ends_with(".mat") {
        MaterialRule::Normal
    } else if p.contains("dark_matter") || p.contains("darkmatter") || p.contains("infinite_mass") {
        MaterialRule::Fixed
    } else if p.contains("hologram") {
        MaterialRule::NoCollide
    } else {
        MaterialRule::Normal
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysicsScene {
    pub bodies: Vec<SceneBody>,
    pub colliders: Vec<SceneCollider>,
    pub joints: Vec<SceneJoint>,
    pub report: SceneReport,
}

impl PhysicsScene {
    /// Top of the highest collider surface at `x` that is not above `y` (plus `SNAP`), in
    /// the slab at `z`: where something dropped from `(x, y)` lands, without simulating
    /// (bodies taken where they are stored).
    pub fn floor_below(&self, [x, y]: [f32; 2], z: f32) -> Option<f32> {
        const SNAP: f32 = 1.;
        let mut best: Option<f32> = None;
        for c in &self.colliders {
            if (z - c.z).abs() > c.half_thickness {
                continue;
            }
            let (s, k) = c.frame.angle.sin_cos();
            let world = |p: [f32; 2]| {
                [
                    c.frame.position[0] + k * p[0] - s * p[1],
                    c.frame.position[1] + s * p[0] + k * p[1],
                ]
            };
            for t in &c.triangles {
                let w = t.map(world);
                let mut top: Option<f32> = None;
                for i in 0..3 {
                    let (a, b) = (w[i], w[(i + 1) % 3]);
                    let (lo, hi) = (a[0].min(b[0]), a[0].max(b[0]));
                    if x < lo || x > hi {
                        continue;
                    }
                    let at = if hi - lo < 1e-6 {
                        a[1].max(b[1])
                    } else {
                        a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0])
                    };
                    top = Some(top.map_or(at, |v: f32| v.max(at)));
                }
                if let Some(top) = top.filter(|t| *t <= y + SNAP) {
                    best = Some(best.map_or(top, |b| b.max(top)));
                }
            }
        }
        best
    }
}

/// Nearest Thing with BODY on the parent chain of `i`, itself included.
/// Topmost Thing with BODY on the parent chain of `i` (itself included), and the BODY
/// Things below it on the chain.
fn body_owner(things: &[ThingRecord], mut i: usize) -> (Option<usize>, Vec<usize>) {
    let mut found = Vec::new();
    for _ in 0..MAX_PARENT_DEPTH {
        let Some(t) = things.get(i) else {
            break;
        };
        if t.data.body.is_some() {
            found.push(i);
        }
        match t.parent {
            Some(p) if p != i => i = p,
            _ => break,
        }
    }
    let top = found.pop();
    (top, found)
}

fn is_world(t: &ThingRecord) -> bool {
    t.data.world.is_some() || t.parts.contains(&"WORLD")
}

/// Score bubbles and other pick-ups (GAMEPLAY_DATA) hang where they are placed.
fn is_pickup(t: &ThingRecord) -> bool {
    t.parts.contains(&"GAMEPLAY_DATA")
}

/// How a physical material behaves in the sandbox (the caller decides per descriptor until
/// RMaterial is read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialRule {
    #[default]
    Normal,
    /// Immovable: the whole glue tree is static (dark matter, infinite mass).
    Fixed,
    /// No physical presence: the shape is not a collider (hologram).
    NoCollide,
}

/// Builds the physics description of a level (`things[i]` must be Thing index `i`) with every
/// material normal: only WORLD-owned and pick-up bodies are static.
pub fn build_scene(things: &[ThingRecord]) -> PhysicsScene {
    build_scene_with(things, &|_| MaterialRule::Normal)
}

/// Builds the physics description of a level with a rule per physical material.
pub fn build_scene_with(
    things: &[ThingRecord],
    material_rule: &dyn Fn(&Descriptor) -> MaterialRule,
) -> PhysicsScene {
    build_scene_inner(things, material_rule, false)
}

/// As [`build_scene_with`], but shapes with vertex z under a tilting matrix are projected
/// through the whole matrix ([`tilted_outline`]). Only for locating a spawn floor
/// ([`PhysicsScene::floor_below`]): applied to every level's colliders it raised the
/// `level_physics --all` movers (5,510 -> 5,780 moved >100) and the largest move
/// (38,482 -> 442,083), so the simulated scene keeps the in-plane outline (PR #83, D212).
pub fn build_spawn_scene_with(
    things: &[ThingRecord],
    material_rule: &dyn Fn(&Descriptor) -> MaterialRule,
) -> PhysicsScene {
    build_scene_inner(things, material_rule, true)
}

fn build_scene_inner(
    things: &[ThingRecord],
    material_rule: &dyn Fn(&Descriptor) -> MaterialRule,
    project_tilted: bool,
) -> PhysicsScene {
    let mut scene = PhysicsScene::default();
    let mut body_of = BTreeMap::<usize, Option<usize>>::new();
    let mut nested = std::collections::BTreeSet::new();
    let mut fixed_shape = Vec::new();
    for (i, t) in things.iter().enumerate() {
        let Some(shape) = &t.data.shape else {
            continue;
        };
        scene.report.shapes += 1;
        let mut reject = |why| *scene.report.rejected.entry(why).or_default() += 1;
        let rule = shape
            .material
            .as_ref()
            .map_or(MaterialRule::Normal, material_rule);
        if shape.flags & COLLIDABLE_GAME == 0 || rule == MaterialRule::NoCollide {
            scene.report.not_collidable += 1;
            continue;
        }
        let Some(pos) = &t.data.pos else {
            reject("no POS");
            continue;
        };
        let Some(own) = Frame::of_matrix(&pos.world) else {
            reject("bad transform");
            continue;
        };
        let m = &pos.world;
        // A shape whose vertices carry z under a matrix that turns local z into the xy plane
        // (the Pod body, Thing 2 of shell 000027b9: local z is world up) has its outline in
        // local x/z, not x/y; the plain x/y outline is a sliver. Such a shape is projected to
        // world xy through the whole matrix first and triangulated there (spawn scene only).
        let tilted = if project_tilted {
            tilted_outline(shape, m)
        } else {
            None
        };
        let projected;
        let outline_of = match &tilted {
            Some(p) => {
                projected = p.clone();
                &projected
            }
            None => shape,
        };
        let outline = match outline_triangles_lenient(outline_of) {
            Ok((v, why)) => {
                if let Some(why) = why {
                    *scene.report.filled.entry(why.reason()).or_default() += 1;
                }
                v
            }
            Err(e) => {
                reject(e.reason());
                continue;
            }
        };
        let (owner, below) = body_owner(things, i);
        nested.extend(below);
        let body = match owner {
            None => None,
            Some(b) => {
                let slot = *body_of.entry(b).or_insert_with(|| {
                    let thing = &things[b];
                    let frame = thing
                        .data
                        .pos
                        .as_ref()
                        .and_then(|p| Frame::of_matrix(&p.world));
                    frame.zip(thing.data.body).map(|(frame, pb)| {
                        scene.bodies.push(SceneBody {
                            thing: b,
                            frame,
                            velocity: [pb.velocity[0], pb.velocity[1]],
                            angular_velocity: pb.angular_velocity,
                            frozen: pb.frozen,
                            fixed: is_world(thing) || is_pickup(thing),
                            anti_gravity: None,
                        });
                        scene.bodies.len() - 1
                    })
                });
                if slot.is_none() {
                    *scene.report.rejected.entry("body without POS").or_default() += 1;
                    continue;
                }
                slot
            }
        };
        let frame = body.map_or(own, |b| scene.bodies[b].frame);
        let world = |p: [f32; 2]| match tilted {
            Some(_) => [p[0] + m[12], p[1] + m[13]],
            None => [
                m[0] * p[0] + m[4] * p[1] + m[12],
                m[1] * p[0] + m[5] * p[1] + m[13],
            ],
        };
        let mut triangles = Vec::with_capacity(outline.triangles.len());
        for t in &outline.triangles {
            let [a, b, c] = t.map(|k| frame.to_local(world(outline.points[k])));
            let area = area(&[a, b, c]);
            if !area.is_finite() || area.abs() < MIN_TRIANGLE_AREA {
                continue;
            }
            // A mirrored transform reverses the winding.
            triangles.push(if area > 0. { [a, b, c] } else { [a, c, b] });
        }
        if triangles.is_empty() {
            // Split by cause: a world matrix that flattens the xy plane (seen edge-on, or
            // scaled to nothing), a tiny outline, or neither.
            let det = m[0] * m[5] - m[1] * m[4];
            let scale = m[0].abs().max(m[1].abs()).max(m[4].abs()).max(m[5].abs());
            let axis = |x: f32, y: f32| x.hypot(y) <= 1e-6 * scale;
            let flat = scale <= 0. || det.abs() <= 1e-6 * scale * scale;
            if flat && scene.report.flat_samples.len() < FLAT_SAMPLES {
                let l = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
                scene.report.flat_samples.push((i, l));
            }
            reject(if scale <= 0. {
                "no triangles: flat transform: zero xy scale"
            } else if axis(m[0], m[1]) || axis(m[4], m[5]) {
                "no triangles: flat transform: one axis zero"
            } else if det.abs() <= 1e-6 * scale * scale {
                // Both axes present but parallel: turned edge-on in 3D, or sheared flat.
                "no triangles: flat transform: axes parallel"
            } else if outline.area.abs() * f64::from(det.abs()) < 1e-3 {
                "no triangles: tiny outline"
            } else {
                "no triangles: other"
            });
            continue;
        }
        fixed_shape.push(rule == MaterialRule::Fixed);
        scene.colliders.push(SceneCollider {
            thing: i,
            body,
            frame,
            triangles,
            z: m[14],
            half_thickness: shape.thickness.abs(),
            density: shape.mass_depth.max(0.) * DENSITY,
        });
    }
    for (c, fixed) in scene.colliders.iter().zip(fixed_shape) {
        if let (Some(b), true) = (c.body, fixed) {
            scene.bodies[b].fixed = true;
        }
    }
    for c in &scene.colliders {
        if c.body.is_some_and(|b| !scene.bodies[b].fixed) {
            scene.report.dynamic += 1;
        } else {
            scene.report.static_ += 1;
        }
    }
    scene.report.fixed_bodies = scene.bodies.iter().filter(|b| b.fixed).count();
    scene.report.nested_bodies = nested.len();
    for (i, t) in things.iter().enumerate() {
        let Some(tw) = t
            .data
            .physics_tweak
            .as_ref()
            .filter(|tw| tw.configuration == 0)
        else {
            continue;
        };
        let slot = body_owner(things, i)
            .0
            .and_then(|b| body_of.get(&b).copied().flatten());
        // FOUND (01.33, 0xc5f1c): each tweak in the body's hierarchy adds gravity × its
        // activation (and dampening × activation) to the body's totals.
        if let Some(b) = slot {
            let a = tw.last_known_activation;
            let total = scene.bodies[b].anti_gravity.get_or_insert(AntiGravity {
                cancelled: 0.,
                dampening: [0.; 3],
                tweaks: vec![],
            });
            total.cancelled += tw.gravity * a;
            total.tweaks.push(AntiGravityTweak {
                thing: i,
                gravity: tw.gravity,
                behavior: tw.behavior,
                saved: a,
            });
            for (d, t) in total.dampening.iter_mut().zip(tw.dampening) {
                *d += t * a;
            }
        }
    }
    scene.add_joints(things);
    scene
}

/// The shape with its vertices projected to world xy (translation left out) when it carries
/// vertex z and the matrix's local z axis has an xy part; `None` for a plain xy shape, which
/// keeps the in-plane route. INFERRED from the Pod (RPCS3 memory, D212): Sackboy stands at
/// y -478 there, 69 above the projected floor edge (y -551..-542), not on the sliver at -152.
pub fn tilted_outline(shape: &Shape, m: &[f32; 16]) -> Option<Shape> {
    let scale = (0..3)
        .map(|c| m[c * 4].hypot(m[c * 4 + 1]))
        .fold(0f32, f32::max);
    if !shape.requires_z || !scale.is_finite() || scale <= 0. || m[8].hypot(m[9]) <= 1e-3 * scale {
        return None;
    }
    let mut out = shape.clone();
    for v in &mut out.vertices {
        *v = [
            m[0] * v[0] + m[4] * v[1] + m[8] * v[2],
            m[1] * v[0] + m[5] * v[1] + m[9] * v[2],
            0.,
        ];
    }
    out.requires_z = false;
    Some(out)
}

/// PJoint type as the sandbox maps it (docs/runtime-research.md §4 joint table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    /// ELASTIC (1): pulls back towards its length, never pushes.
    Elastic,
    /// SPRING (2): holds its length, compliant.
    Spring,
    /// CHAIN (3, winch INFERRED): fixed length for now (animation not modelled).
    Chain,
    /// PISTON (4): slides along the contact line between its min and max length.
    Piston,
    /// STRING (5): at most its length.
    String,
    /// ROD (6): exactly its length.
    Rod,
    /// BOLT (7): shared pivot.
    Bolt,
    /// SPRING_ANGULAR (8) and QUANTIZED (10, wobble bolt INFERRED): pivot pulled back to
    /// its rest angle.
    SpringAngular,
    /// MOTOR (9): pivot turning at the stored speed.
    Motor,
}

impl JointKind {
    /// None for LEGACY (0) and unknown types.
    pub fn of(joint_type: i32) -> Option<Self> {
        Some(match joint_type {
            1 => Self::Elastic,
            2 => Self::Spring,
            3 => Self::Chain,
            4 => Self::Piston,
            5 => Self::String,
            6 => Self::Rod,
            7 => Self::Bolt,
            8 | 10 => Self::SpringAngular,
            9 => Self::Motor,
            _ => return None,
        })
    }
}

/// What a joint end is attached to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endpoint {
    /// Index in [`PhysicsScene::bodies`].
    Body(usize),
    /// Index in [`PhysicsScene::colliders`] of a static shape.
    Static(usize),
    /// A Thing with a POS but no collider (a hologram or non-collidable piece, say): a
    /// fixed point in the world at its frame. INFERRED: such pieces do not fall in the
    /// game, so what hangs from them stays up.
    Anchor { thing: usize, frame: Frame },
}

/// One PJoint between two scene entities.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneJoint {
    pub thing: usize,
    pub kind: JointKind,
    pub a: Endpoint,
    pub b: Endpoint,
    /// Contact points in world xy and in each endpoint's frame.
    pub world_a: [f32; 2],
    pub world_b: [f32; 2],
    pub local_a: [f32; 2],
    pub local_b: [f32; 2],
    /// Rest length (PJoint.length; the contact distance when that is not usable).
    pub length: f32,
    /// Piston travel (tweakTargetMin/MaxLength when usable, else the rest length).
    pub min_length: f32,
    pub max_length: f32,
    pub params: lbp_level::parts::JointParams,
}

/// D1102: FOUND (01.33, FUN_000c79c8) the piston's there-and-back wave at time `x` in
/// frames: 0 for `time` < 1; else over a cycle of `time + pause` frames it holds 0 for
/// pause / 2, rises linearly to 1 over time / 2, holds 1 for pause, and falls over time / 2.
pub fn piston_wave(x: f32, time: f32, pause: f32) -> f32 {
    if time < 1. {
        return 0.;
    }
    let (half, wait) = (time / 2., pause / 2.);
    let m = x.rem_euclid(time + pause);
    if m < wait {
        0.
    } else if m < wait + half {
        (m - wait) / half
    } else if m < pause + half {
        1.
    } else {
        (1. - (m - pause - half) / half).max(0.)
    }
}

impl SceneJoint {
    /// D1102: FOUND (01.33, 0xc95ec, its free-running branch at 0xc9834) the length of a
    /// piston with no switch input at `frame`, from its base length and motion settings
    /// (range, time, phase, speed, pause), by animation pattern: 0 an eased move between
    /// base and base + range over the wave; 1 base + (frame + phase) × speed; 2 a sawtooth
    /// from base + range back to base over `time`, then base for `pause`; else the base.
    /// INFERRED: the ease is a cosine (the call at 0x88f64c), the base length is the stored
    /// length. `frame` is the piston's retimed clock ([`PistonClock::frame`]).
    pub fn free_length(&self, frame: f32) -> f32 {
        let base = self.min_length;
        let [range, time, phase, speed, pause, ..] = self.params.motion;
        let t = frame + phase;
        match self.params.animation_pattern {
            0 => {
                let s = piston_wave(t, time, pause);
                base + range * (0.5 - 0.5 * (std::f32::consts::PI * s).cos())
            }
            1 => base + t * speed,
            2 if time >= 1. => {
                let u = t.rem_euclid(time + pause);
                if u < time {
                    base + range * (1. - u / time)
                } else {
                    base
                }
            }
            _ => base,
        }
    }
}

impl SceneJoint {
    /// D1104: FOUND (01.33, 0xc9790's branch at 0xc993c) the length of a piston at clock
    /// `frame`: a wired one in mode 1 (direction) or 4 (speed) ramps, base + 2 × frame ×
    /// range / time (the base for time < 1), with no phase, pattern or wave; every other
    /// piston follows [`Self::free_length`].
    /// D1105: FOUND (01.33, the solver loop 0x1c89d4, limit rows 0x960988 and 0x962590)
    /// the joint is held between base and base + range (0xc7350 min, 0xc7330 max), so the
    /// length is kept there while the clock runs on unbounded.
    pub fn length(&self, frame: f32, wired: bool) -> f32 {
        let base = self.min_length;
        let [range, time, ..] = self.params.motion;
        let l = match self.params.mode {
            1 | 4 if wired && time >= 1. => base + 2. * frame * range / time,
            1 | 4 if wired => base,
            _ => self.free_length(frame),
        };
        l.clamp(base.min(base + range), base.max(base + range))
    }
}

/// D1104: FOUND (01.33, the per-frame pass 0xcfedc, jump table 0xcffa4) the rate a wired
/// piston's clock runs at, from its mode and the signal on its input (`ternary`,
/// `activation`): held (`+0x1cc`) 0; mode 0 1 while the ternary is set, else 0; mode 1 the
/// ternary (1 or -1), and with the ternary 0 -1 or 0 by the port test 0x900730 (read here
/// as -1, INFERRED: an off switch sends the piston back); mode 2 1; mode 3 the activation;
/// mode 4 (0xc9e80, not decoded) 1.
pub fn piston_scale(mode: i32, held: bool, ternary: i32, activation: f32) -> f32 {
    if held {
        return 0.;
    }
    match mode {
        0 => f32::from(u8::from(ternary != 0)),
        1 if ternary != 0 => ternary as f32,
        1 => -1.,
        3 => activation,
        _ => 1.,
    }
}

/// D1103: a piston's clock (FOUND, 01.33): the retimed frame the length rule reads is
/// `(now - start) × scale + start + delta`, the PJoint's modStartFrame, modDeltaFrames
/// and modScale (`+0x144..+0x14c`, through the pointer at `+0x190`). A switch only
/// changes the scale (0xc7648), keeping the frame continuous. The saved values (scale 0)
/// are not used: play starts from the reset clock 0, 0, 1 (0xd0054, INFERRED for load).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PistonClock {
    pub start: f32,
    pub delta: f32,
    pub scale: f32,
}

impl Default for PistonClock {
    fn default() -> Self {
        Self {
            start: 0.,
            delta: 0.,
            scale: 1.,
        }
    }
}

impl PistonClock {
    pub fn frame(&self, now: f32) -> f32 {
        (now - self.start) * self.scale + self.start + self.delta
    }

    /// 0xc7648: from `now` the clock runs at `scale`, without a jump.
    pub fn set_scale(&mut self, now: f32, scale: f32) {
        if scale == self.scale {
            return;
        }
        self.delta += (self.scale - 1.) * (now - self.start);
        self.start = now;
        self.scale = scale;
    }
}

/// World xy of a point given in a Thing's local frame (its POS matrix).
fn world_point(t: &ThingRecord, p: [f32; 3]) -> Option<[f32; 2]> {
    let m = &t.data.pos.as_ref()?.world;
    let w = [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
    ];
    w.iter().all(|v| v.is_finite()).then_some(w)
}

impl PhysicsScene {
    fn frame_of(&self, e: Endpoint) -> Frame {
        match e {
            Endpoint::Body(i) => self.bodies[i].frame,
            Endpoint::Static(i) => self.colliders[i].frame,
            Endpoint::Anchor { frame, .. } => frame,
        }
    }

    fn endpoint(&self, things: &[ThingRecord], t: usize) -> Option<Endpoint> {
        if let (Some(owner), _) = body_owner(things, t) {
            if let Some(i) = self.bodies.iter().position(|b| b.thing == owner) {
                return Some(Endpoint::Body(i));
            }
        }
        if let Some(i) = self
            .colliders
            .iter()
            .position(|c| c.thing == t && c.body.is_none())
        {
            return Some(Endpoint::Static(i));
        }
        let frame = Frame::of_matrix(&things.get(t)?.data.pos.as_ref()?.world)?;
        Some(Endpoint::Anchor { thing: t, frame })
    }

    /// Adds the level's PJoints between scene entities ([`build_scene`] calls it).
    /// Counted in the report: `joints` added, `rejected_joints` by reason.
    fn add_joints(&mut self, things: &[ThingRecord]) {
        for (i, t) in things.iter().enumerate() {
            let Some(j) = &t.data.joint else {
                continue;
            };
            match self.joint(things, i, j) {
                Ok(joint) => {
                    self.report.joints += 1;
                    self.report.anchored_joint_ends += [joint.a, joint.b]
                        .iter()
                        .filter(|e| matches!(e, Endpoint::Anchor { .. }))
                        .count();
                    self.joints.push(joint);
                }
                Err(why) => *self.report.rejected_joints.entry(why).or_default() += 1,
            }
        }
    }

    fn joint(
        &self,
        things: &[ThingRecord],
        thing: usize,
        j: &lbp_level::parts::Joint,
    ) -> Result<SceneJoint, &'static str> {
        let kind = JointKind::of(j.joint_type).ok_or("legacy or unknown type")?;
        let (Some(ta), Some(tb)) = (j.a, j.b) else {
            return Err("missing endpoint");
        };
        let (Some(a), Some(b)) = (self.endpoint(things, ta), self.endpoint(things, tb)) else {
            return Err("endpoint without a collider or POS");
        };
        if a == b {
            return Err("both ends on one body");
        }
        let fixed = |e: Endpoint| match e {
            Endpoint::Static(_) | Endpoint::Anchor { .. } => true,
            Endpoint::Body(i) => self.bodies[i].fixed,
        };
        // avian2d needs a dynamic body on at least one end (both static panics: "neither
        // body is in an island").
        if fixed(a) && fixed(b) {
            return Err("both ends static");
        }
        let (Some(world_a), Some(world_b)) = (
            things.get(ta).and_then(|t| world_point(t, j.a_contact)),
            things.get(tb).and_then(|t| world_point(t, j.b_contact)),
        ) else {
            return Err("endpoint without POS");
        };
        let gap = ((world_b[0] - world_a[0]).powi(2) + (world_b[1] - world_a[1]).powi(2)).sqrt();
        let length = if j.length.is_finite() && j.length > 0. {
            j.length
        } else {
            gap
        };
        let p = &j.params;
        let (lo, hi) = (p.tweak_target_min_length, p.tweak_target_max_length);
        let (min_length, max_length) =
            if lo.is_finite() && hi.is_finite() && 0. <= lo && lo <= hi && hi > 0. {
                (lo, hi)
            } else {
                (length, length)
            };
        Ok(SceneJoint {
            thing,
            kind,
            a,
            b,
            world_a,
            world_b,
            local_a: self.frame_of(a).to_local(world_a),
            local_b: self.frame_of(b).to_local(world_b),
            length,
            min_length,
            max_length,
            params: *p,
        })
    }
}
