//! U12 Gooey host on an authored synthetic script (no game bytecode).
use lbp_level::script::{FieldDef, FunctionDef, MemberRef, Script, Span, TypeOffset, TypeRef};
use lbp_script_vm::{Value, Vm};
use lbp_ui_script::{entry_registers, functions, outline, run, GooeyHost, Node, Receiver};

fn i(op: u8, ty: u8, a: u16, b: u16, c: u16) -> u64 {
    u64::from(op)
        | u64::from(ty) << 8
        | u64::from(a) << 16
        | u64::from(b) << 32
        | u64::from(c) << 48
}
fn hi(op: u8, a: u16, value: u32) -> u64 {
    u64::from(op) | u64::from(a) << 16 | u64::from(value) << 32
}

const BOOL: u8 = 1;
const INT: u8 = 3;
const OBJ: u8 = 11;

/// class Menu { bool Draw() } calling Gooey and LAMS natives.
fn menu() -> Script {
    let strings: Vec<String> = [
        "Menu",
        "Gooey",
        "LAMS",
        "StartFrame__Q23Gooey.EGooeyBubbleStyle",
        "SetFrameSizing__ff",
        "Translate__i",
        "DoText__Q6StringQ21Gooey.EGooeyTextStyle",
        "DoSpaceButtonNamed__iQ6StringQ21Gooey.EGooeyTextStyleQ23Gooey.EGooeyButtonStateif",
        "EndFrame__",
        "Play",
        "Draw",
        "s32",
        "count",
    ]
    .map(String::from)
    .to_vec();
    let ty = |machine_type, type_name| TypeRef {
        machine_type,
        builtin_type: 0,
        dimension_count: 0,
        array_base_machine_type: 0,
        script: None,
        type_name,
    };
    let f = |type_ref, name| MemberRef { type_ref, name };
    let one = 1.0f32.to_bits();
    let bytecode = vec![
        i(0xb3, INT, 28, 0, 0),  // r28 = this.count (this at register 0)
        hi(0x04, 32, one),       // r32 = 1.0
        i(0x79, 0, 32, 36, 0),   // r36.x = r32 on a fresh register
        hi(0x03, 4, 0),          // r4 = 0
        i(0xa3, 0, 4, 0, 0),     // ARG @0 = r4
        i(0xa4, BOOL, 40, 0, 0), // r40 = StartFrame(0)
        hi(0x04, 8, one),        // r8 = 1.0
        i(0xa3, 0, 8, 0, 0),     // ARG @0 = r8
        i(0xa3, 0, 8, 4, 0),     // ARG @4 = r8
        i(0xa4, 0, 0, 1, 0),     // SetFrameSizing(1, 1)
        hi(0x03, 12, 0x1234),    // r12 = key
        i(0xa3, 0, 12, 0, 0),    // ARG @0 = r12
        i(0xa4, OBJ, 16, 2, 0),  // r16 = LAMS.Translate(key)
        i(0xa3, 0, 16, 0, 0),    // ARG @0 = r16
        i(0xa3, 0, 4, 4, 0),     // ARG @4 = r4
        i(0xa4, 0, 0, 3, 0),     // DoText(r16, 0)
        hi(0xbc, 20, 9),         // r20 = "Play"
        i(0xa3, 0, 4, 0, 0),     // ARG @0 = r4
        i(0xa3, 0, 20, 4, 0),    // ARG @4 = r20
        i(0xa4, BOOL, 24, 4, 0), // r24 = DoSpaceButtonNamed(0, "Play")
        i(0xa4, 0, 0, 5, 0),     // EndFrame()
        i(0xa4, 0, 0, 5, 0),     // EndFrame() with no open frame
        i(0xa5, BOOL, 0, 40, 0), // RETURN r40 (StartFrame result)
    ];
    let n = bytecode.len() as u32;
    Script {
        up_to_date_script: None,
        class_name: "Menu".into(),
        super_class: None,
        modifiers: 0,
        type_refs: vec![ty(OBJ, 1), ty(OBJ, 2), ty(OBJ, 0), ty(INT, 11)],
        field_refs: vec![f(2, 12)],
        function_refs: vec![f(0, 3), f(0, 4), f(1, 5), f(0, 6), f(0, 7), f(0, 8)],
        field_defs: vec![FieldDef {
            modifiers: 0,
            type_ref: 3,
            name: 12,
        }],
        property_defs: vec![],
        function_defs: vec![FunctionDef {
            modifiers: 0,
            type_ref: 2,
            name: 10,
            arguments: Span { begin: 0, end: 1 },
            bytecode: Span { begin: 0, end: n },
            line_numbers: Span { begin: 0, end: 0 },
            locals: Span { begin: 0, end: 0 },
            stack_size: 64,
        }],
        shared_arguments: vec![TypeOffset {
            type_ref: 2,
            offset: 32,
        }],
        bytecode,
        line_numbers: vec![],
        locals: vec![],
        strings_a: strings,
        strings_w: vec![],
        constants_s64: Some(vec![]),
        constants_f32: vec![],
        depending_guids: Some(vec![]),
    }
}

