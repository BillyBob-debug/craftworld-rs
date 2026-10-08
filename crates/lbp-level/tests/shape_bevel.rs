//! D122 synthetic recipes for the RBevel reader and the bevelled extrusion (no game bytes).
mod common;
use common::{rev, W};
use lbp_level::bevel::{read_payload, Bevel, BevelVertex, MappingMode};
use lbp_level::parts::{GeneratedMesh, Shape, ShapeColor};
use lbp_level::serial::{Descriptor, Stop};
use lbp_level::shape_mesh::{
    extrude, extrude_bevelled, profile_depth, BevelOptions, BevelOutcome, BevelSizeRule, FaceKind,
    ShapeMesh, BEVEL_LABEL, IDENTITY, PREVIEW_LABEL,
};
use std::collections::HashMap;

// ---------------------------------------------------------------- reader recipes

fn vertex(w: &mut W, y: f32, z: f32, slot: i8, mode: u8) {
    w.f32(y).f32(z).f32(1.0).u8(1).u8(slot as u8).u8(mode);
}

/// RBevel payload for head >= 0x22f (descriptors with a flags word).
fn payload(flags: u8, verts: &[(f32, f32, i8, u8)]) -> Vec<u8> {
    let mut w = W::new(flags);
    w.u32(4).res_none().res_guid(10812).res_none();
    // Slot 3: SHA1-only descriptor.
    w.u32(0).u8(1).raw(&[0xab; 20]);
    w.u32(4).f32(1.0).f32(1.0).f32(2.0).f32(1.0);
    w.f32(45.0);
    w.u32(verts.len() as u32);
    for &(y, z, s, m) in verts {
        vertex(&mut w, y, z, s, m);
    }
    w.u8(1).u8(0).f32(0.25).f32(0.3).f32(10.0).u8(0);
    w.res_none();
    w.f32(1.0);
    w.b
}

const CHAMFER: [(f32, f32, i8, u8); 4] = [
    (-0.5, -1.0, 1, 4),
    (0.0, -0.5, 1, 4),
    (0.0, 0.5, 1, 4),
    (-0.5, 1.0, 0, 4),
];

#[test]
fn reads_uncompressed_and_compressed_payloads() {
    for flags in [0u8, 7] {
        let b = read_payload(&payload(flags, &CHAMFER), rev(0x3f8, flags), &|| false).unwrap();
        assert_eq!(b.materials[0], None);
        assert_eq!(b.materials[1].and_then(|d| d.guid), Some(10812));
        assert_eq!(b.materials[3].and_then(|d| d.sha1), Some([0xab; 20]));
        assert_eq!(b.uv_scales, [1.0, 1.0, 2.0, 1.0]);
        assert_eq!(b.auto_smooth_cutoff_angle, 45.0);
        assert_eq!(b.vertices.len(), 4);
        assert_eq!(b.vertices[3].z, 1.0);
        assert_eq!(b.vertices[0].gmat_slot, 1);
        assert_eq!(b.vertices[0].mapping_mode(), Some(MappingMode::Planarxz01));
        assert!(b.include_backface && !b.smooth_with_front && !b.spongy);
        assert_eq!(
            (b.relax_strength, b.sub_div_radius, b.fixed_bevel_size),
            (0.25, 0.3, 10.0)
        );
        assert_eq!(b.soft_physics_settings, None);
        assert_eq!(b.texture_repeats, 1.0);
        assert_eq!(b.trailing_bytes, 0);
    }
}

