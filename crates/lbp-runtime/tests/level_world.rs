//! Rt on authored synthetic data (no game bytes): a three-Thing level and a one-function
//! script class. docs/level-scripts.md.
use lbp_level::parts::{Pos, ScriptField, ScriptPart, ScriptValue, World as WorldPart};
use lbp_level::script::{FunctionDef, MemberRef, Script, Span, TypeRef};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{instances, LevelWorld, Outcome, Runner};
use lbp_script_vm::natives::{Bindings, World};
use lbp_script_vm::{MemberKind, Value, Vm};

const GUID: u32 = 0x2d06;

fn i(op: u8, ty: u8, a: u16, b: u16, c: u16) -> u64 {
    u64::from(op)
        | u64::from(ty) << 8
        | u64::from(a) << 16
        | u64::from(b) << 32
        | u64::from(c) << 48
}

fn desc(guid: u32) -> Descriptor {
    Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(guid),
        sha1: None,
    }
}

/// `Lamp.Tick`: `count += 1; Yield(); return count;` on the Thing in register 0.
fn lamp() -> Script {
    let ty = |machine_type, type_name| TypeRef {
        machine_type,
        builtin_type: 0,
        dimension_count: 0,
        array_base_machine_type: 0,
        script: None,
        type_name,
    };
    let code = vec![
        i(0x95, 3, 4, 0, 0),                   // r4 = thing(r0).count
        u64::from(0x03u8) | 8 << 16 | 1 << 32, // r8 = 1
        i(0x2a, 3, 4, 4, 8),                   // r4 = r4 + r8 (ADDi)
        i(0x97, 3, 4, 0, 0),                   // thing(r0).count = r4
        i(0xa4, 0, 0, 0, 0),                   // CALL System.Yield
        i(0xa5, 3, 0, 4, 0),                   // RETURN r4
    ];
    Script {
        up_to_date_script: None,
        class_name: "Lamp".into(),
        super_class: None,
        modifiers: 0,
        type_refs: vec![ty(10, 0), ty(11, 1)],
        field_refs: vec![MemberRef {
            type_ref: 0,
            name: 2,
        }],
        function_refs: vec![MemberRef {
            type_ref: 1,
            name: 3,
        }],
        field_defs: vec![],
        property_defs: vec![],
        function_defs: vec![FunctionDef {
            modifiers: 0,
            type_ref: 0,
            name: 4,
            arguments: Span { begin: 0, end: 0 },
            bytecode: Span {
                begin: 0,
                end: code.len() as u32,
            },
            line_numbers: Span { begin: 0, end: 0 },
            locals: Span { begin: 0, end: 0 },
            stack_size: 16,
        }],
        shared_arguments: vec![],
        bytecode: code,
        line_numbers: vec![],
        locals: vec![],
        strings_a: ["Lamp", "System", "count", "Yield__", "Tick"]
            .map(String::from)
            .to_vec(),
        strings_w: vec![],
        constants_s64: Some(vec![]),
        constants_f32: vec![],
        depending_guids: Some(vec![]),
    }
}

/// World Thing 0; Thing 1 (scripted, UID 7, at (1, 2, 3)) and Thing 2 under it.
fn level() -> Vec<ThingRecord> {
    let mut t: Vec<ThingRecord> = (0..3)
        .map(|index| ThingRecord {
            index,
            ..Default::default()
        })
        .collect();
    t[0].data.world = Some(WorldPart::default());
    t[1].parent = Some(0);
    t[2].parent = Some(0);
    t[1].uid = Some(7);
    t[1].parts = vec!["POS", "SCRIPT"];
    let mut world = [0.; 16];
    world[..4].copy_from_slice(&[1., 0., 0., 0.]);
    world[12..].copy_from_slice(&[1., 2., 3., 1.]);
    t[1].data.pos = Some(Pos {
        bone_of: None,
        anim_hash: 0,
        world,
    });
    t[1].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![
            ScriptField {
                name: "count".into(),
                machine_type: 3,
                value: ScriptValue::Int(41),
            },
            ScriptField {
                name: "label".into(),
                machine_type: 0xb,
                value: ScriptValue::Object {
                    kind: 16,
                    text: Some("hi".into()),
                },
            },
            ScriptField {
                name: "cache".into(),
                machine_type: 3,
                value: ScriptValue::NotStored,
            },
        ],
    });
    t
}

#[test]
fn thing_tree_positions_parts_and_members() {
    let mut w = LevelWorld::from_things(&level());
    assert_eq!(w.world_thing(), Ok(Some(0)));
    assert_eq!(w.first_child(0), Ok(Some(1)));
    assert_eq!(w.next_sibling(1), Ok(Some(2)));
    assert_eq!(w.next_sibling(2), Ok(None));
    assert_eq!(w.parent(2), Ok(Some(0)));
    assert_eq!(w.thing_uid(1), Ok(7));
    assert!(w.thing_uid(2).is_err());
    assert_eq!(w.world_position(1), Ok([1., 2., 3., 1.]));
    assert_eq!(w.has_part(1, "PScript"), Ok(true));
    assert_eq!(w.has_part(1, "PSwitch"), Ok(false));
    let me = Value::Thing(Some(1));
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::Thing, &me, "Lamp", f);
    assert_eq!(get(&mut w, "count"), Ok(Value::Int(41)));
    assert_eq!(get(&mut w, "label"), Ok(Value::Str("hi".into())));
    assert!(get(&mut w, "cache").is_err());
    assert!(w
        .get_member(MemberKind::Native, &me, "Lamp", "count")
        .is_err());
    assert_eq!(w.unconverted, 0);
}

#[test]
fn runner_starts_a_thing_script_and_resumes_it_next_tick() {
    let mut vm = Vm::default();
    vm.load(lamp(), Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    let found = instances(&vm, &host.world);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].thing, found[0].guid), (1, GUID));
    assert!(found[0].class.is_some());
    let mut runner = Runner::default();
    let mut budget = 1000;
    assert_eq!(
        runner.start(&mut vm, &mut host, found[0], "Missing", &mut budget),
        Outcome::NoFunction
    );
    assert_eq!(
        runner.start(&mut vm, &mut host, found[0], "Tick", &mut budget),
        Outcome::Yielded
    );
    let me = Value::Thing(Some(1));
    assert_eq!(
        host.world
            .get_member(MemberKind::Thing, &me, "Lamp", "count"),
        Ok(Value::Int(42))
    );
    assert_eq!(runner.pending(), 1);
    assert_eq!(
        runner.tick(&mut vm, &mut host, &mut budget),
        [(1, Outcome::Done)]
    );
    assert_eq!(runner.pending(), 0);
    let unloaded = lbp_runtime::Instance {
        class: None,
        ..found[0]
    };
    assert_eq!(
        runner.start(&mut vm, &mut host, unloaded, "Tick", &mut budget),
        Outcome::NoClass
    );
}
