//! D114 (plan item Lx, "extrusion v1"): turns one SHAPE polygon (+ its GENERATED_MESH and
//! the Thing's POS world matrix) into renderer-independent triangle geometry.
//!
//! **Preview approximation**: straight extrusion, no bevel, UV scale uncalibrated
//! ([`PREVIEW_LABEL`]). Rules and evidence labels: docs/d114-shape-extrude.md, built on
//! docs/d108-shape-geometry.md. Independent implementation from those notes; the
//! triangulator is written here from the published ear-clipping / hole-bridging method
//! (Eberly, "Triangulation by Ear Clipping"), no community or game code copied.
//!
//! Rules used (labels from docs/formats.md):
//! - `loops` are vertex counts per loop, back to back; loop 0 is the outer boundary and
//!   later loops are holes (CONFIRMED_FROM_COMMUNITY_CODE / STRONG_INFERENCE, D108 P2/P3).
//! - Winding is computed here, never assumed (CONFIRMED_FROM_LOCAL_DATA, D108 P4/P5).
//! - Polygon z is ignored; the slab spans local z in [-thickness, +thickness] with the front
//!   face at +z (STRONG_INFERENCE, D108 P6/D3/D4).
//! - The render material is GENERATED_MESH.gfxMaterial; PShape.material is the physics
//!   material and is not used. The bevel GUID is recorded but unused in v1 (D108 M1).
//! - Front/back UVs: (R*S applied to local xy) * [`UV_SCALE`] + uvOffset.xy with v flipped
//!   (COMMUNITY_BEHAVIOUR shape, UNKNOWN constant, D108 U2). Sides: u = perimeter length *
//!   UV_SCALE, v = depth * UV_SCALE (preview choice, labelled).
//!
//! All inputs are untrusted: budgets on vertices/loops/coordinates/output, checked sums,
//! explicit [`ShapeReject`] reasons, no panics on malformed polygons.
use crate::parts::{GeneratedMesh, Shape};
use crate::serial::Descriptor;
use crate::thing::ThingRecord;
use std::ops::Range;

/// Shown with every generated shape mesh.
pub const PREVIEW_LABEL: &str =
    "preview approximation: straight extrusion, no bevel, UV scale uncalibrated";
/// Planar UV scale K (texture repeats per world unit). Preview constant, uncalibrated
/// (D108 U2: the game's value is UNKNOWN; LBPAS uses a guessed 0.002 = 1/500).
pub const UV_SCALE: f64 = 1.0 / 500.0;
/// Input budgets (observed maxima on 01.33 levels: 442 vertices, 45 loops; D108 P3/P7).
pub const MAX_INPUT_VERTICES: usize = 4096;
pub const MAX_LOOPS: usize = 64;
/// Local coordinates and thickness beyond this are refused as oversized.
pub const MAX_COORDINATE: f64 = 1.0e6;
pub const MAX_THICKNESS: f64 = 1.0e5;
/// Output vertex budget (u32 indices; 6 output vertices per input vertex at most).
pub const MAX_OUTPUT_VERTICES: usize = 1 << 16;
/// Column-major identity (translation in elements 12..14, like `Pos::world`).
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Why a shape produced no geometry. `reason()` gives a stable census key.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeReject {
    /// Selection: the Thing has no SHAPE part data.
    NoShape,
    /// Selection: no GENERATED_MESH (collision hull or invisible collider, D108 T4).
    NoGeneratedMesh,
    /// Selection: no POS world matrix.
    NoPos,
    /// Selection: GENERATED_MESH is not visible in play mode (D108 T5).
    EditOnly,
    VertexBudget {
        vertices: usize,
    },
    LoopBudget {
        loops: usize,
    },
    NoLoops,
    /// Loop counts do not sum to the vertex count (None = the sum overflowed).
    LoopSumMismatch {
        sum: Option<u64>,
        vertices: usize,
    },
    /// A loop has fewer than 3 distinct consecutive vertices.
    LoopTooSmall {
        lp: usize,
        vertices: usize,
    },
    NonFinite,
    Oversized,
    /// Thickness not finite, not positive, or above MAX_THICKNESS.
    BadThickness,
    /// World matrix not finite or singular.
    BadTransform,
    ZeroAreaLoop {
        lp: usize,
    },
    /// Edges of one loop cross, touch or fold back.
    SelfIntersecting {
        lp: usize,
    },
    /// Edges of two different loops cross or touch.
    LoopsIntersect {
        a: usize,
        b: usize,
    },
    HoleOutsideOuter {
        lp: usize,
    },
    /// A hole lies inside another hole.
    NestedHoles {
        lp: usize,
    },
    /// Ear clipping found no ear (numerical or undetected degenerate input).
    Triangulation,
    /// Triangulated area differs from the polygon area.
    AreaMismatch,
    OutputBudget,
}

