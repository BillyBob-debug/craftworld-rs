//! D509: `WorldMooCamera` (script g00004d20, the world script of 220 story levels) and
//! its base `World` (g00002d28, then `BaseWorld`, `Tweakable`, `Thing`) as a
//! [`NativeScript`] port. `WorldMooCamera` itself has only `.init__`/`.ctor__` stubs, so
//! the port is `World`'s code.
//!
//! FOUND (the 01.33 script listing, research copy only): `OnCreate__` resets the
//! multiplier timers and counter, calls `World.SetPieChartMultiplier__i(0)` and then runs
//! `Update__` once a frame until `OnDestroy__` sets `Finish`. In play mode and before the
//! scoreboard, `Update__` goes to the scoreboard (`Game.GotoScoreboard__b(true)`) when the
//! score earned reaches `ScoreLimit`, and counts `Timer` against `TimeLimit` at 30 frames
//! a second (not during a cut-scene movie), playing the countdown sound each second of
//! the last 10 and the game-over sound at 0. Every frame it counts the multiplier timers
//! down (`UpdateMultipliers__`; a timer reaching 0 gives the bonuses) and a player's
//! button press (`Pad.IsJustPressed__Q5Thingii(yh, 1, 4)`) marks `LastScoreFrame`.
//! `OnCollectFluff__Q18Game.EPlayerNumberf` feeds the co-op multiplier (1, 30 frames) and
//! answers 0 during a scoreboard challenge. `ContributeToMultiplier__...`,
//! `ResetAllMultiplierTimers__`, `GiveAllBonuses__` and `GivePlayerBonus__...` set the
//! players' multiplier state and score through the `Game` natives as the script does
//! (counter cap 495, multiplier counter / 5 + 1, bonus bank x counter / 5). The getters,
//! setters, `Kick__`, `SetCurrentChallenge__Q5Thing` and `ShouldShowPieChart__` (mode 0,
//! a challenge, or a score in the last 600 frames) are ported too.
//! Left out: the `Bonus` popups the bonuses create and update and the `VCRDisplay`
//! (script objects with no port; the score itself is given), `BaseWorld.OnCreate__` and
//! `BaseWorld.Update__` (as in the `Pod` port), `Race_Start.OnCollectFluff__` (a race
//! challenge's fluff goes through unchanged), `NotifyDeath__...`, the direct-control
//! prompt list and `GetReverbStrings__` (script arrays), and the create-mode UI and
//! rendering. A field the level does not store reads as its `.init__` value.

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{
    as_bool, as_float, as_int, as_thing, call, site, NativeResult, NativeScript, NativeStep,
};

/// The script GUID of `WorldMooCamera` (FOUND: the 01.33 patch's type tables).
pub const WORLD_MOO_CAMERA_SCRIPT: u32 = 0x4d20;

const WORLD: &str = "World";
const MOO_CAMERA: &str = "WorldMooCamera";
/// The co-op multiplier counter's cap (multiplier x 100).
const COUNTER_CAP: i32 = 495;
const SCORE_GIVER_SOUND: &str = "/move/sfx/gameplay/multiplier";
const TIME_OUT_SOUND: &str = "gameplay/resource/time_out";

pub struct WorldMooCameraScript;

