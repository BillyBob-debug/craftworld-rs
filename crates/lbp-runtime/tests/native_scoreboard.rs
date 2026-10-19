//! D510: the native `Score_Board` on authored synthetic data (no game bytes, no script
//! loaded): a world, two scoreboards and two Sackboys.
use lbp_level::parts::{ScriptPart, World as WorldPart};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{
    start_native, LevelScripts, LevelWorld, NativeScript, NativeStep, SCORE_BOARD_SCRIPT,
    WORLD_MOO_CAMERA_SCRIPT,
};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};

const BUDGET: u64 = 100_000;
const LOCK: &str = "Score_Board.SetScoreLocked__b";
const END: &str = "Score_Board.EnterEndLevelFlow__";

fn script(guid: u32) -> Option<ScriptPart> {
    Some(ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(guid),
            sha1: None,
        }),
        fields: vec![],
    })
}

/// World Thing 0 (the world script); scoreboards 1 and 2.
fn scoreboard_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..3)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].parts = vec!["WORLD", "SCRIPT"];
    t[0].data.world = Some(WorldPart::default());
    t[0].data.script = script(WORLD_MOO_CAMERA_SCRIPT);
    for board in &mut t[1..] {
        board.parent = Some(0);
        board.parts = vec!["SCRIPT"];
        board.data.script = script(SCORE_BOARD_SCRIPT);
    }
    t
}

#[test]
fn d510_the_native_scoreboard_finishes_the_level_when_everyone_is_in() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&scoreboard_level());
    let a = world.add_player([0.; 3], (0., 0.));
    let b = world.add_player([0.; 3], (0., 0.));
    world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    assert_eq!(world.things[1].script_classes[0], "Score_Board");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);

    let board = lbp_runtime::native_script(SCORE_BOARD_SCRIPT).unwrap();
    let moo = lbp_runtime::native_script(WORLD_MOO_CAMERA_SCRIPT).unwrap();
    let run = |port: &dyn NativeScript,
               host: &mut Bindings<LevelWorld>,
               thing: u32,
               function: &str,
               args: &[Value]| {
        let args: Vec<(u16, Value)> = args.iter().map(|v| (4, v.clone())).collect();
        match start_native(port, host, thing, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    let count = |host: &Bindings<LevelWorld>, f: &str| host.calls.get(f).copied().unwrap_or(0);
    let frame = |host: &mut Bindings<LevelWorld>| {
        start_native(board, host, 1, "Update__", &[]).expect("the scoreboard loop")
    };
    let enter = |host: &mut Bindings<LevelWorld>, t: u32| {
        run(board, host, 1, "Enter__Q5Thing", &[Value::Thing(Some(t))]);
    };

    assert_eq!(
        run(board, &mut host, 1, "CanTweak__i", &[Value::Int(0)]),
        Value::Bool(true)
    );
    assert_eq!(
        run(board, &mut host, 1, "GetLastRewardIndex__", &[]),
        Value::Int(0)
    );

    // D511: with nobody in, the frame neither counts the avatars nor writes `Finishers`.
    assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    assert_eq!(count(&host, "Game.GetNumYellowHeadAvatars__"), 0);

    // One of two Sackboys in, then out and the other in: not everyone yet.
    enter(&mut host, a);
    assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    run(
        board,
        &mut host,
        1,
        "Leave__Q5Thing",
        &[Value::Thing(Some(a))],
    );
    enter(&mut host, b);
    enter(&mut host, b);
    assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    assert_eq!(count(&host, LOCK), 0);

    // Both in, but the other scoreboard holds the world's challenge.
    run(
        moo,
        &mut host,
        0,
        "SetCurrentChallenge__Q5Thing",
        &[Value::Thing(Some(2))],
    );
    enter(&mut host, a);
    assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    assert_eq!(count(&host, LOCK), 0);

    // Once the challenge is free this scoreboard takes it and locks the score.
    run(moo, &mut host, 0, "Kick__", &[]);
    assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    assert_eq!(count(&host, LOCK), 1);
    assert_eq!(
        run(moo, &mut host, 0, "GetCurrentChallenge__", &[]),
        Value::Thing(Some(1))
    );
    assert_eq!(
        run(moo, &mut host, 0, "IsScoreLocked__", &[]),
        Value::Bool(true)
    );
    assert_eq!(count(&host, END), 0);

    // The next pass of the loop ends the level and stops.
    assert_eq!(frame(&mut host), Ok(NativeStep::Done(Value::Void)));
    assert_eq!(count(&host, "Score_Board.PostScore__"), 1);
    assert_eq!(count(&host, "Game.SetLevelCompleted__"), 1);
    assert_eq!(count(&host, END), 1);

    // Kick__ clears `Finished`; OnDestroy__ ends the camera override.
    run(board, &mut host, 1, "Kick__", &[]);
    run(board, &mut host, 1, "OnDestroy__", &[]);
    assert_eq!(count(&host, "Score_Board.ClearPostedScore__"), 1);
    assert_eq!(count(&host, "Cam.SetOverrideCameraActivation__bf"), 1);
    assert_eq!(count(&host, "Thing.AddToDeleteList__Q5Thing"), 0);
}
