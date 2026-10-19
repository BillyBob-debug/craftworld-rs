//! D504: `Trigger_Speech_Bubble` (script g000047f4, base `Tweakable` g00004c8e, then
//! `Native`), the speech bubble Story levels speak with, as a [`NativeScript`] port.
//!
//! FOUND (the 01.33 script listing, research copy only): the functions below are its
//! play-mode code, call for call. `OnCreate__` sets the bubble up and then loops forever,
//! running `Update__` while `HasUpdateWorkToDo` and yielding once per tick (so the runner's
//! [`NativeScript::update`] is that loop's next turn); the switch reaction opens the
//! message, `OnMagicMouthRemoteStop__b` (the player's Circle) closes it, and
//! `SwitchGetNewActivation__i` gives the two-tick output pulse the next line hangs off.
//! Left out: the create-mode tweak menus and editor drawing, and starting a VoIP recording
//! (`Update__` keeps its branches for a bubble restored mid-way). A field the level does
//! not store reads as its `.init__` value ([`init_value`]); textures and the random
//! stream are not made.
//!
//! Checked against the VM (docs/level1-play.md, Part 5): on the Rookie Test route the
//! port makes the script's native calls and member writes, apart from the first loop
//! turn's `Update__`, which the VM skips because it starts a Thing's fields at their type
//! defaults rather than running `.init__` (`HasUpdateWorkToDo` is `true` there).

use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{site, NativeResult, NativeScript, NativeStep as Step};

const CLASS: &str = "Trigger_Speech_Bubble";

/// The script GUID of `Trigger_Speech_Bubble` (FOUND: the 01.33 patch's type tables).
pub const TRIGGER_SPEECH_BUBBLE_SCRIPT: u32 = 0x47f4;

pub struct TriggerSpeechBubbleScript;

impl NativeScript for TriggerSpeechBubbleScript {
    fn class(&self) -> &'static str {
        CLASS
    }
    fn guid(&self) -> Option<u32> {
        Some(TRIGGER_SPEECH_BUBBLE_SCRIPT)
    }
    fn superclasses(&self) -> &'static [&'static str] {
        &["Tweakable", "Native", "Thing"]
    }
    fn on_create(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(Bubble(&mut Ctx { host, this: thing }).on_create())
    }
    fn update(&self, host: &mut dyn Host, thing: u32) -> NativeResult {
        Some(Bubble(&mut Ctx { host, this: thing }).resume())
    }
    fn call(
        &self,
        host: &mut dyn Host,
        thing: u32,
        function: &str,
        args: &[(u16, Value)],
    ) -> NativeResult {
        let args: Vec<Value> = args.iter().map(|(_, v)| v.clone()).collect();
        call(&mut Ctx { host, this: thing }, function, &args)
    }
}

/// One bubble's view of the host.
struct Ctx<'h> {
    host: &'h mut dyn Host,
    this: u32,
}

impl Ctx<'_> {
    fn this(&self) -> Value {
        Value::Thing(Some(self.this))
    }

    /// A member of this Thing's script, or its `.init__` value when the level stores none.
    fn get(&mut self, field: &str) -> Result<Value, String> {
        let this = self.this();
        match self
            .host
            .get_member(MemberKind::Thing, &this, "Trigger_Speech_Bubble", field)
        {
            Ok(v) => Ok(v),
            Err(e) => init_value(field).ok_or(e),
        }
    }

    fn set(&mut self, field: &str, value: Value) -> Result<(), String> {
        let this = self.this();
        self.host.set_member(
            MemberKind::Thing,
            &this,
            "Trigger_Speech_Bubble",
            field,
            value,
        )
    }

    fn native(&mut self, class: &str, function: &str, args: &[Value]) -> Result<Value, String> {
        self.host.call_native(class, function, args)
    }
}

/// `.init__` of the class and of `Tweakable`.
fn init_value(field: &str) -> Option<Value> {
    defaults()
        .into_iter()
        .find(|(f, _)| *f == field)
        .map(|(_, v)| v)
}

/// `.init__` of the class and of `Tweakable`, for the fields a level does not store.
fn defaults() -> Vec<(&'static str, Value)> {
    let (b, i, f) = (Value::Bool, Value::Int, Value::Float);
    let null = Value::Object(None);
    let empty = || Value::Str("".into());
    vec![
        // Tweakable.
        ("TweakPlayerNumber", i(0)),
        ("IsTweaking", b(false)),
        ("IsAdvanced", b(false)),
        ("NeedUpdate", i(0)),
        ("LastTweakResult", i(0)),
        ("BigLeft", b(false)),
        ("BigRight", b(false)),
        ("SelectingName", i(0)),
        // Trigger_Speech_Bubble.
        ("Sound", null.clone()),
        ("CurrVoiceIndex", i(-1)),
        ("IsSelectingDialogue", b(false)),
        ("IsRecordingVoIP", b(false)),
        ("IsAboutToRecordVoIP", b(false)),
        ("VoIPRecordFrame", i(0)),
        ("ClapperBoardOffset", f(0.0)),
        ("ClapperBoardSpeed", f(0.0)),
        ("IsPlayingVoIP", b(false)),
        ("VoIPPlaybackFrame", i(0)),
        ("HeadsetConnected", b(false)),
        ("DialogueListFile", i(0)),
        ("DialogueFile", i(0)),
        ("dialogue_theme_index", i(-1)),
        ("dialogue_level_index", i(-1)),
        ("dialogue_actor_index", i(-1)),
        ("ChosenDialogue", empty()),
        ("TestDialogue", null.clone()),
        ("DialogueLengthMs", i(0)),
        ("OutputEndSignalFrames", i(0)),
        ("SpeechBubbleOpen", b(false)),
        ("IsSubtitled", b(false)),
        ("Behaviour", i(1)),
        ("VoIPResource", null.clone()),
        ("VoiceType", i(-1)),
        ("NeedsMoveControllerFixup", i(0)),
        ("ShowInEditor", b(false)),
        ("ActorName", empty()),
        ("UserEntered", b(false)),
        ("UseWebternate", b(true)),
        ("Text", null.clone()),
        ("OriginalText", null.clone()),
        ("OldUserEnteredText", null.clone()),
        ("SpeechAudioLoopHandle", null.clone()),
        ("TestSpeech", null.clone()),
        ("TriggeredOnce", b(false)),
        ("CameraEnabled", b(false)),
        ("OutputActive", b(true)),
        ("IsActive", b(false)),
        ("IsActiveNonDivergent", b(false)),
        ("IsAnimationActive", b(false)),
        ("AnimPos", f(0.0)),
        ("OffscreenTimer", i(0)),
        ("HideInPlayMode", b(false)),
        ("VolumeDuckerDisabled", b(false)),
        ("DisplayActorName", b(false)),
        ("TriggerThing", Value::Thing(None)),
        ("IsSelectingVoice", b(false)),
        ("AudioTime", i(0)),
        ("HasUpdateWorkToDo", b(true)),
    ]
}

