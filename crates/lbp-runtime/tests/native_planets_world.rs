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
    // A profile that has seen the pod introduction.
    h.world.progression_flags = 3;
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

fn vec(h: &mut Bindings<LevelWorld>, field: &str) -> [f32; 4] {
    match member(h, field) {
        Value::Vec4(v) => v,
        other => panic!("{field} is {other:?}"),
    }
}

/// D379: on a fresh profile the pod introduction starts on the first frame with the
/// camera far below the planets, zooms straight in over 450 more frames to the rest
/// framing, then sets the progression flags 3 and is not started again.
#[test]
fn d379_pod_introduction_zooms_in_then_sets_the_flags() {
    let mut h = host();
    assert_eq!(run(&mut h, "OnCreate__", &[]), Value::Void);
    frame(&mut h, true);
    assert_eq!(member(&mut h, "State"), Value::Int(1));
    assert_near(&vec(&mut h, "cur_pos"), &[-500., -2000., 2000., 1.], 1e-3);
    assert_near(&vec(&mut h, "cur_foc"), &[-100., -800., 1000., 1.], 1e-3);
    for _ in 0..225 {
        frame(&mut h, false);
    }
    assert_near(&vec(&mut h, "cur_pos"), &[-250., -1125., 1565., 1.], 1e-2);
    for _ in 0..225 {
        frame(&mut h, false);
    }
    assert_eq!(member(&mut h, "State"), Value::Int(1));
    assert_near(&vec(&mut h, "cur_pos"), &[0., -250., 1130., 1.], 1e-3);
    assert_eq!(h.world.progression_flags, 0);
    frame(&mut h, false);
    assert_eq!(member(&mut h, "State"), Value::Int(0));
    assert_eq!(h.world.progression_flags, 3);
    frame(&mut h, false);
    assert_eq!(member(&mut h, "State"), Value::Int(0));
}

/// D398: the introduction asks for `pod_1st_time` (interrupt and replay set) as it
/// starts and, with the line's length known, waits for it after the zoom: it ends on
/// the frame after the 32-second line's 960 ticks, where the zoom alone takes 451.
#[test]
fn d398_pod_introduction_waits_for_its_voice_over() {
    let mut h = host();
    h.world
        .media
        .voice_over_lengths
        .insert("pod_1st_time".into(), 960);
    run(&mut h, "OnCreate__", &[]);
    frame(&mut h, true);
    h.world.media.tick();
    let asked = &h.world.media.voice_overs;
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].name.as_deref(), Some("pod_1st_time"));
    assert_eq!(asked[0].flags, [true, true]);
    // D1000: the video request comes first, with a null Thing and the flag clear.
    let videos = &h.world.media.videos;
    assert_eq!(videos.len(), 1);
    assert_eq!(
        (videos[0].thing, videos[0].guid, videos[0].flag),
        (None, 139230, false)
    );
    let mut frames = 1;
    while member(&mut h, "State") == Value::Int(1) {
        frame(&mut h, false);
        h.world.media.tick();
        frames += 1;
        assert!(frames < 2000, "the introduction never ended");
    }
    assert_eq!(frames, 961);
    assert_eq!(h.world.progression_flags, 3);
}

/// D1003: with the intro video's length known (here longer than the line), the
/// introduction waits for it too, ending on the frame after its 1200 ticks: the pod computer's handle stays set while
/// `UpdateVideo` answers true and is -1 once it answers false.
#[test]
fn d1003_pod_introduction_waits_for_its_video() {
    let mut h = host();
    h.world
        .media
        .voice_over_lengths
        .insert("pod_1st_time".into(), 960);
    h.world.media.video_lengths.insert(139230, 1200);
    run(&mut h, "OnCreate__", &[]);
    frame(&mut h, true);
    h.world.media.tick();
    assert_eq!(member(&mut h, "IntroVideo"), Value::Int(0));
    let mut frames = 1;
    while member(&mut h, "State") == Value::Int(1) {
        frame(&mut h, false);
        h.world.media.tick();
        frames += 1;
        assert!(frames < 2000, "the introduction never ended");
    }
    assert_eq!(frames, 1201);
    assert_eq!(member(&mut h, "IntroVideo"), Value::Int(-1));
    assert!(h.world.media.video.is_none());
    assert_eq!(h.world.progression_flags, 3);
}

