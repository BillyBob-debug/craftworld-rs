//! Rq: register VM for compiled Fish scripts (RScript), built on `lbp_level::script`.
//!
//! Runtime crate (AGENTS.md, 2026-10-08 scope): the parsing crates never depend on it.
//! Scripts come from the user's own game data; nothing here is translated from game code.
//! Instruction layouts are the U10 reader's (Toolkit d6a2ef80, CONFIRMED_FROM_COMMUNITY_CODE).
//! Everything about *execution* is INFERRED from mnemonics and operand names and is listed
//! in docs/script-vm.md; the uncertain rules are switchable through [`Rules`] so that a
//! measurement (branch census, RPCS3 comparison) can settle them without code changes.
//!
//! Model: each call gets a frame of `stack_size` register slots addressed by the 16-bit
//! operand (one slot per address; a value never spans slots). `ARG` stages a value at an
//! argument address; `CALL` copies staged values into the callee frame at the same
//! addresses. Native functions, Thing members and native members go to a [`Host`].
use lbp_level::script::{class, Class, Script};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub mod check;
pub mod natives;
pub mod signature;

/// Hard limits; every one fails with a [`Fault`], never a panic.
pub const MAX_CALL_DEPTH: usize = 256;
pub const MAX_HEAP_OBJECTS: usize = 1 << 20;
pub const MAX_ARRAY_LEN: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u32);

/// One register value. Pointer kinds stay distinct so the host can tell Things
/// (`safe_ptr`) from engine resources (`raw_ptr`/`ref_ptr`) and script objects.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Void,
    Bool(bool),
    Char(u16),
    Int(i32),
    Float(f32),
    Vec4([f32; 4]),
    /// Rows: `m[r * 4 + c]` (INFERRED; see `GET_M44_*`).
    Mat44([f32; 16]),
    Long(i64),
    Double(f64),
    Str(Arc<str>),
    /// Script object or array on the VM heap; `None` = null.
    Object(Option<ObjectId>),
    /// Thing handle owned by the host; `None` = null.
    Thing(Option<u32>),
    Raw(u32),
    Ref(u32),
}

impl Value {
    fn truthy(&self) -> Option<bool> {
        Some(match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Char(c) => *c != 0,
            Value::Long(l) => *l != 0,
            Value::Object(o) => o.is_some(),
            Value::Thing(t) => t.is_some(),
            _ => return None,
        })
    }
    /// Default for a Toolkit MachineType byte (fields, new arrays).
    pub fn default_for(machine_type: u8) -> Value {
        match machine_type {
            1 => Value::Bool(false),
            2 => Value::Char(0),
            3 => Value::Int(0),
            4 => Value::Float(0.),
            5 => Value::Vec4([0.; 4]),
            6 => Value::Mat44(IDENTITY),
            8 => Value::Raw(0),
            9 => Value::Ref(0),
            10 => Value::Thing(None),
            11 => Value::Object(None),
            12 => Value::Long(0),
            13 => Value::Double(0.),
            _ => Value::Void,
        }
    }
}

const IDENTITY: [f32; 16] = [
    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
];

#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    UnknownOpcode {
        pc: usize,
        opcode: u8,
    },
    Register {
        pc: usize,
        register: u16,
    },
    Type {
        pc: usize,
        op: &'static str,
    },
    DivideByZero {
        pc: usize,
    },
    Branch {
        pc: usize,
    },
    Index {
        pc: usize,
        what: &'static str,
    },
    NullObject {
        pc: usize,
    },
    MissingClass(String),
    MissingFunction {
        class: String,
        function: String,
    },
    Arity {
        expected: usize,
        got: usize,
    },
    Assert {
        pc: usize,
    },
    Native(String),
    Budget,
    CallDepth,
    Heap,
    /// `System.Yield` reached from [`Vm::call`], which cannot suspend; use [`Vm::start`].
    YieldInCall,
}

/// Result of [`Vm::start`] / [`Vm::resume`].
#[derive(Debug)]
pub enum Step {
    Done(Value),
    /// `System.Yield` suspended the script; resume it on a later tick.
    Yielded(Task),
}

/// A suspended script call (its frames), resumed with [`Vm::resume`].
#[derive(Debug)]
pub struct Task {
    stack: Vec<Frame>,
}

/// Which member access reaches the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// `GET/SET_SP_MEMBER`: script member of a Thing (PScript instance data).
    Thing,
    /// `GET/SET_RP_MEMBER`.
    RefPtr,
    /// `GET_SP_NATIVE_MEMBER`: engine field of a Thing.
    Native,
}

/// Engine side of the VM: native functions and members, by name.
pub trait Host {
    fn call_native(&mut self, class: &str, function: &str, args: &[Value])
        -> Result<Value, String>;
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String>;
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String>;
    /// `CASTsp` on a Thing: does it carry a script of this class?
    fn thing_is(&mut self, _thing: u32, _class: &str) -> bool {
        true
    }
    fn write(&mut self, _text: &str) {}
}

