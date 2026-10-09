//! D160: projects STICKERS decals onto the front cap of their owner SHAPE as world-space
//! overlay triangles (renderer-independent geometry, like [`crate::shape_mesh`]).
//!
//! Decal frame (inferred, docs/d160-globe-material-and-stickers.md): a decal's `u`, `v` is
//! its centre and `xvec` / `yvec` its half axes in a surface space where, for a SHAPE owner,
//! `u = x * |sx| / 500` and `v = 1 - y * |sy| / 500` (x, y = local polygon coordinates;
//! |sx|, |sy| = POS column-0 / column-1 lengths; 500 = the D108 planar UV scale). The
//! decal image spans corners `centre - xvec - yvec` (texture 0,0) to
//! `centre + xvec + yvec` (texture 1,1), as the community decal baker lays out its quads.
//! Each front-cap triangle is clipped to the decal parallelogram, so decals stay on the
//! owner's surface and a decal list shared by several pieces only draws on each piece.
//!
//! All inputs are untrusted: non-finite or degenerate decals are refused with a reason and
//! the output vertex count is budgeted.
use crate::parts::Decal;
use crate::shape_mesh::FrontFace;

/// Planar surface scale (world units per decal-space unit), the D108 preview constant.
pub const DECAL_UNITS: f64 = 500.0;
/// Local-z lift of the first decal above the front cap, and the step per later decal of
/// the same owner (draw order; later decals on top). Preview choice.
pub const LIFT: f64 = 0.5;
pub const LIFT_STEP: f64 = 0.02;
/// Output vertex budget per decal.
pub const MAX_DECAL_VERTICES: usize = 1 << 16;

/// Overlay triangles for one decal: world positions, decal texture coordinates (0..1),
/// triangle indices into `positions`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecalMesh {
    pub positions: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Why a decal produced no geometry. `reason()` is a stable census key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecalReject {
    NonFinite,
    /// Zero-area decal axes or a degenerate owner scale.
    Degenerate,
    /// The decal does not overlap the owner's front cap.
    OffSurface,
    OutputBudget,
}

impl DecalReject {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonFinite => "decal: non-finite fields",
            Self::Degenerate => "decal: degenerate axes or owner scale",
            Self::OffSurface => "decal: outside the owner's front face",
            Self::OutputBudget => "decal: output budget",
        }
    }
}

fn column_len(m: &[f32; 16], c: usize) -> f64 {
    let v = |i: usize| f64::from(m[c * 4 + i]);
    (v(0) * v(0) + v(1) * v(1) + v(2) * v(2)).sqrt()
}

fn world_point(m: &[f32; 16], p: [f64; 3]) -> [f32; 3] {
    let g = |r: usize, c: usize| f64::from(m[c * 4 + r]);
    let mut o = [0.0f32; 3];
    for (r, v) in o.iter_mut().enumerate() {
        *v = (g(r, 0) * p[0] + g(r, 1) * p[1] + g(r, 2) * p[2] + g(r, 3)) as f32;
    }
    o
}

