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
    /// D1111: world point `p` in this frame.
    pub fn local(&self, p: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        let d = [p[0] - self.position[0], p[1] - self.position[1]];
        [c * d[0] + s * d[1], c * d[1] - s * d[0]]
    }

    /// D1111: the world point at `at` in this frame.
    pub fn world(&self, at: [f32; 2]) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        [
            self.position[0] + c * at[0] - s * at[1],
            self.position[1] + s * at[0] + c * at[1],
        ]
    }

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
    /// D1109: the gyroscopes and rotators glued to the body (configurations 1 and 9).
    pub spins: Vec<SpinTweak>,
    /// D1110: its movers and followers, in Thing order.
    pub moves: Vec<MoveTweak>,
    /// D1145: its layer switches (configuration 12), in Thing order.
    pub layers: Vec<LayerTweak>,
}

/// D1145: one layer of depth, the step of a layer switch (0xd953ac).
pub const LAYER_DEPTH: f32 = 200.;

/// D1145: the depth band a layer move must stay in (0xd95360, 0xd95364).
pub const LAYER_Z_RANGE: (f32, f32) = (-511., 110.);

/// D1145: a PPhysicsTweak with configuration 12, the layer switch: it moves its body's
/// glued group back or front a layer, or to a set layer, if there is room. Read from
/// 0x1cc660, 0xce1f8 and 0xcd770 (EBOOT 01.33).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayerTweak {
    /// The PPhysicsTweak's Thing index.
    pub thing: usize,
    /// PPhysicsTweak.behavior, read as the input mode (D1101).
    pub behavior: i32,
    /// zBehavior: 0 back, 1 front, 2 to 4 to a set layer.
    pub z_behavior: i32,
    /// maximumMass: the mass the move may carry, 0 for no limit (INFERRED: 0xcc1f4 spends
    /// it on the group's mass; not modelled).
    pub maximum_mass: f32,
    /// waitingToMove and lastKnownActivation, as saved and then as run.
    pub waiting: bool,
    pub last_activation: f32,
}

impl LayerTweak {
    /// D1145: the layers to move for a body at depth `z` when the input's count is `n`.
    /// FOUND (0xce1f8): 0 is -n, 1 is +n; 2 to 4 is zBehavior - 1 - trunc((z + 679) / 200),
    /// so the set layers are 1, 2 and 3 counted from the back (INFERRED).
    pub fn delta(&self, z: f32, n: i32) -> i32 {
        match self.z_behavior {
            0 => -n,
            1 => n,
            d => d - 1 - ((z + 679.) / LAYER_DEPTH).trunc() as i32,
        }
    }