/// Uncertain execution rules (docs/script-vm.md). Defaults are the current best guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    /// Branch target = `pc + offset` (true) or `pc + 1 + offset` (false).
    pub branch_from_current: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            branch_from_current: true,
        }
    }
}

struct LoadedClass {
    script: Script,
    name: String,
    guid: Option<u32>,
    /// Function name -> function definition index (first definition wins).
    functions: HashMap<String, usize>,
}

enum HeapObject {
    Instance {
        class: ClassId,
        fields: BTreeMap<String, Value>,
    },
    Array(Vec<Value>),
}

#[derive(Debug)]
struct Frame {
    class: ClassId,
    function: usize,
    pc: usize,
    regs: Vec<Value>,
    /// Caller register receiving the return value.
    dest: Option<u16>,
}

pub struct Vm {
    pub rules: Rules,
    classes: Vec<LoadedClass>,
    by_guid: HashMap<u32, ClassId>,
    by_name: HashMap<String, ClassId>,
    heap: Vec<HeapObject>,
    /// Calls into functions that matched more than one definition by name.
    pub ambiguous_calls: usize,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new(Rules::default())
    }
}

impl Vm {
    pub fn new(rules: Rules) -> Self {
        Self {
            rules,
            classes: Vec::new(),
            by_guid: HashMap::new(),
            by_name: HashMap::new(),
            heap: Vec::new(),
            ambiguous_calls: 0,
        }
    }

    /// Registers a script class. A GUID lets other scripts' type references find it.
    pub fn load(&mut self, script: Script, guid: Option<u32>) -> ClassId {
        let id = ClassId(self.classes.len() as u32);
        let mut functions = HashMap::new();
        for (i, f) in script.function_defs.iter().enumerate() {
            if let Some(name) = script.name(f.name) {
                functions.entry(name.to_owned()).or_insert(i);
            }
        }
        let name = script.class_name.clone();
        if let Some(g) = guid {
            self.by_guid.insert(g, id);
        }
        self.by_name.entry(name.clone()).or_insert(id);
        self.classes.push(LoadedClass {
            script,
            name,
            guid,
            functions,
        });
        id
    }

    pub fn class_by_name(&self, name: &str) -> Option<ClassId> {
        self.by_name.get(name).copied()
    }

    pub fn class_name(&self, id: ClassId) -> &str {
        &self.classes[id.0 as usize].name
    }

    pub fn class_guid(&self, id: ClassId) -> Option<u32> {
        self.classes[id.0 as usize].guid
    }

    /// Field of a script object (for hosts and tests).
    pub fn field(&self, object: ObjectId, name: &str) -> Option<&Value> {
        match self.heap.get(object.0 as usize)? {
            HeapObject::Instance { fields, .. } => fields.get(name),
            HeapObject::Array(_) => None,
        }
    }

    pub fn array(&self, object: ObjectId) -> Option<&[Value]> {
        match self.heap.get(object.0 as usize)? {
            HeapObject::Array(v) => Some(v),
            HeapObject::Instance { .. } => None,
        }
    }

    /// Creates an instance of a loaded class with default field values.
    pub fn instantiate(&mut self, class: ClassId) -> Result<ObjectId, Fault> {
        let mut fields = BTreeMap::new();
        let mut at = Some(class);
        let mut depth = 0;
        while let Some(c) = at {
            let s = &self.classes[c.0 as usize].script;
            for f in &s.field_defs {
                let ty = s
                    .type_refs
                    .get(f.type_ref as usize)
                    .map_or(0, |t| t.machine_type);
                if let Some(name) = s.name(f.name) {
                    fields
                        .entry(name.to_owned())
                        .or_insert_with(|| Value::default_for(ty));
                }
            }
            at = self.super_class(c);
            depth += 1;
            if depth > MAX_CALL_DEPTH {
                return Err(Fault::MissingClass("superclass cycle".into()));
            }
        }
        self.alloc(HeapObject::Instance { class, fields })
    }

    fn alloc(&mut self, o: HeapObject) -> Result<ObjectId, Fault> {
        if self.heap.len() >= MAX_HEAP_OBJECTS {
            return Err(Fault::Heap);
        }
        self.heap.push(o);
        Ok(ObjectId(self.heap.len() as u32 - 1))
    }

    fn super_class(&self, c: ClassId) -> Option<ClassId> {
        let g = self.classes[c.0 as usize].script.super_class?.guid?;
        self.by_guid.get(&g).copied()
    }

    /// Finds a function with code by name in a class or its superclasses.
    fn find(&self, class: ClassId, name: &str) -> Option<(ClassId, usize)> {
        let mut at = Some(class);
        for _ in 0..MAX_CALL_DEPTH {
            let c = at?;
            let lc = &self.classes[c.0 as usize];
            if let Some(&i) = lc.functions.get(name) {
                let f = &lc.script.function_defs[i];
                // Native functions have no bytecode (modifier bit 1).
                if f.modifiers & 2 == 0 {
                    return Some((c, i));
                }
                return None;
            }
            at = self.super_class(c);
        }
        None
    }