#[test]
fn old_heads_swap_descriptor_bits_and_drop_the_flags_word() {
    // Head 0x132: no flags word; kind bit 1 = GUID, bit 2 = SHA1 (class pointer swap).
    let mut w = W::new(0);
    w.u32(4).u8(1).u32(17482).u8(2).raw(&[1; 20]).u8(0).u8(0);
    w.u32(4).f32(1.0).f32(1.0).f32(1.0).f32(1.0).f32(90.0);
    w.u32(2);
    vertex(&mut w, 0.0, -1.0, 0, 1);
    vertex(&mut w, 0.0, 1.0, 0, 9);
    w.u8(0)
        .u8(1)
        .f32(0.0)
        .f32(0.3)
        .f32(-1.0)
        .u8(1)
        .u8(0)
        .f32(1.0);
    w.u8(0xee); // trailing byte, reported
    let b = read_payload(&w.b, rev(0x132, 0), &|| false).unwrap();
    assert_eq!(b.materials[0].and_then(|d| d.guid), Some(17482));
    assert_eq!(b.materials[0].and_then(|d| d.flags), None);
    assert_eq!(b.materials[1].and_then(|d| d.sha1), Some([1; 20]));
    assert_eq!(b.vertices[1].mapping_mode(), None);
    assert_eq!(b.vertices[1].mapping_mode_raw, 9);
    assert_eq!(b.fixed_bevel_size, -1.0);
    assert!(b.spongy);
    assert_eq!(b.trailing_bytes, 1);
}

#[test]
fn zero_guid_and_zero_hash_are_absent() {
    let mut w = W::new(0);
    w.u32(4)
        .u32(0)
        .u8(2)
        .u32(0)
        .u32(0)
        .u8(1)
        .raw(&[0; 20])
        .res_none()
        .res_none();
    w.u32(4)
        .f32(1.0)
        .f32(1.0)
        .f32(1.0)
        .f32(1.0)
        .f32(90.0)
        .u32(0);
    w.u8(0)
        .u8(1)
        .f32(0.0)
        .f32(0.3)
        .f32(10.0)
        .u8(0)
        .res_none()
        .f32(1.0);
    let b = read_payload(&w.b, rev(0x3f8, 0), &|| false).unwrap();
    assert_eq!(b.materials, [None; 4]);
    assert!(b.vertices.is_empty());
}

#[test]
fn malformed_payloads_are_refused_without_panics() {
    let ok = payload(7, &CHAMFER);
    for n in 0..ok.len() {
        assert!(
            read_payload(&ok[..n], rev(0x3f8, 7), &|| false).is_err(),
            "truncation {n}"
        );
    }
    let mut bad = payload(0, &CHAMFER);
    bad[3] = 3; // material count 3
    assert!(matches!(
        read_payload(&bad, rev(0x3f8, 0), &|| false),
        Err(Stop::Unsupported { .. })
    ));
    // Vertex count far beyond the remaining bytes.
    let mut w = W::new(0);
    w.u32(4).res_none().res_none().res_none().res_none();
    w.u32(4)
        .f32(1.0)
        .f32(1.0)
        .f32(1.0)
        .f32(1.0)
        .f32(90.0)
        .u32(0x7fff_ffff);
    assert!(read_payload(&w.b, rev(0x3f8, 0), &|| false).is_err());
    // Head above LBP2 and cancellation.
    assert!(read_payload(&ok, rev(0x3f9, 7), &|| false).is_err());
    assert!(matches!(
        read_payload(&ok, rev(0x3f8, 7), &|| true),
        Err(Stop::Cancelled)
    ));
    assert!(lbp_level::bevel::read(b"PLNb\0\0\0\0", &|| false).is_err());
}

// ---------------------------------------------------------------- geometry recipes

fn d(g: u32) -> Descriptor {
    Descriptor {
        flags: Some(0),
        kind_byte: 2,
        guid: Some(g),
        sha1: None,
    }
}

fn shape(loops: &[&[[f32; 2]]], thickness: f32, bevel_size: f32) -> Shape {
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
        bevel_size,
        com: None,
        behavior: 0,
        flags: 0,
        contacts: 0,
    }
}

fn gm(bevel: Option<u32>) -> GeneratedMesh {
    GeneratedMesh {
        gfx_material: Some(d(1234)),
        bevel: bevel.map(d),
        uv_offset: [0.25, 0.5, 0.0, 0.0],
        plan_guid: None,
        play_mode_visible: true,
    }
}

