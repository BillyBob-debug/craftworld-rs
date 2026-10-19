//! D510: `Score_Board` (script g00002d4f, the scoreboard at the end of a story level; its
//! base is `Tweakable`) as a [`NativeScript`] port of the game side: who has reached the
//! finish, the hand-over of the world's challenge and the end of the level.
//!
//! FOUND (the 01.33 script listing, research copy only): `OnCreate__` (no `Update__`) runs
//! `DoCoopScoreboard__`, whose first loop calls `DuringGameUpdate__` once a frame until
//! `Finished`. In edit mode the prologue clears `Finished` and deletes the `MagicWall`.
//! `Enter__Q5Thing` (play mode only) adds the entering Sackboy's player number to
//! `Finishers`; `Leave__Q5Thing` removes it. In play mode `DuringGameUpdate__` drops the
//! finishers no longer connected and, when every Sackboy (`GetNumYellowHeadAvatars__`) is
//! a finisher, takes the world's challenge (it returns while another scoreboard holds it),
//! adds the Vita cross-play score to the party leader's, locks the score
//! (`Score_Board.SetScoreLocked__b(true)`), sets `Finished` and `ActivationFrame` (the
//! world's `PWorld.Frame`). `OnDestroy__` clears the posted score, undoes the pie-chart
//! flags, calls `Thing.AddToDeleteList__Q5Thing(null)` when there is a `MagicWall` (the
//! script passes null, not the wall) and ends the camera override. `Kick__` clears
//! `Finished`. `CanTweak__i` and `AttemptCancel__` answer true.
//!
//! INFERRED (an owner decision, D510): after `Finished` the game runs a presentation of
//! screens; the port runs only its game calls and stops: `Score_Board.PostScore__()`,
//! `Score_Board.TrophyWinner__i` for each player `Score_Board.IsWinner__i` holds for
//! (when there is no `MagicWall`), `Game.SetLevelCompleted__()` and
//! `Score_Board.EnterEndLevelFlow__()`, the call the presentation ends with.
//!
//! `Finishers` is a script array; the port keeps it in the same member as an int bit
//! mask of player numbers (only membership and the count are used).
//! Left out: the screens (`ScoreBoardScreen` script objects), the Gooey, the presentation
//! (waits, confetti, zoom, the activate sound, the `MagicWall` and the checkpoint walk),
//! `Race_Start.AbortRace__` when a race holds the challenge (no port), rendering and the
//! create-mode tweak UI. A field the level does not store reads as its `.init__` value.

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{
    as_bool, as_float, as_int, as_thing, call, site, NativeResult, NativeScript, NativeStep,
};

/// The script GUID of `Score_Board` (FOUND: the 01.33 patch's type tables; the constant
/// `BaseWorld.IsScoreBoard__Q5Thing` tests).
pub const SCORE_BOARD_SCRIPT: u32 = lbp_script_vm::natives::SCORE_BOARD_SCRIPT;

const SCORE_BOARD: &str = "Score_Board";
/// `Resource.LoadMeshByFilename__g`'s confetti mesh.
const CONFETTI_MESH: i32 = 39151;

pub struct ScoreBoardScript;

impl NativeScript for ScoreBoardScript {
    fn class(&self) -> &'static str {
        SCORE_BOARD
    }
    fn guid(&self) -> Option<u32> {
        Some(SCORE_BOARD_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["Tweakable", "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(on_create(host, thing))
    }
    /// The `DoCoopScoreboard__` loop after its yield.
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
        site(host, SCORE_BOARD, function);
        Some(match function {
            "Kick__" => done(set(host, thing, "Finished", Value::Bool(false))),
            "Enter__Q5Thing" => done(enter(host, thing, arg(0))),
            "Leave__Q5Thing" => done(leave(host, thing, arg(0))),
            "OnDestroy__" => done(on_destroy(host, thing)),
            "CanTweak__i" | "AttemptCancel__" => Ok(NativeStep::Done(Value::Bool(true))),
            "GetLastRewardIndex__" => get(host, thing, "LastRewardIndex").map(NativeStep::Done),
            "FixupDivergentVariables__" => {
                let mesh = [Value::Int(CONFETTI_MESH)];
                done(call(host, "Resource", "LoadMeshByFilename__g", &mesh).map(|_| ()))
            }
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, SCORE_BOARD, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "Finished"
            | "NothingNewToSee"
            | "VitaScoreRequested"
            | "PieChartHideBubble"
            | "PieChartFocusWinner" => Ok(Value::Bool(false)),
            "ActivationFrame" | "LastRewardIndex" | "Finishers" => Ok(Value::Int(0)),
            "CamPlayerNumber" => Ok(Value::Int(-1)),
            "ZoomedIn" => Ok(Value::Float(0.0)),
            "MagicWall" => Ok(Value::Thing(None)),
            _ => Err(e),
        },
    }
}

fn get_bool(host: &mut dyn Host, thing: u32, field: &str) -> Result<bool, String> {
    get(host, thing, field).map(|v| as_bool(&v))
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, SCORE_BOARD, field, value)
}