    /// Calls `function` on `class` from the host. `args` fill the function's argument
    /// rows in order (receiver first for member functions, INFERRED). A script that
    /// yields fails with [`Fault::YieldInCall`]; coroutines go through [`Vm::start`].
    pub fn call(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        args: &[Value],
        budget: &mut u64,
    ) -> Result<Value, Fault> {
        match self.start(host, class, function, args, budget)? {
            Step::Done(v) => Ok(v),
            Step::Yielded(_) => Err(Fault::YieldInCall),
        }
    }

    /// Like [`Vm::call`], but `System.Yield` suspends the call and returns a [`Task`].
    pub fn start(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        args: &[Value],
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let (c, f) = self
            .find(class, function)
            .ok_or_else(|| Fault::MissingFunction {
                class: self.class_name(class).to_owned(),
                function: function.to_owned(),
            })?;
        let s = &self.classes[c.0 as usize].script;
        let def = &s.function_defs[f];
        let rows = s
            .shared_arguments
            .get(def.arguments.begin as usize..def.arguments.end as usize)
            .ok_or(Fault::Index {
                pc: 0,
                what: "argument rows",
            })?;
        if rows.len() != args.len() {
            return Err(Fault::Arity {
                expected: rows.len(),
                got: args.len(),
            });
        }
        let mut staged = BTreeMap::new();
        for (row, v) in rows.iter().zip(args) {
            let at = u16::try_from(row.offset).map_err(|_| Fault::Index {
                pc: 0,
                what: "argument offset",
            })?;
            staged.insert(at, v.clone());
        }
        let frame = self.frame(c, f, staged, None)?;
        self.run(host, vec![frame], budget)
    }

    /// Continues a task after the instruction that yielded.
    pub fn resume(
        &mut self,
        host: &mut dyn Host,
        task: Task,
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        self.run(host, task.stack, budget)
    }

    /// Does `function` resolve to script code in `class` or a superclass? `false` means
    /// a call to it goes to the host as a native.
    pub fn has_code(&self, class: ClassId, function: &str) -> bool {
        self.find(class, function).is_some()
    }

    /// Starts `function` with registers staged directly (`(register, value)` pairs), for
    /// hosts that place the receiver themselves. Registers past the stack size fault.
    pub fn start_registers(
        &mut self,
        host: &mut dyn Host,
        class: ClassId,
        function: &str,
        registers: &[(u16, Value)],
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let (c, f) = self
            .find(class, function)
            .ok_or_else(|| Fault::MissingFunction {
                class: self.class_name(class).to_owned(),
                function: function.to_owned(),
            })?;
        let staged = registers.iter().cloned().collect();
        let frame = self.frame(c, f, staged, None)?;
        self.run(host, vec![frame], budget)
    }

    fn frame(
        &self,
        class: ClassId,
        function: usize,
        staged: BTreeMap<u16, Value>,
        dest: Option<u16>,
    ) -> Result<Frame, Fault> {
        let def = &self.classes[class.0 as usize].script.function_defs[function];
        let size = usize::try_from(def.stack_size).unwrap_or(0).min(1 << 16);
        let mut regs = vec![Value::Void; size];
        for (at, v) in staged {
            *regs.get_mut(usize::from(at)).ok_or(Fault::Register {
                pc: def.bytecode.begin as usize,
                register: at,
            })? = v;
        }
        Ok(Frame {
            class,
            function,
            pc: def.bytecode.begin as usize,
            regs,
            dest,
        })
    }

