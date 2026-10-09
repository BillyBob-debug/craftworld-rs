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
use lbp_level::parts::{ScriptObject, ScriptPart, ScriptValue, ShapeColor};
use lbp_level::thing::ThingRecord;
use lbp_script_vm::natives::{Bindings, CameraFromToUp, ResourceKind, World};
use lbp_script_vm::{ClassId, Host, MemberKind, ObjectId, Step, Task, Value, Vm};
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
    /// Render-mesh editor colour tint a script set (`SetRenderMeshEditorColourTint`).
    pub render_mesh_tint: Option<[f32; 4]>,
    /// Animation playback rate a script set (`Native.SetAnimSpeed`, INFERRED multiplier).
    pub anim_speed: Option<f32>,
    /// Animation position a script set (`Native.SetAnimPos`, INFERRED time or fraction).
    pub anim_pos: Option<f32>,
}

/// Camera state scripts set through `Cam.*` natives, for the viewer to read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CameraState {
    /// The last `Cam.SetCameraFromToUp__rrrffff`.
    pub from_to_up: Option<CameraFromToUp>,
    pub from_to_up_calls: u64,
    /// The last `Cam.SetDOFAndFog__fff` (as passed).
    pub dof_and_fog: Option<[f32; 3]>,
    pub motion_blur_clears: u64,
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
    /// Stored non-null object references with no VM value: before [`Self::bind_objects`]
    /// every member that is not a string; after it, references (members, array elements,
    /// instance fields) whose object could not be built (resource, audio handle, instance
    /// of a class the VM has not loaded).
    pub unconverted: usize,
    /// Script object contents by the level's object number (first occurrence).
    pub stored_objects: BTreeMap<usize, ScriptObject>,
    /// Members waiting for a VM object: (Thing, field, object number).
    pub object_members: Vec<(u32, String, usize)>,
    /// Parts handed to scripts; a part handle is its index + 1 (`Raw(0)` is null).
    pub part_handles: Vec<PartState>,
    pub camera: CameraState,
    /// The PodComputer script object `GetPodComputer__` returns (see [`add_engine_objects`]).
    pub pod_computer: Option<ObjectId>,
    /// Gooey objects made ahead for `CreateGooey__i` (the host cannot allocate VM objects
    /// during a native call); each call hands out the next one.
    pub gooey_pool: Vec<ObjectId>,
    pub gooeys_created: usize,
    /// Interactive-music commands scripts sent, by name.
    pub audio_commands: BTreeMap<String, u64>,
    /// LBP2 game progression flags set on the profile (`IsLBP2GameProgressionFlagSet__i`).
    /// Empty = a fresh profile, on which PlanetsWorld starts its intro cutscene thread.
    pub progression_flags: BTreeSet<i32>,
    /// The Planets object `PodComputer.GetPlanets__` returns.
    pub planets: Option<ObjectId>,
    /// Planet objects by `GetCrafteroid__i` index - 1, with their Thing.
    pub crafteroids: Vec<(ObjectId, Option<u32>)>,
    /// `Game.SetPlayerScoreMultiplierTimer` by player number.
    pub score_multiplier_timers: BTreeMap<i32, i32>,
}

/// VM value for a stored script field; `None` for a field the instance does not store.
/// Object references other than strings with their text at hand become null and are
/// counted by the caller ([`LevelWorld::bind_objects`] builds them).
pub fn member_value(v: &ScriptValue) -> Option<(Value, bool)> {
    stored_value(v, &|_| None)
}

