//! Rs: host bindings for native functions, keyed by (declaring class, mangled name) as the
//! VM reports them. Written from recorded findings, never from game code
//! (docs/script-natives.md lists each binding with its evidence label).
//!
//! The engine side is a [`World`]; every method defaults to an explicit "not available"
//! error so a partial world (tests, the viewer) still runs the pure bindings. [`Bindings`]
//! adapts a world to the VM's [`Host`] and counts calls to natives that have no binding
//! yet, in the order the G10 census ranked them.
use crate::signature;
use crate::{Host, MemberKind, Value};
use std::collections::BTreeMap;

/// What a Thing's switch input reports on one port (INFERRED field split: the census has
/// separate `IsSwitchTriggered`, binary, analogue and player natives).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SwitchInput {
    pub triggered: bool,
    pub binary: bool,
    pub analogue: f32,
    pub player: i32,
}

/// One `Cam.SetCameraFromToUp__rrrffff` call: eye position, look-at point, up vector and
/// the four trailing floats as passed (PlanetsWorld passes 10, 100000, 0, 1; INFERRED near
/// and far planes then two unknowns).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CameraFromToUp {
    pub from: [f32; 4],
    pub to: [f32; 4],
    pub up: [f32; 4],
    pub params: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Texture,
    Plan,
}

fn not_available(what: &str) -> String {
    format!("world does not provide {what}")
}

