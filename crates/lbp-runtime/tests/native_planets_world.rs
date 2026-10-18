//! D356: the native `PlanetsWorld` port (the Pod planets level's world script) against the
//! scripted run.
//!
//! The golden values are the viewer's trace of the decrypted scripts' Story run
//! (`--level-scripts-trace`): the camera the script writes once the pod computer is in
//! use and the world Thing's matrix once the Story view has settled. Numbers only.
use lbp_runtime::{
    start_native, LevelWorld, NativeStep, PlanetsView, ThingState, PLANETS_WORLD_PORT,
    PLANETS_WORLD_SCRIPT,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Host, MemberKind, Value};

const WORLD: u32 = 0;
const EARTH: u32 = 1;
const MOON: u32 = 2;
const EARTH_AT: [f32; 4] = [-1250., 0., 0., 1.];
const MOON_AT: [f32; 4] = [2000., 0., 0., 1.];

fn at(p: [f32; 4]) -> [f32; 16] {
    let mut m = [0.; 16];
    (m[0], m[5], m[10], m[15]) = (1., 1., 1., 1.);
    m[12..16].copy_from_slice(&p);
    m
}

fn host() -> Bindings<LevelWorld> {
    let thing = |guid: Option<u32>, m| ThingState {
        script_guid: guid,
        world_matrix: Some(m),
        local_matrix: Some(m),
        ..Default::default()
    };
    let world = LevelWorld {
        things: vec![
            thing(Some(PLANETS_WORLD_SCRIPT), at([0., 0., 0., 1.])),
            thing(None, at(EARTH_AT)),
            thing(None, at(MOON_AT)),
        ],
        ..LevelWorld::default()
    };
    Bindings::new(world)
}

fn run(h: &mut Bindings<LevelWorld>, function: &str, args: &[(u16, Value)]) -> Value {
    match start_native(&PLANETS_WORLD_PORT, h, WORLD, function, args) {
        Some(Ok(NativeStep::Done(v))) => v,
        other => panic!("{function} answered {other:?}"),
    }
}

/// One frame of `UpdatePlanetsView`, as the engine calls it; whether the view is still
/// blending.
fn frame(h: &mut Bindings<LevelWorld>, first: bool) -> bool {
    let args = [
        (4, Value::Bool(false)),
        (8, Value::Thing(Some(EARTH))),
        (12, Value::Thing(Some(MOON))),
        (16, Value::Bool(first)),
        (17, Value::Bool(true)),
    ];
    let name = "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb";
    run(h, name, &args) == Value::Bool(true)
}

fn member(h: &mut Bindings<LevelWorld>, field: &str) -> Value {
    let this = Value::Thing(Some(WORLD));
    h.get_member(MemberKind::Thing, &this, "PlanetsWorld", field)
        .unwrap()
}

fn assert_near(got: &[f32], want: &[f32], tol: f32) {
    let off = got
        .iter()
        .zip(want)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max);
    assert!(off <= tol, "off by {off}: got {got:?}, want {want:?}");
}

/// The first frame with nobody at the pod computer, then the pod computer in use and the
/// Story view asked for: the camera eases toward the in-use framing and stops short where
/// the scripted run's does and the level settles where the scripted Story run puts
/// it, with Earth's spin offset the view's.
#[test]
fn d356_story_view_settles_as_the_script_does() {
    let mut h = host();
    assert_eq!(run(&mut h, "OnCreate__", &[]), Value::Void);
    assert!(!frame(&mut h, true));
    h.world.pod_computer_in_use = true;
    let story = [(4, Value::Int(6))];
    run(&mut h, "SetViewFromRemotePodComputer__i", &story);
    assert!(frame(&mut h, false), "a new view blends");
    let mut blending = true;
    for _ in 0..60 {
        blending = frame(&mut h, false);
    }
    assert!(!blending);
    let Value::Vec4(eye) = member(&mut h, "cur_pos") else {
        panic!("cur_pos")
    };
    assert_near(&eye, &[0., -299.480_35, 902.390_44, 1.], 1e-3);
    let world = h.world.things[WORLD as usize].world_matrix.unwrap();
    let want = [
        1.529_684_4,
        0.441_801_64,
        -1.210_321,
        0.,
        0.,
        1.878_745_4,
        0.685_795_6,
        0.,
        1.288_435_3,
        -0.524_525_4,
        1.436_943_8,
        0.,
        1_912.105_6,
        3_423.465_8,
        -9_460.587,
        1.,
    ];
    assert_near(&world, &want, 2e-3);
    let earth = Value::Thing(Some(EARTH));
    let spin = h.get_member(MemberKind::Thing, &earth, "PlanetThing", "SpinOffset");
    assert_eq!(spin, Ok(Value::Float(-0.7)));
}

/// The view table's Story row and the blend speeds (FOUND, `PlanetsView`).
#[test]
fn d356_planets_views() {
    let mut v = PlanetsView::default();
    v.set(6, EARTH_AT, MOON_AT, false);
    assert_eq!(v.offset, [0., 0., -3750., 0.]);
    assert_eq!((v.yaw, v.pitch, v.roll), (0.7, 0.35, 0.));
    assert_eq!((v.focus, v.focus_planet), (EARTH_AT, 1));
    assert_eq!(
        (v.dof_min, v.dof_max, v.earth_spin_offset),
        (3600., 4400., -0.7)
    );
    let mut moon = PlanetsView::default();
    moon.set(10, EARTH_AT, MOON_AT, false);
    assert_eq!((moon.focus, moon.focus_planet), (MOON_AT, 2));
    let mut start = PlanetsView::default();
    start.set(0, EARTH_AT, MOON_AT, false);
    assert_eq!(PlanetsView::transition_speed(&start, &v), 0.05);
    assert_eq!(PlanetsView::transition_speed(&moon, &v), 0.02);
    let mut v4 = PlanetsView::default();
    v4.set(4, EARTH_AT, MOON_AT, false);
    assert_eq!(PlanetsView::transition_speed(&v4, &v), 0.1);
    // A view the table does not name keeps the focus planet it had.
    v.set(46, EARTH_AT, MOON_AT, false);
    assert_eq!((v.focus_planet, v.offset), (1, [0., 0., -15000., 0.]));
    let half = PlanetsView::between(&start, &moon, 0.5);
    assert_eq!((half.kind, half.focus_planet), (46, 0));
}
