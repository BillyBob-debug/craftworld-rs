//! Rt: a level as the script VM's [`World`] (docs/runtime-research.md, item Rt;
//! docs/level-scripts.md). Separate from the parsing crates, which never depend on it.
//!
//! - [`LevelWorld`] builds the Thing tree from a read level: Thing handles are indices in
//!   the level's Thing list; each Thing with a PScript part gets its instance fields as
//!   script members (`GET/SET_SP_MEMBER` on that Thing), converted from the stored values.
//! - [`Runner`] runs a named function of each scripted Thing's class with the Thing as
//!   `this` in register 0 (the convention the test-script and Gooey hosts use, INFERRED for
//!   Thing scripts) and resumes yielded scripts once per tick.
//!
//! - [`LevelScripts`] is the per-tick schedule: [`CREATE`] once on every scripted Thing,
//!   then each tick resume suspended scripts and run [`UPDATE`] on the others.
//!
//! Entry names: `OnCreate__` and `Update__` are the create/update functions (PC D152:
//! across the 49 classes Rookie Test and the other levels use, `OnCreate__` is defined by
//! 24 and `Update__` by 11; the remaining names are UI, switch and enter/leave handlers).
use lbp_level::parts::{ScriptPart, ScriptValue, ShapeColor};
use lbp_level::thing::ThingRecord;
use lbp_script_vm::natives::World;
use lbp_script_vm::{ClassId, Host, MemberKind, Step, Task, Value, Vm};
use std::collections::{BTreeMap, BTreeSet};

/// One Thing as the runtime sees it.
#[derive(Debug, Clone, Default)]
pub struct ThingState {
    pub uid: Option<i32>,
    pub parent: Option<u32>,
    pub first_child: Option<u32>,
    pub next_sibling: Option<u32>,
    /// POS world matrix in stored order (translation at 12..14).
    pub world_matrix: Option<[f32; 16]>,
    pub parts: Vec<&'static str>,
    /// GUID of the PScript resource, when the Thing has one.
    pub script_guid: Option<u32>,
    /// Script members by field name; `None` when the Thing has no PScript layout.
    pub members: Option<BTreeMap<String, Value>>,
    /// PShape colour as a colour vector (`GetShapeEditorColour`).
    pub shape_colour: Option<[f32; 4]>,
}

/// An engine part a script reached through a native member (`thing.SpriteLight`), with
/// the `ref_ptr` members scripts wrote to it.
#[derive(Debug, Clone, Default)]
pub struct PartState {
    pub thing: u32,
    /// The reader's part name (`SPRITE_LIGHT`).
    pub part: &'static str,
    pub members: BTreeMap<String, Value>,
}

/// Packed PShape colour (head > 0x389) as a colour vector. INFERRED byte order
/// 0xAARRGGBB (white 0xffffffff is the common value; nothing yet tells R from B).
pub fn unpack_colour(c: u32) -> [f32; 4] {
    let ch = |shift: u32| ((c >> shift) & 0xff) as f32 / 255.;
    [ch(16), ch(8), ch(0), ch(24)]
}

/// A read level as the VM's world.
#[derive(Debug, Default)]
pub struct LevelWorld {
    pub things: Vec<ThingState>,
    pub world: Option<u32>,
    /// `WRITE` output of the scripts.
    pub written: Vec<String>,
    /// Stored script values with no VM value yet (object references other than strings).
    pub unconverted: usize,
    /// Parts handed to scripts; a part handle is its index + 1 (`Raw(0)` is null).
    pub part_handles: Vec<PartState>,
}

