//! D505: `Launcher` (script g00012d39, base `Tweakable` g00004c8e, then `Native`), the
//! launch pad Story levels throw Sackboy with, as a [`NativeScript`] port.
//!
//! FOUND (the 01.33 script listing, research copy only): the script is small. Its
//! `OnCreate__` sets the light Thing's sprite light: `FarDist` to `Distance * 2.5` and
//! `Color` to (0.7, 0.7, 1, 1), the class's only vector constant. `SetDistance__f` stores
//! the distance and sets `FarDist` again; `SetTeamFilter__i` stores 0 to 4 and ignores any
//! other value; `SwitchPerformReaction__b` does nothing. The launch itself is the engine's.
//! Left out: the create-mode tweak menu (`UpdateUI__`). A field the level does not store
//! reads as its `.init__` value (`Distance` 1, `TeamFilter` 0, `AnimationFactor` 0).

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{site, NativeResult, NativeScript, NativeStep};

/// The script GUID of `Launcher` (FOUND: the 01.33 patch's type tables).
pub const LAUNCHER_SCRIPT: u32 = 0x12d39;

const LAUNCHER: &str = "Launcher";
/// `OnCreate__`'s vector constant, the sprite light's colour.
const LIGHT_COLOUR: [f32; 4] = [0.7, 0.7, 1.0, 1.0];
/// The sprite light's far distance per unit of `Distance`.
const FAR_DIST_SCALE: f32 = 2.5;

pub struct LauncherScript;

impl NativeScript for LauncherScript {
    fn class(&self) -> &'static str {
        LAUNCHER
    }
    fn guid(&self) -> Option<u32> {
        Some(LAUNCHER_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["Tweakable", "Native", "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        site(host, LAUNCHER, "OnCreate__");
        let done = (|| {
            let distance = float(get(host, thing, "Distance")?);
            set_light(
                host,
                thing,
                "FarDist",
                Value::Float(distance * FAR_DIST_SCALE),
            )?;
            set_light(host, thing, "Color", Value::Vec4(LIGHT_COLOUR))
        })();
        Some(done.map(|()| NativeStep::Done(Value::Void)))
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        let arg = args.first().map(|(_, v)| v.clone());
        let done = |r: Result<(), String>| r.map(|()| NativeStep::Done(Value::Void));
        site(host, LAUNCHER, function);
        Some(match function {
            "GetDistance__" => get(host, thing, "Distance").map(NativeStep::Done),
            "SetDistance__f" => done(set_distance(host, thing, float(arg.unwrap_or_default()))),
            "GetTeamFilter__" => get(host, thing, "TeamFilter").map(NativeStep::Done),
            "SetTeamFilter__i" => {
                let filter = match arg {
                    Some(Value::Int(i)) => i,
                    _ => 0,
                };
                done(set_team_filter(host, thing, filter))
            }
            "SwitchPerformReaction__b" => Ok(NativeStep::Done(Value::Void)),
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, LAUNCHER, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "Distance" => Ok(Value::Float(1.0)),
            "TeamFilter" => Ok(Value::Int(0)),
            "AnimationFactor" => Ok(Value::Float(0.0)),
            "LightThing" => Ok(Value::Thing(None)),
            _ => Err(e),
        },
    }
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, LAUNCHER, field, value)
}

fn float(v: Value) -> f32 {
    match v {
        Value::Float(f) => f,
        _ => 0.0,
    }
}

/// `LightThing.SpriteLight.<field> = value`.
fn set_light(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let light = get(host, thing, "LightThing")?;
    let part = host.get_member(MemberKind::Native, &light, "Thing", "SpriteLight")?;
    host.set_member(MemberKind::RefPtr, &part, "PSpriteLight", field, value)
}

/// `SetDistance__f`.
fn set_distance(host: &mut dyn Host, thing: u32, distance: f32) -> Result<(), String> {
    set(host, thing, "Distance", Value::Float(distance))?;
    set_light(
        host,
        thing,
        "FarDist",
        Value::Float(distance * FAR_DIST_SCALE),
    )
}

/// `SetTeamFilter__i`: the five filters are stored, any other value is ignored.
fn set_team_filter(host: &mut dyn Host, thing: u32, filter: i32) -> Result<(), String> {
    if (0..=4).contains(&filter) {
        set(host, thing, "TeamFilter", Value::Int(filter))?;
    }
    Ok(())
}