#[test]
fn u12_gooey_calls_build_a_frame_tree_and_trace() {
    let script = menu();
    let list = functions(&script);
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].name.as_str(), list[0].is_static), ("Draw", false));
    assert_eq!(list[0].arguments, 1);

    let mut vm = Vm::default();
    let class = vm.load(script.clone(), None);
    // The only argument row is at register 32, so the receiver goes to register 0 and the
    // Menu-typed argument gets a fresh Menu instance.
    let none = |_: u32| None;
    let mut h = GooeyHost::new(&none);
    let (mut budget, mut supplied) = (1_000u64, Vec::new());
    let e = entry_registers(
        &mut vm,
        &mut h,
        &script,
        "Draw",
        Value::Int(9),
        &mut budget,
        &mut supplied,
    )
    .unwrap();
    let (regs, receiver, offsets) = (e.registers, e.receiver, e.offsets);
    assert_eq!(receiver, Receiver::ImplicitRegister0);
    assert_eq!(offsets, vec![32]);
    assert_eq!(regs[0], (0, Value::Int(9)));
    assert!(matches!(regs[1], (32, Value::Object(Some(_)))));
    let translate = |k: u32| (k == 0x1234).then(|| "Start".to_owned());
    let mut host = GooeyHost::new(&translate);
    let r = run(&mut vm, &mut host, class, &script, "Draw", 10_000);

    // The stubbed button returns the default of its bool return type.
    assert_eq!(r.result, Ok("true".into()));
    assert_eq!(r.receiver, Some(Receiver::ImplicitRegister0));
    assert_eq!(r.counts.native_calls, 7);
    assert_eq!(r.counts.translated, 1);
    assert_eq!(r.counts.unbalanced_end, 1);
    assert_eq!(r.counts.unclosed, 0);
    assert_eq!(r.trace.len(), 7);
    assert_eq!(r.trace[2].class, "LAMS");

    let [Node::Frame(frame)] = r.tree.children.as_slice() else {
        panic!("one frame: {:?}", r.tree)
    };
    assert_eq!(frame.props.get("Sizing").map(String::as_str), Some("1, 1"));
    assert!(matches!(&frame.children[0], Node::Text { text, .. } if text == "Start"));
    assert!(matches!(&frame.children[1], Node::Button { label, .. } if label == "Play"));
    let lines = outline(&r.tree, 10);
    assert_eq!(lines.len(), 4, "{lines:?}");
    assert!(lines[3].contains("button \"Play\" (DoSpaceButtonNamed)"));
}