/// [`member_value`] with `object` giving the VM value for an object number.
fn stored_value(v: &ScriptValue, object: &dyn Fn(usize) -> Option<Value>) -> Option<(Value, bool)> {
    Some(match v {
        ScriptValue::Bool(b) => (Value::Bool(*b), true),
        ScriptValue::Char(c) => (Value::Char(*c as u16), true),
        ScriptValue::Int(i) => (Value::Int(*i), true),
        ScriptValue::Float(f) => (Value::Float(*f), true),
        ScriptValue::V4(v) => (Value::Vec4(*v), true),
        ScriptValue::M44(m) => (Value::Mat44(*m), true),
        ScriptValue::Thing(t) => (Value::Thing(t.and_then(|i| u32::try_from(i).ok())), true),
        ScriptValue::Object { kind: 0, .. } => (Value::Object(None), true),
        ScriptValue::Object { id, body, .. } => match (body.as_deref(), id.and_then(object)) {
            (Some(ScriptObject::Text(t)), _) => (Value::Str(t.as_str().into()), true),
            (_, Some(v)) => (v, true),
            _ => (Value::Object(None), false),
        },
        ScriptValue::NotStored => return None,
    })
}

/// Object contents by number, from a stored value and everything nested in it.
fn collect_objects(v: &ScriptValue, out: &mut BTreeMap<usize, ScriptObject>) {
    let ScriptValue::Object {
        id: Some(id),
        body: Some(body),
        ..
    } = v
    else {
        return;
    };
    match &**body {
        ScriptObject::Objects(items) => items.iter().for_each(|i| collect_objects(i, out)),
        ScriptObject::Instance(p) => p.fields.iter().for_each(|f| collect_objects(&f.value, out)),
        _ => {}
    }
    out.entry(*id).or_insert_with(|| (**body).clone());
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
        for t in list {
            if let Some(part) = &t.data.script {
                for f in &part.fields {
                    collect_objects(&f.value, &mut w.stored_objects);
                }
            }
        }
        for (i, t) in list.iter().enumerate() {
            if let Some(part) = &t.data.script {
                w.add_script(i, part);
            }
        }
        w
    }

    fn add_script(&mut self, i: usize, part: &ScriptPart) {
        let stored = &self.stored_objects;
        let text = |id: usize| match stored.get(&id) {
            Some(ScriptObject::Text(t)) => Some(Value::Str(t.as_str().into())),
            _ => None,
        };
        let mut members = BTreeMap::new();
        for f in &part.fields {
            if let Some((v, converted)) = stored_value(&f.value, &text) {
                if !converted {
                    self.unconverted += 1;
                    if let ScriptValue::Object { id: Some(id), .. } = f.value {
                        self.object_members.push((i as u32, f.name.clone(), id));
                    }
                }
                members.insert(f.name.clone(), v);
            }
        }
        let state = &mut self.things[i];
        state.script_guid = part.script.and_then(|d| d.guid);
        if !part.fields.is_empty() {
            state.members = Some(members);
        }
    }

    /// Builds the level's stored script objects on the VM heap and points the members that
    /// reference them at those objects. Arrays become script arrays; an instance becomes
    /// an instance of the class with its script GUID (class defaults, then the stored
    /// fields); strings stay string values. Objects with one number are one heap object, so
    /// shared references stay shared. Resources, audio handles and instances of classes
    /// the VM has not loaded stay null and are counted in `unconverted`. Returns the
    /// number of heap objects created.
    pub fn bind_objects(&mut self, vm: &mut Vm) -> usize {
        let thing = |t: &Option<usize>| Value::Thing(t.and_then(|i| u32::try_from(i).ok()));
        let mut built = BTreeMap::<usize, Value>::new();
        let mut created = 0;
        // Allocate first so references in any order (and cycles) resolve.
        for (&id, body) in &self.stored_objects {
            let made = match body {
                ScriptObject::Text(t) => {
                    built.insert(id, Value::Str(t.as_str().into()));
                    continue;
                }
                ScriptObject::Bools(v) => vm.new_array(v.iter().map(|&b| Value::Bool(b)).collect()),
                ScriptObject::Ints(v) => vm.new_array(v.iter().map(|&i| Value::Int(i)).collect()),
                ScriptObject::Floats(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Float(f)).collect())
                }
                ScriptObject::Vectors(v) => {
                    vm.new_array(v.iter().map(|&f| Value::Vec4(f)).collect())
                }
                ScriptObject::Things(v) => vm.new_array(v.iter().map(thing).collect()),
                ScriptObject::Objects(_) => vm.new_array(Vec::new()),
                ScriptObject::Instance(part) => {
                    match part
                        .script
                        .and_then(|d| d.guid)
                        .and_then(|g| vm.class_by_guid(g))
                    {
                        Some(class) => vm.instantiate(class),
                        None => continue,
                    }
                }
                ScriptObject::Resource { .. } | ScriptObject::AudioHandle => continue,
            };
            if let Ok(o) = made {
                built.insert(id, Value::Object(Some(o)));
                created += 1;
            }
        }
        let object = |i: usize| built.get(&i).cloned();
        let mut unconverted = 0;
        for (id, body) in &self.stored_objects {
            let Some(Value::Object(Some(o))) = built.get(id) else {
                continue;
            };
            match body {
                ScriptObject::Objects(items) => {
                    let values: Vec<Value> = items
                        .iter()
                        .map(|v| {
                            let (v, ok) =
                                stored_value(v, &object).unwrap_or((Value::Object(None), true));
                            unconverted += usize::from(!ok);
                            v
                        })
                        .collect();
                    if let Some(a) = vm.array_mut(*o) {
                        *a = values;
                    }
                }
                ScriptObject::Instance(part) => {
                    let mut stored = Vec::new();
                    for f in &part.fields {
                        if let Some((v, ok)) = stored_value(&f.value, &object) {
                            unconverted += usize::from(!ok);
                            stored.push((f.name.clone(), v));
                        }
                    }
                    if let Some(fields) = vm.fields_mut(*o) {
                        fields.extend(stored);
                    }
                }
                _ => {}
            }
        }
        for (t, name, id) in std::mem::take(&mut self.object_members) {
            match object(id) {
                Some(v) => {
                    if let Some(m) = self.things[t as usize].members.as_mut() {
                        m.insert(name, v);
                    }
                }
                None => unconverted += 1,
            }
        }
        self.unconverted = unconverted;
        created
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

    fn thing_mut(&mut self, thing: u32) -> Result<&mut ThingState, String> {
        self.things
            .get_mut(thing as usize)
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
    /// Recorded on the Thing; nothing draws it yet. Any Thing may be tinted (scripts call
    /// it on their own Thing without a `HasPRenderMesh` check).
    fn set_render_mesh_tint(&mut self, thing: u32, colour: [f32; 4]) -> Result<(), String> {
        self.thing_mut(thing)?.render_mesh_tint = Some(colour);
        Ok(())
    }
    /// Recorded on the Thing; nothing animates by it yet.
    fn set_anim_speed(&mut self, thing: u32, speed: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_speed = Some(speed);
        Ok(())
    }
    /// Player `index` is `Game.EPlayerNumber` `index` (INFERRED: no enum values are in the
    /// script tables); outside `0..num_players()` is an error.
    fn player_number(&mut self, index: i32) -> Result<i32, String> {
        let n = self.num_players()?;
        if (0..n).contains(&index) {
            Ok(index)
        } else {
            Err(format!("player index {index} outside 0..{n}"))
        }
    }
    /// The handle is the GUID itself (INFERRED stand-in: nothing is loaded; scripts only
    /// keep the handle and pass it back to natives).
    fn load_resource(&mut self, _kind: ResourceKind, guid: u32) -> Result<u32, String> {
        Ok(guid)
    }
    fn set_world_transform(&mut self, thing: u32, m: [f32; 16]) -> Result<(), String> {
        self.thing_mut(thing)?.world_matrix = Some(m);
        Ok(())
    }
    fn set_anim_pos(&mut self, thing: u32, pos: f32) -> Result<(), String> {
        self.thing_mut(thing)?.anim_pos = Some(pos);
        Ok(())
    }
    fn pod_computer(&mut self) -> Result<Value, String> {
        self.pod_computer
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no PodComputer object (class not loaded)".into())
    }
    fn planets(&mut self) -> Result<Value, String> {
        self.planets
            .map(|o| Value::Object(Some(o)))
            .ok_or_else(|| "no Planets object (class not loaded)".into())
    }
    fn crafteroid(&mut self, index: i32) -> Result<Value, String> {
        usize::try_from(index)
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.crafteroids.get(i))
            .map(|(o, _)| Value::Object(Some(*o)))
            .ok_or_else(|| format!("no crafteroid {index}"))
    }
    fn planet_thing(&mut self, planet: &Value) -> Result<Option<u32>, String> {
        match planet {
            Value::Object(Some(o)) => self
                .crafteroids
                .iter()
                .find(|(p, _)| p == o)
                .map(|(_, t)| *t)
                .ok_or_else(|| format!("object {o:?} is not a planet")),
            other => Err(format!("expected a Planet, got {other:?}")),
        }
    }
    /// INFERRED defaults for the PC run: interactive, no pending links, no avatars spawned
    /// (there is no Sackboy yet), no video, a 16:9 screen, player 0 leads, no music track
    /// (GUID 0), no user music (stopped, not paused), interactive music audible, no
    /// attraction (demo) loop.
    fn state_query(&mut self, native: &str) -> Result<Value, String> {
        Ok(match native {
            "Game.IsNonInteractive__"
            | "Video.IsVideoPlaying__"
            | "System.Is4to3__"
            | "Audio.UserMusicIsPaused__"
            | "Input.IsInAttractionLoop__i" => Value::Bool(false),
            "Audio.UserMusicIsStopped__" | "Audio.IsInteractiveMusicAudible__" => Value::Bool(true),
            "Game.GetNumPendingLinks__"
            | "Game.GetNumAvatars__"
            | "Game.GetLocalLeadersPlayerNumber__"
            | "Audio.GetCurrMusicGuid__" => Value::Int(0),
            _ => return Err(format!("world does not provide {native}")),
        })
    }
    /// Counted by name; nothing plays.
    fn audio_command(&mut self, name: &str, _args: &[Value]) -> Result<(), String> {
        *self.audio_commands.entry(name.to_owned()).or_default() += 1;
        Ok(())
    }
    /// Flags in [`LevelWorld::progression_flags`]; empty is a fresh profile (default).
    fn progression_flag(&mut self, flag: i32) -> Result<bool, String> {
        Ok(self.progression_flags.contains(&flag))
    }
    /// Nobody is at the pod computer in the PC run (INFERRED default).
    fn pod_computer_in_use(&mut self) -> Result<bool, String> {
        Ok(false)
    }
    /// No crash zoom in the PC run (INFERRED default).
    fn crash_zoom(&mut self) -> Result<bool, String> {
        Ok(false)
    }
    fn create_gooey(&mut self, _arg: i32) -> Result<Value, String> {
        let o = self
            .gooey_pool
            .get(self.gooeys_created)
            .copied()
            .ok_or_else(|| format!("Gooey pool of {} used up", self.gooey_pool.len()))?;
        self.gooeys_created += 1;
        Ok(Value::Object(Some(o)))
    }
    fn set_camera(&mut self, camera: CameraFromToUp) -> Result<(), String> {
        self.camera.from_to_up = Some(camera);
        self.camera.from_to_up_calls += 1;
        Ok(())
    }
    fn set_dof_and_fog(&mut self, values: [f32; 3]) -> Result<(), String> {
        self.camera.dof_and_fog = Some(values);
        Ok(())
    }
    fn clear_motion_blur(&mut self) -> Result<(), String> {
        self.camera.motion_blur_clears += 1;
        Ok(())
    }
    /// No cinema mode in the PC run (INFERRED default).
    fn cinema_mode_count(&mut self) -> Result<i32, String> {
        Ok(0)
    }
    fn set_score_multiplier_timer(&mut self, player: i32, timer: i32) -> Result<(), String> {
        let n = self.num_players()?;
        if !(0..n).contains(&player) {
            return Err(format!("player {player} outside 0..{n}"));
        }
        self.score_multiplier_timers.insert(player, timer);
        Ok(())
    }
    fn write(&mut self, text: &str) {
        if self.written.len() < 1000 {
            self.written.push(text.to_owned());
        }
    }
}