    fn run(
        &mut self,
        host: &mut dyn Host,
        mut stack: Vec<Frame>,
        budget: &mut u64,
    ) -> Result<Step, Fault> {
        let mut staged: BTreeMap<u16, Value> = BTreeMap::new();
        loop {
            if *budget == 0 {
                return Err(Fault::Budget);
            }
            *budget -= 1;
            let depth = stack.len();
            let fr = stack.last_mut().expect("non-empty stack");
            let script = &self.classes[fr.class.0 as usize].script;
            let def = &script.function_defs[fr.function];
            let pc = fr.pc;
            if pc < def.bytecode.begin as usize || pc >= def.bytecode.end as usize {
                return Err(Fault::Branch { pc });
            }
            let bits = *script.bytecode.get(pc).ok_or(Fault::Branch { pc })?;
            let op = (bits & 0xff) as u8;
            let ty = ((bits >> 8) & 0xff) as u8;
            let r16 = |s: u32| ((bits >> s) & 0xffff) as u16;
            let hi32 = (bits >> 32) as u32;
            let cls = class(op).ok_or(Fault::UnknownOpcode { pc, opcode: op })?;
            let mut next = pc + 1;
            macro_rules! get {
                ($r:expr) => {
                    fr.regs
                        .get(usize::from($r))
                        .cloned()
                        .ok_or(Fault::Register { pc, register: $r })?
                };
            }
            macro_rules! set {
                ($r:expr, $v:expr) => {{
                    let v = $v;
                    *fr.regs
                        .get_mut(usize::from($r))
                        .ok_or(Fault::Register { pc, register: $r })? = v;
                }};
            }
            let mismatch = || Fault::Type {
                pc,
                op: lbp_level::script::MNEMONICS[op as usize],
            };
            match cls {
                Class::Nop => {}
                Class::LoadConst => {
                    let v = match op {
                        0x01 => Value::Bool(bits >> 63 != 0),
                        0x02 => Value::Char(r16(48)),
                        0x03 => Value::Int(hi32 as i32),
                        0x04 => Value::Float(f32::from_bits(hi32)),
                        0x05 => Value::Str(
                            script
                                .strings_w
                                .get(hi32 as usize)
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "wide string",
                                })?
                                .as_str()
                                .into(),
                        ),
                        0xbc => Value::Str(
                            script
                                .strings_a
                                .get(hi32 as usize)
                                .ok_or(Fault::Index { pc, what: "string" })?
                                .as_str()
                                .into(),
                        ),
                        0x06 => Value::Thing(None),
                        0xb9 => Value::Object(None),
                        0xc3 => Value::Long(
                            *script
                                .constants_s64
                                .as_ref()
                                .and_then(|c| c.get(hi32 as usize))
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "s64 constant",
                                })? as i64,
                        ),
                        // LCv4: four consecutive f32 constants (INFERRED).
                        0xb8 => {
                            let i = hi32 as usize;
                            let c = script
                                .constants_f32
                                .get(i..i.checked_add(4).ok_or(mismatch())?)
                                .ok_or(Fault::Index {
                                    pc,
                                    what: "f32 constants",
                                })?;
                            Value::Vec4([c[0], c[1], c[2], c[3]])
                        }
                        _ => return Err(mismatch()),
                    };
                    set!(r16(16), v);
                }
                Class::Unary => {
                    let v = unary(op, get!(r16(32))).ok_or_else(mismatch)?;
                    set!(r16(16), v);
                }
                Class::Binary => {
                    let v = binary(op, get!(r16(32)), get!(r16(48)))
                        .map_err(|e| e.unwrap_or_else(mismatch))?;
                    let v = match v {
                        Err(()) => return Err(Fault::DivideByZero { pc }),
                        Ok(v) => v,
                    };
                    set!(r16(16), v);
                }
                Class::GetBuiltinMember => {
                    let base = get!(r16(32));
                    let v = if op == 0x9b {
                        match base {
                            Value::Object(Some(o)) => match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Array(a)) => Value::Int(a.len() as i32),
                                _ => return Err(mismatch()),
                            },
                            Value::Object(None) => return Err(Fault::NullObject { pc }),
                            _ => return Err(mismatch()),
                        }
                    } else {
                        get_builtin(op, &base).ok_or_else(mismatch)?
                    };
                    set!(r16(16), v);
                }
                Class::SetBuiltinMember => {
                    let src = get!(r16(16));
                    let base = r16(32);
                    let cur = get!(base);
                    set!(base, set_builtin(op, cur, src).ok_or_else(mismatch)?);
                }
                Class::GetMember | Class::SetMember => {
                    let base = get!(r16(32));
                    let (owner, field) = member_name(script, r16(48)).ok_or(Fault::Index {
                        pc,
                        what: "field reference",
                    })?;
                    let get = cls == Class::GetMember;
                    let kind = match op {
                        0x95 | 0x97 => Some(MemberKind::Thing),
                        0x96 | 0x98 => Some(MemberKind::RefPtr),
                        0xbb => Some(MemberKind::Native),
                        _ => None, // 0xb3 / 0xb4: script object member
                    };
                    if let Some(kind) = kind {
                        if get {
                            let v = host
                                .get_member(kind, &base, &owner, &field)
                                .map_err(Fault::Native)?;
                            set!(r16(16), v);
                        } else {
                            let v = get!(r16(16));
                            host.set_member(kind, &base, &owner, &field, v)
                                .map_err(Fault::Native)?;
                        }
                    } else {
                        let Value::Object(Some(o)) = base else {
                            return Err(if base == Value::Object(None) {
                                Fault::NullObject { pc }
                            } else {
                                mismatch()
                            });
                        };
                        let fields = match self.heap.get_mut(o.0 as usize) {
                            Some(HeapObject::Instance { fields, .. }) => fields,
                            _ => return Err(mismatch()),
                        };
                        if get {
                            let v = fields.get(&field).cloned().ok_or(Fault::Index {
                                pc,
                                what: "object field",
                            })?;
                            set!(r16(16), v);
                        } else {
                            let v = get!(r16(16));
                            fields.insert(field, v);
                        }
                    }
                }
                Class::GetElement => {
                    let (o, arg) = (get!(r16(32)), get!(r16(48)));
                    let a =
                        array_ref(&self.heap, &o, pc).map_err(|e| e.unwrap_or_else(mismatch))?;
                    let v = if op == 0xa0 {
                        Value::Int(a.iter().position(|x| *x == arg).map_or(-1, |i| i as i32))
                    } else {
                        let Value::Int(i) = arg else {
                            return Err(mismatch());
                        };
                        a.get(usize::try_from(i).map_err(|_| Fault::Index {
                            pc,
                            what: "element",
                        })?)
                        .cloned()
                        .ok_or(Fault::Index {
                            pc,
                            what: "element",
                        })?
                    };
                    set!(r16(16), v);
                }
                Class::SetElement => {
                    let o = get!(r16(32));
                    let index = get!(r16(48));
                    let src = get!(r16(16));
                    let Value::Object(Some(id)) = o else {
                        return Err(if o == Value::Object(None) {
                            Fault::NullObject { pc }
                        } else {
                            mismatch()
                        });
                    };
                    let Some(HeapObject::Array(a)) = self.heap.get_mut(id.0 as usize) else {
                        return Err(mismatch());
                    };
                    let idx = |v: &Value| match v {
                        Value::Int(i) => usize::try_from(*i).ok(),
                        _ => None,
                    };
                    let bad = Fault::Index {
                        pc,
                        what: "element",
                    };
                    match op {
                        0x9a => *a.get_mut(idx(&index).ok_or(bad.clone())?).ok_or(bad)? = src,
                        0x9d => {
                            let i = idx(&index).filter(|&i| i <= a.len()).ok_or(bad)?;
                            if a.len() >= MAX_ARRAY_LEN {
                                return Err(Fault::Heap);
                            }
                            a.insert(i, src);
                        }
                        0x9e => {
                            if a.len() >= MAX_ARRAY_LEN {
                                return Err(Fault::Heap);
                            }
                            a.push(src);
                        }
                        0x9f => {
                            let i = idx(&index).filter(|&i| i < a.len()).ok_or(bad)?;
                            a.remove(i);
                        }
                        0xa1 => a.clear(),
                        0xb6 => {
                            let n = idx(&index)
                                .filter(|&n| n <= MAX_ARRAY_LEN)
                                .ok_or(Fault::Heap)?;
                            let fill = a.first().map_or(Value::Void, zero_like);
                            a.resize(n, fill);
                        }
                        0xb7 => {} // ARRAY_RESERVE: capacity hint only
                        _ => return Err(mismatch()),
                    }
                }
                Class::NewArray => {
                    let Value::Int(n) = get!(r16(32)) else {
                        return Err(mismatch());
                    };
                    let n = usize::try_from(n)
                        .ok()
                        .filter(|&n| n <= MAX_ARRAY_LEN)
                        .ok_or(Fault::Heap)?;
                    let t = script
                        .type_refs
                        .get(usize::from(r16(48)))
                        .ok_or(Fault::Index {
                            pc,
                            what: "type reference",
                        })?;
                    let elem = Value::default_for(t.array_base_machine_type);
                    let id = self.alloc(HeapObject::Array(vec![elem; n]))?;
                    let fr = stack.last_mut().expect("frame");
                    *fr.regs
                        .get_mut(usize::from(r16(16)))
                        .ok_or(Fault::Register {
                            pc,
                            register: r16(16),
                        })? = Value::Object(Some(id));
                    fr.pc = next;
                    continue;
                }
                Class::NewObject => {
                    let target = self.type_class(script, r16(48), pc)?;
                    let id = self.instantiate(target)?;
                    let fr = stack.last_mut().expect("frame");
                    *fr.regs
                        .get_mut(usize::from(r16(16)))
                        .ok_or(Fault::Register {
                            pc,
                            register: r16(16),
                        })? = Value::Object(Some(id));
                    fr.pc = next;
                    continue;
                }
                Class::Write => {
                    let v = get!(r16(32));
                    if op == 0xc2 {
                        if v.truthy() != Some(true) {
                            return Err(Fault::Assert { pc });
                        }
                    } else {
                        host.write(&format!("{v:?}"));
                    }
                }
                Class::Arg => {
                    staged.insert(r16(32), get!(r16(16)));
                }
                Class::Cast => {
                    let v = get!(r16(32));
                    let target =
                        script
                            .type_refs
                            .get(usize::from(r16(48)))
                            .ok_or(Fault::Index {
                                pc,
                                what: "type reference",
                            })?;
                    let name = script.name(target.type_name).unwrap_or("").to_owned();
                    let out = match v {
                        Value::Thing(Some(t)) if op == 0xa9 => {
                            if host.thing_is(t, &name) {
                                v
                            } else {
                                Value::Thing(None)
                            }
                        }
                        Value::Object(Some(o)) if op == 0xba => {
                            let ok = match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Instance { class, .. }) => {
                                    self.is_a(*class, &name)
                                }
                                _ => false,
                            };
                            if ok {
                                v
                            } else {
                                Value::Object(None)
                            }
                        }
                        Value::Thing(None) | Value::Object(None) => v,
                        _ => return Err(mismatch()),
                    };
                    set!(r16(16), out);
                }
                Class::Call => {
                    let dest = (ty != 0).then(|| r16(16));
                    let (owner, function, owner_class) =
                        {
                            let fref = script.function_refs.get(usize::from(r16(32))).ok_or(
                                Fault::Index {
                                    pc,
                                    what: "function reference",
                                },
                            )?;
                            let name = script.name(fref.name).ok_or(Fault::Index {
                                pc,
                                what: "function name",
                            })?;
                            let t = script.type_refs.get(fref.type_ref as usize);
                            let owner = t
                                .and_then(|t| script.name(t.type_name))
                                .unwrap_or("")
                                .to_owned();
                            let owner_class = t
                                .and_then(|t| t.script)
                                .and_then(|d| d.guid)
                                .and_then(|g| self.by_guid.get(&g).copied())
                                .or_else(|| self.by_name.get(&owner).copied());
                            (owner, name.to_owned(), owner_class)
                        };
                    let args = std::mem::take(&mut staged);
                    // Virtual calls dispatch on the receiver: the lowest argument address.
                    let receiver_class = if op == 0xc0 {
                        match args.values().next() {
                            Some(Value::Object(Some(o))) => match self.heap.get(o.0 as usize) {
                                Some(HeapObject::Instance { class, .. }) => Some(*class),
                                _ => None,
                            },
                            Some(Value::Object(None)) => return Err(Fault::NullObject { pc }),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let target = receiver_class
                        .or(owner_class)
                        .and_then(|c| self.find(c, &function));
                    let fr = stack.last_mut().expect("frame");
                    fr.pc = next;
                    match target {
                        Some((c, f)) => {
                            if depth >= MAX_CALL_DEPTH {
                                return Err(Fault::CallDepth);
                            }
                            if self.classes[c.0 as usize]
                                .script
                                .function_defs
                                .iter()
                                .filter(|d| {
                                    self.classes[c.0 as usize].script.name(d.name)
                                        == Some(function.as_str())
                                })
                                .count()
                                > 1
                            {
                                self.ambiguous_calls += 1;
                            }
                            let callee = self.frame(c, f, args, dest)?;
                            stack.push(callee);
                        }
                        // INFERRED: Yield suspends the script until the next tick
                        // (docs/script-natives.md).
                        None if owner == "System" && function == "Yield__" => {
                            return Ok(Step::Yielded(Task { stack }));
                        }
                        None => {
                            let values: Vec<Value> = args.into_values().collect();
                            let v = host
                                .call_native(&owner, &function, &values)
                                .map_err(Fault::Native)?;
                            // A host that returns nothing for a typed call (stubs) gets the
                            // default of the instruction's return machine type.
                            let v = if v == Value::Void && ty != 0 {
                                Value::default_for(ty)
                            } else {
                                v
                            };
                            if let Some(d) = dest {
                                let fr = stack.last_mut().expect("frame");
                                *fr.regs
                                    .get_mut(usize::from(d))
                                    .ok_or(Fault::Register { pc, register: d })? = v;
                            }
                        }
                    }
                    continue;
                }
                Class::Return => {
                    let v = if ty == 0 { Value::Void } else { get!(r16(32)) };
                    let done = stack.pop().expect("frame");
                    staged.clear();
                    match stack.last_mut() {
                        None => return Ok(Step::Done(v)),
                        Some(caller) => {
                            if let Some(d) = done.dest {
                                *caller
                                    .regs
                                    .get_mut(usize::from(d))
                                    .ok_or(Fault::Register { pc, register: d })? = v;
                            }
                        }
                    }
                    continue;
                }
                Class::Branch => {
                    let take = match op {
                        0xa6 => true,
                        0xa7 => !get!(r16(16)).truthy().ok_or_else(mismatch)?,
                        _ => get!(r16(16)).truthy().ok_or_else(mismatch)?,
                    };
                    if take {
                        let base = if self.rules.branch_from_current {
                            pc
                        } else {
                            pc + 1
                        };
                        next = base
                            .checked_add_signed(hi32 as i32 as isize)
                            .ok_or(Fault::Branch { pc })?;
                    }
                }
            }
            stack.last_mut().expect("frame").pc = next;
        }
    }

    fn type_class(&self, script: &Script, type_ref: u16, pc: usize) -> Result<ClassId, Fault> {
        let t = script
            .type_refs
            .get(usize::from(type_ref))
            .ok_or(Fault::Index {
                pc,
                what: "type reference",
            })?;
        let name = script.name(t.type_name).unwrap_or("");
        t.script
            .and_then(|d| d.guid)
            .and_then(|g| self.by_guid.get(&g).copied())
            .or_else(|| self.by_name.get(name).copied())
            .ok_or_else(|| Fault::MissingClass(name.to_owned()))
    }

    fn is_a(&self, class: ClassId, name: &str) -> bool {
        let mut at = Some(class);
        for _ in 0..MAX_CALL_DEPTH {
            match at {
                Some(c) if self.classes[c.0 as usize].name == name => return true,
                Some(c) => at = self.super_class(c),
                None => return false,
            }
        }
        false
    }
}