    /// D1145: one tick with the input at `activation` (count `n`) for a body at depth `z`;
    /// `fits(dz)` says whether the group has room `dz` deeper. Returns the depth change.
    /// FOUND (0x1cc660, 0xce1f8): only while waiting; the wait clears, and with a zero
    /// count nothing moves and nothing moves back. A blocked move to a set layer waits
    /// again and tries one layer less, down to one; a blocked step back or front is
    /// dropped. FOUND (0xcc854): a move whose new depth leaves [`LAYER_Z_RANGE`] is
    /// blocked. INFERRED: the wait is set when the activation changes from the last one
    /// (the setter is NOT FOUND; lastKnownActivation is saved beside it).
    pub fn tick(&mut self, activation: f32, n: i32, z: f32, fits: impl Fn(f32) -> bool) -> f32 {
        if activation != self.last_activation {
            self.last_activation = activation;
            self.waiting = activation != 0.;
        }
        if !self.waiting {
            return 0.;
        }
        self.waiting = false;
        if n == 0 {
            return 0.;
        }
        let mut d = self.delta(z, n);
        loop {
            let dz = d as f32 * LAYER_DEPTH;
            let to = z + dz;
            if d == 0 || (to >= LAYER_Z_RANGE.0 && to <= LAYER_Z_RANGE.1 && fits(dz)) {
                return dz;
            }
            if self.z_behavior < 2 {
                return 0.;
            }
            self.waiting = true;
            if d.abs() < 2 {
                return 0.;
            }
            d -= d.signum();
        }
    }
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

/// D1109: a PPhysicsTweak that turns its body: configuration 1 (the gyroscope, turning
/// towards world up) or 9 (the rotator, turning at a set speed). D1144: 3, 4 and 5 turn
/// their body's up axis along its velocity, the stick or a found target. Read from the
/// SPU row packer 0x95fb20 (EBOOT 01.33); angles in radians, speeds per game tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpinTweak {
    /// The PPhysicsTweak's Thing index.
    pub thing: usize,
    pub configuration: u32,
    /// PPhysicsTweak.behavior, read as the input mode (D1101).
    pub behavior: i32,
    /// velRange: the gyroscope's top speed and the scale of both strengths.
    pub range: f32,
    /// accelStrength and decelStrength.
    pub accel: f32,
    pub decel: f32,
    /// The rotator's speed (middleVel z; per tick).
    pub speed: f32,
    /// D1144: configuration 5's search (0x1c3e04, as the follower's, D1110): distance
    /// band, cone (degrees; 180 or more is no cone) and followKey's tag key.
    pub min_range: f32,
    pub max_range: f32,
    pub angle_range: f32,
    pub key: Option<u32>,
}

/// D1144: the dead zone of the turning packer (0xc46400, 0x95fd88): an aim shorter than
/// this squared gives a target speed of 0.
const AIM_DEAD_ZONE: f32 = 0.1;

/// D1109: `angle` wrapped into [-π, π] (the packer's vrfim step at 0x960790).
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    angle - TAU * ((angle + PI) / TAU).floor()
}

impl SpinTweak {
    /// D1109: the turn speed (per tick) the tweak wants for a body at `angle` and its
    /// change limits (lo, hi) for this tick, before the activation. FOUND (0x95fb20):
    /// limits ±accel × range; the rotator's target is its speed, with decel × range on
    /// the side that slows it (0x95fd4c); the gyroscope aims at world up, angle error
    /// wrapped, target clamped to ±range (0x960858). INFERRED: the target shape inside
    /// 0x1c21f4 (a stop-in-time rule), read as sign(e) × sqrt(2 × accel × range × |e|).
    pub fn target(&self, angle: f32) -> (f32, f32, f32) {
        self.target_along(angle, [0., 1.])
    }

    /// D1144: as [`Self::target`], turning the body's up axis towards `aim` (a world
    /// vector) instead of world up. FOUND (0x95fd0c, 0x95fd88): an aim with x² + y² under
    /// 0.1, or accel × range 0, wants speed 0 (still with ±accel × range, so it brakes).
    /// INFERRED: the error is the wrapped angle from the up axis to the aim (the packer
    /// forms two wrapped atan2 terms, 0x960588 to 0x960790; which one 0x1c21f4 takes is not
    /// settled).
    pub fn target_along(&self, angle: f32, aim: [f32; 2]) -> (f32, f32, f32) {
        let a = self.accel * self.range;
        match self.configuration {
            9 => {
                let d = self.decel * self.range;
                let lo = if self.speed >= 0. { -d } else { -a };
                let hi = if self.speed <= 0. { d } else { a };
                (self.speed, lo, hi)
            }
            _ if dot(aim, aim) < AIM_DEAD_ZONE || a == 0. => (0., -a, a),
            _ => {
                // The aim's angle from world up, so world up itself gives exactly -angle.
                let e = wrap_angle((-aim[0]).atan2(aim[1]) - angle);
                let w = e.signum() * (2. * a * e.abs()).sqrt();
                (w.clamp(-self.range, self.range), -a, a)
            }
        }
    }

