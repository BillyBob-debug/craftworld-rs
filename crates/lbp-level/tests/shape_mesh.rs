//! D114 synthetic recipes for SHAPE extrusion v1 (no game bytes).
use lbp_level::parts::{GeneratedMesh, Shape, ShapeColor};
use lbp_level::serial::Descriptor;
use lbp_level::shape_mesh::{
    extrude, outline_triangles_lenient, ShapeMesh, ShapeReject, IDENTITY, PREVIEW_LABEL,
};
use std::collections::HashMap;

fn shape(loops: &[&[[f32; 2]]], thickness: f32) -> Shape {
    let vertices: Vec<[f32; 3]> = loops
        .iter()
        .flat_map(|l| l.iter().map(|p| [p[0], p[1], 0.0]))
        .collect();
    Shape {
        vertex_count: vertices.len(),
        requires_z: false,
        vertices,
        loops: loops.iter().map(|l| l.len() as u32).collect(),
        material: None,
        old_material: None,
        thickness,
        mass_depth: 1.0,
        color: ShapeColor::Packed(0xffff_ffff),
        brightness: 0.0,
        bevel_size: 10.0,
        com: None,
        behavior: 0,
        flags: 0,
        contacts: 0,
    }
}

fn gm() -> GeneratedMesh {
    let d = |g| Descriptor {
        flags: Some(0),
        kind_byte: 2,
        guid: Some(g),
        sha1: None,
    };
    GeneratedMesh {
        gfx_material: Some(d(1234)),
        bevel: Some(d(99327)),
        uv_offset: [0.25, 0.5, 0.0, 0.0],
        plan_guid: None,
        play_mode_visible: true,
    }
}

const SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
const HOLE_CW: [[f32; 2]; 4] = [[25.0, 25.0], [25.0, 75.0], [75.0, 75.0], [75.0, 25.0]];
const L_SHAPE: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [200.0, 0.0],
    [200.0, 50.0],
    [50.0, 50.0],
    [50.0, 200.0],
    [0.0, 200.0],
];

