//! U12: headless Gooey host for the script VM. Runs one UI script function on
//! `lbp-script-vm` and records every native call plus the frame tree the Gooey calls
//! describe. Runtime-side crate (AGENTS.md rule 6); parsing crates never depend on it.
//!
//! Native names come from the U11 census of the user's own 01.33 scripts (D133/D134,
//! /mnt/project-files/lbp2/research/g10-ui-natives.md): owner class plus a mangled name
//! `Base__argcodes`. Their meaning is not known from the engine; what this host does with
//! each is guessed from the name only (UNKNOWN), and every call is kept in the trace so a wrong
//! guess is visible:
//! - `Gooey.StartFrame*` opens a frame, `Gooey.EndFrame*` closes the innermost one.
//! - `Gooey.SetFrame<Prop>` records `<Prop>` = args on the innermost open frame.
//! - `Gooey.DoText*` adds a text node (first string argument), `Gooey.Do*Button*` a button
//!   (first string argument as its label), `Gooey.Do*Break*` a break; other `Gooey.Do*`
//!   calls are kept as generic widgets.
//! - `LAMS.Translate` takes a LAMS key and returns its string from the supplied tables
//!   (U1), else `#xxxxxxxx`.
//! - Everything else returns nothing; the VM then substitutes the default of the call's
//!   return type. Member reads return `Void` and writes are dropped (counted).
use lbp_level::script::Script;
use lbp_script_vm::{ClassId, Fault, Host, MemberKind, Value, Vm};
use serde::Serialize;
use std::collections::BTreeMap;