/// Engine state the bindings read and change. Thing handles are the host's `u32` ids.
pub trait World {
    fn thing_uid(&mut self, _thing: u32) -> Result<i32, String> {
        Err(not_available("thing UIDs"))
    }
    fn parent(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn first_child(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    fn next_sibling(&mut self, _thing: u32) -> Result<Option<u32>, String> {
        Err(not_available("the thing tree"))
    }
    /// The level's world Thing.
    fn world_thing(&mut self) -> Result<Option<u32>, String> {
        Err(not_available("the world thing"))
    }
    fn world_position(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("positions"))
    }
    fn centre_of_mass(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("centres of mass"))
    }
    /// Local transform in game memory order (translation at 12..15).
    fn set_local_transform(&mut self, _thing: u32, _m: [f32; 16]) -> Result<(), String> {
        Err(not_available("transforms"))
    }
    fn is_inactive(&mut self, _thing: u32) -> Result<bool, String> {
        Err(not_available("activity state"))
    }
    /// Does the Thing carry this part (`"PJoint"`, `"PSwitch"`, ...)?
    fn has_part(&mut self, _thing: u32, _part: &str) -> Result<bool, String> {
        Err(not_available("parts"))
    }
    fn switch_input(&mut self, _thing: u32, _port: i32) -> Result<SwitchInput, String> {
        Err(not_available("switch inputs"))
    }
    /// LAMS text by key (the strings the U3 text browser reads).
    fn translate(&mut self, _key: i32) -> Result<String, String> {
        Err(not_available("translated text"))
    }
    fn play_sound(&mut self, _thing: Option<u32>, _name: &str) -> Result<(), String> {
        Err(not_available("audio"))
    }
    /// Handle for a resource loaded by GUID.
    fn load_resource(&mut self, _kind: ResourceKind, _guid: u32) -> Result<u32, String> {
        Err(not_available("resources"))
    }
    fn num_players(&mut self) -> Result<i32, String> {
        Err(not_available("players"))
    }
    /// `Game.EPlayOrEditMode`: 0 play, 1 edit (INFERRED from script use, see
    /// docs/level-scripts.md "Part 2").
    fn play_or_edit_mode(&mut self) -> Result<i32, String> {
        Err(not_available("the play/edit mode"))
    }
    /// File handle of the level's dialogue list (s32, builtin 9 like the scripts'
    /// `DialogueListFile` field); 0 = none loaded.
    fn dialogue_list(&mut self) -> Result<i32, String> {
        Err(not_available("dialogue lists"))
    }
    /// The colour a shape shows in the editor, as a colour vector.
    fn shape_editor_colour(&mut self, _thing: u32) -> Result<[f32; 4], String> {
        Err(not_available("shape colours"))
    }
    fn set_shape_editor_colour(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("shape colours"))
    }
    /// Tint of a Thing's render mesh in the editor colour (`SetRenderMeshEditorColourTint`,
    /// called next to `SetShapeEditorColour` with a colour vector).
    fn set_render_mesh_tint(&mut self, _thing: u32, _colour: [f32; 4]) -> Result<(), String> {
        Err(not_available("render mesh tints"))
    }
    /// Playback rate of a Thing's animation (`Native.SetAnimSpeed`; scripts pass 0 to stop
    /// and 1 to run, INFERRED multiplier of the normal rate).
    fn set_anim_speed(&mut self, _thing: u32, _speed: f32) -> Result<(), String> {
        Err(not_available("animation speeds"))
    }
    /// `Game.EPlayerNumber` of the `index`-th player, for `0 <= index < num_players()`
    /// (scripts loop over that range).
    fn player_number(&mut self, _index: i32) -> Result<i32, String> {
        Err(not_available("players"))
    }
    /// Absolute world transform of a Thing (`Thing.SetWorldPosition__m`), same matrix order
    /// as [`World::set_local_transform`].
    fn set_world_transform(&mut self, _thing: u32, _m: [f32; 16]) -> Result<(), String> {
        Err(not_available("transforms"))
    }
    /// Playback position of a Thing's animation (`Native.SetAnimPos`; scripts pass 0 or the
    /// value they just gave `SetAnimSpeed`, INFERRED a time or fraction).
    fn set_anim_pos(&mut self, _thing: u32, _pos: f32) -> Result<(), String> {
        Err(not_available("animation positions"))
    }
    /// The engine's PodComputer script object (`PodComputer.GetPodComputer__`).
    fn pod_computer(&mut self) -> Result<Value, String> {
        Err(not_available("the pod computer"))
    }
    /// Is a player using the pod computer (`PodComputer.GetPodComputerInUse__`)?
    fn pod_computer_in_use(&mut self) -> Result<bool, String> {
        Err(not_available("the pod computer"))
    }
    /// `PodComputer.GetCrashZoom__` (bool; INFERRED a zoom-to-crash state of the planet view).
    fn crash_zoom(&mut self) -> Result<bool, String> {
        Err(not_available("the pod computer"))
    }
    /// A new Gooey (UI) script object (`Gooey.CreateGooey__i`, called by `Gooey.Create__i`
    /// with its s32 argument; INFERRED an id or flags).
    fn create_gooey(&mut self, _arg: i32) -> Result<Value, String> {
        Err(not_available("Gooey objects"))
    }
    fn set_camera(&mut self, _camera: CameraFromToUp) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `Cam.SetDOFAndFog__fff` as passed (PlanetsWorld: DofMin, DofMax, FogBias, each x 2).
    fn set_dof_and_fog(&mut self, _values: [f32; 3]) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    fn clear_motion_blur(&mut self) -> Result<(), String> {
        Err(not_available("the camera"))
    }
    /// `HUD.GetCinemaModeCount__` (s32; scripts only test it against zero).
    fn cinema_mode_count(&mut self) -> Result<i32, String> {
        Err(not_available("the HUD"))
    }
    /// `Game.SetPlayerScoreMultiplierTimer` (player, s32 timer; scripts pass 0 or a member
    /// named `CoopScoreMultiplierTimer`).
    fn set_score_multiplier_timer(&mut self, _player: i32, _timer: i32) -> Result<(), String> {
        Err(not_available("scores"))
    }
    /// Outcome of a `Test.Check*` / `Test.Fail` native in the game's own test scripts.
    fn test_check(&mut self, _passed: bool, _message: &str, _line: i32) {}
    /// Uniform in [0, 1); only `NonDeterministic*` natives use it.
    fn random_unit(&mut self) -> f32 {
        0.
    }
    fn get_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn set_member(
        &mut self,
        _kind: MemberKind,
        _base: &Value,
        class: &str,
        field: &str,
        _value: Value,
    ) -> Result<(), String> {
        Err(not_available(&format!("member {class}.{field}")))
    }
    fn thing_is(&mut self, _thing: u32, _class: &str) -> bool {
        true
    }
    fn write(&mut self, _text: &str) {}
}

/// A native call split by the mangled signature: `receiver` is present when the VM staged
/// one more value than the signature has parameters (member call, INFERRED).
pub struct Call<'a> {
    pub receiver: Option<&'a Value>,
    pub params: &'a [Value],
}