/// (owning class name, field name) of a field reference.
fn member_name(script: &Script, field: u16) -> Option<(String, String)> {
    let r = script.field_refs.get(usize::from(field))?;
    let name = script.name(r.name)?.to_owned();
    let owner = script
        .type_refs
        .get(r.type_ref as usize)
        .and_then(|t| script.name(t.type_name))
        .unwrap_or("")
        .to_owned();
    Some((owner, name))
}

/// `Err(None)` = type mismatch, `Err(Some(fault))` = a specific fault.
fn array_ref<'a>(
    heap: &'a [HeapObject],
    o: &Value,
    pc: usize,
) -> Result<&'a [Value], Option<Fault>> {
    match o {
        Value::Object(Some(id)) => match heap.get(id.0 as usize) {
            Some(HeapObject::Array(a)) => Ok(a),
            _ => Err(None),
        },
        Value::Object(None) => Err(Some(Fault::NullObject { pc })),
        _ => Err(None),
    }
}

fn zero_like(v: &Value) -> Value {
    match v {
        Value::Bool(_) => Value::Bool(false),
        Value::Char(_) => Value::Char(0),
        Value::Int(_) => Value::Int(0),
        Value::Float(_) => Value::Float(0.),
        Value::Vec4(_) => Value::Vec4([0.; 4]),
        Value::Mat44(_) => Value::Mat44(IDENTITY),
        Value::Long(_) => Value::Long(0),
        Value::Double(_) => Value::Double(0.),
        Value::Object(_) => Value::Object(None),
        Value::Thing(_) => Value::Thing(None),
        _ => Value::Void,
    }
}