impl NativeScript for WorldMooCameraScript {
    fn class(&self) -> &'static str {
        MOO_CAMERA
    }
    fn guid(&self) -> Option<u32> {
        Some(WORLD_MOO_CAMERA_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["World", "BaseWorld", "Tweakable", "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(on_create(host, thing))
    }
    /// The `OnCreate__` loop after its yield.
    fn update(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(loop_step(host, thing))
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
        let value = |r: Result<Value, String>| r.map(NativeStep::Done);
        let field = |host: &mut dyn Host, f: &str| get(host, thing, f).map(NativeStep::Done);
        site(host, WORLD, function);
        Some(match function {
            "OnDestroy__" => done(set(host, thing, "Finish", Value::Bool(true))),
            "GetTeamScoreIncludesPlayerScores__" => field(host, "TeamScoreIncludesPlayerScores"),
            "SetTeamScoreIncludesPlayerScores__b" => done(set(
                host,
                thing,
                "TeamScoreIncludesPlayerScores",
                Value::Bool(as_bool(&arg(0))),
            )),
            "GetTimeLimit__" => field(host, "TimeLimit"),
            "SetTimeLimit__i" => done(
                set(host, thing, "TimeLimit", Value::Int(as_int(&arg(0))))
                    .and_then(|()| set(host, thing, "Timer", Value::Int(0))),
            ),
            "GetTimeElapsedTowardsTimeLimit__" => field(host, "Timer"),
            "GetScoreLimit__" => field(host, "ScoreLimit"),
            "SetScoreLimit__i" => done(set(host, thing, "ScoreLimit", Value::Int(as_int(&arg(0))))),
            "GetCurrentChallenge__" => field(host, "CurrentChallenge"),
            "GetScoreMultiplier__" => field(host, "CoopScoreMultiplierCounter"),
            "IsScoreLocked__" => value(
                get(host, thing, "CurrentChallenge")
                    .and_then(|c| is_score_board(host, c).map(Value::Bool)),
            ),
            "GetScoreEarnedTowardsScoreLimit__" => {
                value(score_earned(host, thing).map(Value::Float))
            }
            "Kick__" => done(set(host, thing, "CurrentChallenge", Value::Thing(None))),
            "SetCurrentChallenge__Q5Thing" => done(set_current_challenge(host, thing, arg(0))),
            "OnCollectFluff__Q18Game.EPlayerNumberf" => {
                value(on_collect_fluff(host, thing, arg(0), as_float(&arg(1))).map(Value::Float))
            }
            "ContributeToMultiplier__Q18Game.EPlayerNumberiiQ5Thingbb" => {
                let a = (
                    as_int(&arg(1)),
                    as_int(&arg(2)),
                    arg(3),
                    arg(4),
                    as_bool(&arg(5)),
                );
                done(contribute(host, thing, arg(0), a.0, a.1, a.2, a.3, a.4))
            }
            "ResetAllMultiplierTimers__" => done(reset_all_multiplier_timers(host, thing)),
            "UpdateMultipliers__" => done(update_multipliers(host, thing)),
            "GiveAllBonuses__" => done(give_all_bonuses(host, thing)),
            "GivePlayerBonus__Q18Game.EPlayerNumber" => {
                done(give_player_bonus(host, thing, arg(0)))
            }
            "ShouldShowPieChart__" => value(should_show_pie_chart(host, thing).map(Value::Bool)),
            "Update__" => done(update(host, thing)),
            "FixupDivergentVariables__" => Ok(NativeStep::Done(Value::Void)),
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, WORLD, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "CurrentChallenge" => Ok(Value::Thing(None)),
            "CoopScoreMultiplierCounter"
            | "CoopScoreMultiplierTimer"
            | "TimeLimit"
            | "Timer"
            | "ScoreLimit"
            | "PreviewIconFrame" => Ok(Value::Int(0)),
            "LastScoreFrame" => Ok(Value::Int(-1)),
            "LastMultiplierFromScoreGiver" | "Finish" | "GoneToScoreboard" => {
                Ok(Value::Bool(false))
            }
            "TeamScoreIncludesPlayerScores" => Ok(Value::Bool(true)),
            "ScoreRed" | "ScoreBlue" => Ok(Value::Float(0.0)),
            _ => Err(e),
        },
    }
}

fn get_int(host: &mut dyn Host, thing: u32, field: &str) -> Result<i32, String> {
    get(host, thing, field).map(|v| as_int(&v))
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, WORLD, field, value)
}

/// A member of this Thing's `PWorld` part (`GameMode`, `Frame`).
fn pworld(host: &mut dyn Host, thing: u32, field: &str) -> Result<i32, String> {
    let this = Value::Thing(Some(thing));
    let part = host.get_member(MemberKind::Native, &this, "Thing", "World")?;
    Ok(as_int(&host.get_member(
        MemberKind::RefPtr,
        &part,
        "PWorld",
        field,
    )?))
}

fn is_score_board(host: &mut dyn Host, challenge: Value) -> Result<bool, String> {
    call(host, "BaseWorld", "IsScoreBoard__Q5Thing", &[challenge]).map(|v| as_bool(&v))
}

/// The players as `Game.GetPlayerNumber__i` numbers them (the count is read again each
/// pass, as the script's loop test does).
fn player(host: &mut dyn Host, i: i32) -> Result<Option<Value>, String> {
    let n = as_int(&call(host, "Game", "GetNumPlayers__", &[])?);
    if i >= n {
        return Ok(None);
    }
    call(host, "Game", "GetPlayerNumber__i", &[Value::Int(i)]).map(Some)
}

fn game(host: &mut dyn Host, function: &str, args: &[Value]) -> Result<Value, String> {
    call(host, "Game", function, args)
}

fn set_pie_chart(host: &mut dyn Host, multiplier: i32) -> Result<(), String> {
    call(
        host,
        WORLD,
        "SetPieChartMultiplier__i",
        &[Value::Int(multiplier)],
    )
    .map(|_| ())
}

fn play_sound(host: &mut dyn Host, on: Value, sound: &str) -> Result<Value, String> {
    let args = [on, Value::Str(sound.into())];
    call(host, "Audio", "PlaySound__Q5ThingQ7StringA", &args)
}

fn set_sound_parameter(host: &mut dyn Host, handle: Value, value: f32) -> Result<(), String> {
    let args = [handle, Value::Int(0), Value::Float(value)];
    call(host, "Audio", "SetSoundParameter__Q11AudioHandleif", &args).map(|_| ())
}

/// The multiplier state the bonuses and resets clear for one player.
fn clear_player_multiplier(host: &mut dyn Host, p: &Value) -> Result<(), String> {
    let p = p.clone();
    game(
        host,
        "SetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberi",
        &[p.clone(), Value::Int(0)],
    )?;
    game(
        host,
        "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
        &[p.clone(), Value::Int(0)],
    )?;
    let null = Value::Thing(None);
    game(
        host,
        "SetPlayerScoreMultiplierSource__Q18Game.EPlayerNumberQ5Thing",
        &[p.clone(), null],
    )?;
    game(
        host,
        "SetPlayerScoreMultiplierVisible__Q18Game.EPlayerNumberb",
        &[p, Value::Bool(false)],
    )
    .map(|_| ())
}

fn on_create(host: &mut dyn Host, thing: u32) -> Result<NativeStep, String> {
    site(host, WORLD, "OnCreate__");
    // `BaseWorld.OnCreate__` is not ported; `Bonuses = new Bonus[0]` has no port.
    reset_all_multiplier_timers(host, thing)?;
    set(host, thing, "CoopScoreMultiplierCounter", Value::Int(0))?;
    set_pie_chart(host, 0)?;
    loop_step(host, thing)
}

/// The `OnCreate__` loop from its test of `Finish`.
fn loop_step(host: &mut dyn Host, thing: u32) -> Result<NativeStep, String> {
    site(host, WORLD, "OnCreate__");
    if as_bool(&get(host, thing, "Finish")?) {
        return Ok(NativeStep::Done(Value::Void));
    }
    update(host, thing)?;
    Ok(NativeStep::Yielded)
}

/// `Update__`.
fn update(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    site(host, WORLD, "Update__");
    let play = as_int(&game(host, "GetPlayOrEditMode__", &[])?) == 0;
    if play && !as_bool(&get(host, thing, "GoneToScoreboard")?) {
        let score_limit = get_int(host, thing, "ScoreLimit")?;
        if score_limit != 0 && score_earned(host, thing)? >= score_limit as f32 {
            go_to_scoreboard(host, thing)?;
        }
        let time_limit = get_int(host, thing, "TimeLimit")?;
        let movie = call(host, "BaseWorld", "IsCutSceneMovieActive__", &[])?;
        if time_limit != 0 && !as_bool(&movie) {
            let timer = get_int(host, thing, "Timer")? + 1;
            set(host, thing, "Timer", Value::Int(timer))?;
            if timer >= time_limit {
                go_to_scoreboard(host, thing)?;
            } else if timer % 30 == 0 {
                let secs = (time_limit - timer) / 30;
                let this = Value::Thing(Some(thing));
                if secs == 0 {
                    play_sound(host, this.clone(), "gameplay/lbp2/level_timer/gameover")?;
                }
                if secs > 0 && secs <= 10 {
                    play_sound(host, this, "gameplay/lbp2/level_timer/countdown")?;
                }
            }
        }
    }
    update_multipliers(host, thing)?;
    // The `Bonus` popups' `Update__` has no port.
    let mut j = 0;
    while let Some(p) = player(host, j)? {
        let yh = game(
            host,
            "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber",
            &[p],
        )?;
        if as_thing(&yh).is_some() {
            let args = [yh, Value::Int(1), Value::Int(4)];
            if as_bool(&call(host, "Pad", "IsJustPressed__Q5Thingii", &args)?) {
                let frame = pworld(host, thing, "Frame")?;
                set(host, thing, "LastScoreFrame", Value::Int(frame))?;
            }
        }
        j += 1;
    }
    // `VCR.Update__` and `BaseWorld.Update__` have no port.
    let frame = get_int(host, thing, "PreviewIconFrame")?;
    set(host, thing, "PreviewIconFrame", Value::Int(frame + 1))
}

fn go_to_scoreboard(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    set(host, thing, "GoneToScoreboard", Value::Bool(true))?;
    game(host, "GotoScoreboard__b", &[Value::Bool(true)]).map(|_| ())
}

/// `GetScoreEarnedTowardsScoreLimit__`.
fn score_earned(host: &mut dyn Host, thing: u32) -> Result<f32, String> {
    if pworld(host, thing, "GameMode")? == 5 {
        let red = as_float(&get(host, thing, "ScoreRed")?);
        let blue = as_float(&get(host, thing, "ScoreBlue")?);
        return Ok(red.max(blue));
    }
    let mut total = 0.0f32;
    let mut i = 0;
    while let Some(p) = player(host, i)? {
        let mode = pworld(host, thing, "GameMode")?;
        if matches!(mode, 0 | 1 | 3) {
            let score = game(host, "GetPlayerScore__Q18Game.EPlayerNumber", &[p])?;
            let score = as_float(&score);
            total = if mode == 1 {
                total.max(score)
            } else {
                total + score
            };
        }
        i += 1;
    }
    Ok(total)
}

/// `SetCurrentChallenge__Q5Thing` (its "challenge already in progress" assert does not
/// stop it).
fn set_current_challenge(host: &mut dyn Host, thing: u32, challenge: Value) -> Result<(), String> {
    if is_score_board(host, challenge.clone())? {
        reset_all_multiplier_timers(host, thing)?;
        give_all_bonuses(host, thing)?;
    }
    set(host, thing, "CurrentChallenge", challenge)?;
    let frame = pworld(host, thing, "Frame")?;
    set(host, thing, "LastScoreFrame", Value::Int(frame))
}

/// `OnCollectFluff__Q18Game.EPlayerNumberf`.
fn on_collect_fluff(host: &mut dyn Host, thing: u32, p: Value, amount: f32) -> Result<f32, String> {
    let frame = pworld(host, thing, "Frame")?;
    set(host, thing, "LastScoreFrame", Value::Int(frame))?;
    if pworld(host, thing, "GameMode")? != 1 {
        contribute(
            host,
            thing,
            p,
            1,
            30,
            Value::Thing(None),
            Value::Bool(true),
            false,
        )?;
    }
    let challenge = get(host, thing, "CurrentChallenge")?;
    // A `Race_Start` challenge's own `OnCollectFluff__` has no port.
    if as_thing(&challenge).is_some() && is_score_board(host, challenge)? {
        return Ok(0.0);
    }
    Ok(amount)
}

/// `ContributeToMultiplier__Q18Game.EPlayerNumberiiQ5Thingbb`.
#[allow(clippy::too_many_arguments)]
fn contribute(
    host: &mut dyn Host,
    thing: u32,
    p: Value,
    amount: i32,
    timer: i32,
    source: Value,
    visible: Value,
    from_score_giver: bool,
) -> Result<(), String> {
    set(
        host,
        thing,
        "LastMultiplierFromScoreGiver",
        Value::Bool(from_score_giver),
    )?;
    let visible = Value::Bool(as_bool(&visible));
    let (old, counter);
    if pworld(host, thing, "GameMode")? != 1 {
        let c = get_int(host, thing, "CoopScoreMultiplierCounter")?;
        old = c / 5 + 1;
        let c = (c + amount).min(COUNTER_CAP);
        set(host, thing, "CoopScoreMultiplierTimer", Value::Int(timer))?;
        set(host, thing, "CoopScoreMultiplierCounter", Value::Int(c))?;
        counter = c;
        let mut i = 0;
        while let Some(q) = player(host, i)? {
            let pairs: [(&str, Value); 4] = [
                (
                    "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
                    Value::Int(timer),
                ),
                (
                    "SetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberi",
                    Value::Int(c),
                ),
                (
                    "SetPlayerScoreMultiplierSource__Q18Game.EPlayerNumberQ5Thing",
                    source.clone(),
                ),
                (
                    "SetPlayerScoreMultiplierVisible__Q18Game.EPlayerNumberb",
                    visible.clone(),
                ),
            ];
            for (function, v) in pairs {
                game(host, function, &[q.clone(), v])?;
            }
            i += 1;
        }
    } else {
        let m = game(
            host,
            "GetPlayerScoreMultiplier__Q18Game.EPlayerNumber",
            std::slice::from_ref(&p),
        )?;
        old = as_int(&m);
        let pairs: [(&str, Value); 3] = [
            (
                "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
                Value::Int(timer),
            ),
            (
                "SetPlayerScoreMultiplierSource__Q18Game.EPlayerNumberQ5Thing",
                source,
            ),
            (
                "SetPlayerScoreMultiplierVisible__Q18Game.EPlayerNumberb",
                visible,
            ),
        ];
        for (function, v) in pairs {
            game(host, function, &[p.clone(), v])?;
        }
        let args = [p.clone(), Value::Int(amount), Value::Int(COUNTER_CAP)];
        let c = game(
            host,
            "IncrementPlayerScoreMultiplierCounter__Q18Game.EPlayerNumberii",
            &args,
        )?;
        counter = as_int(&c);
    }
    let new = counter / 5 + 1;
    if new >= 2 {
        set_pie_chart(host, new)?;
    }
    if new != old {
        let yh = game(
            host,
            "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber",
            &[p],
        )?;
        let h = play_sound(host, yh, "gameplay/resource/collect_special")?;
        let pitch_args = [Value::Int(counter / 5 - 1), Value::Int(10)];
        let pitch = as_int(&call(host, "Audio", "GetPitchParameter__ii", &pitch_args)?);
        set_sound_parameter(host, h, pitch as f32)?;
    }
    Ok(())
}

/// `ResetAllMultiplierTimers__` (the co-op counter is kept).
fn reset_all_multiplier_timers(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    set(host, thing, "CoopScoreMultiplierTimer", Value::Int(0))?;
    let mut i = 0;
    while let Some(p) = player(host, i)? {
        clear_player_multiplier(host, &p)?;
        i += 1;
    }
    Ok(())
}

/// `UpdateMultipliers__`.
fn update_multipliers(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    if pworld(host, thing, "GameMode")? != 1 {
        let timer = get_int(host, thing, "CoopScoreMultiplierTimer")?;
        if timer > 0 {
            let timer = timer - 1;
            set(host, thing, "CoopScoreMultiplierTimer", Value::Int(timer))?;
            let mut i = 0;
            while let Some(p) = player(host, i)? {
                let args = [p, Value::Int(timer)];
                game(
                    host,
                    "SetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumberi",
                    &args,
                )?;
                i += 1;
            }
            if timer == 0 {
                give_all_bonuses(host, thing)?;
            }
        }
        return Ok(());
    }
    let mut i = 0;
    while let Some(p) = player(host, i)? {
        let t = game(
            host,
            "GetPlayerScoreMultiplierTimer__Q18Game.EPlayerNumber",
            std::slice::from_ref(&p),
        )?;
        if as_int(&t) > 0 {
            let t = game(
                host,
                "DecrementPlayerScoreMultiplierTimer__Q18Game.EPlayerNumber",
                std::slice::from_ref(&p),
            )?;
            if as_int(&t) == 0 {
                give_player_bonus(host, thing, p)?;
            }
        }
        i += 1;
    }
    Ok(())
}

/// The bonus sound, unless this is a press demo.
fn bonus_sound(host: &mut dyn Host, thing: u32, param: i32) -> Result<(), String> {
    if as_bool(&game(host, "IsPressDemo__", &[])?) {
        return Ok(());
    }
    let giver = as_bool(&get(host, thing, "LastMultiplierFromScoreGiver")?);
    let sound = if giver {
        SCORE_GIVER_SOUND
    } else {
        TIME_OUT_SOUND
    };
    let h = play_sound(host, Value::Thing(None), sound)?;
    set_sound_parameter(host, h, param as f32)
}

/// A player with a bonus bank above 0 and a counter of 5 or more gets the bank x
/// counter / 5 added to their score.
fn bank_bonus(host: &mut dyn Host, p: &Value, counter: i32, mult: i32) -> Result<bool, String> {
    let bank = call(
        host,
        WORLD,
        "ConsumeScoreBonusBank__i",
        std::slice::from_ref(p),
    )?;
    let bank = as_float(&bank);
    if bank > 0.0 && counter >= 5 {
        call(
            host,
            WORLD,
            "TelemetryMultiplier__ii",
            &[Value::Int(mult), p.clone()],
        )?;
        call(
            host,
            WORLD,
            "TrophyMultiplier__ii",
            &[p.clone(), Value::Int(mult)],
        )?;
        let score = game(
            host,
            "GetPlayerScore__Q18Game.EPlayerNumber",
            std::slice::from_ref(p),
        )?;
        let score = as_float(&score) + bank * (counter / 5) as f32;
        game(
            host,
            "SetPlayerScore__Q18Game.EPlayerNumberf",
            &[p.clone(), Value::Float(score)],
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn has_avatar(host: &mut dyn Host, p: &Value) -> Result<bool, String> {
    let yh = game(
        host,
        "GetYellowheadFromPlayerNumber__Q18Game.EPlayerNumber",
        std::slice::from_ref(p),
    )?;
    let worm = game(
        host,
        "IsYellowheadInWormHoleFromPlayerNumber__Q18Game.EPlayerNumber",
        std::slice::from_ref(p),
    )?;
    Ok(as_thing(&yh).is_some() || as_bool(&worm))
}

/// `GiveAllBonuses__`.
fn give_all_bonuses(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    if pworld(host, thing, "GameMode")? == 1 {
        let mut i = 0;
        while let Some(p) = player(host, i)? {
            give_player_bonus(host, thing, p)?;
            i += 1;
        }
        return Ok(());
    }
    let counter = get_int(host, thing, "CoopScoreMultiplierCounter")?;
    if counter >= 5 {
        bonus_sound(host, thing, (counter / 5 - 1).min(10))?;
        let mult = counter / 5 + 1;
        let mut i = 0;
        while let Some(p) = player(host, i)? {
            if has_avatar(host, &p)? && bank_bonus(host, &p, counter, mult)? {
                clear_player_multiplier(host, &p)?;
            }
            i += 1;
        }
    }
    set(host, thing, "CoopScoreMultiplierCounter", Value::Int(0))?;
    set_pie_chart(host, 0)
}

/// `GivePlayerBonus__Q18Game.EPlayerNumber`.
fn give_player_bonus(host: &mut dyn Host, thing: u32, p: Value) -> Result<(), String> {
    if pworld(host, thing, "GameMode")? != 1 || !has_avatar(host, &p)? {
        return set_pie_chart(host, 0);
    }
    let counter = game(
        host,
        "GetPlayerScoreMultiplierCounter__Q18Game.EPlayerNumber",
        std::slice::from_ref(&p),
    )?;
    let counter = as_int(&counter);
    bonus_sound(host, thing, (counter / 5 - 1).min(10))?;
    let mult = game(
        host,
        "GetPlayerScoreMultiplier__Q18Game.EPlayerNumber",
        std::slice::from_ref(&p),
    )?;
    bank_bonus(host, &p, counter, as_int(&mult))?;
    clear_player_multiplier(host, &p)?;
    set_pie_chart(host, 0)
}

/// `ShouldShowPieChart__`.
fn should_show_pie_chart(host: &mut dyn Host, thing: u32) -> Result<bool, String> {
    if pworld(host, thing, "GameMode")? != 0 {
        return Ok(false);
    }
    if as_thing(&get(host, thing, "CurrentChallenge")?).is_some() {
        return Ok(true);
    }
    let last = get_int(host, thing, "LastScoreFrame")?;
    if last == -1 {
        return Ok(false);
    }
    Ok(pworld(host, thing, "Frame")? - last <= 600)
}
