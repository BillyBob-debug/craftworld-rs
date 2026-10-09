//! Bevy-free physics description of a read level: which SHAPE Things collide, which move,
//! their 2D frames, triangles and z-slabs. [`crate::spawn_scene`] turns it into avian2d
//! entities.
//!
//! Rules (labels from docs/formats.md; evidence in docs/runtime-evidence/physics.md and
//! docs/sim-physics.md):
//! - A Thing with BODY is a rigid body at its POS frame (INFERRED: PBody holds the
//!   velocities, so it is the moving piece).
//! - A SHAPE Thing belongs to the nearest Thing with BODY on its parent chain, itself
//!   included (INFERRED, A1: pieces glued in one parent tree form one body). A SHAPE with
//!   no BODY on the chain is static.
//! - Collision is 2D; a shape's slab is world z ± thickness (INFERRED, A3/A4).
//! - A shape without the COLLIDABLE_GAME flag (0x1) does not collide (COMMUNITY, ShapeFlags).
//! - Mass = area × massDepth × density (INFERRED, A2); density is [`DENSITY`] until RMaterial
//!   is read (PLACEHOLDER).
use lbp_level::shape_mesh::outline_triangles;
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
    /// Shapes left out, by reason.
    pub rejected: BTreeMap<&'static str, usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysicsScene {
    pub bodies: Vec<SceneBody>,
    pub colliders: Vec<SceneCollider>,
    pub report: SceneReport,
}

/// Nearest Thing with BODY on the parent chain of `i`, itself included.
fn body_owner(things: &[ThingRecord], mut i: usize) -> Option<usize> {
    for _ in 0..MAX_PARENT_DEPTH {
        let t = things.get(i)?;
        if t.data.body.is_some() {
            return Some(i);
        }
        match t.parent {
            Some(p) if p != i => i = p,
            _ => return None,
        }
    }
    None
}

/// Builds the physics description of a level (`things[i]` must be Thing index `i`).
pub fn build_scene(things: &[ThingRecord]) -> PhysicsScene {
    let mut scene = PhysicsScene::default();
    let mut body_of = BTreeMap::<usize, Option<usize>>::new();
    for (i, t) in things.iter().enumerate() {
        let Some(shape) = &t.data.shape else {
            continue;
        };
        scene.report.shapes += 1;
        let mut reject = |why| *scene.report.rejected.entry(why).or_default() += 1;
        if shape.flags & COLLIDABLE_GAME == 0 {
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
        let outline = match outline_triangles(shape) {
            Ok(v) => v,
            Err(e) => {
                reject(e.reason());
                continue;
            }
        };
        let body = match body_owner(things, i) {
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
        let m = &pos.world;
        let world = |p: [f32; 2]| {
            [
                m[0] * p[0] + m[4] * p[1] + m[12],
                m[1] * p[0] + m[5] * p[1] + m[13],
            ]
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
            reject("no triangles");
            continue;
        }
        if body.is_some() {
            scene.report.dynamic += 1;
        } else {
            scene.report.static_ += 1;
        }
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
    scene
}