fn unary(op: u8, v: Value) -> Option<Value> {
    use Value::*;
    Some(match (op, v) {
        // Plain moves (MOVb/c/i/f/v4/m44/rp/cp/sp/o/s64); MOVs is a Nop-class opcode.
        (0x07 | 0x09 | 0x0a | 0x11 | 0x18 | 0x1a | 0x1c..=0x1f | 0xc4, v) => v,
        (0x08, Bool(b)) => Bool(!b),
        (0x0b, Int(i)) => Int(i.wrapping_add(1)),
        (0x0c, Int(i)) => Int(i.wrapping_sub(1)),
        (0x0d, Int(i)) => Int(i.wrapping_neg()),
        (0x0e, Int(i)) => Int(!i),
        (0x0f, Int(i)) => Bool(i == 0),
        (0x10, Int(i)) => Int(i.wrapping_abs()),
        (0x12, Float(f)) => Float(-f),
        (0x13, Float(f)) => Float(f.abs()),
        (0x14, Float(f)) => Float(f.sqrt()),
        (0x15, Float(f)) => Float(f.sin()),
        (0x16, Float(f)) => Float(f.cos()),
        (0x17, Float(f)) => Float(f.tan()),
        (0x19, Vec4(v)) => Vec4(v.map(|x| -x)),
        // Conversions: INTb INTc INTf FLOATb FLOATc FLOATi BOOLc BOOLi BOOLf.
        (0xaa, Bool(b)) => Int(i32::from(b)),
        (0xab, Char(c)) => Int(i32::from(c)),
        (0xac, Float(f)) => Int(f as i32),
        (0xad, Bool(b)) => Float(f32::from(u8::from(b))),
        (0xae, Char(c)) => Float(f32::from(c)),
        (0xaf, Int(i)) => Float(i as f32),
        (0xb0, Char(c)) => Bool(c != 0),
        (0xb1, Int(i)) => Bool(i != 0),
        (0xb2, Float(f)) => Bool(f != 0.),
        _ => return None,
    })
}

