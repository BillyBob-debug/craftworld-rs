//! D417: a golden-trace recorder for script runs (dev only).
//!
//! A [`Trace`] kept in [`crate::natives::Bindings::trace`] logs every host call a script
//! makes (natives, member reads and writes, Thing class tests) with its arguments and
//! answer, and the script entries the runtime starts or resumes. A trace of the VM run
//! of a script is the reference a native Rust port of that script is tested against:
//! the port, given the same answers, has to make the same calls in the same order.
//! The trace holds names and values only, never script code.
//!
//! JSON: one event object per line, values as tagged strings ([`encode_value`]).
use crate::{MemberKind, ObjectId, Site, Value};

/// What the runtime tells a tracing host ([`crate::Host::trace`]).
#[derive(Debug, Clone, Copy)]
pub enum TraceMark<'a> {
    /// A new tick (0: the create pass).
    Tick(u64),
    /// The runtime starts `function` on a Thing with its argument registers, or resumes
    /// it (`function` = [`RESUME`]).
    Call {
        thing: u32,
        function: &'a str,
        args: &'a [(u16, Value)],
    },
    /// The call or resume ended: [`END_DONE`], [`END_YIELDED`], [`END_FAULT`].
    End(&'static str),
    /// The script code making the next host calls.
    Site(&'a Site),
}

/// [`TraceMark::Call`] of a resumed script.
pub const RESUME: &str = "resume";
pub const END_DONE: &str = "done";
pub const END_YIELDED: &str = "yielded";
pub const END_FAULT: &str = "fault";
/// The entry had no code: its [`TraceMark::Call`] is taken back (the VM does not mark
/// a call to a function without code).
pub const END_NO_FUNCTION: &str = "no-function";

/// One logged event.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceEvent {
    pub tick: u64,
    /// The Thing whose script is running, from the last [`TraceMark::Call`].
    pub thing: Option<u32>,
    /// `Class.function` of the script code, when known.
    pub site: Option<String>,
    /// `call`, `end`, `native`, `get`, `set` or `is`.
    pub kind: &'static str,
    /// `call`: the function; `end`: the outcome; `native`: `Class.function`;
    /// `get` / `set`: `Kind:Class.field`; `is`: the class tested.
    pub name: String,
    pub args: Vec<Value>,
    /// The answer (`Void` for calls without one); `Err` text for a host error.
    pub result: Result<Value, String>,
}

/// The recorder.
#[derive(Debug, Clone)]
pub struct Trace {
    pub tick: u64,
    pub thing: Option<u32>,
    pub site: Option<String>,
    pub events: Vec<TraceEvent>,
    /// Events past this many are counted in `dropped`, not kept.
    pub max_events: usize,
    pub dropped: u64,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            tick: 0,
            thing: None,
            site: None,
            events: Vec::new(),
            max_events: 1_000_000,
            dropped: 0,
        }
    }
}

/// The `get` / `set` event name of a member access.
pub fn member_name(kind: MemberKind, class: &str, field: &str) -> String {
    format!("{kind:?}:{class}.{field}")
}

impl Trace {
    pub fn mark(&mut self, mark: TraceMark<'_>) {
        match mark {
            TraceMark::Tick(t) => self.tick = t,
            TraceMark::Call {
                thing,
                function,
                args,
            } => {
                self.thing = Some(thing);
                self.site = None;
                let args = args.iter().map(|(_, v)| v.clone()).collect();
                self.push("call", function.to_owned(), args, Ok(Value::Void));
            }
            TraceMark::End(END_NO_FUNCTION) => {
                if self.events.last().is_some_and(|e| e.kind == "call") {
                    self.events.pop();
                }
                self.thing = None;
                self.site = None;
            }
            TraceMark::End(outcome) => {
                self.push("end", outcome.to_owned(), Vec::new(), Ok(Value::Void));
                self.thing = None;
                self.site = None;
            }
            TraceMark::Site(s) => self.site = Some(format!("{}.{}", s.class, s.function)),
        }
    }

    pub fn push(
        &mut self,
        kind: &'static str,
        name: String,
        args: Vec<Value>,
        result: Result<Value, String>,
    ) {
        if self.events.len() >= self.max_events {
            self.dropped += 1;
            return;
        }
        self.events.push(TraceEvent {
            tick: self.tick,
            thing: self.thing,
            site: self.site.clone(),
            kind,
            name,
            args,
            result,
        });
    }

    /// The events as JSON: `{"dropped": N, "events": [ ... ]}`, one event a line.
    pub fn to_json(&self) -> String {
        events_json(&self.events, self.dropped)
    }
}