/// VM value for a stored script field; `None` for a field the instance does not store.
/// Object references other than strings become null and are counted by the caller.
pub fn member_value(v: &ScriptValue) -> Option<(Value, bool)> {
    Some(match v {
        ScriptValue::Bool(b) => (Value::Bool(*b), true),
        ScriptValue::Char(c) => (Value::Char(*c as u16), true),
        ScriptValue::Int(i) => (Value::Int(*i), true),
        ScriptValue::Float(f) => (Value::Float(*f), true),
        ScriptValue::V4(v) => (Value::Vec4(*v), true),
        ScriptValue::M44(m) => (Value::Mat44(*m), true),
        ScriptValue::Thing(t) => (Value::Thing(t.and_then(|i| u32::try_from(i).ok())), true),
        ScriptValue::Object { kind: 0, .. } => (Value::Object(None), true),
        ScriptValue::Object { text: Some(t), .. } => (Value::Str(t.as_str().into()), true),
        ScriptValue::Object { .. } => (Value::Object(None), false),
        ScriptValue::NotStored => return None,
    })
}

/// `"PSpriteLight"` -> `"SPRITE_LIGHT"`, the reader's part name.
fn part_name(native: &str) -> String {
    let base = native.strip_prefix('P').unwrap_or(native);
    let mut out = String::new();
    for (i, c) in base.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

impl LevelWorld {
    pub fn from_things(list: &[ThingRecord]) -> Self {
        let handle = |i: usize| u32::try_from(i).ok();
        let mut w = LevelWorld {
            things: list
                .iter()
                .map(|t| ThingState {
                    uid: t.uid,
                    parent: t.parent.and_then(handle),
                    world_matrix: t.data.pos.as_ref().map(|p| p.world),
                    parts: t.parts.clone(),
                    shape_colour: t.data.shape.as_ref().map(|s| match s.color {
                        ShapeColor::Packed(c) => unpack_colour(c),
                        ShapeColor::Vector(v) => v,
                    }),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        // Children in Thing-list order.
        let mut last_child: Vec<Option<u32>> = vec![None; list.len()];
        for i in 0..list.len() {
            let (Some(me), Some(p)) = (handle(i), w.things[i].parent) else {
                continue;
            };
            let Some(pi) = usize::try_from(p).ok().filter(|&p| p < list.len()) else {
                continue;
            };
            match last_child[pi] {
                None => w.things[pi].first_child = Some(me),
                Some(prev) => w.things[prev as usize].next_sibling = Some(me),
            }
            last_child[pi] = Some(me);
        }
        w.world = list
            .iter()
            .position(|t| t.data.world.is_some())
            .and_then(handle);
        for (i, t) in list.iter().enumerate() {
            if let Some(part) = &t.data.script {
                w.add_script(i, part);
            }
        }
        w
    }

    fn add_script(&mut self, i: usize, part: &ScriptPart) {
        let state = &mut self.things[i];
        state.script_guid = part.script.and_then(|d| d.guid);
        if part.fields.is_empty() {
            return;
        }
        let mut members = BTreeMap::new();
        for f in &part.fields {
            if let Some((v, converted)) = member_value(&f.value) {
                self.unconverted += usize::from(!converted);
                members.insert(f.name.clone(), v);
            }
        }
        state.members = Some(members);
    }

    /// Gives every scripted Thing whose class is loaded the fields of its class and
    /// superclasses that the instance does not store, at their type defaults. The stored
    /// layout only carries serialized fields: a superclass field such as
    /// `Tweakable.IsTweaking` (read by `Trigger_Speech_Bubble.Update`) is never stored but
    /// still exists on the instance (INFERRED, the way `instantiate` builds script objects).
    pub fn add_class_defaults(&mut self, vm: &Vm) {
        for t in &mut self.things {
            let Some(class) = t.script_guid.and_then(|g| vm.class_by_guid(g)) else {
                continue;
            };
            let Ok(defaults) = vm.field_defaults(class) else {
                continue;
            };
            let members = t.members.get_or_insert_with(BTreeMap::new);
            for (name, v) in defaults {
                members.entry(name).or_insert(v);
            }
        }
    }

    /// Handle for a part a Thing carries (index + 1), created on first use.
    fn part_handle(&mut self, thing: u32, part: &'static str) -> u32 {
        if let Some(i) = self
            .part_handles
            .iter()
            .position(|p| p.thing == thing && p.part == part)
        {
            return i as u32 + 1;
        }
        self.part_handles.push(PartState {
            thing,
            part,
            members: BTreeMap::new(),
        });
        self.part_handles.len() as u32
    }

    fn part_of(&mut self, base: &Value) -> Result<&mut PartState, String> {
        match base {
            Value::Raw(0) => Err("null part".into()),
            Value::Raw(h) => self
                .part_handles
                .get_mut(*h as usize - 1)
                .ok_or_else(|| format!("no part handle {h}")),
            other => Err(format!("member base is not a part: {other:?}")),
        }
    }

    /// `thing.SpriteLight` and the like: the part as a raw pointer, null when the Thing
    /// lacks it. Only names of the reader's part list are parts.
    fn native_member(&mut self, base: &Value, class: &str, field: &str) -> Result<Value, String> {
        let want = part_name(field);
        let Some(&(part, ..)) = lbp_level::thing::PARTS.iter().find(|p| p.0 == want) else {
            return Err(format!(
                "world does not provide native member {class}.{field}"
            ));
        };
        let t = Self::thing_of(base)?;
        Ok(if self.state(t)?.parts.contains(&part) {
            Value::Raw(self.part_handle(t, part))
        } else {
            Value::Raw(0)
        })
    }

    fn state(&self, thing: u32) -> Result<&ThingState, String> {
        self.things
            .get(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))
    }

    fn thing_of(base: &Value) -> Result<u32, String> {
        match base {
            Value::Thing(Some(t)) => Ok(*t),
            Value::Thing(None) => Err("null thing".into()),
            other => Err(format!("member base is not a thing: {other:?}")),
        }
    }
}

impl World for LevelWorld {
    fn thing_uid(&mut self, thing: u32) -> Result<i32, String> {
        self.state(thing)?
            .uid
            .ok_or_else(|| format!("thing {thing} has no UID"))
    }
    fn parent(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.parent)
    }
    fn first_child(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.first_child)
    }
    fn next_sibling(&mut self, thing: u32) -> Result<Option<u32>, String> {
        Ok(self.state(thing)?.next_sibling)
    }
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Ok(self.world)
    }
    /// Translation column of the POS world matrix (INFERRED: w as stored, normally 1).
    fn world_position(&mut self, thing: u32) -> Result<[f32; 4], String> {
        let m = self
            .state(thing)?
            .world_matrix
            .ok_or_else(|| format!("thing {thing} has no position"))?;
        Ok([m[12], m[13], m[14], m[15]])
    }
    fn has_part(&mut self, thing: u32, part: &str) -> Result<bool, String> {
        let want = part_name(part);
        Ok(self.state(thing)?.parts.iter().any(|p| *p == want))
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        match kind {
            // Through `Thing` or a script class of a Thing (`World.World`).
            MemberKind::Native => return self.native_member(base, class, field),
            MemberKind::RefPtr => {
                let p = self.part_of(base)?;
                return p
                    .members
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("{} member {class}.{field} not read yet", p.part));
            }
            MemberKind::Thing => {}
        }
        let t = Self::thing_of(base)?;
        self.state(t)?
            .members
            .as_ref()
            .and_then(|m| m.get(field))
            .cloned()
            .ok_or_else(|| format!("thing {t} has no script member {class}.{field}"))
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        if kind == MemberKind::RefPtr {
            self.part_of(base)?.members.insert(field.to_owned(), value);
            return Ok(());
        }
        if kind != MemberKind::Thing {
            return Err(format!(
                "world does not provide {kind:?} member {class}.{field}"
            ));
        }
        let t = Self::thing_of(base)?;
        let state = self
            .things
            .get_mut(t as usize)
            .ok_or_else(|| format!("no thing {t}"))?;
        state
            .members
            .get_or_insert_with(BTreeMap::new)
            .insert(field.to_owned(), value);
        Ok(())
    }
    /// One local player (the PC run has no controllers yet).
    fn num_players(&mut self) -> Result<i32, String> {
        Ok(1)
    }
    /// Play mode (0); levels run as played, not edited.
    fn play_or_edit_mode(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    /// No dialogue list is loaded (0).
    fn dialogue_list(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn shape_editor_colour(&mut self, thing: u32) -> Result<[f32; 4], String> {
        self.state(thing)?
            .shape_colour
            .ok_or_else(|| format!("thing {thing} has no shape"))
    }
    fn set_shape_editor_colour(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        let t = self
            .things
            .get_mut(thing as usize)
            .ok_or_else(|| format!("no thing {thing}"))?;
        if t.shape_colour.is_none() {
            return Err(format!("thing {thing} has no shape"));
        }
        t.shape_colour = Some(colour);
        Ok(())
    }
    fn write(&mut self, text: &str) {
        if self.written.len() < 1000 {
            self.written.push(text.to_owned());
        }
    }
}

/// A scripted Thing and its class, when the class is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instance {
    pub thing: u32,
    pub guid: u32,
    pub class: Option<ClassId>,
}

