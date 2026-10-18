//! D417: native Rust ports of level scripts, for when the scripts cannot be read.
//!
//! All 426 compiled scripts are in the encrypted `patch.edat` (D133); the disc has none.
//! A run from the disc and the installed update alone therefore has no script classes,
//! and a scripted Thing would do nothing. The registry ([`native_script`], keyed by the
//! script GUID the level's PScript part stores) gives such a Thing a port of its script
//! that makes the same host calls (natives, member reads and writes) the script makes, in
//! the same order, so every engine effect stays in the one place the VM's bindings use.
//! [`crate::Runner`] picks it when the class is not loaded, or always with
//! [`crate::Runner::force_native`]; the VM, given the decrypted overlay, is the oracle the
//! ports are checked against ([`lbp_script_vm::trace`]).
//!
//! Ported: `Pod` (the world script of `gamedata/levels/pod/pod.bin`) and `Trigger_Pod`
//! (its pod computer trigger). Not ported: the `BaseWorld` superclass's own `OnCreate__`
//! and `Update__` (score and player-indicator state, docs/d417-native-pod-script.md).
use lbp_script_vm::trace::TraceMark;
use lbp_script_vm::{Host, MemberKind, Site, Value};

/// The script GUID of the class `Pod` (FOUND: the type table of the 01.33 patch's
/// scripts names `Pod` with this GUID; its superclass is `BaseWorld`, g0000a8b6).
pub const POD_SCRIPT: u32 = 0x4905;
/// The script GUID of the class `Trigger_Pod` (superclass `Thing`, g0000248b).
pub const TRIGGER_POD_SCRIPT: u32 = 0x2d07;

/// What a native script's entry did.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeStep {
    Done(Value),
    /// `System.Yield__`: the runner calls [`NativeScript::update`] next tick instead of
    /// starting `Update__` again.
    Yielded,
}

/// The answer of a native entry: `None` when the class has no code for it (the runner
/// reports `Outcome::NoFunction`), else what it did or the fault text.
pub type NativeResult = Option<Result<NativeStep, String>>;

/// A native port of one script class. To add one: implement this for a unit struct,
/// make every engine effect a host call (`host.call_native`, `get_member`,
/// `set_member`) with the names and arguments the script uses, and list it in
/// [`NATIVE_SCRIPTS`].
pub trait NativeScript: Sync {
    /// The class name, as the script's type table has it.
    fn class(&self) -> &'static str;
    /// The script GUID a level's PScript part stores for the class, when known.
    fn guid(&self) -> Option<u32> {
        None
    }
    /// The superclasses, nearest first (they name the Thing's classes for casts and
    /// engine class tests, `ThingState::script_classes`).
    fn superclasses(&self) -> &'static [&'static str] {
        &["Thing"]
    }
    /// `OnCreate__`, once when the level starts.
    fn on_create(&self, _host: &mut dyn Host, _thing: u32) -> NativeResult {
        None
    }
    /// Once a tick: `Update__`, or the script's loop going on after a
    /// [`NativeStep::Yielded`].
    fn update(&self, _host: &mut dyn Host, _thing: u32) -> NativeResult {
        None
    }
    /// Any other entry the engine calls by name (trigger events, engine queries), with
    /// the argument registers as the VM stages them (byte offsets after `this`).
    fn call(
        &self,
        _host: &mut dyn Host,
        _thing: u32,
        _function: &str,
        _args: &[(u16, Value)],
    ) -> NativeResult {
        None
    }
}

/// Every native port.
pub const NATIVE_SCRIPTS: &[&dyn NativeScript] = &[
    &PodScript,
    &TriggerPodScript,
    &EARTH,
    &ORBITING_MOON,
    &crate::PLANETS_WORLD_PORT,
];

/// The native port of the script with this GUID.
pub fn native_script(guid: u32) -> Option<&'static dyn NativeScript> {
    NATIVE_SCRIPTS
        .iter()
        .copied()
        .find(|s| s.guid() == Some(guid))
}