fn bevel(verts: &[(f32, f32, i8, u8)]) -> Bevel {
    Bevel {
        revision: rev(0x3f8, 7),
        materials: [Some(d(10803)), Some(d(10812)), None, None],
        uv_scales: [1.0; 4],
        auto_smooth_cutoff_angle: 45.0,
        vertices: verts
            .iter()
            .map(|&(y, z, s, m)| BevelVertex {
                y,
                z,
                rigidity: 1.0,
                smooth_with_previous: 1,
                gmat_slot: s,
                mapping_mode_raw: m,
            })
            .collect(),
        include_backface: false,
        smooth_with_front: true,
        relax_strength: 0.0,
        sub_div_radius: 0.3,
        fixed_bevel_size: 10.0,
        spongy: false,
        soft_physics_settings: None,
        texture_repeats: 1.0,
        consumed_bytes: 0,
        trailing_bytes: 0,
    }
}

/// A rounded front edge: wall to z = 0.5, quarter arc inwards to y = -0.5 at z = 1.
const ROUND: [(f32, f32, i8, u8); 6] = [
    (0.0, -1.0, 0, 2),
    (0.0, 0.5, 0, 1),
    (-0.0670, 0.75, 0, 1),
    (-0.25, 0.933, 0, 1),
    (-0.5, 1.0, 0, 1),
    (-0.5, 1.0, 0, 1), // duplicate, dropped
];

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

type V3 = [f64; 3];
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn p(m: &ShapeMesh, i: usize) -> V3 {
    m.positions[i].map(f64::from)
}
fn tri(m: &ShapeMesh, t: usize) -> [usize; 3] {
    [0, 1, 2].map(|k| m.indices[3 * t + k] as usize)
}

fn volume(m: &ShapeMesh) -> f64 {
    (0..m.triangle_count())
        .map(|t| {
            let [a, b, c] = tri(m, t);
            dot(p(m, a), cross(p(m, b), p(m, c))) / 6.0
        })
        .sum()
}

fn welded(m: &ShapeMesh) -> Vec<usize> {
    let mut weld: HashMap<[u32; 3], usize> = HashMap::new();
    m.positions
        .iter()
        .map(|q| {
            let n = weld.len();
            *weld.entry(q.map(f32::to_bits)).or_insert(n)
        })
        .collect()
}

/// Closed: every welded directed edge once, its reverse once.
fn assert_closed(m: &ShapeMesh) {
    let ids = welded(m);
    let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
    for t in 0..m.triangle_count() {
        let [a, b, c] = tri(m, t).map(|i| ids[i]);
        assert!(a != b && b != c && c != a, "degenerate triangle {t}");
        for e in [(a, b), (b, c), (c, a)] {
            *directed.entry(e).or_default() += 1;
        }
    }
    for (&(a, b), &n) in &directed {
        assert_eq!(n, 1, "directed edge used {n} times");
        assert_eq!(directed.get(&(b, a)), Some(&1), "edge without a twin");
    }
}

fn assert_normals(m: &ShapeMesh) {
    for t in 0..m.triangle_count() {
        let [a, b, c] = tri(m, t);
        let g = cross(sub(p(m, b), p(m, a)), sub(p(m, c), p(m, a)));
        for i in [a, b, c] {
            let n = m.normals[i].map(f64::from);
            assert!(dot(g, n) > 0.0, "triangle {t} faces against its normal");
        }
    }
}

/// Segment (open) crosses triangle (closed) strictly between its endpoints.
fn seg_hits(a: V3, b: V3, t: [V3; 3]) -> bool {
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let dir = sub(b, a);
    let h = cross(dir, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-9 {
        return false; // parallel or coplanar: not counted
    }
    let s = sub(a, t[0]);
    let u = dot(s, h) / det;
    let q = cross(s, e1);
    let v = dot(dir, q) / det;
    let w = dot(e2, q) / det;
    let eps = 1e-7;
    u > eps && v > eps && u + v < 1.0 - eps && w > eps && w < 1.0 - eps
}

/// No two triangles that share no welded vertex intersect (edge-through-face test).
fn assert_no_self_intersection(m: &ShapeMesh) {
    let ids = welded(m);
    let tris: Vec<[usize; 3]> = (0..m.triangle_count()).map(|t| tri(m, t)).collect();
    for (i, ta) in tris.iter().enumerate() {
        let wa = ta.map(|k| ids[k]);
        let pa = ta.map(|k| p(m, k));
        for tb in &tris[i + 1..] {
            let wb = tb.map(|k| ids[k]);
            if wa.iter().any(|x| wb.contains(x)) {
                continue;
            }
            let pb = tb.map(|k| p(m, k));
            for e in 0..3 {
                assert!(!seg_hits(pa[e], pa[(e + 1) % 3], pb), "self-intersection");
                assert!(!seg_hits(pb[e], pb[(e + 1) % 3], pa), "self-intersection");
            }
        }
    }
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1.0)
}

