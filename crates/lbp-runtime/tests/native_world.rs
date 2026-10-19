//! D509: the native `WorldMooCamera` (`World`'s code) on authored synthetic data (no game
//! bytes, no script loaded): a world Thing with an 11-second time limit.
use lbp_level::parts::{ScriptField, ScriptPart, ScriptValue, World as WorldPart};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{start_native, LevelScripts, LevelWorld, NativeStep, WORLD_MOO_CAMERA_SCRIPT};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};

const BUDGET: u64 = 100_000;
const SOUND: &str = "Audio.PlaySound__Q5ThingQ7StringA";

/// World Thing 0 with the world script, `TimeLimit` 335 frames; Thing 1 a plain Thing.
fn world_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..2)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].parts = vec!["WORLD", "SCRIPT"];
    t[0].data.world = Some(WorldPart::default());
    t[0].data.script = Some(ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(WORLD_MOO_CAMERA_SCRIPT),
            sha1: None,
        }),
        fields: vec![ScriptField {
            name: "TimeLimit".into(),
            machine_type: 3,
            value: ScriptValue::Int(335),
        }],
    });
    t[1].parent = Some(0);
    t
}

#[test]
fn d509_the_native_world_counts_down_and_keeps_the_multiplier() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&world_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    assert_eq!(world.things[0].script_classes[0], "WorldMooCamera");
    assert_eq!(world.things[0].script_classes[1], "World");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);

    let port = lbp_runtime::native_script(WORLD_MOO_CAMERA_SCRIPT).unwrap();
    let run = |host: &mut Bindings<LevelWorld>, function: &str, args: &[Value]| {
        let args: Vec<(u16, Value)> = args.iter().map(|v| (4, v.clone())).collect();
        match start_native(port, host, 0, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    let sounds = |host: &Bindings<LevelWorld>| host.calls.get(SOUND).copied().unwrap_or(0);
    let frame = |host: &mut Bindings<LevelWorld>| {
        start_native(port, host, 0, "Update__", &[]).expect("the world loop")
    };

    // Creating the level ran `OnCreate__`, which ran the first frame and yielded.
    assert_eq!(
        run(&mut host, "GetTimeElapsedTowardsTimeLimit__", &[]),
        Value::Int(1)
    );
    assert_eq!(run(&mut host, "GetTimeLimit__", &[]), Value::Int(335));
    assert_eq!(
        run(&mut host, "ShouldShowPieChart__", &[]),
        Value::Bool(false)
    );

    // Collected fluff starts the co-op multiplier for 30 frames and shows the pie chart.
    let fluff = run(
        &mut host,
        "OnCollectFluff__Q18Game.EPlayerNumberf",
        &[Value::Int(0), Value::Float(2.)],
    );
    assert_eq!(fluff, Value::Float(2.));
    assert_eq!(run(&mut host, "GetScoreMultiplier__", &[]), Value::Int(1));
    assert_eq!(
        run(&mut host, "ShouldShowPieChart__", &[]),
        Value::Bool(true)
    );

    // 10 countdown sounds (frames 30-300), the game-over sound at 330, the scoreboard at 335.
    for _ in 1..334 {
        assert_eq!(frame(&mut host), Ok(NativeStep::Yielded));
    }
    assert_eq!(sounds(&host), 11);
    assert!(host.world.scoreboard_requests.is_empty());
    // The multiplier timer ran out and the bonuses reset the counter.
    assert_eq!(run(&mut host, "GetScoreMultiplier__", &[]), Value::Int(0));
    frame(&mut host).unwrap();
    assert_eq!(host.world.scoreboard_requests, vec![true]);
    frame(&mut host).unwrap();
    assert_eq!(host.world.scoreboard_requests, vec![true]);

    run(
        &mut host,
        "SetCurrentChallenge__Q5Thing",
        &[Value::Thing(Some(1))],
    );
    assert_eq!(
        run(&mut host, "GetCurrentChallenge__", &[]),
        Value::Thing(Some(1))
    );
    assert_eq!(run(&mut host, "IsScoreLocked__", &[]), Value::Bool(false));
    run(&mut host, "Kick__", &[]);
    assert_eq!(
        run(&mut host, "GetCurrentChallenge__", &[]),
        Value::Thing(None)
    );
    run(&mut host, "SetTimeLimit__i", &[Value::Int(0)]);
    assert_eq!(
        run(&mut host, "GetTimeElapsedTowardsTimeLimit__", &[]),
        Value::Int(0)
    );

    run(&mut host, "OnDestroy__", &[]);
    assert_eq!(frame(&mut host), Ok(NativeStep::Done(Value::Void)));
}