fn mode(host: &mut dyn Host) -> Result<i32, String> {
    call(host, "Game", "GetPlayOrEditMode__", &[]).map(|v| as_int(&v))
}

fn finishers(host: &mut dyn Host, thing: u32) -> Result<u32, String> {
    get(host, thing, "Finishers").map(|v| as_int(&v) as u32)
}

fn set_finishers(host: &mut dyn Host, thing: u32, mask: u32) -> Result<(), String> {
    set(host, thing, "Finishers", Value::Int(mask as i32))
}

/// The bit of a player number in the `Finishers` mask; none outside 0-31.
fn bit(player: i32) -> u32 {
    if (0..32).contains(&player) {
        1 << player
    } else {
        0
    }
}

/// `Game.EPlayerNumberFromInt__i(t.YellowHead.PlayerNumber)` for a Thing with a
/// `PYellowHead`, else `None`.
fn yellowhead_player(host: &mut dyn Host, t: &Value) -> Result<Option<i32>, String> {
    let has = call(
        host,
        "PartList",
        "HasPYellowHead__Q5Thing",
        std::slice::from_ref(t),
    )?;
    if !as_bool(&has) {
        return Ok(None);
    }
    let head = host.get_member(MemberKind::Native, t, "Thing", "YellowHead")?;
    let n = host.get_member(MemberKind::RefPtr, &head, "PYellowHead", "PlayerNumber")?;
    let ep = call(host, "Game", "EPlayerNumberFromInt__i", &[n])?;
    Ok(Some(as_int(&ep)))
}

/// `OnCreate__` without the screens and the Gooey, then `DoCoopScoreboard__` up to its
/// first yield.
fn on_create(host: &mut dyn Host, thing: u32) -> Result<NativeStep, String> {
    site(host, SCORE_BOARD, "OnCreate__");
    let this = Value::Thing(Some(thing));
    call(host, "Thing", "GetWorld__", &[this])?;
    if mode(host)? == 1 {
        set(host, thing, "Finished", Value::Bool(false))?;
        let wall = get(host, thing, "MagicWall")?;
        if as_thing(&wall).is_some() {
            call(host, "Thing", "DeleteThing__Q5Thing", &[wall])?;
        }
    }
    set(host, thing, "VitaScoreRequested", Value::Bool(false))?;
    loop_step(host, thing)
}

/// One pass of `DoCoopScoreboard__`'s `while !Finished` loop, or its native tail.
fn loop_step(host: &mut dyn Host, thing: u32) -> Result<NativeStep, String> {
    site(host, SCORE_BOARD, "DoCoopScoreboard__");
    if get_bool(host, thing, "Finished")? {
        finish(host, thing)?;
        return Ok(NativeStep::Done(Value::Void));
    }
    during_game_update(host, thing)?;
    Ok(NativeStep::Yielded)
}

/// `Enter__Q5Thing`.
fn enter(host: &mut dyn Host, thing: u32, t: Value) -> Result<(), String> {
    if mode(host)? != 0 {
        return Ok(());
    }
    if let Some(p) = yellowhead_player(host, &t)? {
        let mask = finishers(host, thing)?;
        set_finishers(host, thing, mask | bit(p))?;
    }
    Ok(())
}

/// `Leave__Q5Thing` (no mode test).
fn leave(host: &mut dyn Host, thing: u32, t: Value) -> Result<(), String> {
    if as_thing(&t).is_none() {
        return Ok(());
    }
    if let Some(p) = yellowhead_player(host, &t)? {
        let mask = finishers(host, thing)?;
        set_finishers(host, thing, mask & !bit(p))?;
    }
    Ok(())
}