pub type NativeFn = fn(&mut dyn World, &Call<'_>) -> Result<Value, String>;

/// Adapts a [`World`] to the VM's [`Host`].
pub struct Bindings<W> {
    pub world: W,
    /// Calls to natives without a binding, by `Class.mangled`.
    pub unbound: BTreeMap<String, u64>,
    /// Every native call that reached the host, bound or not, by `Class.mangled`.
    pub calls: BTreeMap<String, u64>,
}

impl<W: World> Bindings<W> {
    pub fn new(world: W) -> Self {
        Self {
            world,
            unbound: BTreeMap::new(),
            calls: BTreeMap::new(),
        }
    }
}

/// The binding for a native, if one exists.
pub fn lookup(class: &str, mangled: &str) -> Option<NativeFn> {
    TABLE
        .iter()
        .find(|(c, m, _)| *c == class && *m == mangled)
        .map(|(_, _, f)| *f)
}

/// Every bound native as (class, mangled name), in table order.
pub fn bound() -> impl Iterator<Item = (&'static str, &'static str)> {
    TABLE.iter().map(|(c, m, _)| (*c, *m))
}

/// Calls a native through the table, checking arity against the mangled signature.
pub fn dispatch(
    world: &mut dyn World,
    class: &str,
    mangled: &str,
    args: &[Value],
) -> Option<Result<Value, String>> {
    let f = lookup(class, mangled)?;
    let n = signature::parse(mangled).map_or(0, |s| s.params.len());
    let call = if args.len() == n {
        Call {
            receiver: None,
            params: args,
        }
    } else if args.len() == n + 1 {
        Call {
            receiver: args.first(),
            params: &args[1..],
        }
    } else {
        return Some(Err(format!(
            "{class}.{mangled}: {} arguments for {n} parameters",
            args.len()
        )));
    };
    Some(f(world, &call))
}

impl<W: World> Host for Bindings<W> {
    fn call_native(
        &mut self,
        class: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        *self.calls.entry(format!("{class}.{function}")).or_default() += 1;
        match dispatch(&mut self.world, class, function, args) {
            Some(r) => r.map_err(|e| format!("{class}.{function}: {e}")),
            None => {
                *self
                    .unbound
                    .entry(format!("{class}.{function}"))
                    .or_default() += 1;
                Err(format!("unbound native {class}.{function}"))
            }
        }
    }
    fn get_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
    ) -> Result<Value, String> {
        self.world.get_member(kind, base, class, field)
    }
    fn set_member(
        &mut self,
        kind: MemberKind,
        base: &Value,
        class: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        self.world.set_member(kind, base, class, field, value)
    }
    fn thing_is(&mut self, thing: u32, class: &str) -> bool {
        self.world.thing_is(thing, class)
    }
    fn write(&mut self, text: &str) {
        self.world.write(text)
    }
}

// ---- argument helpers ------------------------------------------------------------

fn param<'a>(c: &'a Call<'_>, i: usize) -> Result<&'a Value, String> {
    c.params
        .get(i)
        .ok_or_else(|| format!("missing parameter {i}"))
}

fn float(v: &Value) -> Result<f32, String> {
    match v {
        Value::Float(f) => Ok(*f),
        other => Err(format!("expected f32, got {other:?}")),
    }
}

fn int(v: &Value) -> Result<i32, String> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(format!("expected s32, got {other:?}")),
    }
}

fn vec4(v: &Value) -> Result<[f32; 4], String> {
    match v {
        Value::Vec4(x) => Ok(*x),
        other => Err(format!("expected vector, got {other:?}")),
    }
}

fn text(v: &Value) -> Result<&str, String> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err(format!("expected string, got {other:?}")),
    }
}

/// GUID parameter (`g`): carried as an integer or pointer-sized value (INFERRED).
fn guid(v: &Value) -> Result<u32, String> {
    match v {
        Value::Int(i) => Ok(*i as u32),
        Value::Raw(u) | Value::Ref(u) => Ok(*u),
        other => Err(format!("expected GUID, got {other:?}")),
    }
}

fn thing(v: &Value) -> Result<Option<u32>, String> {
    match v {
        Value::Thing(t) => Ok(*t),
        other => Err(format!("expected Thing, got {other:?}")),
    }
}

fn some_thing(v: &Value) -> Result<u32, String> {
    thing(v)?.ok_or_else(|| "null Thing".to_owned())
}

/// The Thing a member native acts on.
fn this(c: &Call<'_>) -> Result<u32, String> {
    some_thing(
        c.receiver
            .ok_or("member native called without a receiver")?,
    )
}

/// For `Get*__Q5Thing`: the Thing parameter (static form or member form alike).
fn subject(c: &Call<'_>) -> Result<u32, String> {
    some_thing(param(c, 0)?)
}

// ---- math (INFERRED: vector-library conventions, radians, column vectors) -----------

/// Column-major with column vectors: column `k` occupies indices `4k..4k+4`, so the
/// translation sits at 12..15 as in level-file world matrices.
fn columns(c0: [f32; 4], c1: [f32; 4], c2: [f32; 4], c3: [f32; 4]) -> Value {
    let mut m = [0.; 16];
    for (k, col) in [c0, c1, c2, c3].iter().enumerate() {
        m[k * 4..k * 4 + 4].copy_from_slice(col);
    }
    Value::Mat44(m)
}