/// [`Trace::to_json`] for any list of events.
pub fn events_json(events: &[TraceEvent], dropped: u64) -> String {
    let mut out = format!("{{\"dropped\": {dropped}, \"events\": [\n");
    for (i, e) in events.iter().enumerate() {
        let opt = |v: Option<String>| v.map_or("null".to_owned(), |s| json_string(&s));
        let args: Vec<String> = e
            .args
            .iter()
            .map(|a| json_string(&encode_value(a)))
            .collect();
        let result = match &e.result {
            Ok(v) => json_string(&encode_value(v)),
            Err(t) => json_string(&format!("err:{t}")),
        };
        out.push_str(&format!(
            "{{\"tick\": {}, \"thing\": {}, \"site\": {}, \"kind\": \"{}\", \"name\": {}, \"args\": [{}], \"result\": {}}}{}\n",
            e.tick,
            e.thing.map_or("null".to_owned(), |t| t.to_string()),
            opt(e.site.clone()),
            e.kind,
            json_string(&e.name),
            args.join(", "),
            result,
            if i + 1 < events.len() { "," } else { "" }
        ));
    }
    out.push_str("]}\n");
    out
}

/// A JSON string literal.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn floats(v: &[f32]) -> String {
    v.iter().map(f32::to_string).collect::<Vec<_>>().join(",")
}

/// A value as a tagged string: `v` (void), `b:true`, `c:65`, `i:-1`, `f:0.05`,
/// `v4:x,y,z,w`, `m:16 floats`, `l:`, `d:`, `s:text`, `o:N` / `o:null`, `t:N` / `t:null`,
/// `r:N`, `ref:N`. Floats print their shortest form that reads back to the same bits.
pub fn encode_value(v: &Value) -> String {
    let opt = |o: Option<u32>| o.map_or("null".to_owned(), |n| n.to_string());
    match v {
        Value::Void => "v".to_owned(),
        Value::Bool(b) => format!("b:{b}"),
        Value::Char(c) => format!("c:{c}"),
        Value::Int(i) => format!("i:{i}"),
        Value::Float(f) => format!("f:{f}"),
        Value::Vec4(a) => format!("v4:{}", floats(a)),
        Value::Mat44(m) => format!("m:{}", floats(m)),
        Value::Long(l) => format!("l:{l}"),
        Value::Double(d) => format!("d:{d}"),
        Value::Str(s) => format!("s:{s}"),
        Value::Object(o) => format!("o:{}", opt(o.map(|o| o.0))),
        Value::Thing(t) => format!("t:{}", opt(*t)),
        Value::Raw(r) => format!("r:{r}"),
        Value::Ref(r) => format!("ref:{r}"),
    }
}

/// Reads back [`encode_value`]; `None` for text that is not one.
pub fn decode_value(s: &str) -> Option<Value> {
    if s == "v" {
        return Some(Value::Void);
    }
    let (tag, rest) = s.split_once(':')?;
    let opt = |r: &str| -> Option<Option<u32>> {
        if r == "null" {
            Some(None)
        } else {
            r.parse().ok().map(Some)
        }
    };
    let fl = |r: &str| -> Option<Vec<f32>> { r.split(',').map(|x| x.parse().ok()).collect() };
    Some(match tag {
        "b" => Value::Bool(rest.parse().ok()?),
        "c" => Value::Char(rest.parse().ok()?),
        "i" => Value::Int(rest.parse().ok()?),
        "f" => Value::Float(rest.parse().ok()?),
        "v4" => Value::Vec4(fl(rest)?.try_into().ok()?),
        "m" => Value::Mat44(fl(rest)?.try_into().ok()?),
        "l" => Value::Long(rest.parse().ok()?),
        "d" => Value::Double(rest.parse().ok()?),
        "s" => Value::Str(rest.into()),
        "o" => Value::Object(opt(rest)?.map(ObjectId)),
        "t" => Value::Thing(opt(rest)?),
        "r" => Value::Raw(rest.parse().ok()?),
        "ref" => Value::Ref(rest.parse().ok()?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_read_back() {
        let values = [
            Value::Void,
            Value::Bool(true),
            Value::Int(-1),
            Value::Float(0.05),
            Value::Float(1.0 - 0.05 * 3.0),
            Value::Vec4([0.2, 0.4, 0.8, 0.0]),
            Value::Str("BP_ENGAGE_POD".into()),
            Value::Thing(None),
            Value::Thing(Some(12)),
            Value::Object(Some(ObjectId(3))),
            Value::Raw(7),
        ];
        for v in values {
            assert_eq!(decode_value(&encode_value(&v)), Some(v.clone()), "{v:?}");
        }
    }

    #[test]
    fn json_escapes() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
}
