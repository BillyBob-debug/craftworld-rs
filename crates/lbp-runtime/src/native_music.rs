//! D506: `TriggerMusic` (script g00004750, base `Tweakable` g00004c8e, then `Native`), the
//! music player Story levels start their interactive music with, as a [`NativeScript`] port.
//!
//! FOUND (the 01.33 script listing, research copy only): `Play__fi(fade, player)` calls
//! `Audio.SetInteractiveMusic(FSB, SettingsFile, 1, StartPoint, player, this)`, then sets
//! the six stereo pairs to `Pair_<n>_Volume` with a fade of -1 when `fade` is negative and 0
//! otherwise. `Enter__Q5Thing` plays (-1, player 0) when music is safe to trigger and input
//! 0 is unwired; `SwitchPerformReaction__b` plays the same when music is safe to trigger and
//! input 0's binary activation or the reaction's argument is on. `Mute__` sets every pair
//! to 0 with a fade of 1. `SetStartPoint__f` moves the music to the new start point;
//! `SetMixerChannel__if` stores the volume of the pair on that slider and plays (0, 2);
//! `GetMixerChannel__i` reads it back (0 when no pair is on the slider).
//! Left out: the create-mode tweak menu and preview (`UpdateUI__`, `StartTweakingObject__i`,
//! `StopTweakingObject__`, `OnDestroy__`'s stop of a tweak), the editor's radius drawing,
//! and `SetMixerChannel__if`'s sync-point branch (its condition is the constant false).
//! A field the level does not store reads as its `.init__` value (`StartPoint` 0, each
//! `Pair_<n>_Volume` 1, `HideInPlayMode` false).

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{as_bool, as_int, call, site, NativeResult, NativeScript, NativeStep};

/// The script GUID of `TriggerMusic` (FOUND: the 01.33 patch's type tables).
pub const TRIGGER_MUSIC_SCRIPT: u32 = 0x4750;

const TRIGGER_MUSIC: &str = "TriggerMusic";
const PAIRS: [&str; 6] = [
    "Pair_0_Volume",
    "Pair_1_Volume",
    "Pair_2_Volume",
    "Pair_3_Volume",
    "Pair_4_Volume",
    "Pair_5_Volume",
];

pub struct TriggerMusicScript;

impl NativeScript for TriggerMusicScript {
    fn class(&self) -> &'static str {
        TRIGGER_MUSIC
    }
    fn guid(&self) -> Option<u32> {
        Some(TRIGGER_MUSIC_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["Tweakable", "Native", "Thing"]
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
        site(host, TRIGGER_MUSIC, function);
        Some(match function {
            "Enter__Q5Thing" => done(enter(host, thing)),
            "SwitchPerformReaction__b" => done(switch_reaction(host, thing, as_bool(&arg(0)))),
            "Play__fi" => done(play(host, thing, float(&arg(0)), as_int(&arg(1)))),
            "Mute__" => done(mute(host, thing)),
            "GetFileGUID__" => field(host, "FSB"),
            "GetSettingsFileGUID__" => field(host, "SettingsFile"),
            "SetFileGUIDs__gg" => done(
                set(host, thing, "FSB", arg(0))
                    .and_then(|()| set(host, thing, "SettingsFile", arg(1))),
            ),
            "GetStartPoint__" => field(host, "StartPoint"),
            "SetStartPoint__f" => done(set_start_point(host, thing, arg(0))),
            "SetMixerChannel__if" => done(set_mixer_channel(host, thing, as_int(&arg(0)), arg(1))),
            "GetMixerChannel__i" => {
                get_mixer_channel(host, thing, as_int(&arg(0))).map(NativeStep::Done)
            }
            "GetHideInPlayModeOld__" => field(host, "HideInPlayMode"),
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, TRIGGER_MUSIC, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "StartPoint" => Ok(Value::Float(0.0)),
            "HideInPlayMode" | "StoppedUserMusic" => Ok(Value::Bool(false)),
            f if PAIRS.contains(&f) => Ok(Value::Float(1.0)),
            _ => Err(e),
        },
    }
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, TRIGGER_MUSIC, field, value)
}

