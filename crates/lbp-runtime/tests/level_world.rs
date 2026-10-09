//! Rt on authored synthetic data (no game bytes): a three-Thing level and a small script
//! class with a superclass. docs/level-scripts.md.
use lbp_level::parts::{
    Pos, ScriptField, ScriptObject, ScriptPart, ScriptValue, World as WorldPart,
};
use lbp_level::script::{FieldDef, FunctionDef, MemberRef, Script, Span, TypeRef};
use lbp_level::serial::Descriptor;
use lbp_level::thing::ThingRecord;
use lbp_runtime::{instances, LevelScripts, LevelWorld, Outcome, Runner};
use lbp_script_vm::natives::{Bindings, World};
use lbp_script_vm::{MemberKind, Value, Vm};

const GUID: u32 = 0x2d06;
const BASE: u32 = 0x2d07;

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

fn ty(machine_type: u8, type_name: u32) -> TypeRef {
    TypeRef {
        machine_type,
        builtin_type: 0,
        dimension_count: 0,
        array_base_machine_type: 0,
        script: None,
        type_name,
    }
}

fn function(modifiers: u32, name: u32, begin: u32, end: u32) -> FunctionDef {
    FunctionDef {
        modifiers,
        type_ref: 0,
        name,
        arguments: Span { begin: 0, end: 0 },
        bytecode: Span { begin, end },
        line_numbers: Span { begin: 0, end: 0 },
        locals: Span { begin: 0, end: 0 },
        stack_size: 16,
    }
}

/// `Base`: fields `tweaking` (bool, never stored) and `count`, and a native `Mode__`.
fn base() -> Script {
    let field = |type_ref, name| FieldDef {
        modifiers: 0,
        type_ref,
        name,
    };
    Script {
        class_name: "Base".into(),
        super_class: None,
        type_refs: vec![ty(1, 0), ty(3, 0)],
        field_refs: vec![],
        function_refs: vec![],
        field_defs: vec![field(0, 1), field(1, 2)],
        function_defs: vec![function(2, 3, 0, 0)],
        bytecode: vec![],
        strings_a: ["Base", "tweaking", "count", "Mode__"]
            .map(String::from)
            .to_vec(),
        ..lamp()
    }
}