type BinaryResult = Result<Result<Value, ()>, Option<Fault>>;

fn binary(op: u8, a: Value, b: Value) -> BinaryResult {
    use Value::*;
    let ok = |v| Ok(Ok(v));
    match (op, a, b) {
        (0x20, Bool(a), Bool(b)) => ok(Bool(a == b)),
        (0x21, Bool(a), Bool(b)) => ok(Bool(a != b)),
        (0x24..=0x29, Char(a), Char(b)) => ok(Bool(cmp(op - 0x24, a, b))),
        (0x2a..=0x3c, Int(a), Int(b)) => Ok(int_op(op, a, b)),
        (0x3d..=0x42, Float(a), Float(b)) => ok(Float(match op {
            0x3d => a + b,
            0x3e => a - b,
            0x3f => a * b,
            0x40 => a / b,
            0x41 => a.min(b),
            _ => a.max(b),
        })),
        (0x43..=0x48, Float(a), Float(b)) => ok(Bool(cmp(op - 0x43, a, b))),
        (0x49, Vec4(a), Vec4(b)) => ok(Vec4(std::array::from_fn(|i| a[i] + b[i]))),
        (0x4a, Vec4(a), Vec4(b)) => ok(Vec4(std::array::from_fn(|i| a[i] - b[i]))),
        (0x4b, Vec4(a), Float(s)) => ok(Vec4(a.map(|x| x * s))),
        (0x4c, Vec4(a), Float(s)) => ok(Vec4(a.map(|x| x / s))),
        (0x4d, Vec4(a), Vec4(b)) => ok(Float((0..4).map(|i| a[i] * b[i]).sum())),
        (0x4e, Vec4(a), Vec4(b)) => ok(Float((0..3).map(|i| a[i] * b[i]).sum())),
        (0x4f, Vec4(a), Vec4(b)) => ok(Float((0..2).map(|i| a[i] * b[i]).sum())),
        (0x50, Vec4(a), Vec4(b)) => ok(Vec4([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
            0.,
        ])),
        (0x51, Mat44(a), Mat44(b)) => ok(Mat44(std::array::from_fn(|i| {
            let (r, c) = (i / 4, i % 4);
            (0..4).map(|k| a[r * 4 + k] * b[k * 4 + c]).sum()
        }))),
        // Pointer/object/s64 equality: EQrp EQo EQsp EQs64 and their NE forms.
        (0x54 | 0x56 | 0x58 | 0xc6, a, b) => ok(Bool(a == b)),
        (0x55 | 0x57 | 0x59 | 0xc7, a, b) => ok(Bool(a != b)),
        (0xbd, Bool(a), Bool(b)) => ok(Bool(a | b)),
        (0xbe, Bool(a), Bool(b)) => ok(Bool(a & b)),
        (0xbf, Bool(a), Bool(b)) => ok(Bool(a ^ b)),
        (0xc5, Long(a), Long(b)) => ok(Long(a.wrapping_add(b))),
        (0xc8, Long(a), Long(b)) => ok(Long(a | b)),
        (0xc9, Long(a), Long(b)) => ok(Long(a & b)),
        (0xca, Long(a), Long(b)) => ok(Long(a ^ b)),
        _ => Err(None),
    }
}