fn float(v: &Value) -> f32 {
    match v {
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

fn safe_to_trigger(host: &mut dyn Host) -> Result<bool, String> {
    let safe = call(host, "Audio", "IsSafeToTriggerMusic__i", &[Value::Int(0)])?;
    Ok(as_bool(&safe))
}

/// `Enter__Q5Thing`: a player in the trigger plays the music unless input 0 is wired.
fn enter(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    if !safe_to_trigger(host)? {
        return Ok(());
    }
    let this = Value::Thing(Some(thing));
    let input = call(host, "Thing", "GetInput__Q5Thingi", &[this, Value::Int(0)])?;
    if matches!(input, Value::Thing(Some(_))) {
        return Ok(());
    }
    play(host, thing, -1.0, 0)
}

/// `SwitchPerformReaction__b`.
fn switch_reaction(host: &mut dyn Host, thing: u32, on: bool) -> Result<(), String> {
    if !safe_to_trigger(host)? {
        return Ok(());
    }
    let this = Value::Thing(Some(thing));
    let activation = call(
        host,
        "Thing",
        "GetSwitchActivationBinary__i",
        &[this, Value::Int(0)],
    )?;
    if as_bool(&activation) || on {
        play(host, thing, -1.0, 0)?;
    }
    Ok(())
}

/// `Play__fi`.
fn play(host: &mut dyn Host, thing: u32, fade: f32, player: i32) -> Result<(), String> {
    let pair_fade = if fade < 0.0 { -1.0 } else { 0.0 };
    let fsb = get(host, thing, "FSB")?;
    let settings = get(host, thing, "SettingsFile")?;
    let start = get(host, thing, "StartPoint")?;
    let args = [
        fsb.clone(),
        settings,
        Value::Float(1.0),
        start,
        Value::Int(player),
        Value::Thing(Some(thing)),
    ];
    call(host, "Audio", "SetInteractiveMusic__ggffiQ5Thing", &args)?;
    for (pair, field) in PAIRS.iter().enumerate() {
        let volume = get(host, thing, field)?;
        set_pair_volume(host, &fsb, pair, volume, pair_fade)?;
    }
    Ok(())
}

fn set_pair_volume(
    host: &mut dyn Host,
    fsb: &Value,
    pair: usize,
    volume: Value,
    fade: f32,
) -> Result<(), String> {
    let args = [
        fsb.clone(),
        Value::Int(pair as i32),
        volume,
        Value::Float(fade),
    ];
    call(
        host,
        "Audio",
        "SetInteractiveMusicStereoPairVolume__giff",
        &args,
    )
    .map(|_| ())
}

/// `Mute__`.
fn mute(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    let fsb = get(host, thing, "FSB")?;
    for pair in 0..PAIRS.len() {
        set_pair_volume(host, &fsb, pair, Value::Float(0.0), 1.0)?;
    }
    Ok(())
}

/// `SetStartPoint__f`.
fn set_start_point(host: &mut dyn Host, thing: u32, start: Value) -> Result<(), String> {
    set(host, thing, "StartPoint", start)?;
    let fsb = get(host, thing, "FSB")?;
    let start = get(host, thing, "StartPoint")?;
    call(
        host,
        "Audio",
        "SetInteractiveMusicPosition__gf",
        &[fsb, start],
    )
    .map(|_| ())
}

/// The pair on mixer slider `slider`, tested from pair 0 as the script does.
fn pair_on_slider(
    host: &mut dyn Host,
    thing: u32,
    slider: i32,
    pair: usize,
) -> Result<bool, String> {
    let fsb = get(host, thing, "FSB")?;
    let index = call(
        host,
        "Audio",
        "GetInteractiveMusicStereoPairSliderIndex__gi",
        &[fsb, Value::Int(pair as i32)],
    )?;
    Ok(as_int(&index) == slider)
}

/// `SetMixerChannel__if`: every pair on the slider takes the volume, then the music plays.
fn set_mixer_channel(
    host: &mut dyn Host,
    thing: u32,
    slider: i32,
    volume: Value,
) -> Result<(), String> {
    for (pair, field) in PAIRS.iter().enumerate() {
        if pair_on_slider(host, thing, slider, pair)? {
            set(host, thing, field, volume.clone())?;
        }
    }
    play(host, thing, 0.0, 2)
}

/// `GetMixerChannel__i`: the first pair on the slider, or 0.
fn get_mixer_channel(host: &mut dyn Host, thing: u32, slider: i32) -> Result<Value, String> {
    for (pair, field) in PAIRS.iter().enumerate() {
        if pair_on_slider(host, thing, slider, pair)? {
            return get(host, thing, field);
        }
    }
    Ok(Value::Float(0.0))
}
