//! D507: the native `PostItNote` and an unlock note on authored synthetic data (no game
//! bytes, no script loaded): a note with a LAMS key and a costume unlock note.
use lbp_level::parts::{ScriptField, ScriptPart, ScriptValue, World as WorldPart};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{
    start_native, LevelScripts, LevelWorld, NativeStep, POST_IT_NOTE_SCRIPT,
    POST_IT_NOTE_UNLOCK_COSTUME_SCRIPT,
};
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

fn script(guid: u32, fields: Vec<ScriptField>) -> ScriptPart {
    ScriptPart {
        script: Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(guid),
            sha1: None,
        }),
        fields,
    }
}

/// World Thing 0, a note (Thing 1) and a costume unlock note (Thing 2).
fn note_level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..3)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    for (i, guid) in [
        (1, POST_IT_NOTE_SCRIPT),
        (2, POST_IT_NOTE_UNLOCK_COSTUME_SCRIPT),
    ] {
        t[i].parent = Some(0);
        t[i].parts = vec!["SCRIPT"];
        t[i].data.script = Some(script(
            guid,
            vec![field("BubbleVisible", 1, ScriptValue::Bool(true))],
        ));
    }
    t
}

fn s(v: &str) -> Value {
    Value::Str(v.into())
}

#[test]
fn d507_the_native_post_it_note_keeps_its_text_and_unlocks() {
    let mut vm = Vm::default();
    let mut world = LevelWorld::from_things(&note_level());
    world.bind_objects(&mut vm);
    world.add_class_defaults(&vm);
    assert_eq!(world.things[1].script_classes[0], "PostItNote");
    assert_eq!(world.things[2].script_classes[0], "PostItNoteUnlockCostume");
    let mut host = Bindings::defaulting(world);
    let mut scripts = LevelScripts::new(&vm, &host.world);
    let created = scripts.create(&mut vm, &mut host, BUDGET);
    assert!(created.faults.is_empty(), "{:?}", created.faults);

    let run = |host: &mut Bindings<LevelWorld>, thing: u32, function: &str, args: &[Value]| {
        let guid = [0, POST_IT_NOTE_SCRIPT, POST_IT_NOTE_UNLOCK_COSTUME_SCRIPT][thing as usize];
        let port = lbp_runtime::native_script(guid).unwrap();
        let args: Vec<(u16, Value)> = args.iter().map(|v| (4, v.clone())).collect();
        match start_native(port, host, thing, function, &args) {
            Some(Ok(NativeStep::Done(v))) => v,
            other => panic!("{function}: {other:?}"),
        }
    };
    run(&mut host, 1, "SetLAMSKey__Q6String", &[s("rookie_note")]);
    assert_eq!(run(&mut host, 1, "GetLAMSKey__", &[]), s("ROOKIE_NOTE"));
    assert_eq!(
        run(&mut host, 1, "GetBubbleVisible__", &[]),
        Value::Bool(true)
    );

    // A player's text: the filtered and original forms, then the old text kept by a
    // Webternate edit.
    run(
        &mut host,
        1,
        "SetSpeechText__Q6StringQ6String",
        &[s("original"), s("filtered")],
    );
    let text = |host: &mut Bindings<LevelWorld>, filter: bool| {
        run(host, 1, "GetTranslatedText__b", &[Value::Bool(filter)])
    };
    assert_eq!(text(&mut host, true), s("filtered"));
    assert_eq!(text(&mut host, false), s("original"));
    let speech = run(&mut host, 1, "GetSpeechText__", &[]);
    assert!(
        speech == s("filtered") || speech == s("original"),
        "{speech:?}"
    );
    assert_eq!(
        run(&mut host, 1, "GetUserEnteredText__", &[]),
        s("filtered")
    );
    run(&mut host, 1, "WebternateSetText__Q6String", &[s("web")]);
    assert_eq!(run(&mut host, 1, "WebternateGetText__", &[]), s("web"));
    assert_eq!(
        run(&mut host, 1, "GetUserEnteredText__", &[]),
        s("filtered")
    );

    // A LAMS key is upper-cased and clears the player's text.
    run(&mut host, 1, "SetLAMSKey__Q6String", &[s("intro_key")]);
    assert_eq!(run(&mut host, 1, "GetLAMSKey__", &[]), s("INTRO_KEY"));
    assert_eq!(run(&mut host, 1, "GetUserEnteredText__", &[]), s(""));

    run(&mut host, 1, "SetBubbleVisible__b", &[Value::Bool(false)]);
    assert_eq!(
        run(&mut host, 1, "GetBubbleVisible__", &[]),
        Value::Bool(false)
    );

    // The plain note has no reaction of its own; the costume note sets its flag only when on.
    let port = lbp_runtime::native_script(POST_IT_NOTE_SCRIPT).unwrap();
    let on = [(4, Value::Bool(true))];
    assert!(start_native(port, &mut host, 1, "SwitchPerformReaction__b", &on).is_none());
    run(
        &mut host,
        2,
        "SwitchPerformReaction__b",
        &[Value::Bool(false)],
    );
    assert_eq!(host.world.progression_flags, 0);
    run(
        &mut host,
        2,
        "SwitchPerformReaction__b",
        &[Value::Bool(true)],
    );
    assert_eq!(host.world.progression_flags, 128);
    assert_eq!(run(&mut host, 2, "GetLAMSKey__", &[]), Value::Object(None));
}