/// The native port of the class with this name.
pub fn native_script_by_class(class: &str) -> Option<&'static dyn NativeScript> {
    NATIVE_SCRIPTS.iter().copied().find(|s| s.class() == class)
}

/// Runs the entry `function` of a port: `OnCreate__` and `Update__` go to
/// [`NativeScript::on_create`] and [`NativeScript::update`], the rest to
/// [`NativeScript::call`].
pub fn start_native(
    script: &dyn NativeScript,
    host: &mut dyn Host,
    thing: u32,
    function: &str,
    args: &[(u16, Value)],
) -> NativeResult {
    match function {
        "OnCreate__" => script.on_create(host, thing),
        "Update__" => script.update(host, thing),
        _ => script.call(host, thing, function, args),
    }
}

// ---- host-call helpers: values as the VM gives them to the script -----------------

/// A native call. A host that answers nothing gets the default of the declared return
/// type, as the VM does (`Value::default_for`): that is the `as_*` readers below.
pub(crate) fn call(
    host: &mut dyn Host,
    class: &str,
    function: &str,
    args: &[Value],
) -> Result<Value, String> {
    host.call_native(class, function, args)
}

pub(crate) fn site(host: &mut dyn Host, class: &str, function: &str) {
    if host.tracing() {
        let s = Site {
            class: class.to_owned(),
            function: function.to_owned(),
            pc: 0,
        };
        host.trace(TraceMark::Site(&s));
    }
}

pub(crate) fn as_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        _ => false,
    }
}

pub(crate) fn as_int(v: &Value) -> i32 {
    match v {
        Value::Int(i) => *i,
        Value::Raw(r) => *r as i32,
        _ => 0,
    }
}