/// Volume of a square of side `side` swept along a piecewise-linear (offset, depth)
/// profile: sum of the exact integrals of (side + 2 d)^2 over each segment.
fn swept_square_volume(side: f64, prof: &[(f64, f64)]) -> f64 {
    prof.windows(2)
        .map(|w| {
            let ((d0, z0), (d1, z1)) = (w[0], w[1]);
            let a = |d: f64| (side + 2.0 * d).powi(2);
            (z1 - z0) * (a(d0) + 4.0 * a((d0 + d1) / 2.0) + a(d1)) / 6.0
        })
        .sum()
}

fn applied(m: &ShapeMesh) -> (f32, u32) {
    match m.bevel {
        BevelOutcome::Applied {
            size, shrink_steps, ..
        } => (size, shrink_steps),
        o => panic!("bevel not applied: {o:?}"),
    }
}

#[test]
fn profile_depth_maps_ends_to_faces_and_is_continuous() {
    let (t, b) = (90.0, 10.0);
    assert_eq!(profile_depth(1.0, t, b), 90.0);
    assert_eq!(profile_depth(-1.0, t, b), -90.0);
    assert_eq!(profile_depth(0.0, t, b), 0.0);
    assert!((profile_depth(0.5, t, b) - 85.0).abs() < 1e-12);
    assert!((profile_depth(0.4999999, t, b) - 85.0).abs() < 1e-4);
    assert_eq!(profile_depth(1.5, t, b), 95.0);
    // Thin slab: band capped at the half depth, still monotonic.
    let thin: Vec<f64> = [-1.0, -0.75, -0.5, 0.0, 0.5, 0.75, 1.0]
        .iter()
        .map(|&z| profile_depth(z, 10.0, 50.0))
        .collect();
    assert!(thin.windows(2).all(|w| w[0] <= w[1]), "{thin:?}");
}

#[test]
fn chamfered_square_is_closed_with_slot_materials() {
    let s = shape(&[&SQUARE], 90.0, 10.0);
    let bv = bevel(&CHAMFER);
    let m = extrude_bevelled(
        &s,
        &gm(Some(10813)),
        &IDENTITY,
        Some(&bv),
        &BevelOptions::default(),
    )
    .unwrap();
    assert_eq!(m.label, BEVEL_LABEL);
    assert_eq!(applied(&m), (10.0, 0));
    assert_closed(&m);
    assert_normals(&m);
    assert_no_self_intersection(&m);
    // Rings: (-5, -90), (0, -85), (0, 85), (-5, 90).
    let want = swept_square_volume(
        100.0,
        &[(-5.0, -90.0), (0.0, -85.0), (0.0, 85.0), (-5.0, 90.0)],
    );
    assert!(close(volume(&m), want, 1e-6), "{} vs {want}", volume(&m));
    assert_eq!(m.world_z, (-90.0, 90.0));
    // Front cap = last vertex slot 0 = gfx material; back cap and sides = slot 1.
    let front = m.groups.iter().find(|g| g.kind == FaceKind::Front).unwrap();
    assert_eq!((front.slot, front.material), (0, Some(d(1234))));
    let back = m.groups.iter().find(|g| g.kind == FaceKind::Back).unwrap();
    assert_eq!(back.material, Some(d(10812)));
    let sides: Vec<_> = m
        .groups
        .iter()
        .filter(|g| g.kind == FaceKind::Side)
        .collect();
    assert_eq!(sides.len(), 1);
    assert_eq!(sides[0].material, Some(d(10812)));
    assert_eq!(m.slot_material_fallbacks, 0);
    // Groups tile the index buffer.
    let mut covered: Vec<_> = m.groups.iter().map(|g| g.range.clone()).collect();
    covered.sort_by_key(|r| r.start);
    assert_eq!(covered.first().unwrap().start, 0);
    assert!(covered.windows(2).all(|w| w[0].end == w[1].start));
    assert_eq!(covered.last().unwrap().end, m.indices.len());
    // Two materials -> two compacted parts with every triangle once.
    let parts = m.material_parts();
    assert_eq!(parts.len(), 2);
    assert_eq!(
        parts.iter().map(|q| q.triangle_count()).sum::<usize>(),
        m.triangle_count()
    );
    for part in &parts {
        assert!(part
            .indices
            .iter()
            .all(|&i| (i as usize) < part.positions.len()));
    }
    // Front cap lies inset by 5 at z = +90.
    let f = tri(&m, m.front.start / 3)[0];
    let fp = m.positions[f];
    assert_eq!(fp[2], 90.0);
    assert!(fp[0] >= 5.0 - 1e-4 && fp[0] <= 95.0 + 1e-4);
}

