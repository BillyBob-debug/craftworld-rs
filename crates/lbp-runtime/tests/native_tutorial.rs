//! D508: the native `TutorialMonitor` on authored synthetic data (no game bytes, no script
//! loaded): one monitor wired to an on switch and one unwired.
use lbp_level::parts::{
    ScriptField, ScriptPart, ScriptValue, Switch, SwitchOutput, SwitchSignal, World as WorldPart,
};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{start_native, LevelScripts, LevelWorld, NativeStep, TUTORIAL_MONITOR_SCRIPT};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};

const BUDGET: u64 = 100_000;
const PLAY: &str = "Game.PlayTutorialMonitorVideo__gQ5Thing";

fn monitor(t: &mut ThingRecord, video: i32) {
    t.parent = Some(0);
    t.parts = vec!["SCRIPT"];
    t.data.script = Some(ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(TUTORIAL_MONITOR_SCRIPT),
            sha1: None,
        }),
        fields: vec![ScriptField {
            name: "VideoFile".into(),
            machine_type: 3,
            value: ScriptValue::Int(video),
        }],
    });
}

/// World Thing 0; monitors 1 (video 111, input 0 wired to the on switch 2) and 3 (video
/// 333, unwired).
fn monitor_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..4)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    monitor(&mut t[1], 111);
    monitor(&mut t[3], 333);
    t[2].data.switch = Some(Switch {
        outputs: vec![Some(SwitchOutput {
            signal: SwitchSignal {
                activation: 1.,
                ternary: 1,
                player: 0,
            },
            targets: vec![(Some(1), 0)],
        })],
        ..Default::default()
    });
    t
}

#[test]
fn d508_the_native_tutorial_monitor_plays_its_video() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&monitor_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&mut vm);
    assert_eq!(world.things[1].script_classes[0], "TutorialMonitor");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);

    let port = lbp_runtime::native_script(TUTORIAL_MONITOR_SCRIPT).unwrap();
    let run = |host: &mut Bindings<LevelWorld>, thing: u32, function: &str, args: &[Value]| {
        let args: Vec<(u16, Value)> = args.iter().map(|v| (4, v.clone())).collect();
        match start_native(port, host, thing, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    let plays = |host: &Bindings<LevelWorld>| host.calls.get(PLAY).copied().unwrap_or(0);

    assert_eq!(run(&mut host, 1, "GetVideoHandle__", &[]), Value::Int(-1));
    assert_eq!(run(&mut host, 1, "GetVideoGUID__", &[]), Value::Int(111));
    assert_eq!(
        run(&mut host, 1, "GetRTTRes__", &[]),
        Value::Vec4([640., 360., 0., 0.])
    );
    run(&mut host, 3, "SetVideoGUID__g", &[Value::Int(42)]);
    assert_eq!(run(&mut host, 3, "GetVideoGUID__", &[]), Value::Int(42));

    // The unwired monitor ignores a switch reaction, even one that is on.
    run(
        &mut host,
        3,
        "SwitchPerformReaction__b",
        &[Value::Bool(true)],
    );
    assert_eq!(plays(&host), 0);
    // The wired monitor's input is on, so the reaction plays its video.
    run(
        &mut host,
        1,
        "SwitchPerformReaction__b",
        &[Value::Bool(false)],
    );
    assert_eq!(plays(&host), 1);
    run(&mut host, 3, "PlayVideo__", &[]);
    assert_eq!(plays(&host), 2);
    run(
        &mut host,
        1,
        "RenderToTexture__ii",
        &[Value::Int(0), Value::Int(0)],
    );
    assert!(start_native(port, &mut host, 1, "Render__", &[]).is_none());
}