    /// D1144: where the tweak aims (a world vector) for a body moving at `vel` (game units
    /// per tick) whose target, if it found one, is at `d` (target minus body). FOUND
    /// (0x95fd0c): 1 aims at world up; 3 along the body's velocity (state+0x10, x and y);
    /// 4 along stick slots 1 and 2 (0xcb02c, 0xcb074), which nothing fills here, so 0;
    /// 5 at its target (0x1c3e04), 0 with none. Not modelled: 5's lead by the target's
    /// velocity (INFERRED from the vector steps), the mirror flag (negates 4's x) and the
    /// +0x220 angle.
    pub fn aim(&self, vel: [f32; 2], d: Option<[f32; 2]>) -> [f32; 2] {
        match self.configuration {
            3 => vel,
            4 => [0., 0.],
            5 => d.unwrap_or([0., 0.]),
            _ => [0., 1.],
        }
    }

    /// D1144: whether configuration 5 takes a target at `d` (target minus body) for a
    /// body whose up axis is `up` (0x1c3e04, as [`MoveTweak::sees`]).
    pub fn sees(&self, d: [f32; 2], up: [f32; 2]) -> bool {
        self.configuration == 5 && sees(self.min_range, self.max_range, self.angle_range, d, up)
    }

    /// D1109: the body's turn speed (per tick) after this tweak's row at `activation`:
    /// the change towards the target, clamped to the limits scaled by the activation
    /// (0x95fdc8). The 1/mass factor there is left out (INFERRED: the solver undoes it).
    pub fn turn(&self, angle: f32, w: f32, activation: f32) -> f32 {
        self.turn_along(angle, w, activation, [0., 1.])
    }

    /// D1144: as [`Self::turn`], aiming along `aim` ([`Self::aim`]).
    pub fn turn_along(&self, angle: f32, w: f32, activation: f32, aim: [f32; 2]) -> f32 {
        let (target, lo, hi) = self.target_along(angle, aim);
        w + (target - w).clamp(lo * activation, hi * activation)
    }
}

/// D1110: the search of 0x1c3e04 for a target at `d` (target minus body) by a body whose
/// up axis is `up`: min < |d| < max in x, y, max below 0 is 1e8, and a cone under 180
/// degrees. INFERRED: the cone is a half-angle about the body's up axis.
fn sees(min: f32, max: f32, angle: f32, d: [f32; 2], up: [f32; 2]) -> bool {
    let r = d[0].hypot(d[1]);
    let max = if max < 0. { 1e8 } else { max };
    if r <= min || r >= max {
        return false;
    }
    angle >= 180. || dot(d, up) >= r * angle.to_radians().cos()
}

/// D1110: a PPhysicsTweak that pushes its body: configuration 8 (the mover, a set
/// velocity), 6 and 7 (stick movers) or 11 (the follower). Read from 0x957df4, which packs
/// two linear rows for the packer 0x95fb20 (EBOOT 01.33); lengths in game units, speeds
/// per game tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTweak {
    /// The PPhysicsTweak's Thing index.
    pub thing: usize,
    pub configuration: u32,
    /// PPhysicsTweak.behavior, read as the input mode (D1101).
    pub behavior: i32,
    /// velRange, accelStrength and decelStrength.
    pub range: f32,
    pub accel: f32,
    pub decel: f32,
    /// The mover's velocity (middleVel x, y; per tick).
    pub vel: [f32; 2],
    /// localSpace: the mover's velocity turns with its body (0x9582b4).
    pub local: bool,
    /// The follower's distance band and cone (degrees; 180 or more is no cone).
    pub min_range: f32,
    pub max_range: f32,
    pub angle_range: f32,
    /// flee: the follower keeps max_range from its target instead of min_range.
    pub flee: bool,
    /// allowUpDown: off drops the follower's push along its body's up axis.
    pub up_down: bool,
    /// D1111: followKey: the follower follows the tags with this key ([`SceneTag`])
    /// instead of the players.
    pub key: Option<u32>,
}

