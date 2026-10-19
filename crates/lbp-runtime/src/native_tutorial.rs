//! D508: `TutorialMonitor` (script g00006b3c, base `Thing`), the screen a level plays a
//! tutorial video on, as a [`NativeScript`] port.
//!
//! FOUND (the 01.33 script listing, research copy only): `OnCreate__` sets `VideoHandle` to
//! -1. `SetVideoGUID__g` and `GetVideoGUID__` store and read `VideoFile`;
//! `GetVideoHandle__` reads `VideoHandle`; `GetRTTRes__` answers (640, 360, 0, 0).
//! `PlayVideo__` calls `Game.PlayTutorialMonitorVideo__gQ5Thing(VideoFile, this)`;
//! `SwitchPerformReaction__b` makes the same call when input 0's binary activation is on
//! and `VideoHandle` is still -1 (the reaction's argument is not read). `OnDestroy__`,
//! `OnScriptReloaded__` and `RenderToTexture__ii` are empty.
//! Left out: nothing in the listing; the class's static natives (`GetMonitorVideoHandle__`,
//! `PlayTutorialVideo__`, `RestartMonitorVideo__`) have no bytecode and are not ported.
//! `Game.PlayTutorialMonitorVideo__gQ5Thing` is not bound yet, so the call is only counted
//! (no video plays). A field the level does not store reads as its `.init__` value
//! (`VideoHandle` -1, `CurrentVolume` 1, `FadeOutTime` 60, `FadeInTime` 15).

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{as_bool, as_int, call, site, NativeResult, NativeScript, NativeStep};

/// The script GUID of `TutorialMonitor` (FOUND: the 01.33 patch's type tables).
pub const TUTORIAL_MONITOR_SCRIPT: u32 = 0x6b3c;

const TUTORIAL_MONITOR: &str = "TutorialMonitor";

pub struct TutorialMonitorScript;

impl NativeScript for TutorialMonitorScript {
    fn class(&self) -> &'static str {
        TUTORIAL_MONITOR
    }
    fn guid(&self) -> Option<u32> {
        Some(TUTORIAL_MONITOR_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        site(host, TUTORIAL_MONITOR, "OnCreate__");
        let done = set(host, thing, "VideoHandle", Value::Int(-1));
        Some(done.map(|()| NativeStep::Done(Value::Void)))
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        let arg = |n: usize| args.get(n).map(|(_, v)| v.clone()).unwrap_or_default();
        let done = |r: Result<(), String>| r.map(|()| NativeStep::Done(Value::Void));
        let field = |host: &mut dyn Host, f: &str| get(host, thing, f).map(NativeStep::Done);
        site(host, TUTORIAL_MONITOR, function);
        Some(match function {
            "OnDestroy__" | "OnScriptReloaded__" | "RenderToTexture__ii" => {
                Ok(NativeStep::Done(Value::Void))
            }
            "SetVideoGUID__g" => done(set(host, thing, "VideoFile", arg(0))),
            "GetVideoGUID__" => field(host, "VideoFile"),
            "GetVideoHandle__" => field(host, "VideoHandle"),
            "GetRTTRes__" => Ok(NativeStep::Done(Value::Vec4([640.0, 360.0, 0.0, 0.0]))),
            "PlayVideo__" => done(play_video(host, thing)),
            "SwitchPerformReaction__b" => done(switch_reaction(host, thing)),
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, TUTORIAL_MONITOR, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "VideoHandle" => Ok(Value::Int(-1)),
            "CurrentVolume" => Ok(Value::Float(1.0)),
            "FadeOutTime" => Ok(Value::Float(60.0)),
            "FadeInTime" => Ok(Value::Float(15.0)),
            _ => Err(e),
        },
    }
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, TUTORIAL_MONITOR, field, value)
}

/// `PlayVideo__`.
fn play_video(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    let video = get(host, thing, "VideoFile")?;
    let args = [video, Value::Thing(Some(thing))];
    call(host, "Game", "PlayTutorialMonitorVideo__gQ5Thing", &args).map(|_| ())
}

/// `SwitchPerformReaction__b`: input 0 on and no video started yet plays the video.
fn switch_reaction(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    let activation = call(
        host,
        "Thing",
        "GetSwitchActivationBinary__i",
        &[this, Value::Int(0)],
    )?;
    if as_bool(&activation) && as_int(&get(host, thing, "VideoHandle")?) == -1 {
        play_video(host, thing)?;
    }
    Ok(())
}