/// D1003: a Bink header's length in ticks (the Pod intro's numbers, FOUND).
#[test]
fn d1003_bink_ticks() {
    let mut head = vec![0u8; lbp_runtime::BINK_HEADER];
    head[..4].copy_from_slice(b"BIKi");
    for (o, v) in [(8, 378u32), (0x1c, 2997), (0x20, 100)] {
        head[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    assert_eq!(lbp_runtime::bink_ticks(&head), Ok(379));
    assert!(lbp_runtime::bink_ticks(&head[..8]).is_err());
    head[0] = b'X';
    assert!(lbp_runtime::bink_ticks(&head).is_err());
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

/// D381: finishing story level 1 adds the link to level 2 (once, and not for a level
/// whose next one is open); the Pod's world then goes to its unlock state, clears the
/// list and level 2 is unlocked, so a second finish adds nothing.
#[test]
fn d381_a_finished_level_links_to_the_next_and_the_pod_unlocks_it() {
    use lbp_runtime::{PendingLink, SearchSlot};
    let mut h = host();
    h.world.progression_flags = 3;
    let slot = |n, link, locked| SearchSlot {
        id: (0, n),
        link,
        initially_locked: locked,
        ..Default::default()
    };
    h.world.story_slots = vec![
        slot(124806, Some((0, 124819)), false),
        slot(124819, Some((0, 124808)), true),
        slot(124808, None, true),
    ];
    let link = PendingLink {
        from: (0, 124806),
        to: (0, 124819),
    };
    assert_eq!(h.world.finish_story_level((0, 124806)), Some(link));
    assert_eq!(h.world.finish_story_level((0, 124806)), Some(link));
    assert_eq!(h.world.story_progress.pending_links, vec![link]);
    assert_eq!(h.world.story_progress.completed, vec![(0, 124806)]);
    assert_eq!(h.world.finish_story_level((6, 124806)), None);
    assert!(!h.world.slot_unlocked((0, 124819)));

    assert_eq!(run(&mut h, "OnCreate__", &[]), Value::Void);
    frame(&mut h, true);
    assert_eq!(member(&mut h, "State"), Value::Int(2));
    frame(&mut h, false);
    assert_eq!(member(&mut h, "State"), Value::Int(0));
    assert!(h.world.story_progress.pending_links.is_empty());
    assert!(h.world.slot_unlocked((0, 124819)));
    assert_eq!(h.world.finish_story_level((0, 124806)), None);
}

/// D396 (part 143): a group is locked while every slot in it is, and opens with the
/// first of them.
#[test]
fn d396_a_group_is_locked_while_all_its_slots_are() {
    use lbp_runtime::SearchSlot;
    let mut h = host();
    let (group, pack) = ((4, 7), (9, 8));
    let slot = |id, group, locked| SearchSlot {
        id,
        group,
        initially_locked: locked,
        ..Default::default()
    };
    h.world.story_slots = vec![
        slot(group, None, false),
        slot((0, 1), Some(group), true),
        slot((0, 2), Some(group), true),
        slot(pack, None, true),
    ];
    assert!(!h.world.slot_unlocked(group));
    assert!(!h.world.slot_unlocked(pack));
    h.world.story_progress.unlocked.push((0, 2));
    assert!(h.world.slot_unlocked(group));
    h.world.story_slots[3].initially_locked = false;
    assert!(h.world.slot_unlocked(pack));
}

/// D1004: the Pod's music is set once, with a whole-second delay, and its stereo pairs'
/// volumes are set for view 0; later frames set nothing while the view stays.
#[test]
fn d1004_pod_music_is_set_once() {
    let mut h = host();
    run(&mut h, "OnCreate__", &[]);
    frame(&mut h, true);
    for _ in 0..10 {
        frame(&mut h, false);
    }
    assert_eq!(h.world.media.music_starts, 1);
    let music = h.world.media.music.clone().expect("no music set");
    assert_eq!(
        (music.guid, music.settings, music.volume),
        (42103, 42108, 1.)
    );
    assert!((1. ..=8.).contains(&music.delay) && music.delay.fract() == 0.);
    assert_eq!(music.pair_volumes, [0., 0., 0., 1., 0., 0.]);
    let pair_calls = h.world.audio_commands["SetInteractiveMusicStereoPairVolume"];
    assert_eq!(pair_calls, 6);
    assert_eq!(member(&mut h, "IMusicPlaying"), Value::Bool(true));
    assert_eq!(member(&mut h, "IMusicInit"), Value::Bool(false));
}

/// D1004: the stereo pairs by the view type (FOUND, `UpdateInteractiveMusic__`).
#[test]
fn d1004_music_pairs() {
    use lbp_runtime::music_pairs;
    assert_eq!(music_pairs(0), [0., 0., 0., 1., 0., 0.]);
    assert_eq!(music_pairs(1), [1., 0., 0., 1., 0., 0.]);
    assert_eq!(music_pairs(3), [1., 0., 1., 1., 0., 0.]);
    assert_eq!(music_pairs(7), [1., 0., 0., 1., 1., 0.]);
    assert_eq!(music_pairs(28), [1., 1., 0., 1., 0., 0.]);
    assert_eq!(music_pairs(38), [0., 0., 0., 1., 0., 0.]);
}