const W: [f32; 4] = [0., 0., 0., 1.];

fn rotate(axis: usize, a: f32) -> Value {
    let (s, c) = a.sin_cos();
    match axis {
        0 => columns([1., 0., 0., 0.], [0., c, s, 0.], [0., -s, c, 0.], W),
        1 => columns([c, 0., -s, 0.], [0., 1., 0., 0.], [s, 0., c, 0.], W),
        _ => columns([c, s, 0., 0.], [-s, c, 0., 0.], [0., 0., 1., 0.], W),
    }
}

fn len3(v: [f32; 4]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Scales the first `k` components of `v` down to length `max` (`max <= 0` gives zero).
fn clamp_len(v: [f32; 4], k: usize, max: f32) -> [f32; 4] {
    let l = v[..k].iter().map(|x| x * x).sum::<f32>().sqrt();
    let s = if l > max { max.max(0.) / l } else { 1. };
    std::array::from_fn(|i| if i < k { v[i] * s } else { 0. })
}

/// `Smooth*(pos, vel, target, max_acc, max_vel)` on the first `k` components: the new
/// velocity. INFERRED: the parameter order comes from `PlanetThing`, which passes its
/// `MaxAcc` then `MaxVel` fields; the step is "head for the target at most `max_vel` per
/// tick, changing velocity by at most `max_acc` per tick". Callers add it to `pos`.
fn smooth(c: &Call<'_>, k: usize) -> Result<[f32; 4], String> {
    let get = |i| -> Result<[f32; 4], String> {
        match param(c, i)? {
            Value::Float(f) => Ok([*f, 0., 0., 0.]),
            v => vec4(v),
        }
    };
    let (pos, vel, target) = (get(0)?, get(1)?, get(2)?);
    let (max_acc, max_vel) = (float(param(c, 3)?)?, float(param(c, 4)?)?);
    let want = clamp_len(std::array::from_fn(|i| target[i] - pos[i]), k, max_vel);
    let dv = clamp_len(std::array::from_fn(|i| want[i] - vel[i]), k, max_acc);
    Ok(std::array::from_fn(|i| {
        if i < k {
            vel[i] + dv[i]
        } else {
            0.
        }
    }))
}

fn matrix(v: &Value) -> Result<[f32; 16], String> {
    match v {
        Value::Mat44(m) => Ok(*m),
        other => Err(format!("expected m44, got {other:?}")),
    }
}

/// `Test.*(..., message, line)`: reports one check; the message and line are the last two
/// parameters (INFERRED from the `Q7StringAi` suffix every Test native shares).
fn report(w: &mut dyn World, c: &Call<'_>, passed: bool) -> Result<Value, String> {
    let n = c.params.len();
    let message = text(param(c, n.wrapping_sub(2))?)?;
    let line = int(param(c, n.wrapping_sub(1))?)?;
    if passed {
        w.test_check(true, message, line);
    } else {
        // Failed checks carry the compared values for diagnosis.
        let values = &c.params[..n.saturating_sub(2)];
        w.test_check(false, &format!("{message} {values:?}"), line);
    }
    Ok(Value::Void)
}

/// `CheckEqual` on vectors compares the components the type has (V2: 2, V3: 3, V4: 4).
fn equal_components(w: &mut dyn World, c: &Call<'_>, k: usize) -> Result<Value, String> {
    let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
    report(w, c, a[..k] == b[..k])
}

fn equal_values(w: &mut dyn World, c: &Call<'_>) -> Result<Value, String> {
    let (a, b) = (param(c, 0)?, param(c, 1)?);
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return Err(format!("CheckEqual on {a:?} and {b:?}"));
    }
    report(w, c, a == b)
}

fn boolean(v: &Value) -> Result<bool, String> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(format!("expected bool, got {other:?}")),
    }
}