/// Every Thing with a PScript resource GUID, in Thing order.
pub fn instances(vm: &Vm, world: &LevelWorld) -> Vec<Instance> {
    world
        .things
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let guid = t.script_guid?;
            Some(Instance {
                thing: u32::try_from(i).ok()?,
                guid,
                class: vm.class_by_guid(guid),
            })
        })
        .collect()
}

/// Result of starting or resuming one script.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The class (or its superclasses) has no code for the function.
    NoFunction,
    /// The script's class is not loaded.
    NoClass,
    Done,
    Yielded,
    Fault(String),
}

/// Runs entry functions on scripted Things and keeps yielded ones for later ticks.
#[derive(Default)]
pub struct Runner {
    tasks: Vec<(u32, Task)>,
}

impl Runner {
    /// Scripts suspended by `System.Yield`, waiting for the next tick.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Things with a suspended script.
    pub fn pending_things(&self) -> BTreeSet<u32> {
        self.tasks.iter().map(|(t, _)| *t).collect()
    }

    /// Runs `function` on one instance with the Thing as `this` in register 0.
    pub fn start(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        budget: &mut u64,
    ) -> Outcome {
        let Some(class) = inst.class else {
            return Outcome::NoClass;
        };
        if !vm.has_code(class, function) {
            return Outcome::NoFunction;
        }
        let step = vm.start_registers(
            host,
            class,
            function,
            &[(0, Value::Thing(Some(inst.thing)))],
            budget,
        );
        self.settle(vm, inst.thing, step)
    }