#[test]
fn u12b_frame_tree_maps_to_a_gooey_tree_and_screen_size_answers() {
    use lbp_gooey::{FrameInput, Uid, Widget};
    use lbp_script_vm::Host;
    use lbp_ui_script::tree::ui_tree;

    let script = menu();
    let mut vm = Vm::default();
    let class = vm.load(script.clone(), None);
    let none = |_: u32| None;
    let mut host = GooeyHost::new(&none);
    assert_eq!(
        host.call_native("PodComputer", "GetPuterWidth__", &[]),
        Ok(Value::Float(1280.))
    );
    assert_eq!(
        host.call_native("HUD", "GetXRes__", &[]),
        Ok(Value::Int(1280))
    );
    assert_eq!(
        host.call_native("HUD", "GetScreenEdgeCrop__", &[]),
        Ok(Value::Float(0.))
    );
    let r = run(&mut vm, &mut host, class, &script, "Draw", 10_000);
    let tree = ui_tree(&r.tree, Uid(1), &["header".into()], FrameInput::default());
    let labels: Vec<&str> = tree
        .nodes
        .iter()
        .filter_map(|n| match &n.widget {
            Widget::Button { label, .. } => Some(label.as_str()),
            Widget::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["header", "#00001234", "Play"]);
    assert_eq!(tree.focusable().len(), 1);
    assert!(tree.duplicate_ids.is_empty());
}

#[test]
fn u12c_null_script_class_fields_are_supplied() {
    use lbp_ui_script::make_object;
    let holder = Script {
        class_name: "Holder".into(),
        type_refs: vec![TypeRef {
            machine_type: OBJ,
            builtin_type: 0,
            dimension_count: 0,
            array_base_machine_type: 0,
            script: None,
            type_name: 2,
        }],
        field_defs: vec![FieldDef {
            modifiers: 0,
            type_ref: 0,
            name: 1,
        }],
        function_defs: vec![],
        shared_arguments: vec![],
        bytecode: vec![],
        strings_a: ["Holder", "Puter", "Menu"].map(String::from).to_vec(),
        ..menu()
    };
    let mut vm = Vm::default();
    vm.load(menu(), None);
    let class = vm.load(holder, None);
    let none = |_: u32| None;
    let mut host = GooeyHost::new(&none);
    let (mut budget, mut supplied) = (1_000u64, Vec::new());
    let id = make_object(&mut vm, &mut host, class, &mut budget, &mut supplied).unwrap();
    assert_eq!(supplied, ["Holder.Puter: Menu"]);
    assert!(matches!(
        vm.field(id, "Puter"),
        Some(Value::Object(Some(_)))
    ));
}

#[test]
fn u12d_sizing_is_relative_to_the_first_sized_frame() {
    use lbp_gooey::{FrameInput, Uid, Widget};
    use lbp_ui_script::tree::{reference_size, ui_tree};
    use lbp_ui_script::Frame;
    let sized = |w: &str, children: Vec<Node>| Frame {
        start: "StartFrame".into(),
        props: [("Sizing".to_owned(), w.to_owned())].into(),
        children,
    };
    let root = Frame {
        start: "root".into(),
        props: Default::default(),
        children: vec![Node::Frame(sized(
            "2400, 1250",
            vec![
                Node::Frame(sized("1200, 300", vec![])),
                Node::Frame(sized("-1, 500", vec![])),
                Node::Frame(sized("NaN, 0", vec![])),
            ],
        ))],
    };
    assert_eq!(reference_size(&root), Some([2400., 1250.]));
    let tree = ui_tree(&root, Uid(1), &[], FrameInput::default());
    let sizes: Vec<[Option<f32>; 2]> = tree
        .nodes
        .iter()
        .filter_map(|n| match n.widget {
            Widget::Frame { size, .. } => Some(size),
            _ => None,
        })
        .collect();
    assert_eq!(
        sizes,
        [
            [None, None],
            [None, None],
            [Some(1.), Some(1.)],
            [Some(0.5), Some(0.24)],
            [None, Some(0.4)],
            [None, None],
        ]
    );
}
