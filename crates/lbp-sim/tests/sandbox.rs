//! R2 sandbox on authored synthetic shapes (no game bytes): scene rules, then avian2d runs
//! at 30 Hz. docs/sim-physics.md.
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_level::parts::{Body, Joint, JointParams, PhysicsTweak, Pos, Shape, ShapeColor, SwitchKey};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_sim::scene::{
    build_scene, build_scene_with, build_spawn_scene_with, Endpoint, JointKind, MaterialRule,
    PhysicsScene, DENSITY,
};
use lbp_sim::{spawn_scene, to_level, SimPlugin, ThingIndex, TICK_HZ};
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
        .map(|(t, p, r)| (t.0, to_level(p.0), r.as_radians()))
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
    let floor = out[0].1;
    assert!(
        floor[0].abs() < 1e-3 && (floor[1] + 50.).abs() < 1e-3,
        "the floor does not move"
    );
}

#[test]
fn d1100_an_anti_gravity_tweaker_holds_its_body_up() {
    // Two boxes; box 2 carries an anti-gravity tweaker (Thing 3) as saved in the Rookie Test.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(-300., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(
            2,
            matrix(300., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(3, matrix(300., 300., 0., 0., 1., 1.), None, false),
    ];
    things[3].parent = Some(2);
    things[3].data.physics_tweak = Some(PhysicsTweak {
        gravity: 1.,
        dampening: [1., 1., 0.],
        last_known_activation: 1.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    assert!(scene.bodies[0].anti_gravity.is_none());
    assert_eq!(
        scene.bodies[1].anti_gravity.as_ref().map(|a| a.cancelled),
        Some(1.)
    );
    let out = run(&things, 60);
    assert!(
        (out[1].1[1] - 50.).abs() < 5.,
        "the plain box falls: {:?}",
        out[1]
    );
    assert!(
        (out[2].1[1] - 300.).abs() < 1.,
        "the tweaked box stays: {:?}",
        out[2]
    );
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

/// A JOINT Thing from `a` (contact `ca` in a's frame) to `b` (contact `cb`).
fn joint(
    index: usize,
    kind: i32,
    a: usize,
    ca: [f32; 3],
    b: usize,
    cb: [f32; 3],
    length: f32,
) -> ThingRecord {
    joint_with(index, kind, a, ca, b, cb, length, JointParams::default())
}

#[allow(clippy::too_many_arguments)]
fn joint_with(
    index: usize,
    kind: i32,
    a: usize,
    ca: [f32; 3],
    b: usize,
    cb: [f32; 3],
    length: f32,
    params: JointParams,
) -> ThingRecord {
    let mut t = ThingRecord {
        index,
        ..Default::default()
    };
    t.data.joint = Some(Joint {
        a: Some(a),
        b: Some(b),
        a_contact: ca,
        b_contact: cb,
        length,
        angle: 0.,
        settings: None,
        bone_idx: vec![],
        bone_lengths: [0.; 4],
        joint_type: kind,
        render_scale: 1.,
        hide_in_play_mode: false,
        params,
    });
    t
}

/// Static ceiling block at (0, 1000).
fn ceiling() -> ThingRecord {
    thing(
        0,
        matrix(0., 1000., 0., 0., 1., 1.),
        Some(rect(200., 50., 7)),
        false,
    )
}

#[test]
fn joint_scene_rules() {
    let mut things = vec![
        ceiling(),
        thing(
            1,
            matrix(300., 1000., 0., std::f32::consts::FRAC_PI_2, 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        thing(
            2,
            matrix(0., 500., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        thing(
            3,
            matrix(0., 500., 0., 0., 1., 1.),
            Some(rect(10., 10., 7)),
            false,
        ),
        // 4: rod from the ceiling centre to body 1, contact 10 units along body 1's local x.
        joint(4, 6, 0, [0.; 3], 1, [10., 0., 0.], 0.),
        // 5: bolt between two bodies.
        joint(5, 7, 1, [0.; 3], 2, [0.; 3], 0.),
        // 6: both ends on body 2 (Thing 3 is glued under it).
        joint(6, 7, 2, [0.; 3], 3, [0.; 3], 0.),
        // 7: legacy.
        joint(7, 0, 1, [0.; 3], 2, [0.; 3], 0.),
    ];
    things[3].parent = Some(2);
    let s = build_scene(&things);
    assert_eq!(s.report.joints, 2);
    assert_eq!(
        s.report.rejected_joints.get("both ends on one body"),
        Some(&1)
    );
    assert_eq!(
        s.report.rejected_joints.get("legacy or unknown type"),
        Some(&1)
    );
    let rod = &s.joints[0];
    assert_eq!(
        (rod.kind, rod.a, rod.b),
        (JointKind::Rod, Endpoint::Static(0), Endpoint::Body(0))
    );
    // Body 1 is turned 90°: its local x points along world +y.
    let close = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).abs() < 1e-3 && (p[1] - q[1]).abs() < 1e-3;
    assert!(close(rod.world_b, [300., 1010.]), "{:?}", rod.world_b);
    assert!(close(rod.local_b, [10., 0.]), "{:?}", rod.local_b);
    assert!((rod.length - (300f32.powi(2) + 10f32.powi(2)).sqrt()).abs() < 1e-2);
    assert_eq!(s.joints[1].kind, JointKind::Bolt);
}

fn distance(p: [f32; 2], q: [f32; 2]) -> f32 {
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}

#[test]
fn rod_pendulum_keeps_its_length() {
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(300., 1000., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 6, 0, [0.; 3], 1, [0.; 3], 0.),
    ];
    let out = run(&things, 20);
    let p = out[1].1;
    assert!((distance(p, [0., 1000.]) - 300.).abs() < 5., "{p:?}");
    assert!(p[1] < 900., "swung down: {p:?}");
}

#[test]
fn string_stops_a_falling_box_at_its_length() {
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(0., 900., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 5, 0, [0.; 3], 1, [0.; 3], 300.),
    ];
    let out = run(&things, 90);
    let p = out[1].1;
    assert!((p[1] - 700.).abs() < 10. && p[0].abs() < 5., "{p:?}");
}

#[test]
fn motor_turns_a_bolted_wheel_in_place() {
    // D1108: pattern 1 turns at speed (rad per frame) × 30 ticks a second: 0.1 is 3 rad/s.
    let params = JointParams {
        animation_pattern: 1,
        motion: [0., 0., 0., 0.1, 0., 0., 0., 0., 0., 0.],
        ..Default::default()
    };
    let things = vec![
        thing(
            0,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(20., 20., 7)),
            false,
        ),
        thing(
            1,
            matrix(0., 0., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        joint_with(2, 9, 0, [0.; 3], 1, [0.; 3], 0., params),
    ];
    let mut app = app(build_scene(&things));
    for _ in 0..30 {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query::<(&ThingIndex, &Position, &AngularVelocity, Option<&RigidBody>)>();
    let (_, p, w, _) = q.iter(app.world()).find(|(t, ..)| t.0 == 1).expect("wheel");
    assert!(distance(to_level(p.0), [0., 0.]) < 2., "{p:?}");
    assert!(
        (w.0 - 3.).abs() < 0.5,
        "turning at the motor speed: {}",
        w.0
    );
}

#[test]
fn piston_slides_to_its_limit_and_holds_its_line() {
    let params = JointParams {
        tweak_target_min_length: 100.,
        tweak_target_max_length: 200.,
        ..Default::default()
    };
    let things = vec![
        ceiling(),
        thing(
            1,
            matrix(0., 850., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint_with(2, 4, 0, [0.; 3], 1, [0.; 3], 150., params),
    ];
    let out = run(&things, 60);
    let p = out[1].1;
    assert!((p[1] - 800.).abs() < 5. && p[0].abs() < 2., "{p:?}");
    assert!(out[1].2.abs() < 0.01, "does not turn: {}", out[1].2);
}

#[test]
fn dark_matter_and_world_bodies_are_static() {
    const DARK: u32 = 0xd4;
    let mut dark = rect(200., 100., 7);
    dark.material = Some(Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(DARK),
        sha1: None,
    });
    // 0: dark matter block in mid air; 1: box glued to it (its own BODY merges into 0);
    // 2: a loose box; 3: a box owned by the WORLD Thing.
    let mut glued = thing(
        1,
        matrix(150., 500., 0., 0., 1., 1.),
        Some(rect(100., 100., 7)),
        true,
    );
    glued.parent = Some(0);
    let mut world = thing(
        3,
        matrix(-600., 500., 0., 0., 1., 1.),
        Some(rect(100., 100., 7)),
        true,
    );
    world.parts = vec!["WORLD"];
    let things = vec![
        thing(0, matrix(0., 500., 0., 0., 1., 1.), Some(dark), true),
        glued,
        thing(
            2,
            matrix(600., 500., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        world,
        // 4: a bolt between the two fixed bodies (avian panics on a joint with no dynamic
        // end, so it must be rejected).
        joint_with(4, 7, 0, [0.; 3], 3, [0.; 3], 0., JointParams::default()),
    ];
    let scene = build_scene_with(&things, &|d| {
        if d.guid == Some(DARK) {
            MaterialRule::Fixed
        } else {
            MaterialRule::Normal
        }
    });
    let r = &scene.report;
    assert_eq!(
        (r.dynamic, r.static_, r.fixed_bodies, r.nested_bodies),
        (1, 3, 2, 1)
    );
    assert_eq!(
        scene.report.rejected_joints.get("both ends static"),
        Some(&1)
    );
    assert_eq!(build_scene(&things).report.fixed_bodies, 1, "WORLD only");
    let mut app = app(scene);
    for _ in 0..30 {
        app.update();
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position), With<RigidBody>>();
    let mut out: Vec<_> = q
        .iter(app.world())
        .map(|(t, p)| (t.0, to_level(p.0)))
        .collect();
    out.sort_by_key(|o| o.0);
    let near = |p: [f32; 2], q: [f32; 2]| (p[0] - q[0]).abs() < 0.01 && (p[1] - q[1]).abs() < 0.01;
    assert!(
        out[0].0 == 0 && near(out[0].1, [0., 500.]),
        "dark matter (and the glued box) stay: {out:?}"
    );
    assert!(
        out[1].0 == 2 && out[1].1[1] < 400.,
        "loose box falls: {out:?}"
    );
    assert!(
        out[2].0 == 3 && near(out[2].1, [-600., 500.]),
        "WORLD body stays: {out:?}"
    );
}

#[test]
fn frozen_bodies_sleep_until_touched_and_pickups_hang() {
    let asleep = |index, y| {
        let mut t = thing(
            index,
            matrix(0., y, 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        );
        t.data.body.as_mut().unwrap().frozen = 1;
        t
    };
    let mut pickup = asleep(3, 300.);
    pickup.data.pos.as_mut().unwrap().world = matrix(600., 300., 0., 0., 1., 1.);
    pickup.parts = vec!["GAMEPLAY_DATA"];
    // 1: asleep in mid air; 2: awake, dropped onto it from above; 3: a score bubble.
    let things = vec![floor(), asleep(1, 300.), asleep(2, 700.), pickup];
    let mut things = things;
    things[2].data.body.as_mut().unwrap().frozen = 0;
    let before = run(&things[..2], 60);
    assert!(
        (before[1].1[1] - 300.).abs() < 0.01,
        "asleep stays: {before:?}"
    );
    let after = run(&things, 120);
    assert!(after[1].1[1] < 200., "woken by the falling box: {after:?}");
    assert!(
        (after[3].1[1] - 300.).abs() < 0.01 && (after[3].1[0] - 600.).abs() < 0.01,
        "pick-up hangs: {after:?}"
    );
}

#[test]
fn no_collide_material_is_not_a_collider() {
    const HOLO: u32 = 0x40;
    let mut holo = rect(100., 100., 7);
    holo.material = Some(Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(HOLO),
        sha1: None,
    });
    let things = vec![
        floor(),
        thing(1, matrix(0., 300., 0., 0., 1., 1.), Some(holo), true),
    ];
    let s = build_scene_with(&things, &|d| {
        if d.guid == Some(HOLO) {
            MaterialRule::NoCollide
        } else {
            MaterialRule::Normal
        }
    });
    assert_eq!(
        (s.report.not_collidable, s.colliders.len(), s.bodies.len()),
        (1, 1, 0)
    );
}

#[test]
fn crossing_outline_is_filled_and_holds_a_box() {
    // A floor whose second loop crosses its bottom edge: refused by the strict outline,
    // kept by the even-odd fill, and the box still lands on its top.
    let mut shape = rect(4000., 100., 7);
    let notch = [[1400., -70.], [1600., -70.], [1600., -30.], [1400., -30.]];
    shape.vertices.extend(notch.map(|p| [p[0], p[1], 0.]));
    shape.vertex_count = 8;
    shape.loops.push(4);
    let mut floor = floor();
    floor.data.shape = Some(shape);
    let things = vec![
        floor,
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
    ];
    let report = build_scene(&things).report;
    assert_eq!(report.filled.get("loops_intersect"), Some(&1));
    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let p = run(&things, 120)[1].1;
    assert!((p[1] - 50.).abs() < 5., "rests on top of the floor: {p:?}");
}

#[test]
fn string_from_a_thing_without_a_collider_holds_a_box() {
    // The top end is a Thing with a POS but no shape (as a hologram or decoration): the
    // joint anchors it in the world instead of being dropped.
    let things = vec![
        thing(0, matrix(0., 1000., 0., 0., 1., 1.), None, false),
        thing(
            1,
            matrix(0., 900., 0., 0., 1., 1.),
            Some(rect(50., 50., 7)),
            true,
        ),
        joint(2, 5, 0, [0.; 3], 1, [0.; 3], 300.),
    ];
    let scene = build_scene(&things);
    assert_eq!(
        (scene.report.joints, scene.report.anchored_joint_ends),
        (1, 1)
    );
    let out = run(&things, 90);
    let p = out[1].1;
    assert!((p[1] - 700.).abs() < 10. && p[0].abs() < 5., "{p:?}");
}

#[test]
fn tilted_shape_with_vertex_z_collides_in_its_projected_outline() {
    // The Pod body's layout (shell 000027b9 Thing 2): turned so local z is world up and
    // local y is world -z; the outline lives in local x/z. Here a 400 x 200 floor block
    // whose top is 100 above its POS (y -400), so the top is at y -300.
    let mut shape = rect(400., 10., 1);
    shape.requires_z = true;
    shape.vertices = vec![
        [-200., 0., -100.],
        [200., 0., -100.],
        [200., 0., 100.],
        [-200., 0., 100.],
    ];
    let mut m = [0.; 16];
    m[0] = 1.;
    m[6] = -1.;
    m[9] = 1.;
    m[12] = 0.;
    m[13] = -400.;
    m[14] = 0.;
    m[15] = 1.;
    let things = [thing(0, m, Some(shape), false)];
    let scene = build_spawn_scene_with(&things, &|_| MaterialRule::Normal);
    assert_eq!(scene.colliders.len(), 1);
    let top = scene.floor_below([-5., 500.], 0.).unwrap();
    assert!((top + 300.).abs() < 1e-3, "top {top}");
    // Below the block's bottom (y -500) there is nothing.
    assert_eq!(scene.floor_below([-5., -501.], 0.), None);
    // The simulated scene keeps the in-plane outline: no floor at the projected top.
    let plain = build_scene(&things);
    assert_ne!(
        plain
            .floor_below([-5., 500.], 0.)
            .map(|t| (t + 300.).abs() < 1e-3),
        Some(true)
    );
}

/// D1101: a tweak switched off gives its body gravity back; analog activations add up.
#[test]
fn d1101_a_switched_off_anti_gravity_tweaker_lets_its_body_fall() {
    let tweak = |thing, gravity| lbp_sim::scene::AntiGravityTweak {
        thing,
        gravity,
        behavior: 0,
        saved: 1.,
    };
    let a = lbp_sim::scene::AntiGravity {
        cancelled: 1.5,
        dampening: [0.; 3],
        tweaks: vec![tweak(3, 1.), tweak(4, 0.5)],
    };
    assert_eq!(a.gravity_scale(|t| t.saved), 0.);
    assert_eq!(a.gravity_scale(|_| 0.), 1.);
    assert_eq!(
        a.gravity_scale(|t| if t.thing == 3 { 0.25 } else { 1. }),
        0.25
    );
}

#[test]
fn d1102_an_unwired_piston_moves_out_and_back_on_its_wave() {
    use lbp_sim::scene::{piston_wave, SceneJoint};
    // time 120, pause 20: hold 10, rise 60, hold 10, fall 60.
    let w = |x| piston_wave(x, 120., 20.);
    assert_eq!(
        (w(0.), w(10.), w(40.), w(75.), w(110.), w(140.)),
        (0., 0., 0.5, 1., 0.5, 0.)
    );
    assert_eq!(piston_wave(5., 0.5, 0.), 0.);
    let mut params = JointParams::default();
    params.motion[..5].copy_from_slice(&[315., 120., 0., 0.02, 20.]);
    let mut j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 840.,
        min_length: 840.,
        max_length: 840.,
        params,
    };
    // Eased: half way along the rise is half the range; the top is base + range.
    assert_eq!((j.free_length(0.), j.free_length(70.)), (840., 1155.));
    assert!((j.free_length(40.) - 997.5).abs() < 1e-3);
    j.params.animation_pattern = 2;
    assert_eq!(
        (j.free_length(0.), j.free_length(60.), j.free_length(130.)),
        (1155., 997.5, 840.)
    );
    j.params.animation_pattern = 7;
    assert_eq!(j.free_length(40.), 840.);
}

#[test]
fn d1103_a_switched_piston_clock_freezes_while_off_and_never_jumps() {
    use lbp_sim::scene::PistonClock;
    let mut c = PistonClock::default();
    assert_eq!(c.frame(30.), 30.);
    // Off at 30: the frame holds; on again at 50 it carries on from there.
    c.set_scale(30., 0.);
    assert_eq!((c.frame(30.), c.frame(50.)), (30., 30.));
    c.set_scale(50., 1.);
    assert_eq!((c.frame(50.), c.frame(60.)), (30., 40.));
    // Running backwards from 60 returns along the same frames.
    c.set_scale(60., -1.);
    assert_eq!((c.frame(60.), c.frame(70.)), (40., 30.));
}

#[test]
fn d1104_a_direction_piston_ramps_out_while_on_and_back_while_off() {
    use lbp_sim::scene::{piston_scale, Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 1,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[300., 60.]);
    let j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 500.,
        min_length: 500.,
        max_length: 500.,
        params,
    };
    // Rate by mode: on 1, a negative signal -1, off -1 (mode 1); mode 0 holds while off.
    assert_eq!(
        (
            piston_scale(1, false, 1, 0.),
            piston_scale(1, false, -1, 0.)
        ),
        (1., -1.)
    );
    assert_eq!(piston_scale(1, false, 0, 0.), -1.);
    assert_eq!(
        (piston_scale(0, false, 0, 0.), piston_scale(1, true, 1, 0.)),
        (0., 0.)
    );
    assert_eq!(piston_scale(3, false, 1, 0.25), 0.25);
    // The ramp: base + 2 × frame × range / time; D1105: the length is held between base
    // and base + range while the clock runs on.
    let mut c = PistonClock::default();
    let at = |c: &mut PistonClock, now: f32, s: f32| {
        c.set_scale(now, s);
        j.length(c.frame(now), true)
    };
    // Off from the start: held at the base while the clock runs below 0.
    assert_eq!((at(&mut c, 0., -1.), at(&mut c, 10., -1.)), (500., 500.));
    // On at 10 (frame -10): still at the base at 20, half way at 35, out from 50 on.
    assert_eq!((at(&mut c, 10., 1.), at(&mut c, 20., 1.)), (500., 500.));
    assert_eq!(at(&mut c, 35., 1.), 650.);
    assert_eq!((at(&mut c, 50., 1.), at(&mut c, 90., 1.)), (800., 800.));
    // Off at 90 (frame 70): out until frame 30 (tick 130), at the base by tick 160.
    assert_eq!((at(&mut c, 90., -1.), at(&mut c, 130., -1.)), (800., 800.));
    assert_eq!((at(&mut c, 145., -1.), at(&mut c, 160., -1.)), (650., 500.));
    // Unwired, the same joint follows its free-running wave instead.
    assert_eq!(j.length(15., false), j.free_length(15.));
}

/// D1106: a speed piston (mode 4) servos its length to the analog signal's fraction of the
/// way from base to base + range, at full rate until within a tenth (0xc9e80).
#[test]
fn d1106_a_speed_piston_settles_at_its_signal_fraction() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 4,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[300., 60.]);
    let mut j = SceneJoint {
        thing: 0,
        kind: JointKind::Piston,
        a: Endpoint::Static(0),
        b: Endpoint::Static(1),
        world_a: [0.; 2],
        world_b: [0.; 2],
        local_a: [0.; 2],
        local_b: [0.; 2],
        length: 500.,
        min_length: 500.,
        max_length: 500.,
        params,
    };
    // Rate: full out below the target, full back above it, 10 × the error near it.
    assert_eq!(
        (j.servo_scale(0.5, 500.), j.servo_scale(0., 800.)),
        (1., -1.)
    );
    assert!((j.servo_scale(0.5, 620.) - 1.).abs() < 1e-5);
    assert!((j.servo_scale(-1., 530.) + 1.).abs() < 1e-5);
    // Driven on the clock, a signal of 0.5 settles half way out (650).
    let mut c = PistonClock::default();
    c.set_scale(0., 0.);
    let mut l = 500.;
    for now in 0..400 {
        let now = now as f32;
        c.set_scale(now, j.servo_scale(0.5, l));
        l = j.length(c.frame(now), true);
    }
    assert!((l - 650.).abs() < 1., "{l}");
    // A negative range flips the target: signal 0 holds the piston at its base.
    j.params.motion[0] = -300.;
    assert_eq!(j.servo_scale(0., 500.), 0.);
}

#[test]
fn d1107_a_winch_follows_the_piston_rule() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 1,
        ..Default::default()
    };
    params.motion[..2].copy_from_slice(&[-200., 60.]);
    let mut d = lbp_sim::PistonDrive {
        joint: SceneJoint {
            thing: 0,
            kind: JointKind::Chain,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 400.,
            min_length: 400.,
            max_length: 400.,
            params,
        },
        clock: PistonClock::default(),
    };
    let u = lbp_sim::UNIT_SCALE;
    // Switched on (direction mode), it winds in over half its time and stops at 200.
    assert!((d.step(15., Some((1, 1.))) / u - 300.).abs() < 1e-3);
    assert!((d.step(60., Some((1, 1.))) / u - 200.).abs() < 1e-3);
    // Switched off, the clock (run on to 200 meanwhile) counts back down and the rope
    // pays out to its base length.
    assert!((d.step(200., Some((0, 0.))) / u - 200.).abs() < 1e-3);
    assert!((d.step(400., Some((0, 0.))) / u - 400.).abs() < 1e-3);
}

#[test]
fn d1108_a_motor_turns_at_its_speed_on_its_clock() {
    use lbp_sim::scene::{Endpoint, JointKind, PistonClock, SceneJoint};
    let mut params = JointParams {
        mode: 0,
        animation_pattern: 1,
        ..Default::default()
    };
    params.motion[3] = 0.05;
    let mut d = lbp_sim::PistonDrive {
        joint: SceneJoint {
            thing: 0,
            kind: JointKind::Motor,
            a: Endpoint::Static(0),
            b: Endpoint::Static(1),
            world_a: [0.; 2],
            world_b: [0.; 2],
            local_a: [0.; 2],
            local_b: [0.; 2],
            length: 0.,
            min_length: 0.,
            max_length: 0.,
            params,
        },
        clock: PistonClock::default(),
    };
    let hz = lbp_sim::TICK_HZ as f32;
    // Free running: 0.05 rad per frame, one frame a tick.
    assert!((d.turn(10., None) - 0.05 * hz).abs() < 1e-3);
    // Switched (mode 0, on/off): it turns while on and stops when off.
    assert!((d.turn(20., Some((1, 1.))) - 0.05 * hz).abs() < 1e-3);
    assert!(d.turn(30., Some((0, 0.))).abs() < 1e-6);
}

#[test]
fn d1109_gyroscopes_right_their_body_and_rotators_turn_at_their_speed() {
    // A box carrying a gyroscope (Thing 2) as saved in the Rookie Test: range π/15 per
    // tick, strength 1.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 1,
        vel_range: 0.2094394,
        accel_strength: 1.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let gyro = scene.bodies[0].spins[0];
    // Tilted by 1 rad and still: it turns back, no faster than its range per tick.
    let (mut angle, mut w) = (1f32, 0f32);
    for _ in 0..120 {
        w = gyro.turn(angle, w, 1.);
        assert!(w.abs() <= 0.2094394 + 1e-6, "speed {w}");
        angle += w;
    }
    assert!(angle.abs() < 0.05, "upright again: {angle}");
    // Switched off, it leaves the body alone.
    assert_eq!(gyro.turn(1., 0.3, 0.), 0.3);
    // A rotator (configuration 9) speeds up by accel × range per tick to its speed.
    let rot = lbp_sim::scene::SpinTweak {
        thing: 2,
        configuration: 9,
        behavior: 0,
        range: 0.20943,
        accel: 0.125,
        decel: 0.125,
        speed: 0.03490659,
        ..Default::default()
    };
    let w = rot.turn(0., 0., 1.);
    assert!((w - 0.20943 * 0.125).abs() < 1e-6, "{w}");
    assert!((rot.turn(0., w, 1.) - 0.03490659).abs() < 1e-6);
}

#[test]
fn d1144_aim_tweaks_turn_up_along_velocity_stick_and_target() {
    use lbp_sim::scene::SpinTweak;
    let tweak = |configuration| SpinTweak {
        configuration,
        range: 0.2094394,
        accel: 1.,
        min_range: 10.,
        max_range: 500.,
        angle_range: 180.,
        ..Default::default()
    };
    // Configuration 3 aims along the velocity: moving right, an upright body turns
    // clockwise (negative), moving left anticlockwise, upright when moving up.
    let along = tweak(3);
    let right = along.aim([5., 0.], None);
    assert!(along.turn_along(0., 0., 1., right) < 0.);
    assert!(along.turn_along(0., 0., 1., along.aim([-5., 0.], None)) > 0.);
    assert_eq!(along.target_along(0., along.aim([0., 5.], None)).0, 0.);
    // Slower than the dead zone (x² + y² under 0.1) it wants no turn and brakes.
    let slow = along.aim([0.3, 0.], None);
    assert_eq!(along.target_along(0., slow).0, 0.);
    let w = along.turn_along(0., 0.5, 1., slow);
    assert!((w - (0.5 - 0.2094394)).abs() < 1e-6, "{w}");
    // Configuration 4 sees no stick here, so it brakes too.
    let stick = tweak(4);
    assert_eq!(stick.aim([5., 0.], Some([100., 0.])), [0., 0.]);
    // Configuration 5 aims at its target, and turns nothing towards none.
    let seek = tweak(5);
    assert!(seek.turn_along(0., 0., 1., seek.aim([0., 0.], Some([100., 0.]))) < 0.);
    assert_eq!(seek.target_along(0., seek.aim([0., 0.], None)).0, 0.);
    // Its search is the follower's: inside the distance band only.
    assert!(seek.sees([100., 0.], [0., 1.]));
    assert!(!seek.sees([5., 0.], [0., 1.]));
    assert!(!seek.sees([600., 0.], [0., 1.]));
    assert!(!along.sees([100., 0.], [0., 1.]));
    // The gyroscope still aims at world up.
    assert_eq!(tweak(1).aim([5., 0.], None), [0., 1.]);
    // Built from a level: configurations 3, 4 and 5 now ride their body.
    for c in [3, 4, 5] {
        let mut things = vec![
            floor(),
            thing(
                1,
                matrix(0., 300., 0., 0., 1., 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
        ];
        things[2].parent = Some(1);
        things[2].data.physics_tweak = Some(PhysicsTweak {
            configuration: c,
            vel_range: 0.2094394,
            accel_strength: 1.,
            ..Default::default()
        });
        assert_eq!(build_scene(&things).bodies[0].spins[0].configuration, c);
    }
}

#[test]
fn d1146_recorded_moves_play_by_activation_and_restart_or_pause_when_off() {
    use lbp_level::parts::RecordingPlayer;
    use lbp_sim::scene::PlaybackTweak;
    let tweak = |behavior, dir| PlaybackTweak {
        behavior,
        player: RecordingPlayer {
            speed: 1.,
            dir,
            ..Default::default()
        },
        ..Default::default()
    };
    // On, the head moves by activation × speed from a fresh start.
    let mut digital = tweak(0, 0);
    digital.pos = 7.;
    assert!(digital.tick(1.));
    assert_eq!(digital.pos, 1.);
    assert!(digital.tick(1.));
    assert_eq!(digital.pos, 2.);
    // Off, a digital input restarts next time; an analog one pauses where it was.
    assert!(!digital.tick(0.));
    digital.tick(1.);
    assert_eq!(digital.pos, 1.);
    let mut analog = tweak(1, 0);
    analog.tick(0.5);
    analog.tick(0.5);
    assert!(!analog.tick(0.0005));
    analog.tick(0.5);
    assert_eq!(analog.pos, 1.5);
    // Forward holds the last frame, backward runs from it, back and forth turns.
    let mut p = tweak(0, 0);
    p.pos = 12.;
    assert_eq!(p.frame(5), 4.);
    p.player.dir = 1;
    p.pos = 1.;
    assert_eq!(p.frame(5), 3.);
    p.player.dir = 2;
    p.pos = 6.;
    assert_eq!(p.frame(5), 2.);
    // Frames are offsets from the start pose, linear between neighbours.
    let frames = [[0., 0., 0.], [10., 0., 0.], [10., 20., 0.]];
    assert_eq!(PlaybackTweak::sample(&frames, 0.5), [5., 0., 0.]);
    assert_eq!(PlaybackTweak::sample(&frames, 1.25), [10., 5., 0.]);
    assert_eq!(PlaybackTweak::sample(&frames, 9.), [10., 20., 0.]);
    assert_eq!(PlaybackTweak::sample(&[], 1.), [0.; 3]);
    // Built from a level: configuration 13 rides its body with its saved player.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 13,
        behavior: 1,
        recording: Some(RecordingPlayer {
            speed: 0.5,
            dir: 2,
            ..Default::default()
        }),
        ..Default::default()
    });
    let scene = build_scene(&things);
    let b = &scene.bodies[0];
    assert_eq!(b.playbacks[0].player.dir, 2);
    assert_eq!(b.playbacks[0].behavior, 1);
    assert!(b.layers.is_empty() && b.spins.is_empty() && b.moves.is_empty());
}

#[test]
fn d1145_layer_switches_step_a_layer_or_go_to_a_set_one_when_there_is_room() {
    use lbp_sim::scene::LayerTweak;
    let tweak = |z_behavior| LayerTweak {
        z_behavior,
        ..Default::default()
    };
    let room = |_: f32| true;
    // Back (0) and front (1) step one layer on the input's rising edge, once.
    let mut back = tweak(0);
    assert_eq!(back.tick(1., 1, 0., room), -200.);
    assert_eq!(back.tick(1., 1, -200., room), 0.);
    let mut front = tweak(1);
    assert_eq!(front.tick(1., 1, -200., room), 200.);
    // Turning off moves nothing back.
    assert_eq!(front.tick(0., 1, 0., room), 0.);
    // Out of the depth band (-511 to 110) the step is dropped.
    let mut front = tweak(1);
    assert_eq!(front.tick(1., 1, 0., room), 0.);
    assert!(!front.waiting);
    // Set layers 2..4 go to layers 1..3 from the back: z -479 is layer 1, -279 layer 2.
    assert_eq!(tweak(2).delta(-479., 1), 0);
    assert_eq!(tweak(3).delta(-479., 1), 1);
    assert_eq!(tweak(4).delta(-479., 1), 2);
    assert_eq!(tweak(2).delta(-79., 1), -2);
    let mut deep = tweak(4);
    assert_eq!(deep.tick(1., 1, -479., room), 400.);
    // Blocked, a set-layer move waits again and tries one layer less.
    let mut deep = tweak(4);
    assert_eq!(deep.tick(1., 1, -479., |dz| dz < 300.), 200.);
    let mut stuck = tweak(4);
    assert_eq!(stuck.tick(1., 1, -479., |_| false), 0.);
    assert!(stuck.waiting);
    assert_eq!(stuck.tick(1., 1, -479., room), 400.);
    // A zero count clears the wait and moves nothing.
    let mut none = tweak(1);
    assert_eq!(none.tick(1., 0, -200., room), 0.);
    assert!(!none.waiting);
    // Built from a level: configuration 12 rides its body as a layer switch.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 12,
        z_behavior: 3,
        waiting_to_move: true,
        ..Default::default()
    });
    let scene = build_scene(&things);
    assert_eq!(scene.bodies[0].layers[0].z_behavior, 3);
    assert!(scene.bodies[0].layers[0].waiting);
    assert!(scene.bodies[0].spins.is_empty() && scene.bodies[0].moves.is_empty());
}

#[test]
fn d1110_movers_reach_their_velocity_and_followers_stop_short_of_their_target() {
    // A box carrying a mover (Thing 2) as saved in the Rookie Test: 35 up per tick.
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
    ];
    things[2].parent = Some(1);
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 8,
        middle_vel: [0., 35., 0.],
        vel_range: 28.,
        accel_strength: 0.125,
        decel_strength: 0.125,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let mover = scene.bodies[0].moves[0];
    let up = [0., 1.];
    // From a sideways drift it gains accel Ã— range (3.5) a tick up and loses the drift.
    let v = mover.push([2., 0.], up, None, 1.);
    assert!(v[0].abs() < 1e-6 && (v[1] - 3.5).abs() < 1e-5, "{v:?}");
    let mut v = [0f32, 0.];
    for _ in 0..12 {
        v = mover.push(v, up, None, 1.);
    }
    assert!((v[1] - 35.).abs() < 1e-4, "{v:?}");
    assert_eq!(mover.push([1., 2.], up, None, 0.), [1., 2.]);
    // A follower (configuration 11) with the census's numbers walks to 100 short of its
    // target, never faster than its range, and ignores a target past max_range.
    let follower = lbp_sim::scene::MoveTweak {
        configuration: 11,
        min_range: 100.,
        max_range: 2100.,
        angle_range: 360.,
        up_down: true,
        vel: [0., 0.],
        ..mover
    };
    let target = [1000f32, 0.];
    let (mut x, mut v) = (0f32, [0f32, 0.]);
    for _ in 0..200 {
        let d = [target[0] - x, 0.];
        let d = follower.sees(d, up).then_some(d);
        v = follower.push(v, up, d, 1.);
        assert!(v[0].abs() <= 28. + 1e-4, "speed {v:?}");
        x += v[0];
    }
    assert!((895. ..920.).contains(&x), "stopped at {x}");
    assert!(!follower.sees([3000., 0.], up) && !follower.sees([50., 0.], up));
}