    /// Resumes every yielded script once, in the order they yielded.
    pub fn tick(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        budget: &mut u64,
    ) -> Vec<(u32, Outcome)> {
        let tasks = std::mem::take(&mut self.tasks);
        let mut out = Vec::with_capacity(tasks.len());
        for (thing, task) in tasks {
            let step = vm.resume(host, task, budget);
            out.push((thing, self.settle(vm, thing, step)));
        }
        out
    }

    fn settle(&mut self, vm: &Vm, thing: u32, step: Result<Step, lbp_script_vm::Fault>) -> Outcome {
        match step {
            Ok(Step::Done(_)) => Outcome::Done,
            Ok(Step::Yielded(task)) => {
                self.tasks.push((thing, task));
                Outcome::Yielded
            }
            Err(f) => {
                let site = vm.last_site().map_or(String::new(), |s| s.to_string());
                Outcome::Fault(format!("{f:?} at {site}"))
            }
        }
    }
}

/// Function the game runs once when a scripted Thing is created.
pub const CREATE: &str = "OnCreate__";
/// Function the game runs on every scripted Thing each tick.
pub const UPDATE: &str = "Update__";
/// Game logic rate (X1, CONFIRMED_FROM_LOCAL_DATA in RPCS3: 30 ticks per second).
pub const TICK_HZ: u32 = 30;