const TABLE: &[(&str, &str, NativeFn)] = &[
    // System.Yield__ is handled inside the VM (it suspends the call).
    ("Math", "floor__f", |_, c| {
        Ok(Value::Float(float(param(c, 0)?)?.floor()))
    }),
    ("Math", "clamp__fff", |_, c| {
        let (x, lo, hi) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        Ok(Value::Float(x.max(lo).min(hi)))
    }),
    ("Math", "atan2__ff", |_, c| {
        Ok(Value::Float(
            float(param(c, 0)?)?.atan2(float(param(c, 1)?)?),
        ))
    }),
    ("Math", "rotateX__f", |_, c| {
        Ok(rotate(0, float(param(c, 0)?)?))
    }),
    ("Math", "rotateY__f", |_, c| {
        Ok(rotate(1, float(param(c, 0)?)?))
    }),
    ("Math", "rotateZ__f", |_, c| {
        Ok(rotate(2, float(param(c, 0)?)?))
    }),
    ("Math", "translate__r", |_, c| {
        let t = vec4(param(c, 0)?)?;
        Ok(columns(
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [t[0], t[1], t[2], 1.],
        ))
    }),
    ("Math", "scale__r", |_, c| {
        let s = vec4(param(c, 0)?)?;
        Ok(columns(
            [s[0], 0., 0., 0.],
            [0., s[1], 0., 0.],
            [0., 0., s[2], 0.],
            W,
        ))
    }),
    ("Math", "mulPerElem__rr", |_, c| {
        let (a, b) = (vec4(param(c, 0)?)?, vec4(param(c, 1)?)?);
        Ok(Value::Vec4(std::array::from_fn(|i| a[i] * b[i])))
    }),
    ("Math", "length3__r", |_, c| {
        Ok(Value::Float(len3(vec4(param(c, 0)?)?)))
    }),
    ("Math", "normalize3__r", |_, c| {
        let v = vec4(param(c, 0)?)?;
        let l = len3(v);
        if l == 0. {
            return Err("normalize3 of a zero vector".into());
        }
        Ok(Value::Vec4([v[0] / l, v[1] / l, v[2] / l, v[3]]))
    }),
    ("Math", "length__p", |_, c| {
        let v = vec4(param(c, 0)?)?;
        Ok(Value::Float((v[0] * v[0] + v[1] * v[1]).sqrt()))
    }),
    ("Math", "NonDeterministicRandomBetween__ff", |w, c| {
        let (a, b) = (float(param(c, 0)?)?, float(param(c, 1)?)?);
        Ok(Value::Float(a + (b - a) * w.random_unit()))
    }),
    // ---- strings ----
    ("String", "Concat__Q6StringQ6String", |_, c| {
        let s = format!("{}{}", text(param(c, 0)?)?, text(param(c, 1)?)?);
        Ok(Value::Str(s.into()))
    }),
    ("String", "StringFromInt__i", |_, c| {
        Ok(Value::Str(int(param(c, 0)?)?.to_string().into()))
    }),
    ("String", "InfinitySymbol__", |_, _| {
        Ok(Value::Str("\u{221e}".into()))
    }),
    ("LAMS", "Translate__i", |w, c| {
        Ok(Value::Str(w.translate(int(param(c, 0)?)?)?.into()))
    }),
    // ---- thing tree and transforms ----
    ("Thing", "GetThingUID__", |w, c| {
        Ok(Value::Int(w.thing_uid(this(c)?)?))
    }),
    ("Thing", "GetParent__Q5Thing", |w, c| {
        Ok(Value::Thing(w.parent(subject(c)?)?))
    }),
    ("Thing", "GetFirstChild__Q5Thing", |w, c| {
        Ok(Value::Thing(w.first_child(subject(c)?)?))
    }),
    ("Thing", "GetNextSibling__Q5Thing", |w, c| {
        Ok(Value::Thing(w.next_sibling(subject(c)?)?))
    }),
    ("Thing", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    ("System", "GetWorld__", |w, _| {
        Ok(Value::Thing(w.world_thing()?))
    }),
    ("Thing", "GetWorldPosition__", |w, c| {
        Ok(Value::Vec4(w.world_position(this(c)?)?))
    }),
    ("Thing", "GetCOM__", |w, c| {
        Ok(Value::Vec4(w.centre_of_mass(this(c)?)?))
    }),
    ("Thing", "SetLocalPosition__m", |w, c| {
        let m = match param(c, 0)? {
            Value::Mat44(m) => *m,
            other => return Err(format!("expected m44, got {other:?}")),
        };
        w.set_local_transform(this(c)?, m)?;
        Ok(Value::Void)
    }),
    ("Thing", "IsInactive__", |w, c| {
        Ok(Value::Bool(w.is_inactive(this(c)?)?))
    }),
    // ---- switches (the Thing's own inputs, by port) ----
    ("Thing", "IsSwitchTriggered__i", |w, c| {
        Ok(Value::Bool(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.triggered,
        ))
    }),
    ("Thing", "GetSwitchActivationBinary__i", |w, c| {
        Ok(Value::Bool(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.binary,
        ))
    }),
    ("Thing", "GetSwitchActivationAnalogue__i", |w, c| {
        Ok(Value::Float(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.analogue,
        ))
    }),
    ("Thing", "GetSwitchActivationPlayer__i", |w, c| {
        Ok(Value::Int(
            w.switch_input(this(c)?, int(param(c, 0)?)?)?.player,
        ))
    }),
    // ---- parts ----
    ("PartList", "HasPJoint__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PJoint")?))
    }),
    ("PartList", "HasPSwitch__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSwitch")?))
    }),
    ("PartList", "HasPTrigger__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PTrigger")?))
    }),
    ("PartList", "HasPShape__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PShape")?))
    }),
    ("PartList", "HasPSpriteLight__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PSpriteLight")?))
    }),
    ("PartList", "HasPRenderMesh__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PRenderMesh")?))
    }),
    // ---- audio, resources, game ----
    ("Audio", "PlaySound__Q5ThingQ7StringA", |w, c| {
        let t = thing(param(c, 0)?)?;
        w.play_sound(t, text(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("Resource", "LoadTextureByFilename__g", |w, c| {
        Ok(Value::Ref(w.load_resource(
            ResourceKind::Texture,
            guid(param(c, 0)?)?,
        )?))
    }),
    ("Resource", "LoadPlanByFilename__g", |w, c| {
        Ok(Value::Ref(
            w.load_resource(ResourceKind::Plan, guid(param(c, 0)?)?)?,
        ))
    }),
    ("Game", "GetNumPlayers__", |w, _| {
        Ok(Value::Int(w.num_players()?))
    }),
    ("Game", "GetPlayOrEditMode__", |w, _| {
        Ok(Value::Int(w.play_or_edit_mode()?))
    }),
    ("Audio", "GetDialogueList__", |w, _| {
        Ok(Value::Int(w.dialogue_list()?))
    }),
    ("Thing", "GetShapeEditorColour__Q5Thing", |w, c| {
        Ok(Value::Vec4(w.shape_editor_colour(subject(c)?)?))
    }),
    ("Thing", "SetShapeEditorColour__Q5Thingr", |w, c| {
        w.set_shape_editor_colour(subject(c)?, vec4(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    (
        "Thing",
        "SetRenderMeshEditorColourTint__Q5Thingr",
        |w, c| {
            w.set_render_mesh_tint(subject(c)?, vec4(param(c, 1)?)?)?;
            Ok(Value::Void)
        },
    ),
    ("Native", "SetAnimSpeed__Q5Thingf", |w, c| {
        w.set_anim_speed(subject(c)?, float(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("PartList", "HasPCameraTweak__Q5Thing", |w, c| {
        Ok(Value::Bool(w.has_part(subject(c)?, "PCameraTweak")?))
    }),
    ("Game", "GetPlayerNumber__i", |w, c| {
        Ok(Value::Int(w.player_number(int(param(c, 0)?)?)?))
    }),
    // ---- part 5: the Pod camera (PlanetsWorld) and the next Rookie Test natives ----
    ("Math", "lerp__frr", |_, c| {
        // Vector-library order lerp(t, a, b) = a + (b - a) t (INFERRED).
        let (t, a, b) = (
            float(param(c, 0)?)?,
            vec4(param(c, 1)?)?,
            vec4(param(c, 2)?)?,
        );
        Ok(Value::Vec4(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)))
    }),
    ("Math", "SmoothStep__fff", |_, c| {
        // smoothstep(edge0, edge1, x); PlanetsWorld passes (0, 1, ViewTransition).
        let (e0, e1, x) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        if e0 == e1 {
            return Ok(Value::Float(if x < e0 { 0. } else { 1. }));
        }
        let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
        Ok(Value::Float(t * t * (3. - 2. * t)))
    }),
    ("Math", "Smooth3D__rrrff", |_, c| Ok(Value::Vec4(smooth(c, 3)?))),
    ("Math", "Smooth2D__pppff", |_, c| Ok(Value::Vec4(smooth(c, 2)?))),
    ("Math", "Smooth__fffff", |_, c| {
        // The caller reads x as the new value and y as the new velocity.
        let pos = float(param(c, 0)?)?;
        let v = smooth(c, 1)?[0];
        Ok(Value::Vec4([pos + v, v, 0., 0.]))
    }),
    ("Cam", "SetCameraFromToUp__rrrffff", |w, c| {
        let f = |i| float(param(c, i)?);
        w.set_camera(CameraFromToUp {
            from: vec4(param(c, 0)?)?,
            to: vec4(param(c, 1)?)?,
            up: vec4(param(c, 2)?)?,
            params: [f(3)?, f(4)?, f(5)?, f(6)?],
        })?;
        Ok(Value::Void)
    }),
    ("Cam", "SetDOFAndFog__fff", |w, c| {
        let f = |i| float(param(c, i)?);
        w.set_dof_and_fog([f(0)?, f(1)?, f(2)?])?;
        Ok(Value::Void)
    }),
    ("Cam", "ClearMotionBlur__", |w, _| {
        w.clear_motion_blur()?;
        Ok(Value::Void)
    }),
    ("HUD", "GetCinemaModeCount__", |w, _| {
        Ok(Value::Int(w.cinema_mode_count()?))
    }),
    ("PodComputer", "GetPodComputer__", |w, _| w.pod_computer()),
    ("PodComputer", "GetPodComputerInUse__", |w, _| {
        Ok(Value::Bool(w.pod_computer_in_use()?))
    }),
    ("PodComputer", "GetCrashZoom__", |w, _| {
        Ok(Value::Bool(w.crash_zoom()?))
    }),
    ("Thing", "SetWorldPosition__m", |w, c| {
        w.set_world_transform(this(c)?, matrix(param(c, 0)?)?)?;
        Ok(Value::Void)
    }),
    ("Native", "DebugPrint__i", |w, c| {
        w.write(&int(param(c, 0)?)?.to_string());
        Ok(Value::Void)
    }),
    ("Native", "SetAnimPos__Q5Thingf", |w, c| {
        w.set_anim_pos(subject(c)?, float(param(c, 1)?)?)?;
        Ok(Value::Void)
    }),
    ("LAMS", "MakeLamsKeyID__Q6String", |_, c| {
        // Toolkit makeLamsKeyID (CONFIRMED_FROM_COMMUNITY_CODE, docs/u1-translation.md);
        // the key is the s32 that `Translate__i` and `IsInLams__i` take.
        let key = lbp_resource::translation::lams_key(text(param(c, 0)?)?);
        Ok(Value::Int(key as i32))
    }),
    (
        "Game",
        "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
        |w, c| {
            w.set_score_multiplier_timer(int(param(c, 0)?)?, int(param(c, 1)?)?)?;
            Ok(Value::Void)
        },
    ),
    ("Gooey", "CreateGooey__i", |w, c| {
        w.create_gooey(int(param(c, 0)?)?)
    }),
    // ---- the game's own test scripts ----
    ("Test", "Check__bQ7StringAi", |w, c| {
        let ok = boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "CheckFalse__bQ7StringAi", |w, c| {
        let ok = !boolean(param(c, 0)?)?;
        report(w, c, ok)
    }),
    ("Test", "Fail__Q7StringAi", |w, c| report(w, c, false)),
    ("Test", "Succeed__Q7StringAi", |w, c| report(w, c, true)),
    ("Test", "CheckEqual__iiQ7StringAi", equal_values),
    ("Test", "CheckEqual__bbQ7StringAi", equal_values),
    ("Test", "CheckEqual__jjQ7StringAi", equal_values),
    ("Test", "CheckEqual__ffQ7StringAi", equal_values),
    ("Test", "CheckEqual__ppQ7StringAi", |w, c| {
        equal_components(w, c, 2)
    }),
    ("Test", "CheckEqual__qqQ7StringAi", |w, c| {
        equal_components(w, c, 3)
    }),
    ("Test", "CheckEqual__rrQ7StringAi", |w, c| {
        equal_components(w, c, 4)
    }),
    ("Test", "CheckSimilar__fffQ7StringAi", |w, c| {
        let (a, b, eps) = (
            float(param(c, 0)?)?,
            float(param(c, 1)?)?,
            float(param(c, 2)?)?,
        );
        report(w, c, (a - b).abs() <= eps)
    }),
];

/// Natives the VM itself handles; they never reach a host.
pub const VM_HANDLED: [(&str, &str); 1] = [("System", "Yield__")];

#[cfg(test)]
mod tests {
    use super::*;

    struct Tree;
    impl World for Tree {
        fn parent(&mut self, t: u32) -> Result<Option<u32>, String> {
            Ok(t.checked_sub(1))
        }
        fn thing_uid(&mut self, t: u32) -> Result<i32, String> {
            Ok(t as i32 + 100)
        }
        fn switch_input(&mut self, _t: u32, port: i32) -> Result<SwitchInput, String> {
            Ok(SwitchInput {
                analogue: port as f32 / 2.,
                ..Default::default()
            })
        }
    }

    fn call(class: &str, m: &str, args: &[Value]) -> Result<Value, String> {
        dispatch(&mut Tree, class, m, args).expect("bound")
    }

    #[test]
    fn every_entry_has_a_signature_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (c, m) in bound() {
            let s = signature::parse(m).unwrap_or_else(|| panic!("{m}"));
            assert!(
                !s.params
                    .iter()
                    .any(|p| matches!(p, signature::Param::Unknown(_))),
                "{m}"
            );
            assert!(seen.insert((c, m)), "duplicate {c}.{m}");
        }
    }

    #[test]
    fn math() {
        let Value::Mat44(m) =
            call("Math", "translate__r", &[Value::Vec4([1., 2., 3., 0.])]).unwrap()
        else {
            panic!()
        };
        assert_eq!(&m[12..16], &[1., 2., 3., 1.]);
        let Value::Mat44(r) = call(
            "Math",
            "rotateZ__f",
            &[Value::Float(std::f32::consts::FRAC_PI_2)],
        )
        .unwrap() else {
            panic!()
        };
        // Column 0 (the image of +x) is +y.
        assert!((r[0]).abs() < 1e-6 && (r[1] - 1.).abs() < 1e-6);
        assert_eq!(
            call("Math", "length3__r", &[Value::Vec4([3., 4., 0., 9.])]),
            Ok(Value::Float(5.))
        );
        assert_eq!(
            call(
                "Math",
                "clamp__fff",
                &[Value::Float(5.), Value::Float(0.), Value::Float(1.)]
            ),
            Ok(Value::Float(1.))
        );
        assert!(call("Math", "normalize3__r", &[Value::Vec4([0.; 4])]).is_err());
        assert_eq!(
            call(
                "String",
                "Concat__Q6StringQ6String",
                &[Value::Str("a".into()), Value::Str("b".into())]
            ),
            Ok(Value::Str("ab".into()))
        );
    }

    #[test]
    fn receiver_split_and_world() {
        // Member form: receiver plus the port.
        assert_eq!(
            call(
                "Thing",
                "GetSwitchActivationAnalogue__i",
                &[Value::Thing(Some(7)), Value::Int(1)]
            ),
            Ok(Value::Float(0.5))
        );
        assert_eq!(
            call("Thing", "GetThingUID__", &[Value::Thing(Some(7))]),
            Ok(Value::Int(107))
        );
        assert!(call("Thing", "GetThingUID__", &[]).is_err());
        assert_eq!(
            call("Thing", "GetParent__Q5Thing", &[Value::Thing(Some(0))]),
            Ok(Value::Thing(None))
        );
        assert!(call("Thing", "GetThingUID__", &[Value::Thing(None)]).is_err());
        // Arity outside n or n + 1.
        assert!(call("Math", "floor__f", &[]).is_err());
        // A world that lacks a feature says so.
        assert!(call("LAMS", "Translate__i", &[Value::Int(3)])
            .unwrap_err()
            .contains("translated text"));
        assert!(dispatch(&mut Tree, "Gooey", "EndFrame__", &[]).is_none());
    }

    #[test]
    fn test_natives_report_checks() {
        #[derive(Default)]
        struct Log(Vec<(bool, String, i32)>);
        impl World for Log {
            fn test_check(&mut self, passed: bool, message: &str, line: i32) {
                self.0.push((passed, message.into(), line));
            }
        }
        let mut log = Log::default();
        let msg = || Value::Str("m".into());
        let mut run =
            |m: &str, args: Vec<Value>| dispatch(&mut log, "Test", m, &args).expect("bound");
        let v = |a: [f32; 4]| Value::Vec4(a);
        run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(2), Value::Int(2), msg(), Value::Int(9)],
        )
        .unwrap();
        run(
            "CheckEqual__ppQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckEqual__rrQ7StringAi",
            vec![
                v([1., 2., 3., 4.]),
                v([1., 2., 0., 0.]),
                msg(),
                Value::Int(9),
            ],
        )
        .unwrap();
        run(
            "CheckSimilar__fffQ7StringAi",
            vec![
                Value::Float(1.),
                Value::Float(1.05),
                Value::Float(0.1),
                msg(),
                Value::Int(3),
            ],
        )
        .unwrap();
        // Mixed types are a fault, not a failed check.
        assert!(run(
            "CheckEqual__iiQ7StringAi",
            vec![Value::Int(1), Value::Float(1.), msg(), Value::Int(1)]
        )
        .is_err());
        let passed: Vec<_> = log.0.iter().map(|(p, _, l)| (*p, *l)).collect();
        assert_eq!(passed, [(true, 9), (true, 9), (false, 9), (true, 3)]);
    }
}
