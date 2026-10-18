//! D417: the native `Pod` and `Trigger_Pod` ports against the VM's golden trace.
//!
//! `fixtures/d417_pod_vm_trace.rs` holds the VM run of pod.bin's scripts with the
//! decrypted overlay (`level_scripts ... gamedata/levels/pod/pod.bin --play=30 --flags=2
//! --enter-pad-zone=3 --press=6:0x80 --player-colour=ff3366cc --trace=...@0-1,3-8,25-27`):
//! per script entry, every host call the script's own code made (natives, member reads
//! and writes, class tests) with the host's answer. Names and numbers only.
//!
//! Each entry is replayed: the port is run against a host that answers each call with
//! the recorded answer, and must make the same calls with the same arguments in the
//! same order and end the same way.
use lbp_runtime::{native_script_by_class, start_native, NativeStep};
use lbp_script_vm::trace::{decode_value, member_name};
use lbp_script_vm::{Host, MemberKind, Value};
use std::collections::VecDeque;

#[path = "fixtures/d417_pod_vm_trace.rs"]
mod fixture;
use fixture::TRACE as FIXTURE;

/// The Things of pod.bin with a PScript part: 0 is the world (`Pod`), 1 the pod
/// computer's trigger (`Trigger_Pod`).
fn class_of(thing: u32) -> &'static str {
    match thing {
        0 => "Pod",
        1 => "Trigger_Pod",
        other => panic!("thing {other} has no ported script in pod.bin"),
    }
}

#[derive(Debug, Clone)]
struct Event {
    tick: u64,
    thing: u32,
    kind: String,
    name: String,
    args: Vec<Value>,
    result: Result<Value, String>,
}

fn events() -> Vec<Event> {
    let json: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    let value = |s: &serde_json::Value| decode_value(s.as_str().expect("value string"));
    json["events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|e| {
            let result = e["result"].as_str().expect("result");
            Event {
                tick: e["tick"].as_u64().expect("tick"),
                thing: e["thing"].as_u64().expect("thing") as u32,
                kind: e["kind"].as_str().expect("kind").to_owned(),
                name: e["name"].as_str().expect("name").to_owned(),
                args: e["args"]
                    .as_array()
                    .expect("args")
                    .iter()
                    .map(|a| value(a).expect("decodable arg"))
                    .collect(),
                result: match result.strip_prefix("err:") {
                    Some(t) => Err(t.to_owned()),
                    None => Ok(decode_value(result).expect("decodable result")),
                },
            }
        })
        .collect()
}

/// Answers every host call from the recorded events, checking each against them.
struct Replay {
    expected: VecDeque<Event>,
    at: String,
}

impl Replay {
    fn next(&mut self, kind: &str, name: &str, args: &[Value]) -> Result<Value, String> {
        let e = self
            .expected
            .pop_front()
            .unwrap_or_else(|| panic!("{}: extra {kind} {name} {args:?}", self.at));
        assert_eq!(
            (e.kind.as_str(), e.name.as_str(), e.args.as_slice()),
            (kind, name, args),
            "{}: call differs from the VM's",
            self.at
        );
        e.result
    }
}

impl Host for Replay {
    fn call_native(
        &mut self,
        class: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        self.next("native", &format!("{class}.{function}"), args)
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        self.next(
            "get",
            &member_name(kind, class, field),
            std::slice::from_ref(base),
        )
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        self.next(
            "set",
            &member_name(kind, class, field),
            &[base.clone(), value],
        )
        .map(|_| ())
    }
    fn thing_is(&mut self, thing: u32, class: &str) -> bool {
        self.next("is", class, &[Value::Thing(Some(thing))]) == Ok(Value::Bool(true))
    }
}

#[test]
fn native_pod_matches_the_vm_trace() {
    let all = events();
    let mut i = 0;
    let (mut entries, mut host_calls) = (0, 0);
    let mut kinds = std::collections::BTreeSet::new();
    while i < all.len() {
        let call = &all[i];
        assert_eq!(call.kind, "call", "event {i} starts no entry");
        let end = i + all[i..]
            .iter()
            .position(|e| e.kind == "end")
            .expect("entry end");
        let body: VecDeque<Event> = all[i + 1..end].iter().cloned().collect();
        host_calls += body.len();
        kinds.extend(body.iter().map(|e| e.name.clone()));
        let at = format!("tick {} thing {} {}", call.tick, call.thing, call.name);
        let script = native_script_by_class(class_of(call.thing)).expect("ported class");
        let mut host = Replay {
            expected: body,
            at: at.clone(),
        };
        let step = if call.name == "resume" {
            script.update(&mut host, call.thing)
        } else {
            let args: Vec<(u16, Value)> =
                (4u16..).step_by(4).zip(call.args.iter().cloned()).collect();
            start_native(script, &mut host, call.thing, &call.name, &args)
        };
        let step = step.unwrap_or_else(|| panic!("{at}: no native entry"));
        let step = step.unwrap_or_else(|e| panic!("{at}: fault {e}"));
        assert!(
            host.expected.is_empty(),
            "{at}: calls missing: {:?}",
            host.expected
        );
        let ended = match step {
            NativeStep::Done(_) => "done",
            NativeStep::Yielded => "yielded",
        };
        assert_eq!(ended, all[end].name, "{at}: ends unlike the VM");
        entries += 1;
        i = end + 1;
    }
    // The fixture reaches every branch: create, the idle update, the trigger entry, the
    // engage request, and the mesh swap at the end of the change-over.
    assert!(entries >= 10, "{entries} entries");
    assert!(host_calls > 300, "{host_calls} host calls");
    for name in [
        "PodComputer.RequestHostUsePod__b",
        "Input.IsJustClickedByID__iiQ6String",
        "Thing.GetWorld__",
        "RefPtr:PYellowHead.PlayerNumber",
    ] {
        assert!(kinds.contains(name), "{name} not in the fixture");
    }
    assert!(all
        .iter()
        .any(|e| e.name == "FileList.AsGUID__i" && e.args == [Value::Int(145_753)]));
}

#[test]
fn registry_names_the_pod_scripts() {
    use lbp_runtime::{native_script, POD_SCRIPT, TRIGGER_POD_SCRIPT};
    assert_eq!(native_script(POD_SCRIPT).map(|s| s.class()), Some("Pod"));
    let trigger = native_script(TRIGGER_POD_SCRIPT).expect("Trigger_Pod");
    assert_eq!(trigger.class(), lbp_runtime::POD_TRIGGER_CLASS);
    let mut host = Replay {
        expected: VecDeque::new(),
        at: "IsPodComputerTrigger__".into(),
    };
    let answer = start_native(trigger, &mut host, 1, "IsPodComputerTrigger__", &[]);
    assert_eq!(answer, Some(Ok(NativeStep::Done(Value::Bool(true)))));
    assert_eq!(start_native(trigger, &mut host, 1, "Update__", &[]), None);
}
