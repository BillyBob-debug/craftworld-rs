//! D507: `PostItNote` (script g000174fc, base `Tweakable`, then `Native`), the sticky note
//! a level shows a text bubble on, and its three unlock subclasses
//! (`PostItNoteUnlockStickers` g0001e9cf, `PostItNoteUnlockSuicide` g0001e9d0,
//! `PostItNoteUnlockCostume` g0001e9b7) as [`NativeScript`] ports.
//!
//! FOUND (the 01.33 script listing, research copy only): `OnCreate__` sets `NeedInit`;
//! `NeedsRenderInPlayMode__` answers `PostItNote.IsVisibleInPlayMode__` on this Thing.
//! `GetTranslatedText__b(filter)` translates `Text` as a LAMS key when the note was not
//! typed by a player (`OldUserEnteredText` when the key is not in LAMS); a typed note
//! answers `Text` when `filter` is on, else `OriginalText` (`Text` when that is null or
//! empty). `GetSpeechText__` filters as `Messaging_ShouldFilterMessageStrings__` says.
//! `SetSpeechText__Q6StringQ6String(original, text)` stores both cut to 256 units and marks
//! the note typed; `SetLAMSKey__Q6String` stores the key upper-cased and clears the typed
//! state; `WebternateSetText__Q6String` keeps a typed text in `OldUserEnteredText` first.
//! Each unlock subclass's `SwitchPerformReaction__b(true)` sets its progression flag
//! (Stickers 4864, Suicide 2048, Costume 128), on every call that is on.
//! Left out: `Render__Q17Thing.ERenderType`, `UpdateBubble__` and `RenderBubble__` (the
//! bubble drawing; nothing in the runtime or viewer calls a script's `Render__` yet) and
//! the create-mode text entry (`UpdateUI__Q5GooeyQ6Poppet`, `DoTextEntryButton__...`).
//! INFERRED: `SetLAMSKey__`'s call of the inherited `SetNeedsUpdate__b(true)` sets
//! `NeedUpdate` to 1, as `SetSpeechText__` does directly. A field the level does not store
//! reads as its `.init__` value (`UserEntered` false, `BubbleVisible` true, `NeedInit`
//! true, the strings null).

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{as_bool, call, site, NativeResult, NativeScript, NativeStep};

/// The script GUID of `PostItNote` (FOUND: the 01.33 patch's type tables).
pub const POST_IT_NOTE_SCRIPT: u32 = 0x174fc;
/// The script GUID of `PostItNoteUnlockStickers`.
pub const POST_IT_NOTE_UNLOCK_STICKERS_SCRIPT: u32 = 0x1e9cf;
/// The script GUID of `PostItNoteUnlockSuicide`.
pub const POST_IT_NOTE_UNLOCK_SUICIDE_SCRIPT: u32 = 0x1e9d0;
/// The script GUID of `PostItNoteUnlockCostume`.
pub const POST_IT_NOTE_UNLOCK_COSTUME_SCRIPT: u32 = 0x1e9b7;

const POST_IT_NOTE: &str = "PostItNote";
const MAX_TEXT: i32 = 256;

/// `PostItNote` itself (`unlock` None) or one of its unlock subclasses.
pub struct PostItNoteScript {
    class: &'static str,
    guid: u32,
    /// The progression flag `SwitchPerformReaction__b(true)` sets.
    unlock: Option<i32>,
}

pub const POST_IT_NOTE_PORT: PostItNoteScript = PostItNoteScript {
    class: POST_IT_NOTE,
    guid: POST_IT_NOTE_SCRIPT,
    unlock: None,
};
pub const POST_IT_NOTE_UNLOCK_STICKERS: PostItNoteScript = PostItNoteScript {
    class: "PostItNoteUnlockStickers",
    guid: POST_IT_NOTE_UNLOCK_STICKERS_SCRIPT,
    unlock: Some(4864),
};
pub const POST_IT_NOTE_UNLOCK_SUICIDE: PostItNoteScript = PostItNoteScript {
    class: "PostItNoteUnlockSuicide",
    guid: POST_IT_NOTE_UNLOCK_SUICIDE_SCRIPT,
    unlock: Some(2048),
};
pub const POST_IT_NOTE_UNLOCK_COSTUME: PostItNoteScript = PostItNoteScript {
    class: "PostItNoteUnlockCostume",
    guid: POST_IT_NOTE_UNLOCK_COSTUME_SCRIPT,
    unlock: Some(128),
};

impl NativeScript for PostItNoteScript {
    fn class(&self) -> &'static str {
        self.class
    }
    fn guid(&self) -> Option<u32> {
        Some(self.guid)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        if self.unlock.is_some() {
            &["PostItNote", "Tweakable", "Native", "Thing"]
        } else {
            &["Tweakable", "Native", "Thing"]
        }
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        site(host, POST_IT_NOTE, "OnCreate__");
        let done = set(host, thing, "NeedInit", Value::Bool(true));
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
        site(host, self.class, function);
        if let Some(flag) = self.unlock {
            if function == "SwitchPerformReaction__b" {
                return Some(done(unlock(host, flag, as_bool(&arg(0)))));
            }
        }
        Some(match function {
            "OnDestroy__" => Ok(NativeStep::Done(Value::Void)),
            "NeedsRenderInPlayMode__" => call(
                host,
                POST_IT_NOTE,
                "IsVisibleInPlayMode__",
                &[Value::Thing(Some(thing))],
            )
            .map(NativeStep::Done),
            "GetTranslatedText__b" => {
                translated_text(host, thing, as_bool(&arg(0))).map(NativeStep::Done)
            }
            "GetSpeechText__" => speech_text(host, thing).map(NativeStep::Done),
            "SetSpeechText__Q6StringQ6String" => done(set_speech_text(host, thing, arg(0), arg(1))),
            "SetLAMSKey__Q6String" => done(set_lams_key(host, thing, arg(0))),
            "GetLAMSKey__" | "WebternateGetText__" => field(host, "Text"),
            "GetBubbleVisible__" => field(host, "BubbleVisible"),
            "SetBubbleVisible__b" => done(
                set(host, thing, "BubbleVisible", Value::Bool(as_bool(&arg(0))))
                    .and_then(|()| set(host, thing, "NeedInit", Value::Bool(true))),
            ),
            "WebternateSetText__Q6String" => done(webternate_set_text(host, thing, arg(0))),
            "GetUserEnteredText__" => user_entered_text(host, thing).map(NativeStep::Done),
            _ => return None,
        })
    }
}