#[test]
fn d1111_keyed_followers_find_tags_with_their_colour_and_name() {
    // A keyed follower box (Things 1, 2), a red "A" tag on a second box turned a quarter
    // (Things 3, 4), a red "A" tag on the follower's own box (5), a loose red "A" tag (6),
    // a blue "A" tag (7) and a red "A" tag with no position of its own under Thing 4 (8).
    let tag = |colour: i32| SwitchKey {
        colour_index: colour,
        name: "A".into(),
    };
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut things = vec![
        floor(),
        thing(
            1,
            matrix(0., 300., 0., 0., 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(2, matrix(0., 300., 0., 0., 1., 1.), None, false),
        thing(
            3,
            matrix(800., 300., 0., quarter, 1., 1.),
            Some(rect(100., 100., 7)),
            true,
        ),
        thing(4, matrix(800., 340., 0., 0., 1., 1.), None, false),
        thing(5, matrix(10., 300., 0., 0., 1., 1.), None, false),
        thing(6, matrix(-500., 300., 0., 0., 1., 1.), None, false),
        thing(7, matrix(300., 300., 0., 0., 1., 1.), None, false),
        thing(8, matrix(0., 0., 0., 0., 1., 1.), None, false),
    ];
    things[8].data.pos = None;
    for (t, p) in [(2, 1), (4, 3), (5, 1), (8, 4)] {
        things[t].parent = Some(p);
    }
    for (t, c) in [(4, 1), (5, 1), (6, 1), (7, 2), (8, 1)] {
        things[t].data.switch_key = Some(tag(c));
    }
    things[2].data.physics_tweak = Some(PhysicsTweak {
        configuration: 11,
        color_index: 1,
        name: "A".into(),
        follow_key: true,
        vel_range: 28.,
        accel_strength: 0.125,
        max_range: 2100.,
        angle_range: 360.,
        ..Default::default()
    });
    let scene = build_scene(&things);
    let key = scene.bodies[0].moves[0].key.expect("keyed");
    let red: Vec<_> = scene.tags.iter().filter(|g| g.key == key).collect();
    assert_eq!(
        red.iter().map(|g| g.thing).collect::<Vec<_>>(),
        [4, 5, 6, 8]
    );
    assert_eq!(
        (red[0].body, red[1].body, red[2].body),
        (Some(1), Some(0), None)
    );
    // The tag on the turned box sits 40 along that box's x, back at (800, 340) in the world.
    let at = red[0].at;
    assert!((at[0] - 40.).abs() < 1e-3 && at[1].abs() < 1e-3, "{at:?}");
    let world = scene.bodies[1].frame.world(at);
    assert!((world[0] - 800.).abs() < 1e-3 && (world[1] - 340.).abs() < 1e-3);
    // A tag with no position takes its parent's: the same spot on the same box.
    assert_eq!(red[3].body, Some(1));
    assert!((red[3].at[0] - 40.).abs() < 1e-3 && red[3].at[1].abs() < 1e-3);
    // Spots: a body's tags move with it; loose tags keep their world point.
    let mut w = World::new();
    let (a, b) = (w.spawn_empty().id(), w.spawn_empty().id());
    let on_body = lbp_sim::TagMarks(vec![(key, at)]);
    let loose = lbp_sim::TagMarks(vec![(key, [-500., 300.])]);
    let (p, r) = (
        Position::new(Vec2::new(10., 3.)),
        Rotation::radians(quarter),
    );
    let spots = lbp_sim::tag_spots([(a, &on_body, Some((&p, &r))), (b, &loose, None)]);
    let s0 = spots[0].at;
    assert!(
        (s0[0] - 1000.).abs() < 1e-2 && (s0[1] - 340.).abs() < 1e-2,
        "{s0:?}"
    );
    assert_eq!(
        (spots[0].body, spots[1].body, spots[1].at),
        (Some(a), None, [-500., 300.])
    );
}

#[test]
fn d1147_attractors_pick_the_highest_priority_own_or_shared_block() {
    use lbp_level::parts::AttractorBlock;
    use lbp_sim::scene::AttractTweak;
    let tweak = |thing, priority, beyond_group| AttractTweak {
        thing,
        block: AttractorBlock {
            priority,
            beyond_group,
            ..Default::default()
        },
    };
    let (own_low, other_high, shared_mid) =
        (tweak(1, 1., false), tweak(2, 9., false), tweak(3, 5., true));
    // Another group's tweak only counts when it is shared; the highest priority wins.
    let picked =
        AttractTweak::pick(&[(&own_low, true), (&other_high, false), (&shared_mid, false)]);
    assert_eq!(picked.map(|t| t.thing), Some(3));
    let picked = AttractTweak::pick(&[(&own_low, true), (&other_high, false)]);
    assert_eq!(picked.map(|t| t.thing), Some(1));
    assert!(AttractTweak::pick(&[(&other_high, false)]).is_none());
    // Of equal priorities the first walked stays.
    let twin = tweak(4, 1., false);
    let picked = AttractTweak::pick(&[(&own_low, true), (&twin, true)]);
    assert_eq!(picked.map(|t| t.thing), Some(1));
    // The record is active above the input threshold.
    assert!(AttractTweak::active(1.));
    assert!(!AttractTweak::active(0.));
}

#[test]
fn d1148_aim_tweaks_take_the_stick_inputs_mirror_and_lead_the_target() {
    use lbp_sim::scene::SpinTweak;
    // Configuration 4 aims along its wired stick (slots 1 and 2, x then y).
    let mut stick = SpinTweak {
        configuration: 4,
        range: 0.2094394,
        accel: 1.,
        stick: [1., 0.],
        ..Default::default()
    };
    let right = stick.aim([0., 0.], None);
    assert_eq!(right, [1., 0.]);
    assert!(stick.turn_along(0., 0., 1., right) < 0.);
    // Mirrored, x is negated, so the same stick turns the other way.
    stick.flip = true;
    assert_eq!(stick.aim([0., 0.], None), [-1., 0.]);
    // Configuration 5's lead: the target point's velocity, vel + omega x r.
    assert_eq!(SpinTweak::lead([2., 0.], 0., [10., 0.]), [2., 0.]);
    assert_eq!(SpinTweak::lead([0., 0.], 0.5, [10., 0.]), [0., 5.]);
    assert_eq!(SpinTweak::lead([1., 1.], 0.5, [0., 4.]), [-1., 1.]);
    // Built from a level, a tweak on a mirrored body is flipped and an upright one is not.
    for (sx, flip) in [(1., false), (-1., true)] {
        let mut things = vec![
            floor(),
            thing(
                1,
                matrix(0., 300., 0., 0., sx, 1.),
                Some(rect(100., 100., 7)),
                true,
            ),
            thing(2, matrix(0., 300., 0., 0., sx, 1.), None, false),
        ];
        things[2].parent = Some(1);
        things[2].data.physics_tweak = Some(PhysicsTweak {
            configuration: 4,
            vel_range: 0.2094394,
            accel_strength: 1.,
            ..Default::default()
        });
        let spin = build_scene(&things).bodies[0].spins[0];
        assert_eq!((spin.flip, spin.stick), (flip, [0., 0.]));
    }
}