fn sub(a: [f32; 3], b: [f32; 3]) -> [f64; 3] {
    [
        f64::from(a[0]) - f64::from(b[0]),
        f64::from(a[1]) - f64::from(b[1]),
        f64::from(a[2]) - f64::from(b[2]),
    ]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn tri(m: &ShapeMesh, t: usize) -> [usize; 3] {
    [
        m.indices[3 * t] as usize,
        m.indices[3 * t + 1] as usize,
        m.indices[3 * t + 2] as usize,
    ]
}

/// Area of the front-cap triangles (|cross| / 2 in world space).
fn front_area(m: &ShapeMesh) -> f64 {
    (m.front.start / 3..m.front.end / 3)
        .map(|t| {
            let [a, b, c] = tri(m, t);
            let n = cross(
                sub(m.positions[b], m.positions[a]),
                sub(m.positions[c], m.positions[a]),
            );
            dot(n, n).sqrt() / 2.0
        })
        .sum()
}

/// Signed volume (divergence theorem); positive when triangles wind outward.
fn volume(m: &ShapeMesh) -> f64 {
    (0..m.triangle_count())
        .map(|t| {
            let [a, b, c] = tri(m, t);
            let p = |i: usize| m.positions[i].map(f64::from);
            dot(p(a), cross(p(b), p(c))) / 6.0
        })
        .sum()
}

/// Edge-manifold and consistently oriented after welding equal positions: every directed
/// edge occurs once and its reverse once.
fn assert_closed_manifold(m: &ShapeMesh) {
    let mut weld: HashMap<[u32; 3], usize> = HashMap::new();
    let ids: Vec<usize> = m
        .positions
        .iter()
        .map(|p| {
            let k = p.map(f32::to_bits);
            let n = weld.len();
            *weld.entry(k).or_insert(n)
        })
        .collect();
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    for t in 0..m.triangle_count() {
        let [a, b, c] = tri(m, t).map(|i| ids[i]);
        for e in [(a, b), (b, c), (c, a)] {
            *directed.entry(e).or_default() += 1;
        }
    }
    for (&(a, b), &n) in &directed {
        assert_eq!(n, 1, "directed edge used {n} times");
        assert_eq!(directed.get(&(b, a)), Some(&1), "edge without a twin");
    }
}

/// Every triangle's geometric normal agrees with its vertex normals.
fn assert_normals_consistent(m: &ShapeMesh) {
    for t in 0..m.triangle_count() {
        let [a, b, c] = tri(m, t);
        let g = cross(
            sub(m.positions[b], m.positions[a]),
            sub(m.positions[c], m.positions[a]),
        );
        for i in [a, b, c] {
            let n = m.normals[i].map(f64::from);
            assert!(dot(g, n) > 0.0, "triangle {t} faces against its normal");
            assert!((dot(n, n) - 1.0).abs() < 1e-5);
        }
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * b.abs().max(1.0)
}

fn check_solid(m: &ShapeMesh, area: f64, thickness: f64, det: f64) {
    assert_eq!(m.label, PREVIEW_LABEL);
    assert_eq!(m.positions.len(), m.normals.len());
    assert_eq!(m.positions.len(), m.uv0.len());
    assert!(close(m.local_area, area), "{} vs {area}", m.local_area);
    assert_closed_manifold(m);
    assert_normals_consistent(m);
    let v = volume(m);
    assert!(close(v, area * 2.0 * thickness * det.abs()), "volume {v}");
}

#[test]
fn square_is_a_closed_slab() {
    let m = extrude(&shape(&[&SQUARE], 90.0), &gm(), &IDENTITY).unwrap();
    check_solid(&m, 10_000.0, 90.0, 1.0);
    assert_eq!(m.triangle_count(), 2 + 2 + 8);
    assert!(close(front_area(&m), 10_000.0));
    assert_eq!(m.world_z, (-90.0, 90.0));
    assert_eq!(m.material_guid(), Some(1234));
    assert_eq!(m.bevel_unused.and_then(|d| d.guid), Some(99327));
    // Front faces +z, back -z.
    let f = tri(&m, m.front.start / 3)[0];
    assert_eq!(m.normals[f], [0.0, 0.0, 1.0]);
    let b = tri(&m, m.back.start / 3)[0];
    assert_eq!(m.normals[b], [0.0, 0.0, -1.0]);
    // Planar UV: (0,0) maps to the uv offset, v decreases with y.
    let i = m.positions.iter().position(|p| *p == [0.0, 100.0, 90.0]);
    assert_eq!(m.uv0[i.unwrap()], [0.25, 0.5 - 100.0 / 500.0]);
    assert!(!m.mirrored);
}

#[test]
fn clockwise_input_gives_the_same_solid() {
    let mut cw = SQUARE;
    cw.reverse();
    let m = extrude(&shape(&[&cw], 10.0), &gm(), &IDENTITY).unwrap();
    assert_eq!(m.rewound_loops, 1);
    check_solid(&m, 10_000.0, 10.0, 1.0);
}

#[test]
fn square_with_hole() {
    let m = extrude(&shape(&[&SQUARE, &HOLE_CW], 90.0), &gm(), &IDENTITY).unwrap();
    check_solid(&m, 7_500.0, 90.0, 1.0);
    assert!(close(front_area(&m), 7_500.0));
    assert_eq!(m.loops, 2);
    assert_eq!(m.rewound_loops, 0);
    // 8 + 2*2 - 2 = 8 cap triangles per cap, 8 side quads.
    assert_eq!(m.triangle_count(), 8 + 8 + 16);
    // Same-winding hole is accepted and rewound.
    let mut ccw = HOLE_CW;
    ccw.reverse();
    let m = extrude(&shape(&[&SQUARE, &ccw], 90.0), &gm(), &IDENTITY).unwrap();
    assert_eq!(m.same_winding_holes, 1);
    check_solid(&m, 7_500.0, 90.0, 1.0);
}

#[test]
fn several_holes() {
    let big = [[0.0, 0.0], [300.0, 0.0], [300.0, 100.0], [0.0, 100.0]];
    let h = |x: f32| [[x, 25.0], [x, 75.0], [x + 50.0, 75.0], [x + 50.0, 25.0]];
    let (a, b, c) = (h(25.0), h(125.0), h(225.0));
    let m = extrude(&shape(&[&big, &a, &b, &c], 50.0), &gm(), &IDENTITY).unwrap();
    check_solid(&m, 30_000.0 - 3.0 * 2_500.0, 50.0, 1.0);
}

#[test]
fn concave_l() {
    let m = extrude(&shape(&[&L_SHAPE], 40.0), &gm(), &IDENTITY).unwrap();
    let area = 200.0 * 50.0 + 50.0 * 150.0;
    check_solid(&m, area, 40.0, 1.0);
    assert!(close(front_area(&m), area));
}

#[test]
fn mirrored_and_scaled_transform() {
    // Column-major: x mirrored, y scaled by 2, translated.
    let mut w = IDENTITY;
    w[0] = -1.0;
    w[5] = 2.0;
    w[12] = 10.0;
    w[14] = -200.0;
    let m = extrude(&shape(&[&SQUARE, &HOLE_CW], 90.0), &gm(), &w).unwrap();
    assert!(m.mirrored);
    check_solid(&m, 7_500.0, 90.0, -2.0);
    assert!(close(front_area(&m), 15_000.0));
    assert_eq!(m.world_z, (-290.0, -110.0));
    // Clockwise input under the same mirror gives the same volume.
    let mut cw = L_SHAPE;
    cw.reverse();
    let m = extrude(&shape(&[&cw], 10.0), &gm(), &w).unwrap();
    check_solid(&m, 17_500.0, 10.0, -2.0);
}

#[test]
fn rotated_transform_keeps_outward_normals() {
    let (s, c) = (0.6f32, 0.8f32);
    let mut w = IDENTITY;
    w[0] = c;
    w[1] = s;
    w[4] = -s;
    w[5] = c;
    let m = extrude(&shape(&[&L_SHAPE], 90.0), &gm(), &w).unwrap();
    check_solid(&m, 17_500.0, 90.0, 1.0);
}

fn reject(s: &Shape) -> &'static str {
    extrude(s, &gm(), &IDENTITY).unwrap_err().reason()
}

#[test]
fn degenerate_inputs_are_refused() {
    // Collinear outline.
    let line = [[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]];
    assert_eq!(reject(&shape(&[&line], 10.0)), "zero_area_loop");
    // Bow tie.
    let bow = [[0.0, 0.0], [100.0, 100.0], [100.0, 0.0], [0.0, 50.0]];
    assert_eq!(reject(&shape(&[&bow], 10.0)), "self_intersecting");
    // Two vertices after duplicate removal.
    let dup = [[0.0, 0.0], [0.0, 0.0], [10.0, 0.0], [0.0, 0.0]];
    assert_eq!(reject(&shape(&[&dup], 10.0)), "loop_too_small");
    // Hole outside, hole crossing the outer edge, nested holes.
    let far = [
        [500.0, 500.0],
        [500.0, 550.0],
        [550.0, 550.0],
        [550.0, 500.0],
    ];
    assert_eq!(reject(&shape(&[&SQUARE, &far], 10.0)), "hole_outside_outer");
    let cut = [[50.0, 50.0], [50.0, 150.0], [150.0, 150.0], [150.0, 50.0]];
    assert_eq!(reject(&shape(&[&SQUARE, &cut], 10.0)), "loops_intersect");
    let inner = [[40.0, 40.0], [40.0, 60.0], [60.0, 60.0], [60.0, 40.0]];
    assert_eq!(
        reject(&shape(&[&SQUARE, &HOLE_CW, &inner], 10.0)),
        "nested_holes"
    );
    // Loops not summing to the vertex count.
    let mut s = shape(&[&SQUARE], 10.0);
    s.loops = vec![3];
    assert_eq!(reject(&s), "loop_sum_mismatch");
    s.loops = vec![u32::MAX, u32::MAX];
    assert_eq!(reject(&s), "loop_sum_mismatch");
    s.loops = vec![];
    assert_eq!(reject(&s), "no_loops");
    // NaN, oversized, bad thickness, singular transform, budgets.
    let mut s = shape(&[&SQUARE], 10.0);
    s.vertices[1][0] = f32::NAN;
    assert_eq!(reject(&s), "non_finite");
    let mut s = shape(&[&SQUARE], 10.0);
    s.vertices[2][1] = 1.0e9;
    assert_eq!(reject(&s), "oversized");
    assert_eq!(reject(&shape(&[&SQUARE], 0.0)), "bad_thickness");
    assert_eq!(reject(&shape(&[&SQUARE], f32::INFINITY)), "bad_thickness");
    let mut w = IDENTITY;
    w[10] = 0.0;
    let e = extrude(&shape(&[&SQUARE], 10.0), &gm(), &w).unwrap_err();
    assert_eq!(e, ShapeReject::BadTransform);
    let many: Vec<[f32; 2]> = (0..5000)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 5000.0;
            [a.cos() * 100.0, a.sin() * 100.0]
        })
        .collect();
    assert_eq!(reject(&shape(&[&many], 10.0)), "vertex_budget");
    let tiny = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let loops: Vec<&[[f32; 2]]> = (0..65).map(|_| &tiny[..]).collect();
    assert_eq!(reject(&shape(&loops, 10.0)), "loop_budget");
}