impl ShapeReject {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NoShape => "no_shape",
            Self::NoGeneratedMesh => "no_generated_mesh",
            Self::NoPos => "no_pos",
            Self::EditOnly => "edit_only",
            Self::VertexBudget { .. } => "vertex_budget",
            Self::LoopBudget { .. } => "loop_budget",
            Self::NoLoops => "no_loops",
            Self::LoopSumMismatch { .. } => "loop_sum_mismatch",
            Self::LoopTooSmall { .. } => "loop_too_small",
            Self::NonFinite => "non_finite",
            Self::Oversized => "oversized",
            Self::BadThickness => "bad_thickness",
            Self::BadTransform => "degenerate_transform",
            Self::ZeroAreaLoop { .. } => "zero_area_loop",
            Self::SelfIntersecting { .. } => "self_intersecting",
            Self::LoopsIntersect { .. } => "loops_intersect",
            Self::HoleOutsideOuter { .. } => "hole_outside_outer",
            Self::NestedHoles { .. } => "nested_holes",
            Self::Triangulation => "triangulation_failed",
            Self::AreaMismatch => "area_mismatch",
            Self::OutputBudget => "output_budget",
        }
    }

    /// True for the four selection outcomes (not drawn by design, not a geometry failure).
    pub fn is_selection(&self) -> bool {
        matches!(
            self,
            Self::NoShape | Self::NoGeneratedMesh | Self::NoPos | Self::EditOnly
        )
    }
}

impl std::fmt::Display for ShapeReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {self:?}", self.reason())
    }
}

impl std::error::Error for ShapeReject {}

/// Generated geometry in **world space** (the matrix is already applied, winding corrected
/// for mirrored transforms, normals by the inverse transpose). Pass [`IDENTITY`] to get
/// local-space geometry. Triangles are counter-clockwise seen from outside.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// Index ranges (into `indices`) per face group. All groups use `material` in v1.
    pub front: Range<usize>,
    pub back: Range<usize>,
    pub sides: Range<usize>,
    /// GENERATED_MESH.gfxMaterial: the render material for every face (D108 M1).
    pub material: Option<Descriptor>,
    /// GENERATED_MESH.bevel, recorded but not applied (no bevel in v1).
    pub bevel_unused: Option<Descriptor>,
    pub uv_offset: [f32; 4],
    /// det(world 3x3) < 0: triangle order was flipped.
    pub mirrored: bool,
    pub loops: usize,
    /// Loops whose stored winding was reversed to outer-CCW / holes-CW (local xy).
    pub rewound_loops: usize,
    /// Holes stored with the same winding as the outer loop (D108 P4 minority case).
    pub same_winding_holes: usize,
    /// Consecutive duplicate vertices dropped before triangulation.
    pub duplicate_vertices_removed: usize,
    /// Front-face area in local xy (outer minus holes).
    pub local_area: f64,
    /// Local half-depth used (PShape.thickness).
    pub thickness: f32,
    /// World-space z range of the slab (min, max).
    pub world_z: (f32, f32),
    pub label: &'static str,
}

