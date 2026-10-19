//! D504: native script classes on authored synthetic data (no game bytes, no script
//! loaded): a level whose only script is the speech bubble, run with an empty VM.
use lbp_level::parts::{
    Pos, ScriptField, ScriptObject, ScriptPart, ScriptValue, World as WorldPart,
};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{LevelScripts, LevelWorld, Outcome, Runner};
use lbp_script_vm::natives::Bindings;
use lbp_script_vm::{Value, Vm};

const BUBBLE: u32 = 0x47f4;
const BUDGET: u64 = 100_000;

/// World Thing 0 and a speech bubble (Thing 1) whose text key is not in any table.
fn bubble_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..2)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    t[1].parent = Some(0);
    t[1].parts = vec!["POS", "SCRIPT"];
    let mut world = [0.; 16];
    world[0] = 1.;
    world[15] = 1.;
    t[1].data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t[1].data.script = Some(ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(BUBBLE),
            sha1: None,
        }),
        fields: vec![
            ScriptField {
                name: "Text".into(),
                machine_type: 0xb,
                value: ScriptValue::Object {
                    kind: 16,
                    id: Some(0),
                    body: Some(Box::new(ScriptObject::Text("DV_TEST_LINE".into()))),
                },
            },
            ScriptField {
                name: "Behaviour".into(),
                machine_type: 3,
                value: ScriptValue::Int(0),
            },
        ],
    });
    t
}

fn member(host: &Bindings<LevelWorld>, name: &str) -> Value {
    host.world.things[1].members.as_ref().unwrap()[name].clone()
}

#[test]
fn d504_the_native_bubble_opens_closes_and_pulses_without_the_script() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&bubble_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&mut vm);
    // The class's names and its `.init__` values for the fields the level does not store.
    assert_eq!(world.things[1].script_classes[0], "Trigger_Speech_Bubble");
    // D513: the port's defaults fill the members, so reads of them find a value.
    let members = world.things[1].members.as_ref().unwrap();
    assert_eq!(members["IsTweaking"], Value::Bool(false));
    assert_eq!(members["CurrVoiceIndex"], Value::Int(-1));
    assert_eq!(members["TestSpeech"], Value::Object(None));
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    let inst = scripts.instances[0];
    assert_eq!(inst.class, None);
    assert!(lbp_runtime::native_script(BUBBLE).is_some());
    // `OnCreate__` sets the bubble up and stays in its loop (yielded), as the script does.
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert_eq!((created.yielded, created.faults.len()), (1, 0));
    assert_eq!(member(&host, "VoiceType"), Value::Int(3));
    assert_eq!(member(&host, "HasUpdateWorkToDo"), Value::Bool(false));
    scripts.tick(&mut vm, &mut host, BUDGET);

    // The switch turns it on: the message opens at the bubble (behaviour 0: no prompt).
    let mut runner = Runner::default();
    let mut b = BUDGET;
    let on = [(4, Value::Bool(true)), (8, Value::Float(1.0))];
    let o = runner.start_with(
        &mut vm,
        &mut host,
        inst,
        "SwitchPerformReaction__bf",
        &on,
        &mut b,
    );
    assert_eq!(o, Outcome::Done);
    assert_eq!(host.world.messages.open.len(), 1);
    assert_eq!(host.world.messages.open[0].thing, Some(1));
    assert_eq!(member(&host, "IsActive"), Value::Bool(true));

    // Circle closes it and starts the output pulse, on for two ticks.
    let activation = |runner: &mut Runner, vm: &mut Vm, host: &mut Bindings<LevelWorld>| {
        let mut b = BUDGET;
        let arg = [(4, Value::Int(0))];
        runner
            .start_for_value(vm, host, inst, "SwitchGetNewActivation__i", &arg, &mut b)
            .1
    };
    assert_eq!(
        scripts.player_close_message(&mut vm, &mut host, BUDGET),
        Some(Outcome::Done)
    );
    assert!(host.world.messages.open.is_empty());
    assert_eq!(member(&host, "OutputEndSignalFrames"), Value::Int(2));
    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(activation(&mut runner, &mut vm, &mut host));
        scripts.tick(&mut vm, &mut host, BUDGET);
    }
    let f = |x| Some(Value::Float(x));
    assert_eq!(seen, [f(1.0), f(1.0), f(0.0)]);
}