#[test]
fn round_profile_on_concave_and_holed_shapes() {
    let bv = bevel(&ROUND);
    let opts = BevelOptions::default();
    for loops in [&[&L_SHAPE[..]][..], &[&SQUARE[..], &HOLE_CW[..]][..]] {
        let s = shape(loops, 40.0, 10.0);
        let m = extrude_bevelled(&s, &gm(Some(1)), &IDENTITY, Some(&bv), &opts).unwrap();
        assert_eq!(applied(&m), (10.0, 0));
        assert_closed(&m);
        assert_normals(&m);
        assert_no_self_intersection(&m);
        let straight = extrude(&s, &gm(Some(1)), &IDENTITY).unwrap();
        // Front inset removes volume relative to the straight slab.
        assert!(volume(&m) > 0.0 && volume(&m) < volume(&straight));
        assert_eq!(m.material_parts().len(), 1);
        assert_eq!(m.hidden_strips, 0);
    }
}

#[test]
fn mirrored_transform_keeps_outward_winding() {
    let mut w = IDENTITY;
    w[0] = -1.0;
    w[5] = 2.0;
    w[12] = 300.0;
    let s = shape(&[&L_SHAPE], 40.0, 10.0);
    let bv = bevel(&CHAMFER);
    let m = extrude_bevelled(&s, &gm(Some(1)), &w, Some(&bv), &BevelOptions::default()).unwrap();
    assert!(m.mirrored);
    assert_closed(&m);
    assert_normals(&m);
    assert_no_self_intersection(&m);
    let local = extrude_bevelled(
        &s,
        &gm(Some(1)),
        &IDENTITY,
        Some(&bv),
        &BevelOptions::default(),
    )
    .unwrap();
    assert!(close(volume(&m), 2.0 * volume(&local), 1e-6));
    assert!(volume(&m) > 0.0);
}

#[test]
fn thin_pieces_shrink_the_bevel_instead_of_folding() {
    // 20 units wide; inset 1.5 * 10 = 15 would cross the centre line.
    let strip = [[0.0, 0.0], [200.0, 0.0], [200.0, 20.0], [0.0, 20.0]];
    let deep = [(0.0, -1.0, 0, 1), (0.0, 0.5, 0, 1), (-1.5, 1.0, 0, 1)];
    let s = shape(&[&strip], 10.0, 10.0);
    let bv = bevel(&deep);
    let m = extrude_bevelled(
        &s,
        &gm(Some(1)),
        &IDENTITY,
        Some(&bv),
        &BevelOptions::default(),
    )
    .unwrap();
    let (size, steps) = applied(&m);
    assert!(steps >= 1 && size * 1.5 < 10.0, "{size} {steps}");
    assert_closed(&m);
    assert_no_self_intersection(&m);
    // No shrinking allowed: falls back to the straight extrusion.
    let none = BevelOptions {
        max_shrink_steps: 0,
        ..BevelOptions::default()
    };
    let f = extrude_bevelled(&s, &gm(Some(1)), &IDENTITY, Some(&bv), &none).unwrap();
    assert!(
        matches!(f.bevel, BevelOutcome::Fallback(_)),
        "{:?}",
        f.bevel
    );
    assert_eq!(f.label, PREVIEW_LABEL);
}