type R<T> = Result<T, String>;

fn call(cx: &mut Ctx<'_>, function: &str, args: &[Value]) -> NativeResult {
    site(cx.host, CLASS, function);
    let mut s = Bubble(cx);
    let done = |r: R<()>| r.map(|()| Step::Done(Value::Void));
    let arg_bool = |n: usize| args.get(n).is_some_and(truthy);
    Some(match function {
        "Enter__Q5Thing" => done(s.enter()),
        "Leave__Q5Thing" => done(s.leave()),
        "SwitchPerformReaction__bf" => {
            let level = match args.get(1) {
                Some(Value::Float(f)) => *f,
                _ => 0.0,
            };
            done(s.switch_reaction(arg_bool(0), level))
        }
        "SwitchGetNewActivation__i" => s
            .int("OutputEndSignalFrames")
            .map(|n| Step::Done(Value::Float(if n > 0 { 1.0 } else { 0.0 }))),
        "OnMagicMouthClosed__" => done(s.on_closed()),
        "OnMagicMouthRemoteStop__b" => done(s.remote_stop(arg_bool(0))),
        "OnDestroy__" => done(s.on_destroy()),
        "StartAnimatingMouth__" => done(s.start_animating_mouth()),
        "StopAnimatingMouth__" => done(s.stop_animating_mouth()),
        "GetAudioLengthMs__" => s.audio_length_ms().map(|n| Step::Done(Value::Int(n))),
        "GetSpeechText__" => s.speech_text().map(Step::Done),
        _ => return None,
    })
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Thing(t) => t.is_some(),
        Value::Object(o) => o.is_some(),
        Value::Raw(r) => *r != 0,
        _ => false,
    }
}

fn thing(v: &Value) -> Option<u32> {
    match v {
        Value::Thing(t) => *t,
        _ => None,
    }
}

struct Bubble<'c, 'h>(&'c mut Ctx<'h>);

impl Bubble<'_, '_> {
    // ---- members and natives ----------------------------------------------------

    fn get(&mut self, field: &str) -> Value {
        self.0.get(field).unwrap_or(Value::Void)
    }

    fn flag(&mut self, field: &str) -> bool {
        truthy(&self.get(field))
    }

    fn int(&mut self, field: &str) -> R<i32> {
        Ok(match self.get(field) {
            Value::Int(i) => i,
            Value::Bool(b) => i32::from(b),
            _ => 0,
        })
    }

    fn set(&mut self, field: &str, v: Value) -> R<()> {
        self.0.set(field, v)
    }

    fn native(&mut self, class: &str, function: &str, args: &[Value]) -> R<Value> {
        self.0.native(class, function, args)
    }

    fn on_this(&mut self, class: &str, function: &str, rest: &[Value]) -> R<Value> {
        let mut args = vec![self.0.this()];
        args.extend_from_slice(rest);
        self.native(class, function, &args)
    }

    fn is_true(&mut self, class: &str, function: &str, args: &[Value]) -> R<bool> {
        self.native(class, function, args).map(|v| truthy(&v))
    }

    fn this_is(&mut self, class: &str, function: &str) -> R<bool> {
        let this = self.0.this();
        self.is_true(class, function, &[this])
    }

    /// A native part member of a Thing: `thing.Part.field = v`.
    /// `t.<member>.<field> = v`: the script names the part member without its `P`
    /// (`Thing.Trigger`), the part class with it (`PTrigger`).
    fn set_part(&mut self, t: u32, part: &str, field: &str, v: Value) -> R<()> {
        let member = part.strip_prefix('P').unwrap_or(part);
        let base = Value::Thing(Some(t));
        let p = self
            .0
            .host
            .get_member(MemberKind::Native, &base, "Thing", member)?;
        self.0
            .host
            .set_member(MemberKind::RefPtr, &p, part, field, v)
    }

    fn input0(&mut self) -> R<bool> {
        self.on_this("Thing", "GetInput__Q5Thingi", &[Value::Int(0)])
            .map(|v| truthy(&v))
    }

    fn output0(&mut self) -> R<bool> {
        self.on_this(
            "Thing",
            "GetOutput__Q5Thingii",
            &[Value::Int(0), Value::Int(0)],
        )
        .map(|v| truthy(&v))
    }

    fn null_or_empty(&mut self, s: Value) -> R<bool> {
        self.is_true("String", "IsNullOrEmpty__Q6String", &[s])
    }

    fn null_or_empty_a(&mut self, s: Value) -> R<bool> {
        self.is_true("StringA", "IsNullOrEmpty__Q7StringA", &[s])
    }

    fn on_sequencer(&mut self) -> R<bool> {
        self.this_is("Game", "IsOnSequencer__Q5Thing")
    }

    // ---- accessors ------------------------------------------------------------------

    fn behaviour(&mut self) -> R<i32> {
        Ok(self.int("Behaviour")? & 15)
    }

    fn bubble_style(&mut self) -> R<i32> {
        Ok((self.int("Behaviour")? & 240) >> 4)
    }