/// `DuringGameUpdate__` without the screens and the Gooey.
fn during_game_update(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    site(host, SCORE_BOARD, "DuringGameUpdate__");
    if get_bool(host, thing, "Finished")? || mode(host)? != 0 {
        return Ok(());
    }
    let before = finishers(host, thing)?;
    let mut mask = before;
    for p in 0..32 {
        if mask & bit(p) != 0 {
            let connected = call(host, "Game", "IsPlayerConnected__i", &[Value::Int(p)])?;
            if !as_bool(&connected) {
                mask &= !bit(p);
            }
        }
    }
    // D511: the script erases from its array, so the member is written only on a change.
    if mask != before {
        set_finishers(host, thing, mask)?;
    }
    let this = Value::Thing(Some(thing));
    let Some(world) = as_thing(&call(
        host,
        "Thing",
        "GetWorld__",
        std::slice::from_ref(&this),
    )?) else {
        return Ok(());
    };
    // `len(Finishers) > 0 &&` comes first: no avatar count while nobody is in.
    if mask == 0 {
        return Ok(());
    }
    let avatars = as_int(&call(host, "Game", "GetNumYellowHeadAvatars__", &[])?);
    if mask.count_ones() as i32 != avatars {
        return Ok(());
    }
    // `World.GetCurrentChallenge__`, a script function: the world's member.
    let world_value = Value::Thing(Some(world));
    let challenge = host
        .get_member(MemberKind::Thing, &world_value, "World", "CurrentChallenge")
        .unwrap_or(Value::Thing(None));
    if let Some(ch) = as_thing(&challenge) {
        let other_board = call(host, "BaseWorld", "IsScoreBoard__Q5Thing", &[challenge])?;
        if as_bool(&other_board) && ch != thing {
            return Ok(());
        }
        crate::native_world::set_current_challenge(host, world, Value::Thing(None))?;
    }
    crate::native_world::set_current_challenge(host, world, this)?;
    if as_bool(&call(
        host,
        "Network",
        "IsCrossPlayGameSynchronised__",
        &[],
    )?) {
        if !as_bool(&call(host, "Network", "ReceivedVitaScore__", &[])?) {
            if !get_bool(host, thing, "VitaScoreRequested")? {
                call(host, "Network", "RequestVitaScore__", &[])?;
                set(host, thing, "VitaScoreRequested", Value::Bool(true))?;
            }
            return Ok(());
        }
        let vita = call(host, "Network", "GetVitaScore__", &[])?;
        call(host, "Network", "ClearVitaScore__", &[])?;
        let p = call(host, "Game", "GetPartyLeadersPlayerNumber__", &[])?;
        let score = call(
            host,
            "Game",
            "GetPlayerScore__Q18Game.EPlayerNumber",
            std::slice::from_ref(&p),
        )?;
        let total = as_float(&score) + as_float(&vita);
        call(
            host,
            "Game",
            "SetPlayerScore__Q18Game.EPlayerNumberf",
            &[p, Value::Float(total)],
        )?;
    }
    call(host, SCORE_BOARD, "SetScoreLocked__b", &[Value::Bool(true)])?;
    set(host, thing, "Finished", Value::Bool(true))?;
    let part = host.get_member(MemberKind::Native, &world_value, "Thing", "World")?;
    let frame = host.get_member(MemberKind::RefPtr, &part, "PWorld", "Frame")?;
    set(host, thing, "ActivationFrame", Value::Int(as_int(&frame)))
}

/// The native tail after `Finished` (INFERRED, D510): the presentation's game calls.
fn finish(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    call(host, SCORE_BOARD, "PostScore__", &[])?;
    let no_wall = as_thing(&get(host, thing, "MagicWall")?).is_none();
    let mut i = 0;
    while i < as_int(&call(host, "Game", "GetNumPlayers__", &[])?) {
        let p = call(host, "Game", "GetPlayerNumber__i", &[Value::Int(i)])?;
        let winner = call(host, SCORE_BOARD, "IsWinner__i", std::slice::from_ref(&p))?;
        if as_bool(&winner) && no_wall {
            call(host, SCORE_BOARD, "TrophyWinner__i", &[p])?;
        }
        i += 1;
    }
    call(host, "Game", "SetLevelCompleted__", &[])?;
    call(host, SCORE_BOARD, "EnterEndLevelFlow__", &[])?;
    Ok(())
}

/// `OnDestroy__` without the screens and the Gooey.
fn on_destroy(host: &mut dyn Host, thing: u32) -> Result<(), String> {
    call(host, SCORE_BOARD, "ClearPostedScore__", &[])?;
    if get_bool(host, thing, "PieChartFocusWinner")? {
        let args = [Value::Bool(false)];
        call(host, SCORE_BOARD, "SetPieChartFocusWinner__b", &args)?;
    }
    if get_bool(host, thing, "PieChartHideBubble")? {
        let args = [Value::Bool(true)];
        call(host, SCORE_BOARD, "SetPieChartShowBubble__b", &args)?;
    }
    if as_thing(&get(host, thing, "MagicWall")?).is_some() {
        // FOUND: the script passes a null Thing, so the wall is never queued.
        call(
            host,
            "Thing",
            "AddToDeleteList__Q5Thing",
            &[Value::Thing(None)],
        )?;
    }
    let args = [Value::Bool(false), Value::Float(0.05)];
    call(host, "Cam", "SetOverrideCameraActivation__bf", &args).map(|_| ())
}