impl ShapeMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
    pub fn material_guid(&self) -> Option<u32> {
        self.material.and_then(|d| d.guid)
    }
}

/// Selection plus extrusion for one parsed Thing (SHAPE + GENERATED_MESH + POS).
pub fn extrude_thing(thing: &ThingRecord) -> Result<ShapeMesh, ShapeReject> {
    let shape = thing.data.shape.as_ref().ok_or(ShapeReject::NoShape)?;
    let gm = thing
        .data
        .generated_mesh
        .as_ref()
        .ok_or(ShapeReject::NoGeneratedMesh)?;
    let pos = thing.data.pos.as_ref().ok_or(ShapeReject::NoPos)?;
    if !gm.play_mode_visible {
        return Err(ShapeReject::EditOnly);
    }
    extrude(shape, gm, &pos.world)
}

type P2 = [f64; 2];

fn orient(a: P2, b: P2, c: P2) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn signed_area(pts: &[P2], idx: &[usize]) -> f64 {
    let n = idx.len();
    let mut s = 0.0;
    for i in 0..n {
        let a = pts[idx[i]];
        let b = pts[idx[(i + 1) % n]];
        s += a[0] * b[1] - b[0] * a[1];
    }
    s * 0.5
}

fn on_segment(a: P2, b: P2, p: P2) -> bool {
    p[0] >= a[0].min(b[0])
        && p[0] <= a[0].max(b[0])
        && p[1] >= a[1].min(b[1])
        && p[1] <= a[1].max(b[1])
}

/// Closed-segment intersection (crossing, touching or collinear overlap).
fn segments_meet(a: P2, b: P2, c: P2, d: P2) -> bool {
    if a[0].max(b[0]) < c[0].min(d[0])
        || c[0].max(d[0]) < a[0].min(b[0])
        || a[1].max(b[1]) < c[1].min(d[1])
        || c[1].max(d[1]) < a[1].min(b[1])
    {
        return false;
    }
    let d1 = orient(a, b, c);
    let d2 = orient(a, b, d);
    let d3 = orient(c, d, a);
    let d4 = orient(c, d, b);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && on_segment(a, b, c))
        || (d2 == 0.0 && on_segment(a, b, d))
        || (d3 == 0.0 && on_segment(c, d, a))
        || (d4 == 0.0 && on_segment(c, d, b))
}

/// Even-odd point in polygon (p must not lie on the boundary).
fn inside(pts: &[P2], idx: &[usize], p: P2) -> bool {
    let n = idx.len();
    let mut c = false;
    for i in 0..n {
        let a = pts[idx[i]];
        let b = pts[idx[(i + 1) % n]];
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p[0] < x {
                c = !c;
            }
        }
    }
    c
}

/// Validates and normalises the polygon: returns deduplicated points, per-loop index
/// lists (outer CCW, holes CW) and counters (rewound, same-winding holes, removed).
struct Polygon {
    pts: Vec<P2>,
    loops: Vec<Vec<usize>>,
    rewound: usize,
    same_winding_holes: usize,
    removed: usize,
    area: f64,
}