    fn bubble_pos(&mut self) -> R<i32> {
        Ok((self.int("Behaviour")? & 3840) >> 8)
    }

    fn voice(&mut self) -> R<i32> {
        self.int("VoiceType")
    }

    fn dialogue_file_selected(&mut self) -> R<bool> {
        Ok(self.voice()? == 2)
    }

    fn voip_file_selected(&mut self) -> R<bool> {
        Ok(self.voice()? == 1)
    }

    fn gibberish_selected(&mut self) -> R<bool> {
        Ok(self.voice()? == 0)
    }

    fn really_subtitled(&mut self) -> R<bool> {
        Ok(self.flag("IsSubtitled") && self.on_sequencer()?)
    }

    fn can_prevent_player_close(&mut self) -> R<bool> {
        Ok(!self.really_subtitled()?
            && (self.on_sequencer()? || self.input0()?)
            && self.behaviour()? != 0)
    }

    fn really_prevent_player_close(&mut self) -> R<bool> {
        Ok(self.can_prevent_player_close()? && (self.int("Behaviour")? & 4096) != 0)
    }

    fn is_non_divergent(&mut self) -> R<bool> {
        Ok(self.behaviour()? == 0 || self.output0()?)
    }

    fn voip_loaded(&mut self) -> R<bool> {
        let r = self.get("VoIPResource");
        if !truthy(&r) {
            return Ok(false);
        }
        self.is_true("Resource", "IsLoaded__", &[r])
    }

    fn actor_name_index(&mut self) -> R<i32> {
        self.native(
            "Game",
            "GetNumKeyNames__Q25Gooey.EGooeyNetworkAction",
            &[Value::Int(80)],
        )?;
        let name = self.get("ActorName");
        match self.native(
            "Game",
            "GetKeyIndex__Q25Gooey.EGooeyNetworkActionQ6String",
            &[Value::Int(80), name],
        )? {
            Value::Int(i) => Ok(i),
            _ => Ok(-1),
        }
    }