#[test]
fn missing_or_unusable_bevels_fall_back_to_straight_walls() {
    let s = shape(&[&SQUARE], 90.0, 10.0);
    let opts = BevelOptions::default();
    let straight = extrude(&s, &gm(Some(5)), &IDENTITY).unwrap();
    let m = extrude_bevelled(&s, &gm(None), &IDENTITY, None, &opts).unwrap();
    assert_eq!(m.bevel, BevelOutcome::NoBevel);
    let m = extrude_bevelled(&s, &gm(Some(5)), &IDENTITY, None, &opts).unwrap();
    assert_eq!(m.bevel, BevelOutcome::Unavailable);
    assert_eq!(m.positions, straight.positions);
    assert_eq!(m.indices, straight.indices);
    let one = bevel(&[(0.0, 1.0, 0, 1)]);
    let m = extrude_bevelled(&s, &gm(Some(5)), &IDENTITY, Some(&one), &opts).unwrap();
    assert_eq!(m.bevel, BevelOutcome::Fallback("profile_too_short"));
    let nan = bevel(&[(f32::NAN, 1.0, 0, 1), (0.0, -1.0, 0, 1)]);
    let m = extrude_bevelled(&s, &gm(Some(5)), &IDENTITY, Some(&nan), &opts).unwrap();
    assert_eq!(m.bevel, BevelOutcome::Fallback("profile_non_finite"));
    let mut zero = bevel(&CHAMFER);
    zero.fixed_bevel_size = -1.0;
    let s0 = shape(&[&SQUARE], 90.0, 0.0);
    let m = extrude_bevelled(&s0, &gm(Some(5)), &IDENTITY, Some(&zero), &opts).unwrap();
    assert_eq!(m.bevel, BevelOutcome::Fallback("bevel_size"));
    // Geometry rejects still reject.
    let bad = shape(&[&SQUARE], 0.0, 10.0);
    assert_eq!(
        extrude_bevelled(&bad, &gm(Some(5)), &IDENTITY, Some(&zero), &opts)
            .unwrap_err()
            .reason(),
        "bad_thickness"
    );
}

#[test]
fn size_rules_and_slot_fallbacks() {
    let s = shape(&[&SQUARE], 90.0, 20.0);
    let mut bv = bevel(&CHAMFER);
    bv.fixed_bevel_size = 5.0;
    let size = |rule| {
        let o = BevelOptions {
            size_rule: rule,
            ..BevelOptions::default()
        };
        applied(&extrude_bevelled(&s, &gm(Some(1)), &IDENTITY, Some(&bv), &o).unwrap()).0
    };
    assert_eq!(size(BevelSizeRule::ShapeBevelSize), 20.0);
    assert_eq!(size(BevelSizeRule::FixedBevelSize), 5.0);
    assert_eq!(size(BevelSizeRule::Sum), 25.0);
    // Slot 2 has no material and slot 7 is out of range: both use the gfx material.
    let odd = bevel(&[(0.0, -1.0, 2, 1), (0.0, 0.5, 7, 1), (-0.5, 1.0, 0, 0)]);
    let m = extrude_bevelled(
        &s,
        &gm(Some(1)),
        &IDENTITY,
        Some(&odd),
        &BevelOptions::default(),
    )
    .unwrap();
    assert_eq!(m.slot_material_fallbacks, 3);
    assert!(m.groups.iter().all(|g| g.material == Some(d(1234))));
    assert_eq!(m.material_parts().len(), 1);
}

#[test]
fn straight_extrusion_is_one_unchanged_material_part() {
    let s = shape(&[&SQUARE, &HOLE_CW], 90.0, 10.0);
    let m = extrude(&s, &gm(Some(1)), &IDENTITY).unwrap();
    assert_eq!(m.bevel, BevelOutcome::NotRequested);
    assert_eq!(m.groups.len(), 3);
    let parts = m.material_parts();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].positions, m.positions);
    assert_eq!(parts[0].indices, m.indices);
    assert_eq!(parts[0].material_guid(), Some(1234));
}