/// Trace entries kept per run (counts continue past it).
pub const MAX_TRACE: usize = 100_000;
/// Frames nested deeper than this are flattened into the innermost one.
pub const MAX_FRAME_DEPTH: usize = 256;
/// Nodes kept per run.
pub const MAX_NODES: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceCall {
    pub class: String,
    pub function: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Frame {
    /// The `StartFrame*` function and its arguments ("root" for the implicit root).
    pub start: String,
    pub props: BTreeMap<String, String>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Node {
    Frame(Frame),
    Text {
        text: String,
        args: Vec<String>,
    },
    Button {
        label: String,
        function: String,
        args: Vec<String>,
    },
    Break {
        function: String,
    },
    Widget {
        function: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct HostCounts {
    pub native_calls: usize,
    /// "Class.function" -> calls.
    pub by_native: BTreeMap<String, usize>,
    pub member_reads: usize,
    pub member_writes: usize,
    pub translated: usize,
    pub untranslated: usize,
    /// `EndFrame*` with no open frame.
    pub unbalanced_end: usize,
    /// Frames still open when the run ended.
    pub unclosed: usize,
    pub trace_dropped: usize,
    pub nodes_dropped: usize,
}

/// Records native calls and builds the Gooey frame tree.
pub struct GooeyHost<'a> {
    translate: &'a dyn Fn(u32) -> Option<String>,
    pub trace: Vec<TraceCall>,
    pub counts: HostCounts,
    pub output: Vec<String>,
    stack: Vec<Frame>,
    nodes: usize,
}

/// Mangled native name without its argument codes.
pub fn base_name(function: &str) -> &str {
    function.split("__").next().unwrap_or(function)
}

fn show(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

fn first_string(args: &[Value]) -> String {
    args.iter()
        .find_map(|a| match a {
            Value::Str(s) => Some(s.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

impl<'a> GooeyHost<'a> {
    pub fn new(translate: &'a dyn Fn(u32) -> Option<String>) -> Self {
        Self {
            translate,
            trace: Vec::new(),
            counts: HostCounts::default(),
            output: Vec::new(),
            stack: vec![Frame {
                start: "root".into(),
                ..Default::default()
            }],
            nodes: 0,
        }
    }

    fn push(&mut self, node: Node) {
        if self.nodes >= MAX_NODES {
            self.counts.nodes_dropped += 1;
            return;
        }
        self.nodes += 1;
        self.stack
            .last_mut()
            .expect("root frame")
            .children
            .push(node);
    }

    fn gooey(&mut self, function: &str, args: &[Value], shown: Vec<String>) {
        let base = base_name(function);
        if base.starts_with("StartFrame") {
            if self.stack.len() > MAX_FRAME_DEPTH {
                self.counts.nodes_dropped += 1;
                return;
            }
            self.stack.push(Frame {
                start: format!("{function}({})", shown.join(", ")),
                ..Default::default()
            });
        } else if base.starts_with("EndFrame") {
            if self.stack.len() > 1 {
                let f = self.stack.pop().expect("open frame");
                self.push(Node::Frame(f));
            } else {
                self.counts.unbalanced_end += 1;
            }
        } else if let Some(prop) = base.strip_prefix("SetFrame") {
            self.stack
                .last_mut()
                .expect("root frame")
                .props
                .insert(prop.to_owned(), shown.join(", "));
        } else if base.starts_with("DoText") {
            self.push(Node::Text {
                text: first_string(args),
                args: shown,
            });
        } else if base.starts_with("Do") && base.contains("Button") {
            self.push(Node::Button {
                label: first_string(args),
                function: function.to_owned(),
                args: shown,
            });
        } else if base.starts_with("Do") && base.contains("Break") {
            self.push(Node::Break {
                function: function.to_owned(),
            });
        } else if base.starts_with("Do") {
            self.push(Node::Widget {
                function: function.to_owned(),
                args: shown,
            });
        }
    }

    /// Closes frames left open and returns the root frame.
    pub fn finish(&mut self) -> Frame {
        while self.stack.len() > 1 {
            self.counts.unclosed += 1;
            let f = self.stack.pop().expect("open frame");
            self.push(Node::Frame(f));
        }
        let root = std::mem::take(&mut self.stack[0]);
        self.stack[0].start = "root".into();
        self.nodes = 0;
        root
    }
}

impl Host for GooeyHost<'_> {
    fn call_native(
        &mut self,
        class: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let shown: Vec<String> = args.iter().map(show).collect();
        self.counts.native_calls += 1;
        *self
            .counts
            .by_native
            .entry(format!("{class}.{function}"))
            .or_default() += 1;
        if self.trace.len() < MAX_TRACE {
            self.trace.push(TraceCall {
                class: class.to_owned(),
                function: function.to_owned(),
                args: shown.clone(),
            });
        } else {
            self.counts.trace_dropped += 1;
        }
        match (class, base_name(function)) {
            ("Gooey", _) => {
                self.gooey(function, args, shown);
                Ok(Value::Void)
            }
            ("LAMS", "Translate") => {
                let key = match args.first() {
                    Some(Value::Int(k)) => *k as u32,
                    _ => return Ok(Value::Void),
                };
                Ok(Value::Str(match (self.translate)(key) {
                    Some(t) => {
                        self.counts.translated += 1;
                        t.into()
                    }
                    None => {
                        self.counts.untranslated += 1;
                        format!("#{key:08x}").into()
                    }
                }))
            }
            _ => Ok(Value::Void),
        }
    }

    fn get_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        _class: &str,
        _field: &str,
    ) -> Result<Value, String> {
        self.counts.member_reads += 1;
        Ok(Value::Void)
    }

    fn set_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        _class: &str,
        _field: &str,
        _value: Value,
    ) -> Result<(), String> {
        self.counts.member_writes += 1;
        Ok(())
    }

    fn write(&mut self, text: &str) {
        if self.output.len() < MAX_TRACE {
            self.output.push(text.to_owned());
        }
    }
}

/// One function of a class as the trace tool lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub is_static: bool,
    pub is_native: bool,
    pub arguments: usize,
    pub instructions: usize,
}

/// Functions defined by `script`, in file order (ModifierType bit 0 STATIC, bit 1 NATIVE).
pub fn functions(script: &Script) -> Vec<FunctionInfo> {
    script
        .function_defs
        .iter()
        .map(|f| FunctionInfo {
            name: script.name(f.name).unwrap_or("<name?>").to_owned(),
            is_static: f.modifiers & 1 != 0,
            is_native: f.modifiers & 2 != 0,
            arguments: f.arguments.end.saturating_sub(f.arguments.begin) as usize,
            instructions: f.bytecode.end.saturating_sub(f.bytecode.begin) as usize,
        })
        .collect()
}

/// Arguments for calling `function` of `script` headlessly: the receiver `this` first for
/// a member function (VM rule, INFERRED), defaults of each argument's machine type after.
pub fn default_args(script: &Script, function: &str, this: Option<Value>) -> Option<Vec<Value>> {
    let f = script
        .function_defs
        .iter()
        .find(|f| script.name(f.name) == Some(function))?;
    let rows = script
        .shared_arguments
        .get(f.arguments.begin as usize..f.arguments.end as usize)?;
    let member = f.modifiers & 1 == 0;
    Some(
        rows.iter()
            .enumerate()
            .map(|(i, row)| match (&this, i == 0 && member) {
                (Some(t), true) => t.clone(),
                _ => Value::default_for(
                    script
                        .type_refs
                        .get(row.type_ref as usize)
                        .map_or(0, |t| t.machine_type),
                ),
            })
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunReport {
    pub class: String,
    pub function: String,
    /// Returned value, or the VM fault that stopped the run.
    pub result: Result<String, String>,
    pub instructions: u64,
    pub counts: HostCounts,
    pub tree: Frame,
    pub trace: Vec<TraceCall>,
    pub output: Vec<String>,
}

/// Instantiates `class` (for a member function), calls `function` with default
/// arguments under `budget` instructions, and returns what the host saw.
pub fn run(
    vm: &mut Vm,
    host: &mut GooeyHost,
    class: ClassId,
    script: &Script,
    function: &str,
    budget: u64,
) -> RunReport {
    let mut left = budget;
    let result = (|| -> Result<Value, Fault> {
        let this = Value::Object(Some(vm.instantiate(class)?));
        let args =
            default_args(script, function, Some(this)).ok_or_else(|| Fault::MissingFunction {
                class: vm.class_name(class).to_owned(),
                function: function.to_owned(),
            })?;
        vm.call(host, class, function, &args, &mut left)
    })();
    RunReport {
        class: vm.class_name(class).to_owned(),
        function: function.to_owned(),
        result: result.map(|v| show(&v)).map_err(|f| format!("{f:?}")),
        instructions: budget - left,
        tree: host.finish(),
        counts: host.counts.clone(),
        trace: std::mem::take(&mut host.trace),
        output: std::mem::take(&mut host.output),
    }
}

/// Indented outline of a frame tree, at most `max_lines` lines.
pub fn outline(root: &Frame, max_lines: usize) -> Vec<String> {
    fn walk(f: &Frame, depth: usize, out: &mut Vec<String>, max: usize) {
        let pad = "  ".repeat(depth);
        if out.len() < max {
            let props: Vec<String> = f.props.iter().map(|(k, v)| format!("{k}={v}")).collect();
            out.push(format!("{pad}frame {} [{}]", f.start, props.join("; ")));
        }
        for c in &f.children {
            if out.len() >= max {
                return;
            }
            match c {
                Node::Frame(f) => walk(f, depth + 1, out, max),
                Node::Text { text, .. } => out.push(format!("{pad}  text {text:?}")),
                Node::Button {
                    label, function, ..
                } => out.push(format!("{pad}  button {label:?} ({})", base_name(function))),
                Node::Break { function } => out.push(format!("{pad}  {}", base_name(function))),
                Node::Widget { function, args } => out.push(format!(
                    "{pad}  {}({})",
                    base_name(function),
                    args.join(", ")
                )),
            }
        }
    }
    let mut out = Vec::new();
    walk(root, 0, &mut out, max_lines);
    out
}