/// D1110: `v` per unit length, or None for a zero vector.
fn unit(v: [f32; 2]) -> Option<[f32; 2]> {
    let l = v[0].hypot(v[1]);
    (l > 0.).then(|| [v[0] / l, v[1] / l])
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

impl MoveTweak {
    /// D1110: whether a target at `d` (target minus body) is one this follower takes, for a
    /// body whose up axis is `up`. FOUND (0x1c3e04): min_range < |d| < max_range in x, y,
    /// max_range below 0 is 1e8, and a cone under 180 degrees. INFERRED: the cone is a
    /// half-angle about the body's up axis.
    pub fn sees(&self, d: [f32; 2], up: [f32; 2]) -> bool {
        sees(self.min_range, self.max_range, self.angle_range, d, up)
    }

    /// D1110: the rows (axis, target, lo, hi) this tweak packs for a body with up axis `up`
    /// and, for a follower, its target at `d` (target minus body), before the activation.
    /// FOUND (0x957df4): row 1 runs along the wanted velocity v with limits -decel and
    /// +accel times range; a zero v brakes along the body's velocity (or world up) with
    /// ±decel × range; row 2 holds the cross axis at 0 with ±decel × range, only when that
    /// is not 0. The follower aims at the point min_range (max_range when fleeing) short of
    /// its target, with ±accel × range on both rows and its speed clamped to ±range.
    /// INFERRED: the follower's 0x1c21f4 speed, read as sqrt(2 × accel × range × distance)
    /// (as the gyroscope's, D1109); the input scale 0xcbed4 read as 1; the mirror flag and
    /// the follower's +0x220 angle not modelled; stick movers (6, 7) see no stick.
    pub fn rows(&self, vel: [f32; 2], up: [f32; 2], d: Option<[f32; 2]>) -> Vec<Row> {
        let a = self.accel * self.range;
        let k = self.decel * self.range;
        let follow = self.configuration == 11;
        let side = if follow { a } else { k };
        let want = match self.configuration {
            8 if self.local => {
                let [x, y] = self.vel;
                [up[0] * y + up[1] * x, up[1] * y - up[0] * x]
            }
            8 => self.vel,
            11 => d.map_or([0., 0.], |d| {
                let r = d[0].hypot(d[1]);
                let keep = if self.flee {
                    self.max_range
                } else {
                    self.min_range
                };
                let mut off = [d[0] - d[0] / r * keep, d[1] - d[1] / r * keep];
                if !self.up_down {
                    let s = dot(off, up);
                    off = [off[0] - up[0] * s, off[1] - up[1] * s];
                }
                off
            }),
            _ => [0., 0.],
        };
        let mut rows = Vec::with_capacity(2);
        let axis = match unit(want) {
            Some(axis) => {
                let target = if follow {
                    (2. * a * want[0].hypot(want[1])).sqrt().min(self.range)
                } else {
                    want[0].hypot(want[1])
                };
                let lo = if follow { -a } else { -k };
                rows.push(Row {
                    axis,
                    target,
                    lo,
                    hi: a,
                });
                axis
            }
            None => {
                let axis = unit(vel).unwrap_or([0., 1.]);
                rows.push(Row {
                    axis,
                    target: 0.,
                    lo: -side,
                    hi: side,
                });
                axis
            }
        };
        if k != 0. {
            let cross = [axis[1], -axis[0]];
            rows.push(Row {
                axis: cross,
                target: 0.,
                lo: -side,
                hi: side,
            });
        }
        rows
    }

    /// D1110: the body's velocity (per tick) after this tweak's rows at `activation`: along
    /// each axis the change towards the target, clamped to the limits scaled by the
    /// activation (0x95fdc8; 1/mass left out as for D1109).
    pub fn push(&self, vel: [f32; 2], up: [f32; 2], d: Option<[f32; 2]>, act: f32) -> [f32; 2] {
        let mut v = vel;
        for r in self.rows(vel, up, d) {
            let s = dot(v, r.axis);
            let ds = (r.target - s).clamp(r.lo * act, r.hi * act);
            v = [v[0] + r.axis[0] * ds, v[1] + r.axis[1] * ds];
        }
        v
    }
}

/// D1110: one packed linear row: a unit axis, the speed along it the row wants, and the
/// change limits for a tick before the activation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    pub axis: [f32; 2],
    pub target: f32,
    pub lo: f32,
    pub hi: f32,
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
    /// D1111: every tag (PSwitchKey) a follower can follow.
    pub tags: Vec<SceneTag>,
    pub report: SceneReport,
}