#[test]
fn duplicate_closing_vertex_is_dropped() {
    let closed = [
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 100.0],
        [0.0, 100.0],
        [0.0, 0.0],
    ];
    let m = extrude(&shape(&[&closed], 10.0), &gm(), &IDENTITY).unwrap();
    assert_eq!(m.duplicate_vertices_removed, 1);
    check_solid(&m, 10_000.0, 10.0, 1.0);
}

#[test]
fn many_sided_concave_star_with_hole() {
    let mut star = Vec::new();
    for i in 0..40 {
        let a = i as f64 * std::f64::consts::TAU / 40.0;
        let r = if i % 2 == 0 { 300.0 } else { 150.0 };
        star.push([(a.cos() * r) as f32, (a.sin() * r) as f32]);
    }
    let hole = [[-50.0, -50.0], [-50.0, 50.0], [50.0, 50.0], [50.0, -50.0]];
    let s = shape(&[&star, &hole], 90.0);
    let m = extrude(&s, &gm(), &IDENTITY).unwrap();
    let mut area = 0.0;
    for i in 0..star.len() {
        let (a, b) = (star[i], star[(i + 1) % star.len()]);
        area += f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1]);
    }
    let area = area / 2.0 - 10_000.0;
    check_solid(&m, area, 90.0, 1.0);
}