fn validate(shape: &Shape) -> Result<Polygon, ShapeReject> {
    let n = shape.vertices.len();
    if n > MAX_INPUT_VERTICES || shape.vertex_count > MAX_INPUT_VERTICES {
        return Err(ShapeReject::VertexBudget {
            vertices: n.max(shape.vertex_count),
        });
    }
    if shape.loops.len() > MAX_LOOPS {
        return Err(ShapeReject::LoopBudget {
            loops: shape.loops.len(),
        });
    }
    if shape.loops.is_empty() {
        return Err(ShapeReject::NoLoops);
    }
    let sum = shape
        .loops
        .iter()
        .try_fold(0u64, |s, &c| s.checked_add(u64::from(c)));
    if sum != Some(n as u64) || shape.vertex_count != n {
        return Err(ShapeReject::LoopSumMismatch { sum, vertices: n });
    }
    let mut pts = Vec::with_capacity(n);
    for v in &shape.vertices {
        let (x, y) = (f64::from(v[0]), f64::from(v[1]));
        if !x.is_finite() || !y.is_finite() {
            return Err(ShapeReject::NonFinite);
        }
        if x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE {
            return Err(ShapeReject::Oversized);
        }
        pts.push([x, y]);
    }
    let mut loops = Vec::with_capacity(shape.loops.len());
    let mut start = 0usize;
    let mut removed = 0;
    for (lp, &count) in shape.loops.iter().enumerate() {
        // Sum already checked equal to n, so these additions cannot overflow.
        let end = start + count as usize;
        let mut idx: Vec<usize> = Vec::with_capacity(count as usize);
        for i in start..end {
            if idx.last().is_some_and(|&l| pts[l] == pts[i]) {
                removed += 1;
            } else {
                idx.push(i);
            }
        }
        while idx.len() > 1 && pts[idx[0]] == pts[*idx.last().unwrap_or(&0)] {
            idx.pop();
            removed += 1;
        }
        if idx.len() < 3 {
            return Err(ShapeReject::LoopTooSmall {
                lp,
                vertices: idx.len(),
            });
        }
        loops.push(idx);
        start = end;
    }
    // Zero-area loops, relative to the loop's extent.
    let mut areas = Vec::with_capacity(loops.len());
    for (lp, idx) in loops.iter().enumerate() {
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for &i in idx {
            for k in 0..2 {
                lo[k] = lo[k].min(pts[i][k]);
                hi[k] = hi[k].max(pts[i][k]);
            }
        }
        let extent = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        let a = signed_area(&pts, idx);
        if extent <= 0.0 || a.abs() <= 1e-9 * extent * extent {
            return Err(ShapeReject::ZeroAreaLoop { lp });
        }
        areas.push(a);
    }
    // Edge intersections: within a loop (non-adjacent edges, plus fold-backs of adjacent
    // ones), then between loops.
    for (lp, idx) in loops.iter().enumerate() {
        let m = idx.len();
        for i in 0..m {
            let (a, b) = (pts[idx[i]], pts[idx[(i + 1) % m]]);
            let c = pts[idx[(i + 2) % m]];
            // Adjacent edge (b, c) folds back over (a, b).
            if orient(a, b, c) == 0.0
                && (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1]) < 0.0
            {
                return Err(ShapeReject::SelfIntersecting { lp });
            }
            for j in i + 2..m {
                if i == 0 && j == m - 1 {
                    continue;
                }
                let (c, d) = (pts[idx[j]], pts[idx[(j + 1) % m]]);
                if segments_meet(a, b, c, d) {
                    return Err(ShapeReject::SelfIntersecting { lp });
                }
            }
        }
    }
    for la in 0..loops.len() {
        for lb in la + 1..loops.len() {
            let (ia, ib) = (&loops[la], &loops[lb]);
            for i in 0..ia.len() {
                let (a, b) = (pts[ia[i]], pts[ia[(i + 1) % ia.len()]]);
                for j in 0..ib.len() {
                    let (c, d) = (pts[ib[j]], pts[ib[(j + 1) % ib.len()]]);
                    if segments_meet(a, b, c, d) {
                        return Err(ShapeReject::LoopsIntersect { a: la, b: lb });
                    }
                }
            }
        }
    }
    // Containment: every hole inside the outer loop and outside every other hole.
    for lp in 1..loops.len() {
        let p = pts[loops[lp][0]];
        if !inside(&pts, &loops[0], p) {
            return Err(ShapeReject::HoleOutsideOuter { lp });
        }
        for (other, hole) in loops.iter().enumerate().skip(1) {
            if other != lp && inside(&pts, hole, p) {
                return Err(ShapeReject::NestedHoles { lp });
            }
        }
    }
    // Winding normalisation: outer CCW, holes CW.
    let mut rewound = 0;
    let mut same_winding_holes = 0;
    let outer_sign = areas[0] > 0.0;
    let mut area = 0.0;
    for (lp, idx) in loops.iter_mut().enumerate() {
        let ccw = areas[lp] > 0.0;
        if lp > 0 && ccw == outer_sign {
            same_winding_holes += 1;
        }
        let want_ccw = lp == 0;
        if ccw != want_ccw {
            idx.reverse();
            rewound += 1;
        }
        area += if lp == 0 {
            areas[lp].abs()
        } else {
            -areas[lp].abs()
        };
    }
    Ok(Polygon {
        pts,
        loops,
        rewound,
        same_winding_holes,
        removed,
        area,
    })
}

