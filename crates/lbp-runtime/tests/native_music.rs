//! D506: the native `TriggerMusic` on authored synthetic data (no game bytes, no script
//! loaded): a music trigger with a bank, a settings file and one quiet pair.
use lbp_level::parts::{ScriptField, ScriptPart, ScriptValue, World as WorldPart};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{start_native, LevelScripts, LevelWorld, NativeStep, TRIGGER_MUSIC_SCRIPT};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};

const BUDGET: u64 = 100_000;

fn field(name: &str, machine_type: u8, value: ScriptValue) -> ScriptField {
    ScriptField {
        name: name.into(),
        machine_type,
        value,
    }
}

/// World Thing 0 and a music trigger (Thing 1): bank 1234, settings 5678, start point 2,
/// pair 2 at half volume.
fn music_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..2)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    t[1].parent = Some(0);
    t[1].parts = vec!["SCRIPT"];
    t[1].data.script = Some(ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(TRIGGER_MUSIC_SCRIPT),
            sha1: None,
        }),
        fields: vec![
            field("FSB", 3, ScriptValue::Int(1234)),
            field("SettingsFile", 3, ScriptValue::Int(5678)),
            field("StartPoint", 4, ScriptValue::Float(2.)),
            field("Pair_2_Volume", 4, ScriptValue::Float(0.5)),
        ],
    });
    t
}

#[test]
fn d506_the_native_trigger_music_plays_and_mutes() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&music_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&mut vm);
    assert_eq!(world.things[1].script_classes[0], "TriggerMusic");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    assert_eq!(scripts.instances[0].class, None);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);
    assert_eq!(host.world.media.music, None);

    let port = lbp_runtime::native_script(TRIGGER_MUSIC_SCRIPT).unwrap();
    let run = |host: &mut Bindings<LevelWorld>, function: &str, args: &[Value]| {
        let args: Vec<(u16, Value)> = args.iter().map(|v| (4, v.clone())).collect();
        match start_native(port, host, 1, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    // A player entering the unwired trigger starts the music with the stored pairs.
    run(&mut host, "Enter__Q5Thing", &[Value::Thing(Some(2))]);
    assert_eq!(host.world.media.music_starts, 1);
    let music = host.world.media.music.clone().expect("no music set");
    assert_eq!(
        (music.guid, music.settings, music.volume, music.delay),
        (1234, 5678, 1., 2.)
    );
    assert_eq!(music.pair_volumes, [1., 1., 0.5, 1., 1., 1.]);

    run(&mut host, "Mute__", &[]);
    let music = host.world.media.music.clone().unwrap();
    assert_eq!(music.pair_volumes, [0.; 6]);

    // A switch turning on plays again.
    run(&mut host, "SwitchPerformReaction__b", &[Value::Bool(true)]);
    assert_eq!(host.world.media.music_starts, 2);

    run(&mut host, "SetStartPoint__f", &[Value::Float(4.)]);
    assert_eq!(run(&mut host, "GetStartPoint__", &[]), Value::Float(4.));
    assert_eq!(run(&mut host, "GetFileGUID__", &[]), Value::Int(1234));
    run(
        &mut host,
        "SetFileGUIDs__gg",
        &[Value::Int(42), Value::Int(43)],
    );
    assert_eq!(run(&mut host, "GetSettingsFileGUID__", &[]), Value::Int(43));
    assert_eq!(
        run(&mut host, "GetHideInPlayModeOld__", &[]),
        Value::Bool(false)
    );
}