/// A member of this Thing's script, or its `.init__` value when the level stores none.
fn get(host: &mut dyn Host, thing: u32, field: &str) -> Result<Value, String> {
    let this = Value::Thing(Some(thing));
    match host.get_member(MemberKind::Thing, &this, POST_IT_NOTE, field) {
        Ok(v) => Ok(v),
        Err(e) => match field {
            "UserEntered" => Ok(Value::Bool(false)),
            "BubbleVisible" | "NeedInit" => Ok(Value::Bool(true)),
            "Text" | "OriginalText" | "OldUserEnteredText" => Ok(Value::Object(None)),
            "NeedUpdate" => Ok(Value::Int(0)),
            _ => Err(e),
        },
    }
}

fn set(host: &mut dyn Host, thing: u32, field: &str, value: Value) -> Result<(), String> {
    let this = Value::Thing(Some(thing));
    host.set_member(MemberKind::Thing, &this, POST_IT_NOTE, field, value)
}

fn user_entered(host: &mut dyn Host, thing: u32) -> Result<bool, String> {
    get(host, thing, "UserEntered").map(|v| as_bool(&v))
}

fn sub_str(host: &mut dyn Host, s: Value) -> Result<Value, String> {
    let args = [s, Value::Int(0), Value::Int(MAX_TEXT)];
    call(host, "String", "SubStr__Q6Stringii", &args)
}

/// `GetTranslatedText__b`.
fn translated_text(host: &mut dyn Host, thing: u32, filter: bool) -> Result<Value, String> {
    let text = get(host, thing, "Text")?;
    if !user_entered(host, thing)? {
        let id = call(host, "LAMS", "MakeLamsKeyID__Q6String", &[text])?;
        if as_bool(&call(
            host,
            "LAMS",
            "IsInLams__i",
            std::slice::from_ref(&id),
        )?) {
            return call(host, "LAMS", "Translate__i", &[id]);
        }
        return get(host, thing, "OldUserEnteredText");
    }
    if filter {
        return Ok(text);
    }
    let original = get(host, thing, "OriginalText")?;
    let empty = call(
        host,
        "String",
        "IsNullOrEmpty__Q6String",
        std::slice::from_ref(&original),
    )?;
    Ok(if as_bool(&empty) { text } else { original })
}

/// `GetSpeechText__`.
fn speech_text(host: &mut dyn Host, thing: u32) -> Result<Value, String> {
    let filter = call(
        host,
        "GameMessagingManager",
        "Messaging_ShouldFilterMessageStrings__",
        &[],
    )?;
    translated_text(host, thing, as_bool(&filter))
}

/// `SetSpeechText__Q6StringQ6String(original, text)`.
fn set_speech_text(
    host: &mut dyn Host,
    thing: u32,
    original: Value,
    text: Value,
) -> Result<(), String> {
    let text = sub_str(host, text)?;
    set(host, thing, "Text", text)?;
    let original = sub_str(host, original)?;
    set(host, thing, "OriginalText", original)?;
    set(host, thing, "UserEntered", Value::Bool(true))?;
    set(host, thing, "NeedUpdate", Value::Int(1))?;
    set(host, thing, "NeedInit", Value::Bool(true))
}

/// `SetLAMSKey__Q6String`.
fn set_lams_key(host: &mut dyn Host, thing: u32, key: Value) -> Result<(), String> {
    let key = call(host, "String", "ToUpper__Q6String", &[key])?;
    set(host, thing, "Text", key)?;
    set(host, thing, "UserEntered", Value::Bool(false))?;
    set(host, thing, "OriginalText", Value::Str("".into()))?;
    set(host, thing, "OldUserEnteredText", Value::Str("".into()))?;
    set(host, thing, "NeedUpdate", Value::Int(1))
}

/// `WebternateSetText__Q6String`.
fn webternate_set_text(host: &mut dyn Host, thing: u32, text: Value) -> Result<(), String> {
    if user_entered(host, thing)? {
        set(host, thing, "UserEntered", Value::Bool(false))?;
        let old = get(host, thing, "Text")?;
        set(host, thing, "OldUserEnteredText", old)?;
    }
    set(host, thing, "Text", text)?;
    set(host, thing, "NeedInit", Value::Bool(true))
}

/// `GetUserEnteredText__`.
fn user_entered_text(host: &mut dyn Host, thing: u32) -> Result<Value, String> {
    if user_entered(host, thing)? {
        get(host, thing, "Text")
    } else {
        get(host, thing, "OldUserEnteredText")
    }
}

/// An unlock subclass's `SwitchPerformReaction__b`.
fn unlock(host: &mut dyn Host, flag: i32, on: bool) -> Result<(), String> {
    if on {
        call(
            host,
            "Profile",
            "SetLBP2GameProgressionFlag__i",
            &[Value::Int(flag)],
        )?;
    }
    Ok(())
}