/// Is `m` inside the interior cone at vertex `p` (neighbours `a` before, `b` after) of a
/// CCW boundary?
fn in_cone(a: P2, p: P2, b: P2, m: P2) -> bool {
    if orient(a, p, b) >= 0.0 {
        orient(a, p, m) > 0.0 && orient(p, b, m) > 0.0
    } else {
        orient(a, p, m) > 0.0 || orient(p, b, m) > 0.0
    }
}

/// Merges holes into the outer boundary with bridge edges (holes processed by decreasing
/// maximum x; each bridge joins the hole's rightmost vertex to a visible boundary vertex).
fn bridge_holes(pts: &[P2], loops: &[Vec<usize>]) -> Option<Vec<usize>> {
    let mut poly = loops[0].clone();
    let mut holes: Vec<(usize, usize)> = loops[1..]
        .iter()
        .enumerate()
        .map(|(h, idx)| {
            let mut best = 0;
            for (k, &i) in idx.iter().enumerate() {
                let (p, q) = (pts[i], pts[idx[best]]);
                if p[0] > q[0] || (p[0] == q[0] && p[1] < q[1]) {
                    best = k;
                }
            }
            (h + 1, best)
        })
        .collect();
    holes.sort_by(|x, y| {
        let px = pts[loops[x.0][x.1]];
        let py = pts[loops[y.0][y.1]];
        py[0]
            .total_cmp(&px[0])
            .then(px[1].total_cmp(&py[1]))
            .then(x.0.cmp(&y.0))
    });
    for (h, mk) in holes {
        let hole = &loops[h];
        let m = pts[hole[mk]];
        let n = poly.len();
        // Nearest boundary edge hit by the ray from m towards +x (edges running upward,
        // i.e. with the interior on their -x side).
        let mut hit: Option<(f64, usize)> = None;
        for i in 0..n {
            let (a, b) = (pts[poly[i]], pts[poly[(i + 1) % n]]);
            if !(a[1] <= m[1] && m[1] <= b[1] && a[1] < b[1]) {
                continue;
            }
            let x = a[0] + (m[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if x >= m[0] && hit.is_none_or(|(hx, _)| x < hx) {
                hit = Some((x, i));
            }
        }
        let (ix, ei) = hit?;
        let ipt = [ix, m[1]];
        let (a, b) = (pts[poly[ei]], pts[poly[(ei + 1) % n]]);
        let mut target = if a == ipt {
            Some(poly[ei])
        } else if b == ipt {
            Some(poly[(ei + 1) % n])
        } else {
            None
        };
        if target.is_none() {
            let p_id = if a[0] > b[0] {
                poly[ei]
            } else {
                poly[(ei + 1) % n]
            };
            let p = pts[p_id];
            // Any boundary vertex inside triangle (m, i, p) may block the bridge; the one
            // with the smallest angle to the ray is visible.
            let (t0, t1, t2) = if orient(m, ipt, p) >= 0.0 {
                (m, ipt, p)
            } else {
                (m, p, ipt)
            };
            let mut best = p_id;
            let mut best_key = (f64::MIN, f64::MAX);
            let dx = p[0] - m[0];
            let dy = p[1] - m[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                best_key = (dx / len, len);
            }
            for &v in &poly {
                let q = pts[v];
                if v == p_id || q == m || q == p {
                    continue;
                }
                if orient(t0, t1, q) >= 0.0 && orient(t1, t2, q) >= 0.0 && orient(t2, t0, q) >= 0.0
                {
                    let (qx, qy) = (q[0] - m[0], q[1] - m[1]);
                    let ql = (qx * qx + qy * qy).sqrt();
                    if ql == 0.0 {
                        continue;
                    }
                    let key = (qx / ql, ql);
                    if key.0 > best_key.0 || (key.0 == best_key.0 && key.1 < best_key.1) {
                        best_key = key;
                        best = v;
                    }
                }
            }
            target = Some(best);
        }
        let t = target?;
        // A vertex id can occur twice after earlier bridges: pick the occurrence whose
        // interior cone contains m.
        let occurrences: Vec<usize> = (0..n).filter(|&k| poly[k] == t).collect();
        let pos = occurrences
            .iter()
            .copied()
            .find(|&k| {
                in_cone(
                    pts[poly[(k + n - 1) % n]],
                    pts[poly[k]],
                    pts[poly[(k + 1) % n]],
                    m,
                )
            })
            .or(occurrences.first().copied())?;
        let mut merged = Vec::with_capacity(n + hole.len() + 2);
        merged.extend_from_slice(&poly[..=pos]);
        for k in 0..=hole.len() {
            merged.push(hole[(mk + k) % hole.len()]);
        }
        merged.push(poly[pos]);
        merged.extend_from_slice(&poly[pos + 1..]);
        poly = merged;
    }
    Some(poly)
}

/// Ear clipping of a CCW boundary (possibly with bridge duplicates). Returns CCW triangles.
fn ear_clip(pts: &[P2], poly: Vec<usize>) -> Option<Vec<[usize; 3]>> {
    let mut v = poly;
    let mut tris = Vec::with_capacity(v.len().saturating_sub(2));
    let mut cursor = 0usize;
    let mut misses = 0usize;
    while v.len() > 3 {
        let n = v.len();
        let i = cursor % n;
        let (ia, ib, ic) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
        let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
        let cr = orient(a, b, c);
        let mut ear = cr > 0.0;
        if ear {
            for &q_id in &v {
                let q = pts[q_id];
                if q == a || q == b || q == c {
                    continue;
                }
                if orient(a, b, q) >= 0.0 && orient(b, c, q) >= 0.0 && orient(c, a, q) >= 0.0 {
                    ear = false;
                    break;
                }
            }
        }
        if ear {
            tris.push([ia, ib, ic]);
            v.remove(i);
            misses = 0;
            cursor = if i == 0 { 0 } else { i - 1 };
            continue;
        }
        misses += 1;
        cursor = i + 1;
        if misses >= n {
            // No ear in a full pass: drop one zero-area (collinear) vertex if any, which
            // adds no area; otherwise give up.
            let k = (0..n).find(|&k| {
                orient(pts[v[(k + n - 1) % n]], pts[v[k]], pts[v[(k + 1) % n]]) == 0.0
            })?;
            v.remove(k);
            misses = 0;
            cursor = 0;
        }
    }
    if v.len() == 3 {
        let cr = orient(pts[v[0]], pts[v[1]], pts[v[2]]);
        if cr > 0.0 {
            tris.push([v[0], v[1], v[2]]);
        } else if cr < 0.0 {
            return None;
        }
    }
    Some(tris)
}

/// Front-cap triangulation of a validated polygon (CCW triangles in local xy).
fn triangulate(poly: &Polygon) -> Result<Vec<[usize; 3]>, ShapeReject> {
    let merged = bridge_holes(&poly.pts, &poly.loops).ok_or(ShapeReject::Triangulation)?;
    let tris = ear_clip(&poly.pts, merged).ok_or(ShapeReject::Triangulation)?;
    let sum: f64 = tris
        .iter()
        .map(|t| orient(poly.pts[t[0]], poly.pts[t[1]], poly.pts[t[2]]) * 0.5)
        .sum();
    if (sum - poly.area).abs() > 1e-6 * poly.area.abs().max(1e-12) {
        return Err(ShapeReject::AreaMismatch);
    }
    Ok(tris)
}

struct Xform {
    l: [[f64; 3]; 3], // l[row][col]
    t: [f64; 3],
    n: [[f64; 3]; 3], // inverse transpose
    det: f64,
}

impl Xform {
    fn new(m: &[f32; 16]) -> Result<Self, ShapeReject> {
        if m.iter().any(|v| !v.is_finite()) {
            return Err(ShapeReject::BadTransform);
        }
        let g = |r: usize, c: usize| f64::from(m[c * 4 + r]);
        let l = [
            [g(0, 0), g(0, 1), g(0, 2)],
            [g(1, 0), g(1, 1), g(1, 2)],
            [g(2, 0), g(2, 1), g(2, 2)],
        ];
        let cof = |r: usize, c: usize| {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
            let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            l[r1][c1] * l[r2][c2] - l[r1][c2] * l[r2][c1]
        };
        let det = l[0][0] * cof(0, 0) + l[0][1] * cof(0, 1) + l[0][2] * cof(0, 2);
        let scale = l.iter().flatten().fold(0.0f64, |s, v| s.max(v.abs()));
        if !det.is_finite() || scale == 0.0 || det.abs() <= 1e-12 * scale * scale * scale {
            return Err(ShapeReject::BadTransform);
        }
        // inverse transpose = cofactor matrix / det.
        let mut n = [[0.0; 3]; 3];
        for (r, row) in n.iter_mut().enumerate() {
            for (c, v) in row.iter_mut().enumerate() {
                *v = cof(r, c) / det;
            }
        }
        Ok(Self {
            l,
            t: [g(0, 3), g(1, 3), g(2, 3)],
            n,
            det,
        })
    }
    fn linear(&self, p: [f64; 3]) -> [f64; 3] {
        let mut o = [0.0; 3];
        for (r, v) in o.iter_mut().enumerate() {
            *v = self.l[r][0] * p[0] + self.l[r][1] * p[1] + self.l[r][2] * p[2];
        }
        o
    }
    fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let o = self.linear(p);
        [o[0] + self.t[0], o[1] + self.t[1], o[2] + self.t[2]]
    }
    fn normal(&self, v: [f64; 3]) -> [f64; 3] {
        let mut o = [0.0; 3];
        for (r, x) in o.iter_mut().enumerate() {
            *x = self.n[r][0] * v[0] + self.n[r][1] * v[1] + self.n[r][2] * v[2];
        }
        let len = (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt();
        if len > 0.0 {
            [o[0] / len, o[1] / len, o[2] / len]
        } else {
            o
        }
    }
}

fn f3(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

fn len3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Extrudes one SHAPE into a closed slab: front cap at local z=+thickness, back cap at
/// -thickness, one flat-shaded quad per loop edge. `world` is `Pos::world` (column-major).
pub fn extrude(
    shape: &Shape,
    generated: &GeneratedMesh,
    world: &[f32; 16],
) -> Result<ShapeMesh, ShapeReject> {
    let t = f64::from(shape.thickness);
    if !t.is_finite() || t <= 0.0 || t > MAX_THICKNESS {
        return Err(ShapeReject::BadThickness);
    }
    let xf = Xform::new(world)?;
    let poly = validate(shape)?;
    let tris = triangulate(&poly)?;
    let ring: Vec<usize> = poly.loops.iter().flatten().copied().collect();
    let edges = ring.len();
    let out_vertices = ring
        .len()
        .checked_mul(2)
        .and_then(|v| edges.checked_mul(4).and_then(|s| v.checked_add(s)))
        .ok_or(ShapeReject::OutputBudget)?;
    if out_vertices > MAX_OUTPUT_VERTICES {
        return Err(ShapeReject::OutputBudget);
    }
    let off = generated.uv_offset;
    let planar = |x: f64, y: f64| {
        let w = xf.linear([x, y, 0.0]);
        [
            (w[0] * UV_SCALE + f64::from(off[0])) as f32,
            (f64::from(off[1]) - w[1] * UV_SCALE) as f32,
        ]
    };
    let mut positions = Vec::with_capacity(out_vertices);
    let mut normals = Vec::with_capacity(out_vertices);
    let mut uv0 = Vec::with_capacity(out_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(tris.len() * 6 + edges * 6);
    // Point id -> front vertex index.
    let mut slot = vec![u32::MAX; poly.pts.len()];
    for (k, &i) in ring.iter().enumerate() {
        slot[i] = k as u32;
    }
    for (z, nz) in [(t, 1.0), (-t, -1.0)] {
        let nrm = f3(xf.normal([0.0, 0.0, nz]));
        for &i in &ring {
            let [x, y] = poly.pts[i];
            positions.push(f3(xf.point([x, y, z])));
            normals.push(nrm);
            uv0.push(planar(x, y));
        }
    }
    let back_base = ring.len() as u32;
    for tri in &tris {
        let [a, b, c] = tri.map(|i| slot[i]);
        indices.extend_from_slice(&[a, b, c]);
    }
    let front = 0..indices.len();
    for tri in &tris {
        let [a, b, c] = tri.map(|i| slot[i] + back_base);
        indices.extend_from_slice(&[a, c, b]);
    }
    let back = front.end..indices.len();
    let depth_v = len3(xf.linear([0.0, 0.0, 2.0 * t])) * UV_SCALE;
    for lp in &poly.loops {
        let m = lp.len();
        let mut u = 0.0f64;
        for k in 0..m {
            let [ax, ay] = poly.pts[lp[k]];
            let [bx, by] = poly.pts[lp[(k + 1) % m]];
            let (dx, dy) = (bx - ax, by - ay);
            // Outward side normal: right of the edge for CCW outers and CW holes.
            let nrm = f3(xf.normal([dy, -dx, 0.0]));
            let u1 = u + len3(xf.linear([dx, dy, 0.0])) * UV_SCALE;
            let base = positions.len() as u32;
            for (p, uv) in [
                ([ax, ay, -t], [u, depth_v]),
                ([bx, by, -t], [u1, depth_v]),
                ([bx, by, t], [u1, 0.0]),
                ([ax, ay, t], [u, 0.0]),
            ] {
                positions.push(f3(xf.point(p)));
                normals.push(nrm);
                uv0.push([uv[0] as f32, uv[1] as f32]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            u = u1;
        }
    }
    let sides = back.end..indices.len();
    let mirrored = xf.det < 0.0;
    if mirrored {
        for tri in indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    if positions.iter().flatten().any(|v| !v.is_finite()) {
        return Err(ShapeReject::NonFinite);
    }
    let (mut zmin, mut zmax) = (f32::MAX, f32::MIN);
    for p in &positions {
        zmin = zmin.min(p[2]);
        zmax = zmax.max(p[2]);
    }
    Ok(ShapeMesh {
        positions,
        normals,
        uv0,
        indices,
        front,
        back,
        sides,
        material: generated.gfx_material,
        bevel_unused: generated.bevel,
        uv_offset: off,
        mirrored,
        loops: poly.loops.len(),
        rewound_loops: poly.rewound,
        same_winding_holes: poly.same_winding_holes,
        duplicate_vertices_removed: poly.removed,
        local_area: poly.area,
        thickness: shape.thickness,
        world_z: (zmin, zmax),
        label: PREVIEW_LABEL,
    })
}