    fn animate_mouth(&mut self) -> R<bool> {
        if self.is_true("Game", "IsTarsierDLCEnabled__", &[])? {
            let style = self.bubble_style()?;
            if style == 1 || style == 2 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // ---- text -----------------------------------------------------------------------

    fn translated_text(&mut self, use_text: bool) -> R<Value> {
        let text = self.get("Text");
        let or_empty = |v: Value| match v {
            Value::Object(None) | Value::Void => Value::Str("".into()),
            v => v,
        };
        if !self.flag("UserEntered") {
            let key = self.native(
                "LAMS",
                "MakeLamsKeyID__Q6String",
                std::slice::from_ref(&text),
            )?;
            if self.is_true("LAMS", "IsInLams__i", std::slice::from_ref(&key))? {
                let t = self.native("LAMS", "Translate__i", &[key])?;
                if self.int("NeedsMoveControllerFixup")? == 0 {
                    return Ok(t);
                }
                return self.native("System", "ReplaceMoveControllerText__Q6String", &[t]);
            }
            let old = self.get("OldUserEnteredText");
            if !matches!(old, Value::Object(None) | Value::Void) {
                return Ok(old);
            }
            if !self.null_or_empty(text.clone())? {
                return self.native(
                    "String",
                    "Concat__Q6StringQ6String",
                    &[Value::Str("Missing key: ".into()), text],
                );
            }
            return Ok(Value::Str("".into()));
        }
        if use_text {
            return Ok(or_empty(text));
        }
        let original = self.get("OriginalText");
        if self.null_or_empty(original.clone())? {
            return Ok(or_empty(text));
        }
        Ok(or_empty(original))
    }

    fn speech_text(&mut self) -> R<Value> {
        let filter = self.is_true(
            "GameMessagingManager",
            "Messaging_ShouldFilterMessageStrings__",
            &[],
        )?;
        self.translated_text(filter)
    }

    fn update_dialogue_length(&mut self) -> R<()> {
        if self.dialogue_file_selected()? {
            let file = self.get("DialogueFile");
            let ms = self.native("Audio", "TryGetProgrammerSoundLength__g", &[file])?;
            self.set("DialogueLengthMs", ms)?;
        } else if self.voip_file_selected()? {
            let r = self.get("VoIPResource");
            let ms = self.native("Audio", "TryGetVoIPLength__Q8Resource", &[r])?;
            self.set("DialogueLengthMs", ms)?;
        } else {
            let text = self.speech_text()?;
            let len = match self.native("String", "Length__Q6String", &[text])? {
                Value::Int(n) => n,
                _ => 0,
            };
            let ms = (len.max(4) as f32 * 1.7 * 1000.0 / 30.0) as i32;
            // The script stores the text length, then reads it back to cap it at 8 s.
            self.set("DialogueLengthMs", Value::Int(ms))?;
            if self.int("DialogueLengthMs")? > 8000 {
                self.set("DialogueLengthMs", Value::Int(8000))?;
            }
        }
        self.on_this("Thing", "UpdateScale__", &[])?;
        Ok(())
    }

    fn set_subtitle_from_dialogue(&mut self, force: bool) -> R<()> {
        let chosen = self.get("ChosenDialogue");
        if self.null_or_empty_a(chosen.clone())? {
            return Ok(());
        }
        let text = self.get("Text");
        if !(force || self.null_or_empty(text)?) {
            return Ok(());
        }
        let key = self.dialogue_to_lams_key(chosen)?;
        self.0.host.write("Upgrading magic mouth LAMS key to ");
        if let Value::Str(k) = &key {
            self.0.host.write(k);
        }
        self.0.host.write("\n");
        let empty = Value::Str("".into());
        self.set("Text", empty.clone())?;
        self.set("OldUserEnteredText", empty.clone())?;
        self.set("OriginalText", empty)?;
        self.set("UserEntered", Value::Bool(false))?;
        self.set_lams_key(key)
    }

    fn dialogue_to_lams_key(&mut self, path: Value) -> R<Value> {
        let int = |v: Value| match v {
            Value::Int(i) => i,
            _ => -1,
        };
        let mut s = path;
        let mut last_slash = -1;
        loop {
            let at = int(self.native(
                "StringA",
                "Find__Q7StringAQ7StringA",
                &[s.clone(), Value::Str("/".into())],
            )?);
            if at == -1 {
                break;
            }
            let len = int(self.native("StringA", "Length__Q7StringA", &[s.clone()])?);
            s = self.native(
                "StringA",
                "SubStr__Q7StringAii",
                &[s, Value::Int(at + 1), Value::Int(len)],
            )?;
            last_slash = at;
        }
        let mut r = Value::Str("".into());
        if last_slash != -1 {
            let len = int(self.native("StringA", "Length__Q7StringA", &[s.clone()])?);
            r = self.native(
                "StringA",
                "SubStr__Q7StringAii",
                &[s, Value::Int(0), Value::Int(len - 4)],
            )?;
        }
        let r = self.native("String", "StringFromStringA__Q7StringA", &[r])?;
        self.native("String", "ToUpper__Q6String", &[r])
    }

    fn set_lams_key(&mut self, key: Value) -> R<()> {
        let key = self.native("String", "ToUpper__Q6String", &[key])?;
        self.set("Text", key)?;
        self.set("UserEntered", Value::Bool(false))?;
        for f in ["OriginalText", "OldUserEnteredText", "EditorBubble"] {
            self.set(f, Value::Object(None))?;
        }
        self.update_dialogue_length()?;
        self.set("NeedUpdate", Value::Int(1))
    }

    // ---- entries ----------------------------------------------------------------------

    fn on_create(&mut self) -> R<Step> {
        site(self.0.host, CLASS, "OnCreate__");
        let list = self.native("Audio", "GetDialogueList__", &[])?;
        self.set("DialogueListFile", list)?;
        for f in [
            "dialogue_theme_index",
            "dialogue_level_index",
            "dialogue_actor_index",
        ] {
            self.set(f, Value::Int(-1))?;
        }
        let tt = self.descendent_with_ptrigger(self.0.this)?;
        self.set("TriggerThing", Value::Thing(tt))?;
        if let Some(tt) = tt {
            self.set_part(tt, "PTrigger", "AllZLayers", Value::Bool(true))?;
            self.set_part(tt, "PTrigger", "HysteresisMultiplier", Value::Float(2.0))?;
        }
        if self.this_is("PartList", "HasPRenderMesh__Q5Thing")? {
            self.on_this("Native", "SetAnimSpeed__Q5Thingf", &[Value::Float(0.0)])?;
            self.on_this("Native", "SetAnimPos__Q5Thingf", &[Value::Float(0.0)])?;
        }
        if self.this_is("PartList", "HasPCameraTweak__Q5Thing")? {
            let this = self.0.this;
            self.set_part(this, "PCameraTweak", "CameraType", Value::Int(2))?;
        }
        self.set("TriggeredOnce", Value::Bool(false))?;
        self.set("IsActive", Value::Bool(false))?;
        self.set("IsAnimationActive", Value::Bool(false))?;
        self.set("OffscreenTimer", Value::Int(0))?;
        for use_text in [true, false] {
            let t = self.translated_text(use_text)?;
            self.native(
                "HUD",
                "PreloadGlyphs__Q11Fonts.EFontQ6String",
                &[Value::Int(1), t],
            )?;
        }
        let chosen = self.get("ChosenDialogue");
        if !self.null_or_empty_a(chosen.clone())? {
            let file = self.get("DialogueFile");
            if self.native("FileList", "AsInt__g", &[file])? == Value::Int(0) {
                let g = self.native("Resource", "FindGuidByPath__Q7StringA", &[chosen])?;
                self.set("DialogueFile", g)?;
            }
        }
        if self.voice()? == -1 {
            let file = self.get("DialogueFile");
            let voice = if self.native("FileList", "AsInt__g", &[file])? != Value::Int(0) {
                2
            } else if self.voip_loaded()? {
                1
            } else if self.int("CurrVoiceIndex")? != -1 {
                0
            } else {
                3
            };
            self.set("VoiceType", Value::Int(voice))?;
        }
        self.update_dialogue_length()?;
        self.set_subtitle_from_dialogue(false)?;
        if self.flag("IsActiveNonDivergent") {
            self.trigger_start_message()?;
        }
        if self.voip_file_selected()? {
            self.on_this("Audio", "SetVoIPLimitDirty__Q5Thing", &[])?;
        }
        self.resume()
    }

    /// One turn of `OnCreate__`'s loop: `Update__` when there is work, then the yield.
    fn resume(&mut self) -> R<Step> {
        site(self.0.host, CLASS, "OnCreate__");
        if self.flag("HasUpdateWorkToDo") {
            self.update()?;
        }
        Ok(Step::Yielded)
    }

    /// `Thing.GetDescendentWithPTrigger__Q5Thing` (a script function of the `Thing`
    /// class): the Thing itself or its first child with a PTrigger, else null.
    fn descendent_with_ptrigger(&mut self, t: u32) -> R<Option<u32>> {
        let has = |s: &mut Self, t: u32| {
            s.is_true("PartList", "HasPTrigger__Q5Thing", &[Value::Thing(Some(t))])
        };
        if has(self, t)? {
            return Ok(Some(t));
        }
        let mut c =
            thing(&self.native("Thing", "GetFirstChild__Q5Thing", &[Value::Thing(Some(t))])?);
        while let Some(child) = c {
            if has(self, child)? {
                return Ok(Some(child));
            }
            c = thing(&self.native(
                "Thing",
                "GetNextSibling__Q5Thing",
                &[Value::Thing(Some(child))],
            )?);
        }
        Ok(None)
    }

    fn update(&mut self) -> R<()> {
        site(self.0.host, CLASS, "Update__");
        self.set("HasUpdateWorkToDo", Value::Bool(false))?;
        if self.flag("IsTweaking") {
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            let plugged = self.native("Voip", "GetHeadsetPluggedIn__", &[])?;
            if truthy(&plugged) != self.flag("HeadsetConnected") {
                self.set("NeedUpdate", Value::Int(1))?;
            }
            self.set("HeadsetConnected", Value::Bool(truthy(&plugged)))?;
        }
        if self.flag("IsActive") || self.flag("IsPlayingVoIP") {
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            if self.scoreboard_active()? {
                self.trigger_stop_message(true)?;
            }
            if self.flag("IsActive") && self.input0()? {
                self.update_offscreen_timer()?;
            }
            self.update_audio()?;
        }
        if self.int("OutputEndSignalFrames")? > 0 {
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            let frames = self.int("OutputEndSignalFrames")?;
            self.set("OutputEndSignalFrames", Value::Int(frames - 1))?;
        }
        if self.voip_file_selected()? && self.flag("IsPlayingVoIP") {
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            let idx = self.actor_name_index()?;
            let am = self.animate_mouth()?;
            let r = self.get("VoIPResource");
            let frame = self.int("VoIPPlaybackFrame")?;
            let this = self.0.this();
            self.native(
                "Audio",
                "RequestVoIPPlayback__Q8ResourceQ5Thingiib",
                &[r, this, Value::Int(frame), Value::Int(idx), Value::Bool(am)],
            )?;
            self.set("VoIPPlaybackFrame", Value::Int(frame + 1))?;
        }
        if self.flag("IsAboutToRecordVoIP") {
            return self.count_in_recording();
        }
        if self.flag("IsRecordingVoIP") {
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            let frame = self.int("VoIPRecordFrame")? + 1;
            self.set("VoIPRecordFrame", Value::Int(frame))?;
            if frame as f32 > 30.0 {
                let offset = match self.get("ClapperBoardOffset") {
                    Value::Float(f) => f,
                    _ => 0.0,
                };
                let y = match self.native("HUD", "GetYRes__", &[])? {
                    Value::Int(y) => y,
                    _ => 0,
                };
                if offset < y as f32 {
                    self.set(
                        "ClapperBoardOffset",
                        Value::Float(offset + (frame as f32 - 30.0) * 8.0),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// `Update__`'s branch while the three-second count-in before a VoIP recording runs.
    fn count_in_recording(&mut self) -> R<()> {
        self.set("HasUpdateWorkToDo", Value::Bool(true))?;
        let frame = self.int("VoIPRecordFrame")?;
        if frame % 30 == 0 {
            if frame == 60 {
                self.native(
                    "Audio",
                    "SetMixOverlay__Q7StringAf",
                    &[Value::Str("Record Voice".into()), Value::Float(0.9)],
                )?;
            }
            self.native(
                "Audio",
                "PlaySound__Q5ThingQ7StringA",
                &[
                    Value::Thing(None),
                    Value::Str("poppet/camera_countdown".into()),
                ],
            )?;
        }
        let frame = frame + 1;
        self.set("VoIPRecordFrame", Value::Int(frame))?;
        if frame as f32 == 90.0 {
            self.set("VoIPRecordFrame", Value::Int(0))?;
            self.set("IsAboutToRecordVoIP", Value::Bool(false))?;
            self.set("ClapperBoardOffset", Value::Float(0.0))?;
            let started = self.on_this("Audio", "BeginRecordingVoIP__Q5Thing", &[])?;
            self.set("IsRecordingVoIP", Value::Bool(truthy(&started)))?;
            if truthy(&started) {
                self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            } else {
                self.native(
                    "Audio",
                    "PlaySound__Q5ThingQ7StringA",
                    &[Value::Thing(None), Value::Str("poppet/error".into())],
                )?;
                self.set("NeedUpdate", Value::Int(1))?;
            }
            return Ok(());
        }
        let num = |v: Value| match v {
            Value::Float(f) => f,
            _ => 0.0,
        };
        let offset = num(self.get("ClapperBoardOffset"));
        let speed = (num(self.get("ClapperBoardSpeed")) - offset * 0.3) * 0.5;
        self.set("ClapperBoardSpeed", Value::Float(speed))?;
        self.set("ClapperBoardOffset", Value::Float(offset + speed))
    }

    fn scoreboard_active(&mut self) -> R<bool> {
        let world = self.on_this("Thing", "GetWorld__", &[])?;
        // `World.GetCurrentChallenge__`, a script function: the world's member.
        let challenge = self
            .0
            .host
            .get_member(MemberKind::Thing, &world, "World", "CurrentChallenge")
            .unwrap_or(Value::Thing(None));
        self.is_true("BaseWorld", "IsScoreBoard__Q5Thing", &[challenge])
    }

    fn update_offscreen_timer(&mut self) -> R<()> {
        if self.really_subtitled()? {
            return self.set("OffscreenTimer", Value::Int(0));
        }
        let proxy = self.on_this("Thing", "GetMicroChipProxyFromThing__Q5Thing", &[])?;
        let pos = self
            .0
            .host
            .get_member(MemberKind::Native, &proxy, "Thing", "Pos")?;
        let m = self
            .0
            .host
            .get_member(MemberKind::RefPtr, &pos, "PPos", "WorldPosition")?;
        let w = match m {
            Value::Mat44(m) => [m[12], m[13], m[14], m[15]],
            other => return Err(format!("PPos.WorldPosition is {other:?}")),
        };
        let at = match self.native("HUD", "GetScreenSpacePosition__r", &[Value::Vec4(w)])? {
            Value::Vec4(v) => v,
            other => return Err(format!("GetScreenSpacePosition gave {other:?}")),
        };
        let res = |v: Value| match v {
            Value::Int(n) => n,
            _ => 0,
        };
        // The four tests short-circuit, so a resolution is read only when its test is
        // reached (D512: on the route the VM skips `GetYRes__` once x is off-screen).
        let mut off = at[0] < -100.0;
        if !off {
            let x = res(self.native("HUD", "GetXRes__", &[])?);
            off = at[0] > (x + 100) as f32 || at[1] < -100.0;
        }
        if !off {
            let y = res(self.native("HUD", "GetYRes__", &[])?);
            off = at[1] > (y + 100) as f32;
        }
        if !off {
            return self.set("OffscreenTimer", Value::Int(0));
        }
        let timer = self.int("OffscreenTimer")? + 1;
        self.set("OffscreenTimer", Value::Int(timer))?;
        // The test reads the member back, as the script does.
        if self.int("OffscreenTimer")? > 60 && !self.is_non_divergent()? {
            self.trigger_stop_message(false)?;
        }
        Ok(())
    }

    fn audio_length_ms(&mut self) -> R<i32> {
        if self.on_sequencer()? {
            let v = self.voice()?;
            if v != 1 && v != 2 {
                let len = match self.this_value("Game", "GetTimeLengthOnSequencer__Q5Thing")? {
                    Value::Float(f) => f,
                    Value::Int(i) => i as f32,
                    _ => 0.0,
                };
                return Ok((len * 1000.0 / 30.0) as i32);
            }
        }
        self.int("DialogueLengthMs")
    }

    fn this_value(&mut self, class: &str, function: &str) -> R<Value> {
        let this = self.0.this();
        self.native(class, function, &[this])
    }

    fn update_audio(&mut self) -> R<()> {
        if !(self.flag("IsAnimationActive") || self.flag("IsPlayingVoIP")) {
            return Ok(());
        }
        let len = self.audio_length_ms()?;
        let frames = if len != 0 { len * 30 / 1000 } else { 0 };
        let sound = self.get("Sound");
        if self.gibberish_selected()? && !self.null_or_empty_a(sound.clone())? {
            let idx = self.actor_name_index()?;
            let am = self.animate_mouth()?;
            let handle = self.get("SpeechAudioLoopHandle");
            let this = self.0.this();
            let h = self.native(
                "Audio",
                "PlayGibberDialogue__Q5ThingQ11AudioHandleiQ7StringAb",
                &[this, handle, Value::Int(idx), sound, Value::Bool(am)],
            )?;
            self.set("SpeechAudioLoopHandle", h)?;
        }
        let time = self.int("AudioTime")?;
        if time >= frames {
            self.stop_animating_mouth()?;
        }
        if frames > 0 && self.really_subtitled()? {
            let text = self.speech_text()?;
            let this = self.0.this();
            self.native(
                "GameMessagingManager",
                "Messaging_UpdateSubtitles__Q6StringQ5Thingf",
                &[text, this, Value::Float(time as f32 / frames as f32)],
            )?;
        }
        if !self.flag("VolumeDuckerDisabled") {
            let mut duck = false;
            if self.dialogue_file_selected()? || self.gibberish_selected()? {
                let handle = self.get("SpeechAudioLoopHandle");
                duck = self.is_true("Audio", "GetIsPlaying__Q11AudioHandle", &[handle])?;
            } else if self.voip_file_selected()? {
                duck = self.flag("IsPlayingVoIP");
            }
            if duck {
                self.native(
                    "Audio",
                    "SetMixOverlay__Q7StringAf",
                    &[Value::Str("Gibberish".into()), Value::Float(0.4)],
                )?;
            }
        }
        if !self.is_true("Game", "IsGamePaused__", &[])? {
            let time = self.int("AudioTime")?;
            self.set("AudioTime", Value::Int(time + 1))?;
        }
        Ok(())
    }

    fn players_in_zone(&mut self) -> R<i32> {
        let mut count = 0;
        let Some(tt) = thing(&self.get("TriggerThing")) else {
            return Ok(0);
        };
        let tt = Value::Thing(Some(tt));
        let mut i = 0;
        loop {
            let n = match self.native(
                "Native",
                "GetNumInThings__Q5Thing",
                std::slice::from_ref(&tt),
            )? {
                Value::Int(n) => n,
                _ => 0,
            };
            if i >= n {
                break;
            }
            let t = self.native(
                "Native",
                "GetInThing__Q5Thingi",
                &[tt.clone(), Value::Int(i)],
            )?;
            if truthy(&t)
                && self.is_true(
                    "PartList",
                    "HasPYellowHead__Q5Thing",
                    std::slice::from_ref(&t),
                )?
            {
                let p = self
                    .0
                    .host
                    .get_member(MemberKind::Native, &t, "Thing", "YellowHead")?;
                let n = self.0.host.get_member(
                    MemberKind::RefPtr,
                    &p,
                    "PYellowHead",
                    "PlayerNumber",
                )?;
                let player = self.native("Game", "EPlayerNumberFromInt__i", &[n])?;
                if self.is_true("Game", "IsLocalPlayerID__Q18Game.EPlayerNumber", &[player])? {
                    count += 1;
                }
            }
            i += 1;
        }
        Ok(count)
    }

    fn enter(&mut self) -> R<()> {
        if self.input0()? {
            return Ok(());
        }
        if (self.is_non_divergent()? || self.players_in_zone()? >= 1)
            && !self.scoreboard_active()?
        {
            self.trigger_start_message()?;
        }
        Ok(())
    }

    fn leave(&mut self) -> R<()> {
        if self.input0()? {
            return Ok(());
        }
        if self.is_non_divergent()? || self.players_in_zone()? == 0 {
            self.trigger_stop_message(false)?;
        }
        Ok(())
    }

    fn switch_reaction(&mut self, on: bool, level: f32) -> R<()> {
        let start = on || (self.behaviour()? == 1 && level >= 1.0 && !self.flag("IsActive"));
        if start {
            self.trigger_start_message()?;
        }
        if level <= 0.0 && self.flag("IsActive") {
            self.trigger_stop_message(false)?;
        }
        Ok(())
    }

    fn on_closed(&mut self) -> R<()> {
        self.set("SpeechBubbleOpen", Value::Bool(false))?;
        self.stop_animating_mouth()?;
        self.deactivate_camera_zone()
    }

    fn remote_stop(&mut self, force: bool) -> R<()> {
        self.set("SpeechBubbleOpen", Value::Bool(false))?;
        self.trigger_stop_message(force)
    }

    fn on_destroy(&mut self) -> R<()> {
        if self.flag("IsTweaking") {
            // `Tweakable.StopTweaking__`: the bubble's StopTweakingObject has no play-mode
            // effect here.
            self.set("IsTweaking", Value::Bool(false))?;
        }
        let saved = self.get("IsActiveNonDivergent");
        self.trigger_stop_message(true)?;
        self.set("IsActiveNonDivergent", saved)?;
        self.on_this("Thing", "RemoveFromAnimatingDivergentList__", &[])?;
        if self.voip_file_selected()? {
            self.on_this("Audio", "SetVoIPLimitDirty__Q5Thing", &[])?;
        }
        Ok(())
    }

    fn ready_to_play(&mut self) -> R<bool> {
        let text = self.speech_text()?;
        Ok(!self.null_or_empty(text)?
            || (self.voip_file_selected()? && self.voip_loaded()?)
            || self.dialogue_file_selected()?
            || self.flag("CameraEnabled")
            || self.on_sequencer()?)
    }

    fn trigger_start_message(&mut self) -> R<()> {
        if !self.ready_to_play()? {
            if self.output0()? {
                self.set("OutputEndSignalFrames", Value::Int(2))?;
                self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            }
            return Ok(());
        }
        let filter = self.is_true(
            "GameMessagingManager",
            "Messaging_ShouldFilterMessageStrings__",
            &[],
        )?;
        if self.native("Poppet", "GetTextEntryType__", &[])? == Value::Int(6) {
            self.debug_print(filter)?;
        }
        let key = self.get("Text");
        self.native("Game", "TrackSpeechKey__Q6String", &[key])?;
        let this = self.0.this();
        if self.really_subtitled()? {
            let has_audio = (self.voip_file_selected()? && self.voip_loaded()?)
                || self.dialogue_file_selected()?;
            let t = self.translated_text(filter)?;
            self.native(
                "GameMessagingManager",
                "Messaging_NewTriggerMessageSubtitles__Q6StringQ5Thingbb",
                &[t, this, Value::Bool(false), Value::Bool(has_audio)],
            )?;
        } else if self.is_true("Game", "IsTarsierDLCEnabled__", &[])? {
            let style = self.bubble_style()?;
            if self.behaviour()? == 0
                || self.is_non_divergent()?
                || self.on_sequencer()?
                || self.really_prevent_player_close()?
            {
                self.set("IsActiveNonDivergent", Value::Bool(true))?;
            }
            let kind = match style {
                0 | 3 => Some("NewTriggerMessage"),
                2 => Some("NewTriggerMessageCaption"),
                1 => Some("NewTriggerMessageThink"),
                _ => None,
            };
            if let Some(kind) = kind {
                let no_prompt = self.flag("IsActiveNonDivergent");
                let t = self.translated_text(filter)?;
                let name = self.actor_name_shown()?;
                let pos = self.bubble_pos()?;
                let prevent = self.really_prevent_player_close()?;
                let (function, once) = if no_prompt {
                    (
                        format!("Messaging_{kind}NoPrompt__Q6StringQ6StringQ5Thingbib"),
                        false,
                    )
                } else {
                    (
                        format!("Messaging_{kind}__Q6StringQ6StringQ5Thingbib"),
                        self.flag("TriggeredOnce"),
                    )
                };
                self.native(
                    "GameMessagingManager",
                    &function,
                    &[
                        t,
                        name,
                        this,
                        Value::Bool(once),
                        Value::Int(pos),
                        Value::Bool(prevent),
                    ],
                )?;
            }
        } else if self.behaviour()? == 0 || self.is_non_divergent()? || self.on_sequencer()? {
            self.set("IsActiveNonDivergent", Value::Bool(true))?;
            let t = self.translated_text(filter)?;
            let name = self.actor_name_shown()?;
            self.native(
                "GameMessagingManager",
                "Messaging_NewTriggerMessageNoPrompt__Q6StringQ6StringQ5Thingbib",
                &[
                    t,
                    name,
                    this,
                    Value::Bool(false),
                    Value::Int(0),
                    Value::Bool(false),
                ],
            )?;
        } else {
            let t = self.translated_text(filter)?;
            let name = self.actor_name_shown()?;
            let once = self.flag("TriggeredOnce");
            self.native(
                "GameMessagingManager",
                "Messaging_NewTriggerMessage__Q6StringQ6StringQ5Thingbib",
                &[
                    t,
                    name,
                    this,
                    Value::Bool(once),
                    Value::Int(0),
                    Value::Bool(false),
                ],
            )?;
        }
        self.set("TriggeredOnce", Value::Bool(true))?;
        self.set("IsActive", Value::Bool(true))?;
        self.set("HasUpdateWorkToDo", Value::Bool(true))?;
        self.set("OffscreenTimer", Value::Int(0))?;
        let idx = self.actor_name_index()?;
        self.on_this(
            "Trigger_Speech_Bubble",
            "NotifyNpcsOfMouthStart__i",
            &[Value::Int(idx)],
        )?;
        Ok(())
    }

    /// The actor name a message shows: the bubble's when `DisplayActorName`, else null.
    fn actor_name_shown(&mut self) -> R<Value> {
        Ok(if self.flag("DisplayActorName") {
            self.get("ActorName")
        } else {
            Value::Object(None)
        })
    }

    /// The text-entry debug mode's line (`Poppet.GetTextEntryType__` 6).
    fn debug_print(&mut self, filter: bool) -> R<()> {
        let t = self.translated_text(filter)?;
        let len = match self.native("String", "Length__Q6String", std::slice::from_ref(&t))? {
            Value::Int(n) => n,
            _ => 0,
        };
        let head = self.native(
            "String",
            "SubStr__Q6Stringii",
            &[t, Value::Int(0), Value::Int(len.min(20))],
        )?;
        let s = self.native(
            "String",
            "Concat__Q6StringQ6String",
            &[head, Value::Str("...".into())],
        )?;
        let text = self.get("Text");
        self.0.host.write("Text: ");
        for v in [s, Value::Str("[ ".into()), text, Value::Str(" ]\n".into())] {
            if let Value::Str(x) = v {
                self.0.host.write(&x);
            }
        }
        Ok(())
    }

    fn trigger_stop_message(&mut self, force: bool) -> R<()> {
        if !(force || self.behaviour()? == 1) {
            return Ok(());
        }
        let this = self.0.this();
        self.native(
            "GameMessagingManager",
            "Messaging_NewGameEvent__iiQ5Thingb",
            &[Value::Int(4), Value::Int(3), this, Value::Bool(false)],
        )?;
        if self.flag("IsActive") && self.is_non_divergent()? {
            self.set("OutputEndSignalFrames", Value::Int(2))?;
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
        }
        self.deactivate_camera_zone()?;
        self.stop_animating_mouth()?;
        self.set("IsActiveNonDivergent", Value::Bool(false))?;
        self.set("IsActive", Value::Bool(false))
    }

    fn activate_camera_zone(&mut self) -> R<()> {
        if self.flag("CameraEnabled") && self.this_is("PartList", "HasPCameraTweak__Q5Thing")? {
            let this = self.0.this;
            self.set_part(this, "PCameraTweak", "CameraType", Value::Int(2))?;
            self.on_this(
                "Native",
                "SetSpeechBubbleActive__Q5Thingb",
                &[Value::Bool(true)],
            )?;
        }
        Ok(())
    }

    fn deactivate_camera_zone(&mut self) -> R<()> {
        if self.this_is("PartList", "HasPCameraTweak__Q5Thing")? {
            self.on_this(
                "Native",
                "SetSpeechBubbleActive__Q5Thingb",
                &[Value::Bool(false)],
            )?;
        }
        Ok(())
    }

    fn start_animating_mouth(&mut self) -> R<()> {
        if self.flag("IsAnimationActive") {
            return Ok(());
        }
        self.activate_camera_zone()?;
        self.set("IsAnimationActive", Value::Bool(true))?;
        self.set("AudioTime", Value::Int(0))?;
        if !self.really_subtitled()? {
            let proxy = self.on_this("Thing", "GetMicroChipProxyFromThing__Q5Thing", &[])?;
            self.native(
                "Audio",
                "PlaySound__Q5ThingQ7StringA",
                &[proxy, Value::Str("gameplay/speech_bubbles/open".into())],
            )?;
        }
        self.set("AnimPos", Value::Float(0.0))?;
        let mut has_audio = false;
        if self.dialogue_file_selected()? {
            let idx = self.actor_name_index()?;
            let am = self.animate_mouth()?;
            let file = self.get("DialogueFile");
            let this = self.0.this();
            let h = self.native(
                "Audio",
                "PlayCharacterDialogue__gQ5Thingib",
                &[file, this, Value::Int(idx), Value::Bool(am)],
            )?;
            self.set("SpeechAudioLoopHandle", h)?;
            has_audio = true;
        } else if self.voip_file_selected()? {
            self.set("IsPlayingVoIP", Value::Bool(true))?;
            self.set("HasUpdateWorkToDo", Value::Bool(true))?;
            self.set("VoIPPlaybackFrame", Value::Int(0))?;
            has_audio = true;
        } else if self.gibberish_selected()? {
            let sound = self.get("Sound");
            has_audio = !self.null_or_empty_a(sound)?;
        }
        if has_audio {
            let name = self.get("ActorName");
            if self.null_or_empty(name)? && self.animate_mouth()? {
                self.on_this("Thing", "AddToAnimatingDivergentList__", &[])?;
            }
        }
        Ok(())
    }

    fn stop_animating_mouth(&mut self) -> R<()> {
        self.set("IsAnimationActive", Value::Bool(false))?;
        self.set("AnimPos", Value::Float(0.0))?;
        self.on_this("Thing", "RemoveFromAnimatingDivergentList__", &[])?;
        self.stop_audio()?;
        self.on_this("Trigger_Speech_Bubble", "NotifyNpcsOfMouthEnd__", &[])?;
        Ok(())
    }

    fn stop_audio(&mut self) -> R<()> {
        // StopPreviewSound__.
        for h in ["TestSpeech", "TestDialogue"] {
            let v = self.get(h);
            self.native("Audio", "StopSound__Q11AudioHandle", &[v])?;
            self.set(h, Value::Object(None))?;
        }
        let v = self.get("SpeechAudioLoopHandle");
        self.native("Audio", "StopSound__Q11AudioHandle", &[v])?;
        self.set("SpeechAudioLoopHandle", Value::Object(None))?;
        let fade = if self.dialogue_file_selected()? || self.voip_file_selected()? {
            1.0
        } else {
            0.7
        };
        if !self.flag("VolumeDuckerDisabled") {
            self.native(
                "Audio",
                "RemoveMixOverlay__Q7StringAf",
                &[Value::Str("Gibberish".into()), Value::Float(fade)],
            )?;
        }
        self.native(
            "Audio",
            "RemoveMixOverlay__Q7StringAf",
            &[Value::Str("Record Voice".into()), Value::Float(0.4)],
        )?;
        if self.voip_file_selected()? {
            if self.flag("IsRecordingVoIP") {
                self.on_this("Audio", "FinishRecordingVoIP__Q5Thing", &[])?;
            }
            if self.flag("IsPlayingVoIP") {
                let r = self.get("VoIPResource");
                let this = self.0.this();
                self.native("Audio", "StopVoIPPlayback__Q8ResourceQ5Thing", &[r, this])?;
            }
        }
        self.set("VoIPPlaybackFrame", Value::Int(0))?;
        self.set("IsRecordingVoIP", Value::Bool(false))?;
        self.set("IsAboutToRecordVoIP", Value::Bool(false))?;
        self.set("IsPlayingVoIP", Value::Bool(false))
    }
}