/// Gooey objects made ahead for `Gooey.CreateGooey__i`.
pub const GOOEY_POOL: usize = 16;

/// Runs `function` (`.init__`, `.ctor__`) on a heap object with it as `this` in register 0;
/// a fault comes back as text with its site.
fn run_on_object(
    vm: &mut Vm,
    host: &mut dyn Host,
    class: ClassId,
    object: ObjectId,
    function: &str,
    budget: u64,
) -> Result<(), String> {
    if !vm.has_code(class, function) {
        return Ok(());
    }
    let mut b = budget;
    let regs = [(0, Value::Object(Some(object)))];
    match vm.start_registers(host, class, function, &regs, &mut b) {
        Ok(Step::Done(_)) => Ok(()),
        Ok(Step::Yielded(_)) => Err(format!("{function} yielded")),
        Err(f) => {
            let site = vm.last_site().map_or(String::new(), |s| s.to_string());
            Err(format!("{function}: {f:?} at {site}"))
        }
    }
}

/// Makes the engine-owned script objects natives hand out: [`GOOEY_POOL`] Gooey objects
/// (`.init__`) and the PodComputer (`.init__`, then `.ctor__`). INFERRED: the engine holds
/// one PodComputer and creates Gooeys on demand. The PodComputer is registered before its
/// `.init__` runs, because that code already asks for it (`GetPodComputer__`). Returns the
/// faults, if any; objects whose `.init__`/`.ctor__` faulted are still kept.
pub fn add_engine_objects(
    vm: &mut Vm,
    host: &mut Bindings<LevelWorld>,
    budget: u64,
) -> Vec<String> {
    let mut faults = Vec::new();
    let make = |vm: &mut Vm, host: &mut Bindings<LevelWorld>, class, pod: bool| {
        let o = vm.instantiate(class).map_err(|f| format!("{f:?}"))?;
        if pod {
            host.world.pod_computer = Some(o);
        } else {
            host.world.gooey_pool.push(o);
        }
        run_on_object(vm, host, class, o, ".init__", budget)?;
        if pod {
            run_on_object(vm, host, class, o, ".ctor__", budget)?;
        }
        Ok::<(), String>(())
    };
    if let Some(class) = vm.class_by_name("Gooey") {
        for _ in 0..GOOEY_POOL {
            if let Err(e) = make(vm, host, class, false) {
                faults.push(format!("Gooey {e}"));
            }
        }
    }
    // Planets and its crafteroids: INFERRED index 1 = the level's Earth (a PlanetThing),
    // 2 = its OrbitingMoon, from PlanetsView, which reads crafteroids 1 and 2 and returns
    // early when either Thing is null.
    if let Some(class) = vm.class_by_name("Planets") {
        match vm.instantiate(class) {
            Ok(o) => {
                host.world.planets = Some(o);
                if let Err(e) = run_on_object(vm, host, class, o, ".init__", budget) {
                    faults.push(format!("Planets {e}"));
                }
            }
            Err(f) => faults.push(format!("Planets {f:?}")),
        }
    }
    if let Some(class) = vm.class_by_name("Planet") {
        for name in ["Earth", "OrbitingMoon"] {
            let thing = host.world.things.iter().position(|t| {
                t.script_guid
                    .and_then(|g| vm.class_by_guid(g))
                    .is_some_and(|c| vm.class_name(c) == name)
            });
            match vm.instantiate(class) {
                Ok(o) => host
                    .world
                    .crafteroids
                    .push((o, thing.and_then(|t| u32::try_from(t).ok()))),
                Err(f) => faults.push(format!("Planet {f:?}")),
            }
        }
    }
    if let Some(class) = vm.class_by_name("PodComputer") {
        if let Err(e) = make(vm, host, class, true) {
            faults.push(format!("PodComputer {e}"));
        }
    }
    faults.dedup();
    faults
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
        self.start_with(vm, host, inst, function, &[], budget)
    }

    /// [`Runner::start`] with argument registers (byte offsets) staged after `this`.
    pub fn start_with(
        &mut self,
        vm: &mut Vm,
        host: &mut dyn Host,
        inst: Instance,
        function: &str,
        args: &[(u16, Value)],
        budget: &mut u64,
    ) -> Outcome {
        let Some(class) = inst.class else {
            return Outcome::NoClass;
        };
        if !vm.has_code(class, function) {
            return Outcome::NoFunction;
        }
        let mut regs = vec![(0, Value::Thing(Some(inst.thing)))];
        regs.extend_from_slice(args);
        let step = vm.start_registers(host, class, function, &regs, budget);
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

/// What a [`FrameEntry`] builds its arguments from.
pub struct FrameContext<'a> {
    pub vm: &'a Vm,
    pub instances: &'a [Instance],
    /// Ticks run before this one.
    pub tick: u64,
}

