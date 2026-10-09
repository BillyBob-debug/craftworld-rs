//! R2 sandbox on authored synthetic shapes (no game bytes): scene rules, then avian2d runs
//! at 30 Hz. docs/sim-physics.md.
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_level::parts::{Body, Pos, Shape, ShapeColor};
use lbp_level::thing::ThingRecord;
use lbp_sim::scene::{build_scene, PhysicsScene, DENSITY};
use lbp_sim::{spawn_scene, SimPlugin, ThingIndex, TICK_HZ};
use std::time::Duration;

/// Column-major matrix: rotation `angle` about z, scale (sx, sy), translation.
fn matrix(x: f32, y: f32, z: f32, angle: f32, sx: f32, sy: f32) -> [f32; 16] {
    let (s, c) = angle.sin_cos();
    let mut m = [0.; 16];
    m[0] = c * sx;
    m[1] = s * sx;
    m[4] = -s * sy;
    m[5] = c * sy;
    m[10] = 1.;
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m[15] = 1.;
    m
}

/// Axis-aligned w×h rectangle centred on the origin, counter-clockwise.
fn rect(w: f32, h: f32, flags: i16) -> Shape {
    let (x, y) = (w / 2., h / 2.);
    Shape {
        vertex_count: 4,
        requires_z: false,
        vertices: vec![[-x, -y, 0.], [x, -y, 0.], [x, y, 0.], [-x, y, 0.]],
        loops: vec![4],
        material: None,
        old_material: None,
        thickness: 90.,
        mass_depth: 1.,
        color: ShapeColor::Packed(0xffff_ffff),
        brightness: 0.,
        bevel_size: 10.,
        com: None,
        behavior: 0,
        flags,
        contacts: 0,
    }
}

fn thing(index: usize, world: [f32; 16], shape: Option<Shape>, body: bool) -> ThingRecord {
    let mut t = ThingRecord {
        index,
        ..Default::default()
    };
    t.data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t.data.shape = shape;
    t.data.body = body.then_some(Body {
        velocity: [0.; 3],
        angular_velocity: 0.,
        frozen: 0,
    });
    t
}

#[test]
fn scene_rules_bodies_static_children_flags_and_frames() {
    let mut things = vec![
        // 0: static floor, 2000 wide, top at y = 0.
        thing(
            0,
            matrix(0., -50., 0., 0., 1., 1.),
            Some(rect(2000., 100., 7)),
            false,
        ),
        // 1: body with its own 100×100 box at (0, 200), rotated 90°.
        thing(
            1,
            matrix(0., 200., 0., std::f32::consts::FRAC_PI_2, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        // 2: a piece glued under Thing 1 (no BODY), mirrored in x.
        thing(
            2,
            matrix(100., 200., 0., 0., -1., 1.),
            Some(rect(100., 50., 7)),
            false,
        ),
        // 3: not collidable in game.
        thing(
            3,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(10., 10., 2)),
            false,
        ),
        // 4: a BODY Thing without a shape and no POS: its shape child is rejected.
        ThingRecord {
            index: 4,
            ..Default::default()
        },
        thing(
            5,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(10., 10., 7)),
            false,
        ),
    ];
    things[2].parent = Some(1);
    things[4].data.body = things[1].data.body;
    things[5].parent = Some(4);
    let s = build_scene(&things);
    let r = &s.report;
    assert_eq!(
        (r.shapes, r.dynamic, r.static_, r.not_collidable),
        (5, 2, 1, 1)
    );
    assert_eq!(r.rejected.get("body without POS"), Some(&1));
    assert_eq!(s.bodies.len(), 1);
    assert_eq!(s.bodies[0].thing, 1);
    let piece = s.colliders.iter().find(|c| c.thing == 2).unwrap();
    assert_eq!(piece.body, Some(0));
    assert_eq!(piece.frame, s.bodies[0].frame);
    // Every triangle is counter-clockwise, the mirrored piece included, and the pieces
    // keep their area.
    for c in &s.colliders {
        let mut area = 0.;
        for [a, b, t] in &c.triangles {
            let tri = 0.5 * ((b[0] - a[0]) * (t[1] - a[1]) - (b[1] - a[1]) * (t[0] - a[0]));
            assert!(tri > 0., "thing {}", c.thing);
            area += tri;
        }
        let want = match c.thing {
            0 => 200_000.,
            1 => 10_000.,
            _ => 5_000.,
        };
        assert!(
            (area - want).abs() < 1e-2 * want,
            "thing {}: {area}",
            c.thing
        );
        assert_eq!(c.density, DENSITY);
    }
    // The glued piece sits at world x 50..150 = body-local y -150..-50 after the 90° turn.
    let ys: Vec<f32> = piece.triangles.iter().flatten().map(|p| p[1]).collect();
    let (lo, hi) = ys
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), &y| (l.min(y), h.max(y)));
    assert!(
        (lo + 150.).abs() < 1e-3 && (hi + 50.).abs() < 1e-3,
        "{lo} {hi}"
    );
}

fn app(scene: PhysicsScene) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, SimPlugin::default()))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1. / TICK_HZ,
        )))
        .add_systems(Startup, move |mut commands: Commands| {
            spawn_scene(&mut commands, &scene);
        });
    app.finish();
    app
}

/// Body centres by Thing index after `ticks` game ticks.
fn run(things: &[ThingRecord], ticks: usize) -> Vec<(usize, [f32; 2], f32)> {
    let mut app = app(build_scene(things));
    for _ in 0..ticks {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position, &Rotation), With<RigidBody>>();
    let mut out: Vec<_> = q
        .iter(app.world())
        .map(|(t, p, r)| (t.0, [p.x, p.y], r.as_radians()))
        .collect();
    out.sort_by_key(|o| o.0);
    out
}

fn floor() -> ThingRecord {
    thing(
        0,
        matrix(0., -50., 0., 0., 1., 1.),
        Some(rect(4000., 100., 7)),
        false,
    )
}

#[test]
fn box_falls_onto_the_floor_and_rests() {
    let things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let out = run(&things, 120);
    let (_, p, angle) = out[1];
    assert!((p[1] - 50.).abs() < 5., "rests on top of the floor: {p:?}");
    // The bounce (restitution 0.3) resolves one corner first and slides the box a little:
    // about 6 units here, none with restitution 0.
    assert!(p[0].abs() < 10. && angle.abs() < 0.05, "{p:?} {angle}");
    assert_eq!(out[0].1, [0., -50.], "the floor does not move");
}

#[test]
fn shapes_in_separate_slabs_pass_each_other() {
    // The box is one thick layer behind the floor (slabs 0±90 and 300±90 do not overlap).
    let things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 300., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let out = run(&things, 30);
    assert!(out[1].1[1] < -500., "fell through: {:?}", out[1].1);
}

#[test]
fn stack_of_boxes_stays_put() {
    let mut things = vec![floor()];
    for i in 0..4 {
        let y = 50. + 100. * i as f32;
        things.push(thing(
            i + 1,
            matrix(0., y, 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ));
    }
    let out = run(&things, 300);
    for (i, p, angle) in &out[1..] {
        let want = 50. + 100. * (*i as f32 - 1.);
        assert!(
            (p[1] - want).abs() < 5. && p[0].abs() < 5. && angle.abs() < 0.05,
            "box {i}: {p:?} {angle}"
        );
    }
}