/// Outcomes of one schedule step, by kind, with the faults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepReport {
    pub done: usize,
    pub yielded: usize,
    pub resumed: usize,
    pub no_function: usize,
    pub no_class: usize,
    pub faults: Vec<(u32, String)>,
}

impl StepReport {
    fn add(&mut self, thing: u32, o: Outcome) {
        match o {
            Outcome::Done => self.done += 1,
            Outcome::Yielded => self.yielded += 1,
            Outcome::NoFunction => self.no_function += 1,
            Outcome::NoClass => self.no_class += 1,
            Outcome::Fault(f) => self.faults.push((thing, f)),
        }
    }
}

/// Per-tick script schedule for one level.
///
/// INFERRED (docs/level-scripts.md): `create` runs [`CREATE`] on every instance in Thing
/// order; each `tick` first resumes scripts suspended by `System.Yield` (in the order
/// they yielded), then runs [`UPDATE`] on every instance that was not suspended at the
/// start of the tick, in Thing order. A Thing whose script faults is not run again.
pub struct LevelScripts {
    pub instances: Vec<Instance>,
    runner: Runner,
    /// Things stopped by a fault.
    pub stopped: BTreeSet<u32>,
    /// Ticks run so far.
    pub ticks: u64,
}

impl LevelScripts {
    pub fn new(vm: &Vm, world: &LevelWorld) -> Self {
        Self {
            instances: instances(vm, world),
            runner: Runner::default(),
            stopped: BTreeSet::new(),
            ticks: 0,
        }
    }

    /// Scripts suspended by `System.Yield`.
    pub fn pending(&self) -> usize {
        self.runner.pending()
    }

    /// Runs [`CREATE`] on every instance; `budget` is per call.
    pub fn create(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        self.run_all(vm, host, CREATE, budget, &BTreeSet::new())
    }

    /// One game tick; `budget` is per call or resume.
    pub fn tick(&mut self, vm: &mut Vm, host: &mut dyn Host, budget: u64) -> StepReport {
        // A Thing suspended at the start of the tick is resumed, not updated again.
        let busy = self.runner.pending_things();
        let mut resume_budget = budget.saturating_mul(self.runner.pending() as u64);
        let mut report = StepReport::default();
        for (thing, o) in self.runner.tick(vm, host, &mut resume_budget) {
            if matches!(o, Outcome::Done | Outcome::Yielded) {
                report.resumed += 1;
            }
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(thing);
                report.add(thing, o);
            }
        }
        let update = self.run_all(vm, host, UPDATE, budget, &busy);
        report.done += update.done;
        report.yielded += update.yielded;
        report.no_function += update.no_function;
        report.no_class += update.no_class;
        report.faults.extend(update.faults);
        self.ticks += 1;
        report
    }

    fn run_all(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        function: &str,
        budget: u64,
        skip: &BTreeSet<u32>,
    ) -> StepReport {
        let mut report = StepReport::default();
        for inst in self.instances.clone() {
            if skip.contains(&inst.thing) || self.stopped.contains(&inst.thing) {
                continue;
            }
            let mut b = budget;
            let o = self.runner.start(vm, host, inst, function, &mut b);
            if matches!(o, Outcome::Fault(_)) {
                self.stopped.insert(inst.thing);
            }
            report.add(inst.thing, o);
        }
        report
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn part_names() {
        assert_eq!(super::part_name("PSpriteLight"), "SPRITE_LIGHT");
        assert_eq!(super::part_name("PJoint"), "JOINT");
        assert_eq!(super::part_name("PSwitch"), "SWITCH");
    }
}
