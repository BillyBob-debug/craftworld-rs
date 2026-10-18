//! D355: the native `Earth` and `OrbitingMoon` ports (the Pod planets' `PlanetThing`)
//! against the scripted run.
//!
//! The golden matrices are the planets level's world matrices from the viewer's trace
//! (`--level-scripts-trace`): the decrypted scripts' Story run for the settled Earth, and
//! the no-scripts run for the first ticks. Numbers only.
use lbp_runtime::{
    native_script, start_native, LevelWorld, NativeStep, ThingState, EARTH_SCRIPT,
    ORBITING_MOON_SCRIPT,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Host, MemberKind, Value};

const EARTH: u32 = 0;
const MOON: u32 = 1;

/// Earth's and the Moon's world matrices in the level file: only their axis lengths
/// matter to the script (MEASURED, the planets level's trace).
fn diag(s: [f32; 3], at: [f32; 3]) -> [f32; 16] {
    let mut m = [0.; 16];
    (m[0], m[5], m[10], m[15]) = (s[0], s[1], s[2], 1.);
    m[12..15].copy_from_slice(&at);
    m
}

fn host() -> Bindings<LevelWorld> {
    let thing = |guid, m| ThingState {
        script_guid: Some(guid),
        world_matrix: Some(m),
        local_matrix: Some(m),
        ..Default::default()
    };
    let world = LevelWorld {
        things: vec![
            thing(
                EARTH_SCRIPT,
                diag([0.961_007_1, 0.961_007_83, 0.961_007_1], [-1250., 0., 0.]),
            ),
            thing(
                ORBITING_MOON_SCRIPT,
                diag([2.112_109, 2.112_109, 2.112_109_7], [2000., 0., 0.]),
            ),
        ],
        ..LevelWorld::default()
    };
    Bindings::new(world)
}

fn create(host: &mut Bindings<LevelWorld>, thing: u32, guid: u32) {
    let port = native_script(guid).expect("a port");
    let done = start_native(port, host, thing, "OnCreate__", &[]);
    assert_eq!(done, Some(Ok(NativeStep::Done(Value::Void))));
}

/// One `Update__rb(point, arrived)`; the answer.
fn update(
    host: &mut Bindings<LevelWorld>,
    thing: u32,
    guid: u32,
    p: [f32; 4],
    arrived: bool,
) -> bool {
    let port = native_script(guid).expect("a port");
    let args = [(16, Value::Vec4(p)), (32, Value::Bool(arrived))];
    match start_native(port, host, thing, "Update__rb", &args) {
        Some(Ok(NativeStep::Done(Value::Bool(b)))) => b,
        other => panic!("Update__rb answered {other:?}"),
    }
}

fn world(host: &Bindings<LevelWorld>, thing: u32) -> [f32; 16] {
    host.world.things[thing as usize].world_matrix.unwrap()
}

fn assert_near(got: [f32; 16], want: [f32; 16], tol: f32) {
    let off = got
        .iter()
        .zip(want)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max);
    assert!(off <= tol, "off by {off}: got {got:?}, want {want:?}");
}

/// The point the Story view hands Earth (the default story group's badge, MEASURED).
const STORY_POINT: [f32; 4] = [629.903, -250.958_97, 1_613.301_4, 0.];
const NO_TURN: [f32; 4] = [0., 0., 1., 0.];

/// The first update snaps, the next ones ease (the no-scripts run's ticks 1 and 2), and
/// Earth comes to rest where the scripted Story run leaves it.
#[test]
fn d355_earth_turns_to_the_story_group_as_the_script_does() {
    let mut h = host();
    create(&mut h, EARTH, EARTH_SCRIPT);
    // Tick 1: no point yet; the first update snaps (Updated is false) and the planet is
    // where its class puts it, at its own axis lengths.
    let arrived = update(&mut h, EARTH, EARTH_SCRIPT, NO_TURN, false);
    assert!(arrived);
    assert_near(
        world(&h, EARTH),
        diag([0.961_007_1, 0.961_007_83, 0.961_007_1], [-1250., 0., 0.]),
        1e-6,
    );
    // Story opens: the view's spin offset, a new point, eased from here.
    // The Story view's `EarthSpinOffset` (PlanetsView view 6), as `PlanetsWorld` hands it.
    let offset = Value::Float(-0.7);
    let this = Value::Thing(Some(EARTH));
    h.set_member(
        MemberKind::Thing,
        &this,
        "PlanetThing",
        "SpinOffset",
        offset,
    )
    .unwrap();
    let arrived = update(&mut h, EARTH, EARTH_SCRIPT, STORY_POINT, false);
    assert!(!arrived);
    assert_near(
        world(&h, EARTH),
        [
            0.729_212_4,
            3.232_043_3e-5,
            0.625_926_5,
            0.,
            0.002_232_314,
            0.961_001_6,
            -0.002_650_296_5,
            0.,
            -0.625_922_56,
            0.003_465_001_2,
            0.729_207_6,
            0.,
            -1250.,
            0.,
            0.,
            1.,
        ],
        1e-5,
    );
    let mut arrived = false;
    let mut ticks = 1;
    while !arrived {
        arrived = update(&mut h, EARTH, EARTH_SCRIPT, STORY_POINT, arrived);
        ticks += 1;
        assert!(ticks < 100, "Earth never arrives");
    }
    // At rest, then snapped there for good.
    update(&mut h, EARTH, EARTH_SCRIPT, STORY_POINT, true);
    assert_near(
        world(&h, EARTH),
        [
            0.461_839_85,
            0.050_123_28,
            0.841_264_67,
            0.,
            0.088_781_92,
            0.951_074_96,
            -0.105_405_62,
            0.,
            -0.838_067_05,
            0.128_375_23,
            0.452_435_73,
            0.,
            -1250.,
            0.,
            0.,
            1.,
        ],
        1e-5,
    );
}

/// The Moon is put at its class's place with its own axis lengths on its first update.
#[test]
fn d355_moon_is_placed_on_its_first_update() {
    let mut h = host();
    create(&mut h, MOON, ORBITING_MOON_SCRIPT);
    assert!(update(&mut h, MOON, ORBITING_MOON_SCRIPT, NO_TURN, false));
    assert_near(
        world(&h, MOON),
        diag([2.112_109, 2.112_109, 2.112_109_7], [2000., 0., 0.]),
        1e-6,
    );
}