pub(crate) fn as_float(v: &Value) -> f32 {
    match v {
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

pub(crate) fn as_thing(v: &Value) -> Option<u32> {
    match v {
        Value::Thing(t) => *t,
        _ => None,
    }
}

pub(crate) fn as_vec4(v: &Value) -> [f32; 4] {
    match v {
        Value::Vec4(a) => *a,
        _ => [0.0; 4],
    }
}

// ---- Pod -----------------------------------------------------------------------------

/// The world script of the Pod level (part 21, part 24).
///
/// FOUND (the 01.33 script, docs/level-scripts/part-21.md): `OnCreate__` sets `Finish`
/// false, sets the `pod_checkpoint` Thing's checkpoint active, then loops "update, yield"
/// until `Finish`. An update takes the party leader's avatar; with one, the engage test
/// (no video, the pod computer not in use, progression flag 2, the avatar in the pad
/// zone, no photo, no start menu, `BP_ENGAGE_POD` clicked) ends in
/// `RequestHostUsePod(true)`; every update then runs `SetPodTransition(false)`.
pub struct PodScript;

const POD: &str = "Pod";
/// The pod mesh and the platform mesh `SetPodTransition` puts on the pod Thing.
const POD_MESH_GUID: i32 = 12140;
const POD_PLATFORM_MESH_GUID: i32 = 145_753;
/// The change-over texture by the planets' backdrop (`AsInt(GetPlanetsBackgroundGUID())`),
/// and the texture for any other backdrop.
const TRANSITION_TEXTURES: [(i32, i32); 6] = [
    (102_775, 132_329),
    (102_704, 132_332),
    (102_683, 132_331),
    (102_728, 132_243),
    (121_102, 132_330),
    (105_587, 132_343),
];
const DEFAULT_TRANSITION_TEXTURE: i32 = 132_243;
/// `Math.Delt` step of the change-over value a tick (20 ticks end to end).
const TRANSITION_STEP: f32 = 0.05;

fn get_this(host: &mut dyn Host, this: u32, field: &str) -> Result<Value, String> {
    host.get_member(MemberKind::Thing, &Value::Thing(Some(this)), POD, field)
}

fn set_this(host: &mut dyn Host, this: u32, field: &str, v: Value) -> Result<(), String> {
    host.set_member(MemberKind::Thing, &Value::Thing(Some(this)), POD, field, v)
}

/// `PodTransition`; a Thing that stores none has the `.init__` value 0.
fn pod_transition(host: &mut dyn Host, this: u32) -> f32 {
    get_this(host, this, "PodTransition").map_or(0.0, |v| as_float(&v))
}

fn party_leader(host: &mut dyn Host) -> Result<i32, String> {
    Ok(as_int(&call(
        host,
        "Game",
        "GetPartyLeadersPlayerNumber__",
        &[],
    )?))
}

fn avatar(host: &mut dyn Host, player: i32) -> Result<Option<u32>, String> {
    let v = call(
        host,
        "Game",
        "GetAvatarFromPlayerID__i",
        &[Value::Int(player)],
    )?;
    Ok(as_thing(&v))
}

fn pod_computer_in_use(host: &mut dyn Host) -> Result<bool, String> {
    Ok(as_bool(&call(
        host,
        "PodComputer",
        "GetPodComputerInUse__",
        &[],
    )?))
}

impl PodScript {
    fn on_create(host: &mut dyn Host, this: u32) -> Result<NativeStep, String> {
        site(host, POD, "OnCreate__");
        call(host, "Native", "DebugPrint__i", &[Value::Int(19)])?;
        // `BaseWorld.OnCreate__` (FixupDivergentVariables) is not ported.
        set_this(host, this, "Finish", Value::Bool(false))?;
        let name = Value::Str("pod_checkpoint".into());
        let checkpoint = call(host, "Thing", "FindThing__Q7StringA", &[name])?;
        if as_thing(&checkpoint).is_some() {
            let part = host.get_member(MemberKind::Native, &checkpoint, "Thing", "Checkpoint")?;
            host.set_member(
                MemberKind::RefPtr,
                &part,
                "PCheckpoint",
                "Active",
                Value::Bool(true),
            )?;
        }
        Self::loop_step(host, this)
    }

    /// The `OnCreate__` loop from its test of `Finish`.
    fn loop_step(host: &mut dyn Host, this: u32) -> Result<NativeStep, String> {
        site(host, POD, "OnCreate__");
        if as_bool(&get_this(host, this, "Finish")?) {
            call(host, "Native", "DebugPrint__i", &[Value::Int(20)])?;
            return Ok(NativeStep::Done(Value::Void));
        }
        Self::update(host, this)?;
        Ok(NativeStep::Yielded)
    }

    fn update(host: &mut dyn Host, this: u32) -> Result<(), String> {
        site(host, POD, "Update__");
        let leader = party_leader(host)?;
        if leader != -1 {
            if let Some(avatar) = avatar(host, leader)? {
                Self::engage(host, leader, avatar)?;
            }
        }
        Self::set_pod_transition(host, this, false)?;
        // `BaseWorld.Update__` is not ported.
        Ok(())
    }

    /// The engage test of `Update__` with an avatar.
    fn engage(host: &mut dyn Host, leader: i32, avatar: u32) -> Result<(), String> {
        let no_video = !as_bool(&call(host, "Video", "IsVideoPlaying__", &[])?);
        let mut go = !pod_computer_in_use(host)? && no_video;
        if go {
            let flag = call(
                host,
                "Profile",
                "IsLBP2GameProgressionFlagSet__i",
                &[Value::Int(2)],
            )?;
            go = as_bool(&flag);
        }
        if !go {
            return Ok(());
        }
        let a = [Value::Thing(Some(avatar))];
        let mut go = as_bool(&call(host, "Native", "GetIsInPadZone__Q5Thing", &a)?);
        if go {
            go = !as_bool(&call(host, POD, "IsTakingPhoto__Q5Thing", &a)?);
        }
        if go {
            go = !as_bool(&call(host, "Game", "IsStartMenuOpen__", &[])?);
        }
        if !go {
            return Ok(());
        }
        let args = [
            Value::Int(leader),
            Value::Int(8),
            Value::Str("BP_ENGAGE_POD".into()),
        ];
        if as_bool(&call(
            host,
            "Input",
            "IsJustClickedByID__iiQ6String",
            &args,
        )?) {
            call(
                host,
                "PodComputer",
                "RequestHostUsePod__b",
                &[Value::Bool(true)],
            )?;
        }
        Ok(())
    }

    /// `SetPodTransition__b`: `snap` sets the change-over value to its goal at once.
    fn set_pod_transition(host: &mut dyn Host, this: u32, snap: bool) -> Result<(), String> {
        site(host, POD, "SetPodTransition__b");
        let pod = call(host, "Thing", "GetPodThing__b", &[Value::Bool(true)])?;
        let goal = if pod_computer_in_use(host)? { 1.0 } else { 0.0 };
        let value = if snap {
            goal
        } else {
            let now = pod_transition(host, this);
            let args = [
                Value::Float(now),
                Value::Float(goal),
                Value::Float(TRANSITION_STEP),
            ];
            as_float(&call(host, "Math", "Delt__fff", &args)?)
        };
        set_this(host, this, "PodTransition", Value::Float(value))?;
        let leader = party_leader(host)?;
        let mut colour = [0.0; 4];
        if leader != -1 {
            if let Some(avatar) = avatar(host, leader)? {
                let a = [Value::Thing(Some(avatar))];
                colour = as_vec4(&call(host, "Native", "GetPlayerColour__Q5Thing", &a)?);
            }
        }
        colour[3] = 1.0 - pod_transition(host, this);
        let args = [pod.clone(), Value::Vec4(colour)];
        call(host, "Thing", "SetShapeEditorColour__Q5Thingr", &args)?;
        let shape = host.get_member(MemberKind::Native, &pod, "Thing", "Shape")?;
        host.set_member(
            MemberKind::RefPtr,
            &shape,
            "PShape",
            "EditorColourBrightness",
            Value::Float(1.0),
        )?;
        let t = pod_transition(host, this);
        call(
            host,
            "PodComputer",
            "SetFadePodDecorations__f",
            &[Value::Float(t)],
        )?;
        let mesh = if pod_transition(host, this) < 1.0 {
            POD_MESH_GUID
        } else {
            POD_PLATFORM_MESH_GUID
        };
        let guid = call(host, "FileList", "AsGUID__i", &[Value::Int(mesh)])?;
        let resource = call(host, "Resource", "LoadMeshByFilename__g", &[guid])?;
        call(
            host,
            "Thing",
            "SetRenderMeshMesh__Q5ThingQ8Resource",
            &[pod, resource],
        )?;
        let backdrop = call(host, "PodComputer", "GetPlanetsBackgroundGUID__", &[])?;
        let backdrop = as_int(&call(host, "FileList", "AsInt__g", &[backdrop])?);
        let texture = TRANSITION_TEXTURES
            .iter()
            .find(|(b, _)| *b == backdrop)
            .map_or(DEFAULT_TRANSITION_TEXTURE, |(_, t)| *t);
        let args = [Value::Int(texture)];
        call(host, "PodComputer", "SetPodTransitionTexture__g", &args)?;
        Ok(())
    }

    /// `EnterPuterTrigger__Q5Thing`, which `Trigger_Pod.Enter__` calls on the world: the
    /// party leader entering while the pod computer is in use hears it start.
    fn enter_puter_trigger(host: &mut dyn Host, thing: Option<u32>) -> Result<(), String> {
        site(host, POD, "EnterPuterTrigger__Q5Thing");
        let leader = party_leader(host)?;
        let thing = Value::Thing(thing);
        let head = host.get_member(MemberKind::Native, &thing, "Thing", "YellowHead")?;
        let player = host.get_member(MemberKind::RefPtr, &head, "PYellowHead", "PlayerNumber")?;
        if as_int(&player) != leader || !pod_computer_in_use(host)? {
            return Ok(());
        }
        let args = [thing, Value::Str("pod/pod_comps_startup".into())];
        call(host, "Audio", "PlaySound__Q5ThingQ7StringA", &args)?;
        Ok(())
    }
}

/// The Thing argument the VM staged at byte offset 4.
fn thing_arg(args: &[(u16, Value)]) -> Option<u32> {
    args.iter()
        .find(|(o, _)| *o == 4)
        .and_then(|(_, v)| as_thing(v))
}

const DONE: NativeStep = NativeStep::Done(Value::Void);

impl NativeScript for PodScript {
    fn class(&self) -> &'static str {
        POD
    }
    fn guid(&self) -> Option<u32> {
        Some(POD_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["BaseWorld", "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(Self::on_create(host, thing))
    }
    /// The `OnCreate__` loop after its yield (the script's own `Update__` is private
    /// and runs only from that loop).
    fn update(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(Self::loop_step(host, thing))
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        Some(match function {
            "OnDestroy__" => {
                site(host, POD, "OnDestroy__");
                set_this(host, thing, "Finish", Value::Bool(true)).map(|()| DONE)
            }
            "EnterPuterTrigger__Q5Thing" => {
                Self::enter_puter_trigger(host, thing_arg(args)).map(|()| DONE)
            }
            "Kick__" | "OnScriptReloaded__" | "LeavePuterTrigger__Q5Thing" => Ok(DONE),
            _ => return None,
        })
    }
}

// ---- Trigger_Pod -----------------------------------------------------------------------

/// The pod computer's trigger. FOUND (the 01.33 script): `Enter__` hands the Thing that
/// entered to the world's `Pod.EnterPuterTrigger__` when the world is a `Pod`;
/// `IsPodComputerTrigger__` answers true (the engine's pad-zone test asks it).
pub struct TriggerPodScript;

const TRIGGER_POD: &str = "Trigger_Pod";

impl TriggerPodScript {
    fn world(host: &mut dyn Host, this: u32) -> Result<Value, String> {
        call(host, "Thing", "GetWorld__", &[Value::Thing(Some(this))])
    }

    /// `Enter__Q5Thing`: the world's `Pod.EnterPuterTrigger__` when the world is a Pod.
    fn enter(host: &mut dyn Host, this: u32, other: Option<u32>) -> Result<NativeStep, String> {
        site(host, TRIGGER_POD, "Enter__Q5Thing");
        let world = as_thing(&Self::world(host, this)?);
        let world = world.filter(|&w| host.thing_is(w, POD));
        if world.is_some() && other.is_some() {
            PodScript::enter_puter_trigger(host, other)?;
        }
        Ok(DONE)
    }
}

impl NativeScript for TriggerPodScript {
    fn class(&self) -> &'static str {
        TRIGGER_POD
    }
    fn guid(&self) -> Option<u32> {
        Some(TRIGGER_POD_SCRIPT)
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        Some(match function {
            "Enter__Q5Thing" => Self::enter(host, thing, thing_arg(args)),
            "Leave__Q5Thing" => {
                site(host, TRIGGER_POD, "Leave__Q5Thing");
                Self::world(host, thing).map(|_| DONE)
            }
            "IsPodComputerTrigger__" => Ok(NativeStep::Done(Value::Bool(true))),
            _ => return None,
        })
    }
}

// ---- PlanetThing, Earth, OrbitingMoon ------------------------------------------------

/// The script GUIDs of the Pod planets' classes (FOUND: the 01.33 patch's script rows
/// `gamedata/scripts/earth.ff`, `orbitingmoon.ff`; their superclass `PlanetThing`,
/// `planetthing.ff`, is g00016e09).
pub const EARTH_SCRIPT: u32 = 0x53d0;
pub const ORBITING_MOON_SCRIPT: u32 = 0x53ce;

/// A planet of the Pod's planets level: `PlanetThing` with the place its subclass gives
/// (`GetPosition__`, the subclass's only override).
///
/// FOUND (the 01.33 scripts): `OnCreate__` zeroes `Pitch` and `Spin`, keeps the world
/// matrix as `OriginalPos`, clears `Updated` and sets `MaxVel` 10 and `MaxAcc` 0.01.
/// `Update__rb(point, arrived)` turns the planet so that `point` (in the planet's own
/// space, scaled by the axis lengths of `OriginalPos` times `Scale`) faces the camera:
/// the target spin is `-atan2(x, z)` taken within half a turn of `Spin`, the target
/// pitch `atan2(y, |xz|)` clamped to a quarter turn. After the first update and while
/// not `arrived` it eases there (`Math.Smooth2D` on spin and pitch, acceleration
/// `MaxAcc`, speed `MaxVel`), else it snaps. The world matrix becomes
/// `rotateY(SpinOffset) * rotateX(Pitch) * rotateY(Spin) * scale` at the subclass's
/// place, and the answer is whether both angles are within 0.01 of their targets.
pub struct PlanetScript {
    class: &'static str,
    guid: u32,
    /// `GetPosition__` (FOUND: a constant of the subclass; MEASURED Earth (-1250, 0, 0),
    /// the Moon (2000, 0, 0), the places the scripted run gives them).
    position: [f32; 4],
}

/// `Earth` (g000053d0).
pub static EARTH: PlanetScript = PlanetScript {
    class: "Earth",
    guid: EARTH_SCRIPT,
    position: [-1250., 0., 0., 1.],
};

/// `OrbitingMoon` (g000053ce).
pub static ORBITING_MOON: PlanetScript = PlanetScript {
    class: "OrbitingMoon",
    guid: ORBITING_MOON_SCRIPT,
    position: [2000., 0., 0., 1.],
};

pub(crate) const PLANET_THING: &str = "PlanetThing";
const HALF_TURN: f32 = std::f32::consts::PI;
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;
/// `GetDefaultMaxVel__`, `GetDefaultMaxAcc__`; 0.01 is also the "arrived" tolerance.
const PLANET_MAX_VEL: f32 = 10.;
const PLANET_MAX_ACC: f32 = 0.01;
/// [`crate::PLANET_UPDATE`].
const PLANET_UPDATE_NAME: &str = "Update__rb";

fn get_planet(host: &mut dyn Host, this: u32, field: &str) -> Result<Value, String> {
    host.get_member(
        MemberKind::Thing,
        &Value::Thing(Some(this)),
        PLANET_THING,
        field,
    )
}

fn set_planet(host: &mut dyn Host, this: u32, field: &str, v: Value) -> Result<(), String> {
    host.set_member(
        MemberKind::Thing,
        &Value::Thing(Some(this)),
        PLANET_THING,
        field,
        v,
    )
}

/// A float field; a Thing that stores none has `default` (the `.init__` value).
fn planet_float(host: &mut dyn Host, this: u32, field: &str, default: f32) -> f32 {
    match get_planet(host, this, field) {
        Ok(Value::Float(f)) => f,
        _ => default,
    }
}

pub(crate) fn as_mat44(v: &Value) -> [f32; 16] {
    match v {
        Value::Mat44(m) => *m,
        _ => crate::transform::IDENTITY,
    }
}

fn math(host: &mut dyn Host, function: &str, args: &[Value]) -> Result<Value, String> {
    call(host, "Math", function, args)
}

fn column(m: &[f32; 16], c: usize) -> [f32; 4] {
    [m[c * 4], m[c * 4 + 1], m[c * 4 + 2], m[c * 4 + 3]]
}

fn atan2(host: &mut dyn Host, y: f32, x: f32) -> Result<f32, String> {
    let args = [Value::Float(y), Value::Float(x)];
    Ok(as_float(&math(host, "atan2__ff", &args)?))
}

/// The argument the VM staged at byte offset `offset`.
fn arg_at(args: &[(u16, Value)], offset: u16) -> Option<&Value> {
    args.iter().find(|(o, _)| *o == offset).map(|(_, v)| v)
}

impl PlanetScript {
    fn create(host: &mut dyn Host, this: u32) -> Result<NativeStep, String> {
        site(host, PLANET_THING, "OnCreate__");
        set_planet(host, this, "Pitch", Value::Float(0.))?;
        set_planet(host, this, "Spin", Value::Float(0.))?;
        Self::initialise(host, this)?;
        set_planet(host, this, "Updated", Value::Bool(false))?;
        set_planet(host, this, "MaxVel", Value::Float(PLANET_MAX_VEL))?;
        set_planet(host, this, "MaxAcc", Value::Float(PLANET_MAX_ACC))?;
        Ok(DONE)
    }

    /// `Initialise__`: `OriginalPos` is the world matrix now.
    fn initialise(host: &mut dyn Host, this: u32) -> Result<(), String> {
        let at = call(
            host,
            "Thing",
            "GetWorldPosition__",
            &[Value::Thing(Some(this))],
        )?;
        set_planet(host, this, "OriginalPos", at)
    }

    fn update(
        &self,
        host: &mut dyn Host,
        this: u32,
        point: [f32; 4],
        arrived: bool,
    ) -> Result<NativeStep, String> {
        site(host, PLANET_THING, PLANET_UPDATE_NAME);
        let max_vel = planet_float(host, this, "MaxVel", 0.);
        let max_acc = planet_float(host, this, "MaxAcc", 0.);
        let original = as_mat44(&get_planet(host, this, "OriginalPos")?);
        let mut axes = [0., 0., 0., 1.];
        for (c, axis) in axes.iter_mut().take(3).enumerate() {
            let len = as_float(&math(
                host,
                "length3__r",
                &[Value::Vec4(column(&original, c))],
            )?);
            *axis = len * planet_float(host, this, "Scale", 1.);
        }
        let mut place = as_mat44(&math(host, "scale__r", &[Value::Vec4(axes)])?);
        place[12..16].copy_from_slice(&self.position);
        let mut p = point;
        p[3] = 0.;
        let args = [Value::Mat44(place), Value::Vec4(p)];
        let p = as_vec4(&math(host, "transform__mr", &args)?);
        let mut spin_to = -atan2(host, p[0], p[2])?;
        if spin_to - planet_float(host, this, "Spin", 0.) > HALF_TURN {
            spin_to -= 2. * HALF_TURN;
        }
        if spin_to - planet_float(host, this, "Spin", 0.) < -HALF_TURN {
            spin_to += 2. * HALF_TURN;
        }
        let flat = (p[0] * p[0] + p[2] * p[2]).sqrt();
        let pitch_to = atan2(host, p[1], flat)?.clamp(-QUARTER_TURN, QUARTER_TURN);
        let updated = as_bool(&get_planet(host, this, "Updated")?);
        if updated && !arrived {
            let now = [
                planet_float(host, this, "Spin", 0.),
                planet_float(host, this, "Pitch", 0.),
                0.,
                0.,
            ];
            let speed = [
                planet_float(host, this, "SpinSpeed", 0.),
                planet_float(host, this, "PitchSpeed", 0.),
                0.,
                0.,
            ];
            let args = [
                Value::Vec4(now),
                Value::Vec4(speed),
                Value::Vec4([spin_to, pitch_to, 0., 0.]),
                Value::Float(max_acc),
                Value::Float(max_vel),
            ];
            let step = as_vec4(&math(host, "Smooth2D__pppff", &args)?);
            set_planet(host, this, "Spin", Value::Float(now[0] + step[0]))?;
            let spin = planet_float(host, this, "Spin", 0.);
            let spin = math(host, "ModAngleToWithinPI__f", &[Value::Float(spin)])?;
            set_planet(host, this, "Spin", spin)?;
            set_planet(host, this, "SpinSpeed", Value::Float(step[0]))?;
            set_planet(host, this, "Pitch", Value::Float(now[1] + step[1]))?;
            set_planet(host, this, "PitchSpeed", Value::Float(step[1]))?;
        } else {
            set_planet(host, this, "Spin", Value::Float(spin_to))?;
            set_planet(host, this, "Pitch", Value::Float(pitch_to))?;
            set_planet(host, this, "SpinSpeed", Value::Float(0.))?;
            set_planet(host, this, "PitchSpeed", Value::Float(0.))?;
        }
        let offset = planet_float(host, this, "SpinOffset", 0.);
        let a = as_mat44(&math(host, "rotateY__f", &[Value::Float(offset)])?);
        let pitch = planet_float(host, this, "Pitch", 0.);
        let b = as_mat44(&math(host, "rotateX__f", &[Value::Float(pitch)])?);
        let ab = crate::transform::mul(&a, &b);
        let spin = planet_float(host, this, "Spin", 0.);
        let c = as_mat44(&math(host, "rotateY__f", &[Value::Float(spin)])?);
        let mut world = crate::transform::mul(&crate::transform::mul(&ab, &c), &place);
        world[12..16].copy_from_slice(&place[12..16]);
        let args = [Value::Thing(Some(this)), Value::Mat44(world)];
        call(host, "Thing", "SetWorldPosition__m", &args)?;
        set_planet(host, this, "Updated", Value::Bool(true))?;
        let mut done = (planet_float(host, this, "Spin", 0.) - spin_to).abs() < PLANET_MAX_ACC;
        if done {
            done = (planet_float(host, this, "Pitch", 0.) - pitch_to).abs() < PLANET_MAX_ACC;
        }
        Ok(NativeStep::Done(Value::Bool(done)))
    }
}

impl NativeScript for PlanetScript {
    fn class(&self) -> &'static str {
        self.class
    }
    fn guid(&self) -> Option<u32> {
        Some(self.guid)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &[PLANET_THING, "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(Self::create(host, thing))
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        Some(match function {
            PLANET_UPDATE_NAME => {
                let point = arg_at(args, 16).map_or([0.; 4], as_vec4);
                let arrived = arg_at(args, 32).is_some_and(as_bool);
                self.update(host, thing, point, arrived)
            }
            "Initialise__" => {
                site(host, PLANET_THING, "Initialise__");
                Self::initialise(host, thing).map(|()| DONE)
            }
            "Snap__" => {
                site(host, PLANET_THING, "Snap__");
                set_planet(host, thing, "Updated", Value::Bool(false)).map(|()| DONE)
            }
            "GetPosition__" => Ok(NativeStep::Done(Value::Vec4(self.position))),
            "GetDefaultMaxVel__" => Ok(NativeStep::Done(Value::Float(PLANET_MAX_VEL))),
            "GetDefaultMaxAcc__" => Ok(NativeStep::Done(Value::Float(PLANET_MAX_ACC))),
            "OnScriptReloaded__" => Ok(DONE),
            _ => return None,
        })
    }
}

// ---- golden traces -----------------------------------------------------------------

/// The events of a trace that belong to the native ports: the entries started or
/// resumed on a Thing whose script has a port ([`native_script`]), and the host calls
/// made from the code of a ported class (the VM's calls from a superclass that is not
/// ported, such as `BaseWorld`, are left out). `ticks`: keep only these ticks (none:
/// all). The VM trace and the port's trace of the same run filter to the same list.
pub fn ported_events(
    events: &[lbp_script_vm::trace::TraceEvent],
    script_guid: impl Fn(u32) -> Option<u32>,
    ticks: Option<&dyn Fn(u64) -> bool>,
) -> Vec<lbp_script_vm::trace::TraceEvent> {
    let ported = |e: &lbp_script_vm::trace::TraceEvent| {
        let thing_ported = e
            .thing
            .and_then(&script_guid)
            .and_then(native_script)
            .is_some();
        if !thing_ported {
            return false;
        }
        if matches!(e.kind, "call" | "end") {
            return true;
        }
        let class = e.site.as_deref().and_then(|s| s.split('.').next());
        class.is_some_and(|c| native_script_by_class(c).is_some())
    };
    events
        .iter()
        .filter(|e| ticks.is_none_or(|f| f(e.tick)))
        .filter(|e| ported(e))
        .cloned()
        .collect()
}