/// D1111: a tag (PSwitchKey) where followers find it. `key` numbers the tag's colour and
/// name, the pair the game's tag lists are keyed by (0xceac0, EBOOT 01.33); a follower
/// with followKey follows the tags whose key is its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTag {
    pub thing: usize,
    pub key: u32,
    /// The body the tag rides on, with `at` in that body's frame; None: `at` is the
    /// tag's world position.
    pub body: Option<usize>,
    pub at: [f32; 2],
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

/// The position of a thing, or of its nearest parent that has one.
fn placed(things: &[ThingRecord], mut i: usize) -> Option<&lbp_level::parts::Pos> {
    for _ in 0..MAX_PARENT_DEPTH {
        let t = things.get(i)?;
        if let Some(pos) = &t.data.pos {
            return Some(pos);
        }
        i = t.parent.filter(|&p| p != i)?;
    }
    None
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
                            spins: vec![],
                            moves: vec![],
                            layers: vec![],
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
    // D1111: tags are keyed by colour and name.
    let mut keys = BTreeMap::<(i32, String), u32>::new();
    let mut key_of = |colour: i32, name: &str| -> u32 {
        let n = keys.len() as u32;
        *keys.entry((colour, name.to_owned())).or_insert(n)
    };
    for (i, t) in things.iter().enumerate() {
        // A tag with no position of its own (one placed on a chip or a part) sits where
        // its nearest positioned parent is.
        let Some(k) = &t.data.switch_key else {
            continue;
        };
        let Some(pos) = placed(things, i) else {
            continue;
        };
        let world = [pos.world[12], pos.world[13]];
        let body = body_owner(things, i)
            .0
            .and_then(|b| body_of.get(&b).copied().flatten());
        let at = match body {
            Some(b) => scene.bodies[b].frame.local(world),
            None => world,
        };
        scene.tags.push(SceneTag {
            thing: i,
            key: key_of(k.colour_index, &k.name),
            body,
            at,
        });
    }
    for (i, t) in things.iter().enumerate() {
        let Some(tw) = t
            .data
            .physics_tweak
            .as_ref()
            .filter(|tw| matches!(tw.configuration, 1 | 3..=5 | 9))
        else {
            continue;
        };
        let slot = body_owner(things, i)
            .0
            .and_then(|b| body_of.get(&b).copied().flatten());
        if let Some(b) = slot {
            scene.bodies[b].spins.push(SpinTweak {
                thing: i,
                configuration: tw.configuration,
                behavior: tw.behavior,
                range: tw.vel_range,
                accel: tw.accel_strength,
                decel: tw.decel_strength,
                speed: tw.middle_vel[2],
                min_range: tw.min_range,
                max_range: tw.max_range,
                angle_range: tw.angle_range,
                key: (tw.configuration == 5 && tw.follow_key)
                    .then(|| key_of(tw.color_index, &tw.name)),
            });
        }
    }
    for (i, t) in things.iter().enumerate() {
        let Some(tw) = t
            .data
            .physics_tweak
            .as_ref()
            .filter(|tw| tw.configuration == 12)
        else {
            continue;
        };
        let slot = body_owner(things, i)
            .0
            .and_then(|b| body_of.get(&b).copied().flatten());
        if let Some(b) = slot {
            scene.bodies[b].layers.push(LayerTweak {
                thing: i,
                behavior: tw.behavior,
                z_behavior: tw.z_behavior,
                maximum_mass: tw.maximum_mass,
                waiting: tw.waiting_to_move,
                last_activation: tw.last_known_activation,
            });
        }
    }
    for (i, t) in things.iter().enumerate() {
        let Some(tw) = t
            .data
            .physics_tweak
            .as_ref()
            .filter(|tw| matches!(tw.configuration, 6..=8 | 11))
        else {
            continue;
        };
        let slot = body_owner(things, i)
            .0
            .and_then(|b| body_of.get(&b).copied().flatten());
        if let Some(b) = slot {
            scene.bodies[b].moves.push(MoveTweak {
                thing: i,
                configuration: tw.configuration,
                behavior: tw.behavior,
                range: tw.vel_range,
                accel: tw.accel_strength,
                decel: tw.decel_strength,
                vel: [tw.middle_vel[0], tw.middle_vel[1]],
                local: tw.local_space,
                min_range: tw.min_range,
                max_range: tw.max_range,
                angle_range: tw.angle_range,
                flee: tw.flee,
                up_down: tw.allow_up_down,
                key: (tw.configuration == 11 && tw.follow_key)
                    .then(|| key_of(tw.color_index, &tw.name)),
            });
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
    /// CHAIN (3, winch INFERRED): its length follows the piston rule (D1107).
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

    /// D1108: FOUND (01.33, 0xc99d0 for types 7 and up) a motor's target angle (radians)
    /// at clock `frame` is the piston length rule less its base, plus a centre that is
    /// synced to the measured angle when the joint first runs (0xc9af4, INFERRED), so only
    /// its change matters. The motor row (0x95f494) is unbounded for pattern 1 or a range
    /// of 2π or more, so the angle is clamped to the range only otherwise.
    pub fn motor_angle(&self, frame: f32, wired: bool) -> f32 {
        let base = self.min_length;
        let range = self.params.motion[0];
        let l = match self.params.mode {
            1 | 4 if wired => self.length(frame, true),
            _ => self.free_length(frame),
        };
        if self.params.animation_pattern == 1 || range.abs() >= std::f32::consts::TAU {
            return l - base;
        }
        l.clamp(base.min(base + range), base.max(base + range)) - base
    }
}

/// D1104: FOUND (01.33, the per-frame pass 0xcfedc, jump table 0xcffa4) the rate a wired
/// piston's clock runs at, from its mode and the signal on its input (`ternary`,
/// `activation`): held (`+0x1cc`) 0; mode 0 1 while the ternary is set, else 0; mode 1 the
/// ternary (1 or -1), and with the ternary 0 -1 or 0 by the port test 0x900730 (read here
/// as -1, INFERRED: an off switch sends the piston back); mode 2 1; mode 3 the activation;
/// mode 4 (speed) is a servo on the length ([`SceneJoint::servo_scale`], D1106).
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

impl SceneJoint {
    /// D1106: FOUND (01.33, 0xc9e80, called from 0xcfedc's mode 4 case at 0xd00c8) the
    /// clock rate of a wired speed piston: the target s = max(`activation`, 0), flipped to
    /// 1 - s when the range is negative and the pattern is not 2, is a fraction of the way
    /// from the short end to the long end (0xc7350 min, 0xc7330 max of base and base +
    /// range); the error e = s - (length - min) / (max - min) gives the rate 1 above 0.1,
    /// -1 below -0.1, else 10 × e. The game measures the joint's length (0xc9d38); `length`
    /// here is the commanded one (INFERRED stand-in).
    pub fn servo_scale(&self, activation: f32, length: f32) -> f32 {
        let base = self.min_length;
        let range = self.params.motion[0];
        let (lo, hi) = (base.min(base + range), base.max(base + range));
        let mut s = activation.max(0.);
        if range < 0. && self.params.animation_pattern != 2 {
            s = 1. - s;
        }
        let frac = if hi > lo {
            (length - lo) / (hi - lo)
        } else {
            0.
        };
        let e = s - frac;
        if e > 0.1 {
            1.
        } else if e < -0.1 {
            -1.
        } else {
            10. * e
        }
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