/// A per-tick function the engine calls with arguments on instances of one class, in
/// place of [`UPDATE`] (an `export` entry no script calls).
pub struct FrameEntry {
    /// Exact script class name of the instance.
    pub class: &'static str,
    pub function: &'static str,
    /// Argument registers (byte offsets) and values; `this` is register 0.
    pub args: fn(&FrameContext<'_>) -> Vec<(u16, Value)>,
}

/// PlanetsWorld (the Pod's planet view): no script calls the export
/// `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb`, and it is the only caller of
/// `UpdateState__` (private), `UpdatePodCamera__Q11PlanetThingbb` and
/// `UpdateInteractiveMusic__`, so the engine calls it each frame (INFERRED from the
/// listings; `OnEnterPod__`/`OnExitPod__` are called by PodComputer on pod entry/exit).
/// Arguments (INFERRED from how the function uses them): +4 bool, passed on to
/// `UpdatePodCamera` (false); +8 the Earth (a PlanetThing; it gets `EarthSpinOffset`); +12 the
/// moon (`MoonSpinOffset`); +16 bool "snap the view" (true on the first tick only); +17
/// bool "take the zoom from the pod computer's state" (true).
fn planets_view_args(cx: &FrameContext<'_>) -> Vec<(u16, Value)> {
    let find = |name: &str| {
        cx.instances
            .iter()
            .find(|i| i.class.is_some_and(|c| cx.vm.class_name(c) == name))
            .map(|i| i.thing)
    };
    vec![
        (4, Value::Bool(false)),
        (8, Value::Thing(find("Earth"))),
        (12, Value::Thing(find("OrbitingMoon"))),
        (16, Value::Bool(cx.tick == 0)),
        (17, Value::Bool(true)),
    ]
}

/// Per-tick entries other than [`UPDATE`], by class (docs/level-scripts.md "Part 6").
pub const FRAME_ENTRIES: &[FrameEntry] = &[FrameEntry {
    class: "PlanetsWorld",
    function: "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb",
    args: planets_view_args,
}];

/// The per-tick entry for a class, if it has one other than [`UPDATE`].
pub fn frame_entry(vm: &Vm, class: ClassId) -> Option<&'static FrameEntry> {
    let name = vm.class_name(class);
    FRAME_ENTRIES.iter().find(|e| e.class == name)
}

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
/// A class listed in [`FRAME_ENTRIES`] runs that entry with its arguments instead of
/// [`UPDATE`].
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
        self.run_all(vm, host, Some(CREATE), budget, &BTreeSet::new())
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
        let update = self.run_all(vm, host, None, budget, &busy);
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
        function: Option<&str>,
        budget: u64,
        skip: &BTreeSet<u32>,
    ) -> StepReport {
        let mut report = StepReport::default();
        for inst in self.instances.clone() {
            if skip.contains(&inst.thing) || self.stopped.contains(&inst.thing) {
                continue;
            }
            let mut b = budget;
            // `None`: the tick's entry, a frame entry or `Update__`.
            let entry = match function {
                Some(_) => None,
                None => inst.class.and_then(|c| frame_entry(vm, c)),
            };
            let o = match entry {
                Some(e) => {
                    let args = (e.args)(&FrameContext {
                        vm,
                        instances: &self.instances,
                        tick: self.ticks,
                    });
                    self.runner
                        .start_with(vm, host, inst, e.function, &args, &mut b)
                }
                None => {
                    let f = function.unwrap_or(UPDATE);
                    self.runner.start(vm, host, inst, f, &mut b)
                }
            };
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