/// `Lamp : Base`. `Tick`: `count += 1; Yield(); return count;` on the Thing in register
/// 0. `Ask`: calls the inherited native `Mode__` through Lamp's own function reference.
fn lamp() -> Script {
    let code = vec![
        i(0x95, 3, 4, 0, 0),                   // r4 = thing(r0).count
        u64::from(0x03u8) | 8 << 16 | 1 << 32, // r8 = 1
        i(0x2a, 3, 4, 4, 8),                   // r4 = r4 + r8 (ADDi)
        i(0x97, 3, 4, 0, 0),                   // thing(r0).count = r4
        i(0xa4, 0, 0, 0, 0),                   // CALL System.Yield
        i(0xa5, 3, 0, 4, 0),                   // RETURN r4
        i(0xa4, 0, 0, 1, 0),                   // CALL Lamp.Mode__ (declared on Base)
        i(0xa5, 0, 0, 0, 0),                   // RETURN
    ];
    Script {
        up_to_date_script: None,
        class_name: "Lamp".into(),
        super_class: Some(desc(BASE)),
        modifiers: 0,
        type_refs: vec![ty(10, 0), ty(11, 1)],
        field_refs: vec![MemberRef {
            type_ref: 0,
            name: 2,
        }],
        function_refs: vec![
            MemberRef {
                type_ref: 1,
                name: 3,
            },
            MemberRef {
                type_ref: 0,
                name: 5,
            },
        ],
        field_defs: vec![],
        property_defs: vec![],
        function_defs: vec![function(0, 4, 0, 6), function(0, 6, 6, 8)],
        shared_arguments: vec![],
        bytecode: code,
        line_numbers: vec![],
        locals: vec![],
        strings_a: [
            "Lamp", "System", "count", "Yield__", "Tick", "Mode__", "Ask",
        ]
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
                    id: Some(0),
                    body: Some(Box::new(ScriptObject::Text("hi".into()))),
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

#[test]
fn class_defaults_inherited_natives_and_parts() {
    let mut vm = Vm::default();
    vm.load(base(), Some(BASE));
    vm.load(lamp(), Some(GUID));
    let mut things = level();
    things[1].parts.push("SPRITE_LIGHT");
    let mut world = LevelWorld::from_things(&things);
    let me = Value::Thing(Some(1));
    assert!(world
        .get_member(MemberKind::Thing, &me, "Lamp", "tweaking")
        .is_err());
    world.add_class_defaults(&vm);
    // A superclass field the instance never stores exists at its default; stored wins.
    let get = |w: &mut LevelWorld, f| w.get_member(MemberKind::Thing, &me, "Lamp", f);
    assert_eq!(get(&mut world, "tweaking"), Ok(Value::Bool(false)));
    assert_eq!(get(&mut world, "count"), Ok(Value::Int(41)));
    // `thing.SpriteLight`: a part handle whose ref_ptr members keep what scripts write;
    // null for a part the Thing lacks; an error for a name that is not a part.
    let light = world
        .get_member(MemberKind::Native, &me, "Thing", "SpriteLight")
        .unwrap();
    assert_eq!(light, Value::Raw(1));
    world
        .set_member(
            MemberKind::RefPtr,
            &light,
            "PSpriteLight",
            "FarDist",
            Value::Float(2.5),
        )
        .unwrap();
    assert_eq!(
        world.get_member(MemberKind::RefPtr, &light, "PSpriteLight", "FarDist"),
        Ok(Value::Float(2.5))
    );
    let other = Value::Thing(Some(2));
    assert_eq!(
        world.get_member(MemberKind::Native, &other, "Thing", "SpriteLight"),
        Ok(Value::Raw(0))
    );
    assert!(world
        .get_member(MemberKind::Native, &me, "Thing", "Colour")
        .is_err());
    assert_eq!(world.num_players(), Ok(1));
    assert_eq!(world.play_or_edit_mode(), Ok(0));
    // A native inherited through Lamp's reference reaches the host as Base.Mode__.
    let mut host = Bindings::new(world);
    let inst = instances(&vm, &host.world)[0];
    let mut budget = 1000;
    let o = Runner::default().start(&mut vm, &mut host, inst, "Ask", &mut budget);
    assert!(matches!(o, Outcome::Fault(_)));
    assert_eq!(host.unbound.keys().collect::<Vec<_>>(), ["Base.Mode__"]);
}

#[test]
fn part4_anim_speed_tint_camera_tweak_and_player_numbers() {
    let mut things = level();
    things[1].parts.push("CAMERA_TWEAK");
    let mut world = LevelWorld::from_things(&things);
    assert_eq!(world.has_part(1, "PCameraTweak"), Ok(true));
    assert_eq!(world.has_part(2, "PCameraTweak"), Ok(false));
    world.set_anim_speed(1, 0.).unwrap();
    world.set_render_mesh_tint(2, [1., 0.5, 0., 1.]).unwrap();
    assert_eq!(world.things[1].anim_speed, Some(0.));
    assert_eq!(world.things[2].render_mesh_tint, Some([1., 0.5, 0., 1.]));
    assert!(world.set_anim_speed(99, 1.).is_err());
    assert_eq!(world.player_number(0), Ok(0));
    assert!(world.player_number(1).is_err());
    assert!(world.player_number(-1).is_err());
}

#[test]
fn schedule_creates_then_updates_and_resumes_suspended_scripts() {
    // Lamp with `Tick` renamed to `Update__` and no `OnCreate__`.
    let mut ticker = lamp();
    ticker.strings_a[4] = lbp_runtime::UPDATE.into();
    let mut vm = Vm::default();
    vm.load(ticker, Some(GUID));
    let mut host = Bindings::new(LevelWorld::from_things(&level()));
    let mut level = LevelScripts::new(&vm, &host.world);
    let count = |host: &mut Bindings<LevelWorld>| {
        host.world
            .get_member(MemberKind::Thing, &Value::Thing(Some(1)), "Lamp", "count")
    };
    let created = level.create(&mut vm, &mut host, 1000);
    assert_eq!(created.no_function, 1);
    // Tick 1: Update runs and yields.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((t.yielded, t.resumed), (1, 0));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    // Tick 2: the suspended Update finishes; no new Update for that Thing this tick.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!((t.yielded, t.resumed, t.done), (0, 1, 0));
    assert_eq!(count(&mut host), Ok(Value::Int(42)));
    // Tick 3: Update runs again.
    let t = level.tick(&mut vm, &mut host, 1000);
    assert_eq!(t.yielded, 1);
    assert_eq!(count(&mut host), Ok(Value::Int(43)));
    assert_eq!((level.ticks, level.pending()), (3, 1));
    assert!(level.stopped.is_empty());
}

fn object(kind: u32, id: usize, body: Option<ScriptObject>) -> ScriptValue {
    ScriptValue::Object {
        kind,
        id: Some(id),
        body: body.map(Box::new),
    }
}

fn field(name: &str, value: ScriptValue) -> ScriptField {
    ScriptField {
        name: name.into(),
        machine_type: 0xb,
        value,
    }
}

#[test]
fn stored_objects_become_shared_heap_objects() {
    let mut vm = Vm::default();
    vm.load(base(), Some(BASE));
    let mut things = level();
    // An object array holding a back-reference to the `label` string, an instance of
    // `Base` and a second reference to that instance; a Thing array; a resource; and on
    // Thing 2 a reference to the same object array.
    let instance = ScriptPart {
        script: Some(desc(BASE)),
        fields: vec![ScriptField {
            name: "count".into(),
            machine_type: 3,
            value: ScriptValue::Int(5),
        }],
    };
    let items = vec![
        object(16, 0, None),
        object(13, 2, Some(ScriptObject::Instance(instance))),
        object(13, 2, None),
        ScriptValue::Object {
            kind: 0,
            id: None,
            body: None,
        },
    ];
    let part = things[1].data.script.as_mut().unwrap();
    part.fields.push(field(
        "items",
        object(11, 1, Some(ScriptObject::Objects(items))),
    ));
    part.fields.push(field(
        "targets",
        object(10, 3, Some(ScriptObject::Things(vec![Some(2), None]))),
    ));
    part.fields.push(field(
        "mesh",
        object(
            12,
            4,
            Some(ScriptObject::Resource {
                kind: 2,
                resource: None,
            }),
        ),
    ));
    things[2].data.script = Some(ScriptPart {
        script: Some(desc(GUID)),
        fields: vec![field("same", object(11, 1, None))],
    });
    let mut world = LevelWorld::from_things(&things);
    assert_eq!(world.unconverted, 4);
    assert_eq!(world.bind_objects(&mut vm), 3);
    assert_eq!(world.unconverted, 1, "the resource stays null");
    let get = |w: &mut LevelWorld, t: u32, f| {
        w.get_member(MemberKind::Thing, &Value::Thing(Some(t)), "Lamp", f)
    };
    let Ok(Value::Object(Some(items))) = get(&mut world, 1, "items") else {
        panic!("items is not an object");
    };
    assert_eq!(get(&mut world, 2, "same"), Ok(Value::Object(Some(items))));
    let a = vm.array(items).unwrap().to_vec();
    assert_eq!(a[0], Value::Str("hi".into()));
    let Value::Object(Some(inst)) = a[1] else {
        panic!("instance expected");
    };
    assert_eq!(
        (&a[2], &a[3]),
        (&Value::Object(Some(inst)), &Value::Object(None))
    );
    assert_eq!(vm.field(inst, "count"), Some(&Value::Int(5)));
    assert_eq!(vm.field(inst, "tweaking"), Some(&Value::Bool(false)));
    let Ok(Value::Object(Some(targets))) = get(&mut world, 1, "targets") else {
        panic!("targets is not an object");
    };
    assert_eq!(
        vm.array(targets),
        Some(&[Value::Thing(Some(2)), Value::Thing(None)][..])
    );
    assert_eq!(get(&mut world, 1, "mesh"), Ok(Value::Object(None)));
}