/// Area of an outline's triangles, checking they are all counter-clockwise.
fn filled_area(s: &Shape) -> (f64, Option<&'static str>) {
    let (o, why) = outline_triangles_lenient(s).unwrap();
    let mut sum = 0.0;
    for t in &o.triangles {
        let [a, b, c] = t.map(|k| o.points[k].map(f64::from));
        let x = 0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]));
        assert!(x > 0.0);
        sum += x;
    }
    assert!((sum - o.area).abs() < 1e-6 * sum);
    (sum, why.map(|w| w.reason()))
}

#[test]
fn lenient_outline_fills_crossing_loops_by_even_odd() {
    // Valid shapes take the strict path.
    assert_eq!(filled_area(&shape(&[&L_SHAPE], 10.0)).1, None);
    let (a, why) = filled_area(&shape(&[&SQUARE, &HOLE_CW], 10.0));
    assert_eq!((a, why), (7_500.0, None));
    // Bow tie: two lobes meeting at (100/3, 100/3), 2,500/3 + 10,000/3.
    let bow = [[0.0, 0.0], [100.0, 100.0], [100.0, 0.0], [0.0, 50.0]];
    let (a, why) = filled_area(&shape(&[&bow], 10.0));
    assert_eq!(why, Some("self_intersecting"));
    assert!((a - 12_500.0 / 3.0).abs() < 1e-3, "{a}");
    // A "hole" crossing the outer edge: the overlap is empty, the rest filled.
    let cut = [[50.0, 50.0], [50.0, 150.0], [150.0, 150.0], [150.0, 50.0]];
    let (a, why) = filled_area(&shape(&[&SQUARE, &cut], 10.0));
    assert_eq!(why, Some("loops_intersect"));
    assert!((a - 15_000.0).abs() < 1e-3, "{a}");
    // A hole outside the outer loop becomes a second solid.
    let far = [
        [500.0, 500.0],
        [500.0, 550.0],
        [550.0, 550.0],
        [550.0, 500.0],
    ];
    let (a, _) = filled_area(&shape(&[&SQUARE, &far], 10.0));
    assert!((a - 12_500.0).abs() < 1e-3, "{a}");
    // Shapes with no area are still refused.
    let line = [[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]];
    assert_eq!(
        outline_triangles_lenient(&shape(&[&line], 10.0))
            .unwrap_err()
            .reason(),
        "zero_area_loop"
    );
}