/// Sutherland-Hodgman clip of a convex polygon (decal space) to `a, b` in [-1, 1].
fn clip_square(mut poly: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    for (axis, sign) in [(0, 1.0), (0, -1.0), (1, 1.0), (1, -1.0)] {
        if poly.is_empty() {
            break;
        }
        // Inside: sign * p[axis] <= 1.
        let d = |p: &[f64; 2]| 1.0 - sign * p[axis];
        let mut out = Vec::with_capacity(poly.len() + 1);
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (da, db) = (d(&a), d(&b));
            if da >= 0.0 {
                out.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        poly = out;
    }
    poly
}

/// Projects `decal` (the `order`-th decal of its owner) onto `face`.
pub fn project(face: &FrontFace, decal: &Decal, order: usize) -> Result<DecalMesh, DecalReject> {
    let f = [
        decal.u,
        decal.v,
        decal.xvec[0],
        decal.xvec[1],
        decal.yvec[0],
        decal.yvec[1],
    ];
    if f.iter().any(|v| !v.is_finite()) {
        return Err(DecalReject::NonFinite);
    }
    let (sx, sy) = (
        column_len(&face.world, 0) / DECAL_UNITS,
        column_len(&face.world, 1) / DECAL_UNITS,
    );
    if !(sx.is_finite() && sy.is_finite()) || sx <= 1e-12 || sy <= 1e-12 {
        return Err(DecalReject::Degenerate);
    }
    // Decal space (u, v) -> local (x, y): x = u / sx, y = (1 - v) / sy.
    let centre = [f64::from(decal.u) / sx, (1.0 - f64::from(decal.v)) / sy];
    let ax = [
        f64::from(decal.xvec[0]) / sx,
        -f64::from(decal.xvec[1]) / sy,
    ];
    let ay = [
        f64::from(decal.yvec[0]) / sx,
        -f64::from(decal.yvec[1]) / sy,
    ];
    let det = ax[0] * ay[1] - ax[1] * ay[0];
    let size = (ax[0].abs() + ax[1].abs()) * (ay[0].abs() + ay[1].abs());
    if !det.is_finite() || size <= 0.0 || det.abs() <= 1e-9 * size {
        return Err(DecalReject::Degenerate);
    }
    // local -> (a, b) with p = centre + a * ax + b * ay.
    let to_ab = |p: [f64; 2]| {
        let (dx, dy) = (p[0] - centre[0], p[1] - centre[1]);
        [
            (dx * ay[1] - dy * ay[0]) / det,
            (ax[0] * dy - ax[1] * dx) / det,
        ]
    };
    let z = face.z + LIFT + LIFT_STEP * order.min(10_000) as f64;
    let mut out = DecalMesh::default();
    for tri in &face.triangles {
        let poly = clip_square(tri.iter().map(|&i| to_ab(face.points[i])).collect());
        if poly.len() < 3 {
            continue;
        }
        if out.positions.len() + poly.len() > MAX_DECAL_VERTICES {
            return Err(DecalReject::OutputBudget);
        }
        let base = out.positions.len() as u32;
        for [a, b] in &poly {
            let local = [
                centre[0] + a * ax[0] + b * ay[0],
                centre[1] + a * ax[1] + b * ay[1],
                z,
            ];
            out.positions.push(world_point(&face.world, local));
            out.uv
                .push([((a + 1.0) * 0.5) as f32, ((b + 1.0) * 0.5) as f32]);
        }
        for k in 1..poly.len() as u32 - 1 {
            out.indices.extend([base, base + k, base + k + 1]);
        }
    }
    if out.indices.is_empty() {
        return Err(DecalReject::OffSurface);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decal(u: f32, v: f32, x: [f32; 2], y: [f32; 2]) -> Decal {
        Decal {
            texture: None,
            u,
            v,
            xvec: x,
            yvec: y,
            color: 0,
            kind: 0,
            metadata_index: 0,
            num_metadata: None,
            placed_by: 0,
            play_mode_frame: 0,
            scorch_mark: false,
            plan: None,
        }
    }

    /// 1000 x 1000 local square at POS scale 2 translated by (10, 20, 30), thickness 5.
    fn face() -> FrontFace {
        let mut world = crate::shape_mesh::IDENTITY;
        world[0] = 2.0;
        world[5] = 2.0;
        world[12] = 10.0;
        world[13] = 20.0;
        world[14] = 30.0;
        FrontFace {
            points: vec![
                [-500.0, -500.0],
                [500.0, -500.0],
                [500.0, 500.0],
                [-500.0, 500.0],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            z: 5.0,
            world,
        }
    }

    #[test]
    fn inside_decal_maps_centre_axes_and_corners() {
        // Centre u = 0 -> x = 0; v = 1 -> y = 0. Half axes 0.4 -> 100 local units at scale 2.
        let m = project(&face(), &decal(0.0, 1.0, [0.4, 0.0], [0.0, 0.4]), 0).unwrap();
        // The two face triangles together cover the whole decal: texture-space area 1.
        let area: f32 = m
            .indices
            .chunks(3)
            .map(|t| {
                let [a, b, c] = [
                    m.uv[t[0] as usize],
                    m.uv[t[1] as usize],
                    m.uv[t[2] as usize],
                ];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-5, "{area}");
        let z = 30.0 + 5.0 + LIFT;
        for (p, uv) in m.positions.iter().zip(&m.uv) {
            assert!((f64::from(p[2]) - z).abs() < 1e-4);
            // texture (0,0) at centre - x - y: world x = 10 + 2 * (-100), y = 20 + 2 * (+100).
            let ex = 10.0 + 2.0 * (f64::from(uv[0]) * 2.0 - 1.0) * 100.0;
            let ey = 20.0 - 2.0 * (f64::from(uv[1]) * 2.0 - 1.0) * 100.0;
            assert!((f64::from(p[0]) - ex).abs() < 1e-3, "{p:?} {uv:?}");
            assert!((f64::from(p[1]) - ey).abs() < 1e-3, "{p:?} {uv:?}");
        }
    }

    #[test]
    fn decals_clip_to_the_face_and_report_rejects() {
        // Centred on the right edge (x = 500 -> u = 2): only the left half remains.
        let m = project(&face(), &decal(2.0, 1.0, [0.4, 0.0], [0.0, 0.4]), 3).unwrap();
        assert!(m.uv.iter().all(|uv| uv[0] <= 0.5 + 1e-6));
        assert!(m.positions.iter().all(|p| p[0] <= 10.0 + 1000.0 + 1e-3));
        assert!(m.uv.iter().any(|uv| uv[0] < 0.01));
        // Far away, degenerate and non-finite decals.
        let r = |d| project(&face(), &d, 0).unwrap_err();
        assert_eq!(
            r(decal(9.0, 1.0, [0.1, 0.0], [0.0, 0.1])),
            DecalReject::OffSurface
        );
        assert_eq!(
            r(decal(0.0, 1.0, [0.1, 0.0], [0.2, 0.0])),
            DecalReject::Degenerate
        );
        assert_eq!(
            r(decal(f32::NAN, 1.0, [0.1, 0.0], [0.0, 0.1])),
            DecalReject::NonFinite
        );
    }
}