fn cmp<T: PartialOrd>(k: u8, a: T, b: T) -> bool {
    match k {
        0 => a < b,
        1 => a <= b,
        2 => a > b,
        3 => a >= b,
        4 => a == b,
        _ => a != b,
    }
}

fn int_op(op: u8, a: i32, b: i32) -> Result<Value, ()> {
    use Value::*;
    Ok(match op {
        0x2a => Int(a.wrapping_add(b)),
        0x2b => Int(a.wrapping_sub(b)),
        0x2c => Int(a.wrapping_mul(b)),
        0x2d => Int(a.checked_div(b).ok_or(())?),
        0x2e => Int(a.checked_rem(b).ok_or(())?),
        0x2f => Int(a.min(b)),
        0x30 => Int(a.max(b)),
        0x31 => Int(a.wrapping_shl(b as u32)),
        0x32 => Int(a.wrapping_shr(b as u32)),
        0x33 => Int((a as u32).wrapping_shr(b as u32) as i32),
        0x34 => Int(a | b),
        0x35 => Int(a & b),
        0x36 => Int(a ^ b),
        _ => Bool(cmp(op - 0x37, a, b)),
    })
}

fn get_builtin(op: u8, base: &Value) -> Option<Value> {
    use Value::*;
    Some(match (op, base) {
        (0x5a..=0x5d, Vec4(v)) => Float(v[usize::from(op - 0x5a)]),
        (0x5e..=0x60, Vec4(v)) => {
            let n = usize::from(op - 0x5e) + 2;
            Float(v[..n].iter().map(|x| x * x).sum::<f32>().sqrt())
        }
        (0x61..=0x70, Mat44(m)) => Float(m[usize::from(op - 0x61)]),
        (0x71..=0x74, Mat44(m)) => {
            let r = usize::from(op - 0x71);
            Vec4([m[r * 4], m[r * 4 + 1], m[r * 4 + 2], m[r * 4 + 3]])
        }
        (0x75..=0x78, Mat44(m)) => {
            let c = usize::from(op - 0x75);
            Vec4([m[c], m[4 + c], m[8 + c], m[12 + c]])
        }
        _ => return None,
    })
}

fn set_builtin(op: u8, base: Value, src: Value) -> Option<Value> {
    use Value::*;
    // A register never written reads as zeroed storage (INFERRED: v4/m44 locals are built
    // component by component; the PC U12 trace faulted on SET_V4_X of a fresh register).
    let base = match (op, base) {
        (0x79..=0x7c, Void) => Vec4([0.; 4]),
        (0x7d..=0x94, Void) => Mat44([0.; 16]),
        (_, b) => b,
    };
    Some(match (op, base, src) {
        (0x79..=0x7c, Vec4(mut v), Float(f)) => {
            v[usize::from(op - 0x79)] = f;
            Vec4(v)
        }
        (0x7d..=0x8c, Mat44(mut m), Float(f)) => {
            m[usize::from(op - 0x7d)] = f;
            Mat44(m)
        }
        (0x8d..=0x90, Mat44(mut m), Vec4(v)) => {
            let r = usize::from(op - 0x8d);
            m[r * 4..r * 4 + 4].copy_from_slice(&v);
            Mat44(m)
        }
        (0x91..=0x94, Mat44(mut m), Vec4(v)) => {
            let c = usize::from(op - 0x91);
            for (r, x) in v.iter().enumerate() {
                m[r * 4 + c] = *x;
            }
            Mat44(m)
        }
        _ => return None,
    })
}
