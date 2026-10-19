//! D505: the native `Launcher` on authored synthetic data (no game bytes, no script
//! loaded): a launch pad whose light Thing has a sprite light, run with an empty VM.
use lbp_level::parts::{ScriptField, ScriptPart, ScriptValue, World as WorldPart};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{start_native, LevelScripts, LevelWorld, NativeStep, LAUNCHER_SCRIPT};
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

/// World Thing 0, a launcher (Thing 1) with distance 3 and Thing 2 as its light.
fn launcher_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..3)
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
            guid: Some(LAUNCHER_SCRIPT),
            sha1: None,
        }),
        fields: vec![
            field("Distance", 4, ScriptValue::Float(3.)),
            field("LightThing", 0xa, ScriptValue::Thing(Some(2))),
        ],
    });
    t[2].parent = Some(0);
    t[2].parts = vec!["SPRITE_LIGHT"];
    t
}

fn light(host: &Bindings<LevelWorld>, member: &str) -> Option<Value> {
    let p = host.world.part_handles.iter().find(|p| p.thing == 2)?;
    assert_eq!(p.part, "SPRITE_LIGHT");
    p.members.get(member).cloned()
}

#[test]
fn d505_the_native_launcher_sets_its_light() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&launcher_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&mut vm);
    assert_eq!(world.things[1].script_classes[0], "Launcher");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    assert_eq!(scripts.instances[0].class, None);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);
    assert_eq!(light(&host, "FarDist"), Some(Value::Float(7.5)));
    assert_eq!(light(&host, "Color"), Some(Value::Vec4([0.7, 0.7, 1., 1.])));

    let port = lbp_runtime::native_script(LAUNCHER_SCRIPT).unwrap();
    let mut run = |function: &str, arg: Option<Value>| {
        let args: Vec<(u16, Value)> = arg.into_iter().map(|v| (4, v)).collect();
        match start_native(port, &mut host, 1, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    run("SetDistance__f", Some(Value::Float(2.)));
    assert_eq!(run("GetDistance__", None), Value::Float(2.));
    // A filter outside 0 to 4 is ignored.
    run("SetTeamFilter__i", Some(Value::Int(3)));
    run("SetTeamFilter__i", Some(Value::Int(9)));
    assert_eq!(run("GetTeamFilter__", None), Value::Int(3));
    run("SwitchPerformReaction__b", Some(Value::Bool(true)));
    assert_eq!(light(&host, "FarDist"), Some(Value::Float(5.)));
}
