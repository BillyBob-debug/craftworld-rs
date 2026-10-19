//! D270 (docs/level-scripts.md, part 21): what the Pod's world script asks the engine for.
//! The Things a script finds by name ([`LevelWorld::find_thing`]), the pod Thing
//! ([`LevelWorld::pod_thing`]), the mesh a script gives a Thing's PRenderMesh, and the pod
//! computer's values that are only kept ([`PodState`]).
//!
//! D273 (part 24): the pod computer is entered. The pad zone
//! ([`LevelWorld::in_pad_zone`]), the request to use the pod computer and its answer
//! ([`LevelWorld::request_host_use_pod`], [`LevelWorld::begin_session_tick`]), and the
//! state the game keeps outside a level, handed from one level world to another
//! ([`LevelWorld::hand_session_to`], [`tick_session`]).
use crate::{LevelScripts, LevelWorld, StepReport};
use lbp_script_vm::natives::{
    Bindings, DEFAULT_PLANETS_BACKGROUND, PAD_ZONE_MIN_Z, POD_MESH, POD_PLATFORM_MESH,
};
use lbp_script_vm::{ObjectId, Vm};
use std::collections::{BTreeMap, BTreeSet};

/// The script class of the pod computer's trigger. INFERRED: the game tests the script
/// instance of each entry of a PWorld list for one class and calls a function of it by
/// name without arguments (01.33, 0x620504, 0x6205b4); the class and the function names
/// were not read. `Trigger_Pod` has a function `IsPodComputerTrigger__` that returns
/// true and nothing else calls it.
pub const POD_TRIGGER_CLASS: &str = "Trigger_Pod";

/// The sounds the game plays at the primary player's avatar when "in use" changes by
/// the host's answer (FOUND, 01.33, 0x18ed30).
pub const POD_COMPUTER_STARTUP_SOUND: &str = "pod/pod_comps_startup";
pub const POD_COMPUTER_SHUTDOWN_SOUND: &str = "pod/pod_comps_shutdown";

/// One change of [`LevelWorld::pod_computer_in_use`] by the host's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InUseChange {
    pub in_use: bool,
    /// `PWorld.Frame` of this world at the start of the tick that took the answer.
    pub frame: i32,
}

/// One `PodComputer.RequestHostUsePod__b` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostUsePodRequest {
    /// True asks to use the pod computer, false to stop.
    pub in_use: bool,
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// The Pod state of a level world ([`LevelWorld::pod`]).
#[derive(Debug, Clone, Default)]
pub struct PodState {
    /// The backdrop plan `PodComputer.GetPlanetsBackgroundGUID__` answers with; none
    /// while the planets level is not loaded, and the native then gives
    /// [`DEFAULT_PLANETS_BACKGROUND`] (FOUND). Nothing sets it here: the runtime plays
    /// one level, and the field of the planets level's PWorld the game reads was not
    /// matched to a stored value.
    pub planets_background: Option<u32>,
    /// The resource handles `Resource.LoadMeshByFilename__g` handed out: the handles
    /// that are a mesh, for `Thing.SetRenderMeshMesh`.
    pub mesh_resources: BTreeSet<u32>,
    /// `Thing.FindThing__Q7StringA` calls, and those that found a Thing.
    pub find_thing_calls: u64,
    pub find_thing_hits: u64,
    /// `Thing.GetPodThing__b` calls, and those that found a Thing.
    pub pod_thing_calls: u64,
    pub pod_thing_hits: u64,
    /// `Thing.SetRenderMeshMesh` calls on a Thing with a PRenderMesh, and those that
    /// changed its mesh.
    pub mesh_sets: u64,
    pub mesh_changes: u64,
    /// The last value `PodComputer.SetFadePodDecorations__f` was given, and the calls.
    /// Nothing fades by it (the game's arithmetic was not read).
    pub fade: Option<f32>,
    pub fade_calls: u64,
    /// The `PodComputer.RequestHostUsePod__b` calls that passed the game's test (our
    /// Pod, owned by the local user), oldest first, at most
    /// [`MAX_HOST_USE_POD_REQUESTS`]; the calls that did not pass are counted in
    /// `host_use_pod_refused`.
    pub host_use_pod_requests: Vec<HostUsePodRequest>,
    pub host_use_pod_refused: u64,
    /// The request the host has not answered yet
    /// ([`LevelWorld::begin_session_tick`]).
    pub host_use_pod_pending: Option<bool>,
    /// The answers that changed "in use" in this world, oldest first, at most
    /// [`MAX_HOST_USE_POD_REQUESTS`].
    pub in_use_changes: Vec<InUseChange>,
    /// The last GUID `PodComputer.SetPodTransitionTexture__g` was given, and the calls.
    /// Nothing is textured by it.
    pub transition_texture: Option<u32>,
    pub transition_texture_calls: u64,
    /// `Native.GetIsInPadZone__Q5Thing` calls with a Thing, and those that were true.
    pub pad_zone_calls: u64,
    pub pad_zone_hits: u64,
    /// `Input.IsJustClickedByID__iiQ6String` calls, and those that were true.
    pub action_click_calls: u64,
    pub action_clicks: u64,
    /// The calls with an action whose test was not read (answered false), by action.
    pub actions_not_read: BTreeMap<i32, u64>,
    /// The on-screen prompts those calls asked for, by the key of the prompt.
    pub action_prompts: BTreeMap<String, u64>,
    /// `Native.GetPlayerColour__Q5Thing` calls with a Thing that has a PYellowHead.
    pub player_colour_calls: u64,
    /// D327: what the pod's search state told the planets and the pod computer.
    pub search: PlanetsSearch,
    /// D358: the pod computer `PodComputer.TriggerOnPlanetCancel__` was called on this
    /// tick (the search state's Back), for [`crate::LevelScripts::planet_cancel`].
    pub planet_cancel: Option<lbp_script_vm::ObjectId>,
    /// D359: the pod computer, the last receiver of a `PodComputer` native (for the
    /// engine's calls into its script exports).
    pub puter: Option<lbp_script_vm::ObjectId>,
    /// D359: the SlotID the last group choice passed (`SearchDetails.CreateGroup`), which
    /// the group's details answer from `SearchDetailsGroup.GetSlotID__`.
    pub group_slot: Option<lbp_script_vm::ObjectId>,
    /// D362: the level a Cross on a story level asked to launch (the pod's pending
    /// SlotID, pod+0x954, FOUND 0x4033b4); `PodComputer.IsLaunchingLevel__` is true
    /// while it is set. Nothing loads the level yet: the load's end was not traced.
    pub launch: Option<(i32, i32)>,
    /// D363: the level the pod handed to the game's level load (FUN_004022fc), once the
    /// launch's request is done; the launch stays pending until the level takes over.
    pub level_start: Option<LevelStart>,
    /// D362: the level the pod is in (the current SlotID the classifier compares with,
    /// 0x3fce40); none in the pod itself (INFERRED).
    pub current_level: Option<(i32, i32)>,
    /// D367: the local players' controllers ([`party_or_default`] when empty).
    pub party: Vec<PartyPad>,
    /// D367: the launch warnings still up (the first is shown); the hand-off waits
    /// until they are answered ([`crate::LevelWorld::answer_launch_warning`]).
    pub launch_warnings: Vec<LaunchWarning>,
    /// D367: the world's warning flags (world+0x3b0), set again by each launch.
    pub warning_flags: u32,
    /// D362: a group's `[completed, completable]` level counts
    /// (`Slot.GetNumCompletedLevelsInGroup`, `GetNumCompletableLevelsInGroup`); nothing
    /// fills it offline, so every group reads 0 as the unbound natives do.
    pub group_levels: BTreeMap<(i32, i32), [u32; 2]>,
}

/// D327 (part 78): the values the search state (`PodStateSearch`) hands the planets and
/// the pod computer, kept as the last one given; nothing moves or draws by them yet.
#[derive(Debug, Clone, Default)]
pub struct PlanetsSearch {
    /// `Planets.SelectPlanet__i`: a crafteroid index (1 = Earth, `GetCrafteroid__i`);
    /// `Planets.GetFocusPlanet__` answers that crafteroid, null before any.
    pub selected_planet: Option<i32>,
    /// `Planets.SetCursorMode__i`, `SetBrowseMode__Q25Planets.EPlanetBrowseMode`,
    /// `SetAllowedBrowseModes__i`.
    pub cursor_mode: Option<i32>,
    pub browse_mode: Option<i32>,
    pub allowed_browse_modes: Option<i32>,
    /// `PodComputer.SetLoader__Q23PodComputer.ELoaderType`, `SetEarthHasControl__b`.
    pub loader: Option<i32>,
    pub earth_has_control: Option<bool>,
    /// `PodComputerPlanets.SetWantSearchDetails__Q13SearchDetailsbbb` calls.
    pub want_search_details_calls: u64,
    /// `Planets.SetPublishCursorBadge__Q18SearchResultHandle` calls.
    pub publish_cursor_badge_calls: u64,
    /// `SearchDetails.ShowErrorMessage__` calls (the search answered no results on its
    /// first page); no message is drawn.
    pub error_message_calls: u64,
    /// D328: `SearchResults.SetDefaultResult__` / the default found when the results were
    /// made, and the Earth planet's cursor result (`Planet.GetCurResult__`), as indices
    /// into [`LevelWorld::story_results`].
    pub default_result: Option<usize>,
    pub cur_result: Option<usize>,
    /// D360: the group whose search is up (a Story group chosen), `None` for the Story
    /// search itself. FOUND (01.33, 0x44dfc0): a group search's results ask for the zoom
    /// (`GetWantZoom__`, view 7) and pick their own default result
    /// ([`crate::LevelWorld::group_default_result`]).
    pub group: Option<(i32, i32)>,
    /// D328: `Planet.SetSearchResults__Q13SearchResults` calls.
    pub set_search_results_calls: u64,
    /// D368: the Earth planet's cursor direction (Planet+0x190, a unit vector), `None`
    /// before the cursor first moves; the flight in progress (Planet+0x140 set) and the
    /// move step (`GetMoveCursorStep__`, Planet+0x148).
    pub cursor_dir: Option<[f32; 3]>,
    pub cursor_flight: Option<CursorFlight>,
    pub cursor_step: f32,
    /// D374: the planets' land-at-once byte (Planets+0xb0), from `SetWantSearchDetails`'
    /// first flag: a flight lands on its first frame (FOUND 01.33, 0x4298e8). INFERRED:
    /// set at the call; the game copies it over on its next planet change (0x43c9dc).
    pub land_at_once: bool,
    /// D340: `PWorld.Frame` when the Story info bubble was first up (the page's frame
    /// count starts there; [`crate::LevelWorld::story_info_bubble`]), `None` while it is not.
    pub info_bubble_since: Option<i32>,
}

/// D368 (part 115): the cursor's flight to a result (FOUND 01.33, set by
/// `MoveCursorToResult` 0x429554, flown by 0x4298e8): the start and target directions
/// (Planet+0x160, +0x150) and the progress (+0x144, an angle in radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorFlight {
    pub start: [f32; 3],
    pub target: [f32; 3],
    pub progress: f32,
}

/// D368: Planet+0x1c in the flight's speed, unidentified. INFERRED 0 (no size read
/// offline), which gives the fastest flight the formula allows.
pub const CURSOR_FLIGHT_PLANET_SIZE: f32 = 0.;

fn unit3(l: [f32; 4]) -> Option<[f32; 3]> {
    let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
    (n > 0.).then(|| [l[0] / n, l[1] / n, l[2] / n])
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl PlanetsSearch {
    /// D368: `Planet.MoveCursorToResult` (FOUND 01.33, 0x429554): the result changes at
    /// once; with `snap` the cursor jumps to the result's direction (its slot's
    /// `location`), else a flight starts from the cursor's direction to it, progress 0
    /// and step 0. A cursor not yet placed starts from `from` (the result it was on).
    pub fn move_cursor_to(
        &mut self,
        result: usize,
        location: Option<[f32; 4]>,
        from: Option<[f32; 4]>,
        snap: bool,
    ) {
        self.cur_result = Some(result);
        let Some(target) = location.and_then(unit3) else {
            self.cursor_flight = None;
            return;
        };
        let start = self.cursor_dir.or_else(|| from.and_then(unit3));
        match start {
            Some(start) if !snap => {
                self.cursor_flight = Some(CursorFlight {
                    start,
                    target,
                    progress: 0.,
                });
                self.cursor_step = 0.;
            }
            _ => {
                self.cursor_flight = None;
                self.cursor_dir = Some(target);
            }
        }
    }

    /// D368: one frame of the cursor's flight (FOUND 01.33, 0x4298e8, last in the planet
    /// update 0x42f43c). With `d` the angle from start to target, the progress grows by
    /// `((d / pi) * 240 + 60) / (size * 0.5 + 2500)`; once it reaches `d` (or `d` is
    /// under 0.001) the cursor lands: step 1, the target direction, no flight. Else
    /// `s` = smoothstep(progress / d), the step `s * s` and the direction the slerp from
    /// start to target by `s` (a normalized lerp above a dot of 0.999). Returns the result
    /// landed on. D374: it lands at once while [`Self::land_at_once`] is set. Left out:
    /// the cursor's height and blend (+0x1a0, +0x1b0), which nothing reads (part 121).
    pub fn cursor_flight_step(&mut self) -> Option<usize> {
        let f = self.cursor_flight.as_mut()?;
        let cos = dot3(f.start, f.target).clamp(-1., 1.);
        let d = cos.acos();
        f.progress +=
            ((d / std::f32::consts::PI) * 240. + 60.) / (CURSOR_FLIGHT_PLANET_SIZE * 0.5 + 2500.);
        if f.progress >= d || d < 0.001 || self.land_at_once {
            let target = f.target;
            self.cursor_flight = None;
            self.cursor_step = 1.;
            self.cursor_dir = Some(target);
            return self.cur_result;
        }
        let t = f.progress / d;
        let s = (3. * t * t - 2. * t * t * t).clamp(0., 1.);
        let (a, b) = if cos > 0.999 {
            (1. - s, s)
        } else {
            let sin = d.sin();
            (((1. - s) * d).sin() / sin, (s * d).sin() / sin)
        };
        let v = [0, 1, 2].map(|k| a * f.start[k] + b * f.target[k]);
        self.cursor_step = s * s;
        self.cursor_dir = unit3([v[0], v[1], v[2], 0.]);
        None
    }
}

/// D363: what the pod hands to the game when a launch's request is done (FUN_004022fc ->
/// FUN_002467b8): the slot's root level (game+0x2f0), its SlotID (game+0x328/+0x32c) and
/// the launch mode (game+0x120; 0 Play), after which the game enters its level load
/// (state 7, or 6 or 8 by its flags; FUN_002258bc).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelStart {
    pub slot: (i32, i32),
    pub root: Option<lbp_level::serial::Descriptor>,
    pub mode: u32,
}

/// D328: one slot of the Story search, as the engine keeps it for a result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchSlot {
    /// `SlotID` (`Type`, `SlotNumber`).
    pub id: (i32, i32),
    /// The slot's group, when not (0, 0).
    pub group: Option<(i32, i32)>,
    pub location: [f32; 4],
    /// D339: the slot's translation tag, stored name and author name (the info bubble's
    /// text, [`crate::LevelWorld::story_info_bubble`]).
    pub tag: Option<String>,
    pub name: String,
    pub author: Option<String>,
    /// D340: the slot's `IsSubLevel` and `DeveloperLevelType` (the goodies row's group
    /// sum, [`crate::LevelWorld::story_goodies`]).
    pub is_sub_level: bool,
    pub developer_level_type: Option<u32>,
    /// D342: the slot's icon (the picture on its badge card).
    pub icon: Option<lbp_level::serial::Descriptor>,
    /// D363: the slot's root level (the descriptor the launch hands to the game's load).
    pub root: Option<lbp_level::serial::Descriptor>,
    /// D367: the slot's `MinPlayers`, `MaxPlayers` (slot+0xc5, +0xc6) and
    /// `MoveRecommended` (slot+0xc7), which the launch warnings read.
    pub min_players: Option<u8>,
    pub max_players: Option<u8>,
    pub move_recommended: bool,
    /// D381: the slot's `PrimaryLinkLevel` when not (0, 0): the next story level, the
    /// one a finish of this one unlocks (MEASURED on the disc's story list: level N's
    /// link is level N+1), and its `InitiallyLocked`.
    pub link: Option<(i32, i32)>,
    pub initially_locked: bool,
}

/// D381: a pending link of the profile (FOUND, 01.33: a 16-byte element of the vector
/// at profile+0x2b0, count +0x2b4): the finished level's SlotID and the SlotID it
/// unlocks. The Pod's `LevelUnlockingThread` plays them out and clears them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingLink {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

/// D381: the story state of the session's profile: the levels finished, the slots
/// unlocked since the start (beyond those not `InitiallyLocked`) and the pending links.
/// Like the progression flags it is the profile's, not a level's
/// ([`LevelWorld::hand_session_to`]). Nothing is saved.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StoryProgress {
    pub completed: Vec<(i32, i32)>,
    pub unlocked: Vec<(i32, i32)>,
    pub pending_links: Vec<PendingLink>,
    /// D383: the badges `LevelUnlockingThread` marks unlocking until they unlock.
    pub unlocking: Vec<(i32, i32)>,
}

impl StoryProgress {
    /// D1011 (FOUND, 0x19ab60 -> 0x19aa7c): clearing a pending link removes it and, when
    /// its target's SlotID type is below 2 or is 8, sets bit 1 of the target's record
    /// (+0x32, made if missing), the bit the lock query 0x1a7efc reads as unlocked
    /// (part 141). So the target opens whether or not the unlock cutscene played it.
    pub fn clear_pending_link(&mut self, link: PendingLink) {
        self.pending_links.retain(|x| *x != link);
        let to = link.to;
        if (to.0 < 2 || to.0 == 8) && !self.unlocked.contains(&to) {
            self.unlocked.push(to);
        }
    }
}

/// D381: the slot numbers from which the level-complete path links a slot (FOUND,
/// 0x1aa43c: below 0x506ca; the other branch, numbers from 0x506cc but not 0x54747,
/// is not ported: no story slot is there).
const LINKED_SLOT_NUMBERS: i32 = 0x506ca;

/// D367: one local player's controller, as the launch warnings see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyPad {
    /// A Motion Controller (the player's pad entry, +0x6a0 then +0x150, is set).
    pub motion: bool,
    /// The player holds the pad the menus answer to (the primary user's).
    pub primary: bool,
}

/// D367 (part 114): a warning popup (type 0x4b, buttons Continue and Cancel) that a
/// level launch raises (FOUND 0x402b24, mode Play). The bit is the one the launch sets
/// in the world's warning flags (world+0x3b0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchWarning {
    /// The level recommends Move and the primary player has no Motion Controller.
    MotionNeeded,
    /// The level recommends Move and another player has no Motion Controller.
    MotionNotAll,
    /// Fewer players than the level's minimum; the slot's minimum and maximum (D375,
    /// text from 0x16e3b0).
    TooFewPlayers(u8, u8),
    /// More players than the level's maximum; the slot's minimum and maximum (D375,
    /// text from 0x16de78).
    TooManyPlayers(u8, u8),
}

impl LaunchWarning {
    /// D375 (part 122): the LAMS key of the popup's text and the numbers put in it.
    /// FOUND (01.33): the player counts' texts are built by 0x16e3b0 (too few) and
    /// 0x16de78 (too many) from the slot's minimum and maximum, never the party's count:
    /// a range text with both bounds (too many: the maximum first), or with equal bounds
    /// a text with the one bound; too many with both bounds 1 has a text of its own.
    pub fn text(self) -> (u32, Vec<i64>) {
        let n = |v: u8| i64::from(v);
        match self {
            LaunchWarning::MotionNeeded => (0x5d46982a, vec![]),
            LaunchWarning::MotionNotAll => (0xae61784b, vec![]),
            LaunchWarning::TooFewPlayers(a, b) if a == b => (0xf5796f9e, vec![n(a)]),
            LaunchWarning::TooFewPlayers(a, b) => (0x018707a4, vec![n(a), n(b)]),
            LaunchWarning::TooManyPlayers(1, 1) => (0x1a2ff9c1, vec![]),
            LaunchWarning::TooManyPlayers(a, b) if a == b => (0x0ada56d4, vec![n(a)]),
            LaunchWarning::TooManyPlayers(a, b) => (0x16e7eeda, vec![n(b), n(a)]),
        }
    }

    /// The bit in the world's warning flags (world+0x3b0); none for `MotionNotAll`.
    pub fn flag(self) -> u32 {
        match self {
            LaunchWarning::MotionNeeded => 1,
            LaunchWarning::MotionNotAll => 0,
            LaunchWarning::TooFewPlayers(..) => 0x10,
            LaunchWarning::TooManyPlayers(..) => 0x20,
        }
    }
}

/// LAMS keys of the warning popups' buttons, Continue and Cancel.
pub const LAUNCH_WARNING_BUTTONS: [u32; 2] = [0x119fbf3b, 0x5162e2b2];

/// D378: the launch warning popup that is up ([`crate::LevelWorld::launch_warning_popup`]):
/// the warning, its text (`None` with no LAMS text) and the labels of its first button
/// (Continue: the launch goes ahead) and second (Cancel: the launch is dropped).
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchWarningPopup {
    pub warning: LaunchWarning,
    pub text: Option<String>,
    pub buttons: [String; 2],
}

/// D367: the local players' controllers; with none set, one player on a Wireless
/// Controller, the viewer's pad.
pub fn party_or_default(party: &[PartyPad]) -> Vec<PartyPad> {
    if party.is_empty() {
        vec![PartyPad {
            motion: false,
            primary: true,
        }]
    } else {
        party.to_vec()
    }
}

/// D367: the warnings a Play launch of `slot` raises for `party`, in the order the
/// game shows them. FOUND (01.33, 0x402b24 mode 0): when the slot recommends Move, the
/// first player without a Motion Controller raises one warning (its text by whether that
/// player is the primary user); then the player count is checked against the slot's
/// minimum, or else its maximum. INFERRED: a slot without the fields (older revisions)
/// allows 1 to 4 players, the slot defaults.
pub fn launch_warnings(slot: &SearchSlot, party: &[PartyPad]) -> Vec<LaunchWarning> {
    let mut out = Vec::new();
    if slot.move_recommended {
        if let Some(p) = party.iter().find(|p| !p.motion) {
            out.push(if p.primary {
                LaunchWarning::MotionNeeded
            } else {
                LaunchWarning::MotionNotAll
            });
        }
    }
    let n = party.len();
    let min = slot.min_players.unwrap_or(1);
    let max = slot.max_players.unwrap_or(4);
    if n < usize::from(min) {
        out.push(LaunchWarning::TooFewPlayers(min, max));
    } else if usize::from(max) < n {
        out.push(LaunchWarning::TooManyPlayers(min, max));
    }
    out
}

/// D375: the game's text formatting (FOUND 01.33, 0x518d0): `%` and a digit 1-9 puts
/// that argument (1-based); `%d`, `%s` or `%l` and a character put the next argument in
/// turn; numbers in decimal (0x514d4). A placeholder past the arguments is kept as
/// written. INFERRED: `%` before any other character is kept, and a numbered placeholder
/// does not move the next-in-turn argument.
pub fn format_lams(text: &str, args: &[i64]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut next = 0;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let (index, written) = match chars.peek().copied() {
            Some(d @ '1'..='9') => {
                chars.next();
                (d as usize - '1' as usize, format!("%{d}"))
            }
            Some(k @ ('d' | 's')) => {
                chars.next();
                next += 1;
                (next - 1, format!("%{k}"))
            }
            Some('l') => {
                chars.next();
                let mut w = String::from("%l");
                w.extend(chars.next());
                next += 1;
                (next - 1, w)
            }
            _ => {
                out.push('%');
                continue;
            }
        };
        match args.get(index) {
            Some(v) => out.push_str(&v.to_string()),
            None => out.push_str(&written),
        }
    }
    out
}

impl SearchSlot {
    /// The search slots of a slot list, in list order (every slot, groups included).
    pub fn from_list(list: &lbp_level::slots::SlotList) -> Vec<Self> {
        let id = |s: lbp_level::slots::SlotId| (s.slot_type as i32, s.number as i32);
        list.slots
            .iter()
            .map(|s| SearchSlot {
                id: id(s.id),
                group: s.group.map(id).filter(|g| *g != (0, 0)),
                location: s.location,
                tag: s.translation_tag.clone().filter(|t| !t.is_empty()),
                name: s.name.clone(),
                author: s.author_name.clone().filter(|a| !a.is_empty()),
                is_sub_level: s.is_sub_level.unwrap_or(false),
                developer_level_type: s.developer_level_type,
                icon: s.icon,
                root: s.root,
                min_players: s.min_players.map(|n| n as u8),
                max_players: s.max_players.map(|n| n as u8),
                move_recommended: s.move_recommended.unwrap_or(false),
                link: Some(id(s.primary_link_level)).filter(|l| *l != (0, 0)),
                initially_locked: s.initially_locked,
            })
            .collect()
    }
}

/// D328: the script objects made for one [`SearchSlot`]: its SearchResultSlot, the
/// SearchResultHandle `GetHandle__` gives (`_Type` 1, `_SlotID`) and that SlotID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryResult {
    pub result: ObjectId,
    pub handle: ObjectId,
    pub slot_id: ObjectId,
    /// The index of the group's result, when the slot has a group in the list.
    pub parent: Option<usize>,
}

/// Most requests [`PodState::host_use_pod_requests`] keeps.
pub const MAX_HOST_USE_POD_REQUESTS: usize = 256;

impl LevelWorld {
    /// The first Thing, in Thing order, whose PScriptName is exactly `name`.
    ///
    /// FOUND (01.33, 0xd3360): the game walks a list of the PWorld and compares each
    /// entry's byte string with the name by the C library's string compare: every byte,
    /// case sensitive. The first equal entry wins; none gives null. INFERRED: the list
    /// is the level's PScriptName parts in Thing order. A name with a NUL inside is
    /// kept up to the NUL by the reader, as a C string ends there.
    pub fn find_thing(&self, name: &[u8]) -> Option<u32> {
        let named = |t: &crate::ThingState| t.script_name.as_deref() == Some(name);
        self.things.iter().position(named).map(|i| i as u32)
    }

    /// How many Things have a PScriptName, and how many of those names are not empty.
    pub fn script_names(&self) -> (usize, usize) {
        let names = || self.things.iter().filter_map(|t| t.script_name.as_ref());
        (names().count(), names().filter(|n| !n.is_empty()).count())
    }

    /// The pod Thing: what `Thing.GetPodThing__b` answers.
    ///
    /// FOUND (01.33, 0xd32dc): the game walks a list of the PWorld whose entries have a
    /// Thing and a resource. The first entry whose resource is [`POD_MESH`] gives its
    /// Thing. An entry whose resource is [`POD_PLATFORM_MESH`] gives its Thing when
    /// `platform_too`, and ends the search with nothing when not. No such entry: nothing.
    /// INFERRED: the list is the level's PRenderMesh parts in Thing order. The mesh is
    /// the one the PRenderMesh shows now, so the answer follows `SetRenderMeshMesh`.
    pub fn pod_thing(&self, platform_too: bool) -> Option<u32> {
        for (i, t) in self.things.iter().enumerate() {
            if t.render_mesh_visibility.is_none() {
                continue;
            }
            match t.mesh_guid {
                Some(POD_MESH) => return Some(i as u32),
                Some(POD_PLATFORM_MESH) => return platform_too.then_some(i as u32),
                _ => {}
            }
        }
        None
    }

    /// What `PodComputer.GetPlanetsBackgroundGUID__` answers.
    pub fn planets_background(&self) -> u32 {
        self.pod
            .planets_background
            .unwrap_or(DEFAULT_PLANETS_BACKGROUND)
    }

    /// The Pod's change-over value: 0 with the tent, 1 with the platform; none before
    /// the Pod's script has run an update.
    ///
    /// FOUND (the script class `Pod`, the world script of pod.bin): the value is the
    /// private field `PodTransition`, stepped by 0.05 a tick toward 1 while the pod
    /// computer is in use and toward 0 while it is not, and handed to
    /// `PodComputer.SetFadePodDecorations__f` on every update. This is the last value
    /// that native was given ([`PodState::fade`]), so a host need not know the class.
    pub fn pod_transition(&self) -> Option<f32> {
        self.pod.fade
    }

    /// The number of the primary local player, if there is a player.
    ///
    /// FOUND (01.33, 0x240a88): the game picks one entry of the session's player list.
    /// INFERRED: with one local player that is him; here the first player record.
    pub fn primary_player(&self) -> Option<i32> {
        self.player_records.first().map(|r| r.number)
    }

    /// What `Native.GetIsInPadZone__Q5Thing` answers for a Thing: the player stands
    /// where he can engage the pod computer.
    ///
    /// FOUND (01.33, 0x13bac0): false unless the level is our Pod
    /// ([`LevelWorld::our_pod`]) and the Thing's PYellowHead has the primary player's
    /// number. Then the game walks a list of the PWorld; an entry counts when its
    /// Thing's script instance passes a class test and a call by name, and the Thing
    /// asked about is in the entry's list of Things inside. The first such entry
    /// decides: false when the Thing has no PPos, else his world position z is at least
    /// [`PAD_ZONE_MIN_Z`]. No entry: false. There is no distance test: the zone is the
    /// trigger's own.
    ///
    /// INFERRED: the list is the level's PTriggers in Thing order and the class is
    /// [`POD_TRIGGER_CLASS`]. The call by name is not made here (the class's one
    /// function of that kind returns true). The Things with a player number are the
    /// Sackboys [`LevelWorld::add_player`] made. The classes are known after
    /// [`LevelWorld::add_class_defaults`]; before it no trigger counts.
    pub fn in_pad_zone(&self, thing: u32) -> bool {
        let Some(player) = self.players.iter().find(|p| p.thing == thing) else {
            return false;
        };
        if !self.our_pod || Some(player.number) != self.primary_player() {
            return false;
        }
        let holds = |t: &crate::ThingState| {
            t.script_classes.iter().any(|c| c == POD_TRIGGER_CLASS)
                && t.trigger
                    .as_ref()
                    .is_some_and(|g| g.inside.contains(&thing))
        };
        if !self.things.iter().any(holds) {
            return false;
        }
        let matrix = self.things.get(thing as usize).and_then(|t| t.world_matrix);
        matrix.is_some_and(|m| m[14] >= PAD_ZONE_MIN_Z)
    }

    /// Where a host puts a Sackboy to have him in the pod computer's pad zone: the world
    /// position of the first Thing, in Thing order, that has a PTrigger, a position and
    /// the script class [`POD_TRIGGER_CLASS`], with the trigger's z offset added. None
    /// in a level without such a trigger, and before
    /// [`LevelWorld::add_class_defaults`] (the classes are not known yet).
    ///
    /// PLACEHOLDER for a player who walks there: the middle of the trigger is a spot
    /// that is inside it, not a spot the game puts anyone on. [`LevelWorld::in_pad_zone`]
    /// is true for a Sackboy [`LevelWorld::add_player`] made there once a tick has put
    /// him in the trigger's list, when the level is our Pod and the spot's z is at
    /// least [`PAD_ZONE_MIN_Z`].
    pub fn pod_trigger_spot(&self) -> Option<[f32; 3]> {
        self.things.iter().find_map(|t| {
            let (g, m) = (t.trigger.as_ref()?, t.world_matrix?);
            let pod = t.script_classes.iter().any(|c| c == POD_TRIGGER_CLASS);
            pod.then_some([m[12], m[13], m[14] + g.z_offset])
        })
    }

    /// `PodComputer.RequestHostUsePod__b`.
    ///
    /// FOUND (01.33, 0x3fa974, 0x3fa9cc): only when the level is our Pod and the Pod
    /// belongs to the local user, the game stores the request (use or stop) with the
    /// time and sends a message to the host. "In use" does not change here: the
    /// receiver of the message sets it (0x3fa868).
    ///
    /// INFERRED: the one local player owns the Pod he starts and is the host, so only
    /// [`LevelWorld::our_pod`] is tested, and the host's answer is the request itself,
    /// taken at the start of the next tick ([`LevelWorld::begin_session_tick`]; the
    /// delay in the game was not measured). A second request before the answer
    /// replaces the first (the game keeps one request state).
    pub fn request_host_use_pod(&mut self, in_use: bool) {
        if !self.our_pod {
            self.pod.host_use_pod_refused += 1;
            return;
        }
        self.pod.host_use_pod_pending = Some(in_use);
        if self.pod.host_use_pod_requests.len() < MAX_HOST_USE_POD_REQUESTS {
            self.pod.host_use_pod_requests.push(HostUsePodRequest {
                in_use,
                frame: self.pworld.frame,
            });
        }
    }

    /// The start of a tick for the state outside the level: the host's answer to a
    /// pending [`LevelWorld::request_host_use_pod`] arrives. [`LevelScripts::tick_level`]
    /// calls it; [`tick_session`] calls it on every level before any of them ticks.
    ///
    /// FOUND (01.33, 0x3fa868 -> 0x18ed30): the receiver of the host's message sets "in
    /// use" to the message's value with sound, and clears the request. With sound, and
    /// when the primary player's avatar exists, a change to true plays
    /// [`POD_COMPUTER_STARTUP_SOUND`] and a change to false
    /// [`POD_COMPUTER_SHUTDOWN_SOUND`]. Nothing plays here: the sound is counted in
    /// [`LevelWorld::sounds_played`] of the world that has the avatar.
    /// Left out: on turning off the game also tells the pod computer object (0x402478).
    pub fn begin_session_tick(&mut self) {
        let Some(in_use) = self.pod.host_use_pod_pending.take() else {
            return;
        };
        if self.pod_computer_in_use == in_use {
            return;
        }
        self.pod_computer_in_use = in_use;
        if self.pod.in_use_changes.len() < MAX_HOST_USE_POD_REQUESTS {
            self.pod.in_use_changes.push(InUseChange {
                in_use,
                frame: self.pworld.frame,
            });
        }
        let primary = self.primary_player();
        if self.players.iter().any(|p| Some(p.number) == primary) {
            let sound = if in_use {
                POD_COMPUTER_STARTUP_SOUND
            } else {
                POD_COMPUTER_SHUTDOWN_SOUND
            };
            *self.sounds_played.entry(sound.to_owned()).or_default() += 1;
        }
    }

    /// Copies the state the game keeps outside a level from this world to `other`, a
    /// level played in the same session: the profile's progression flags, "our Pod",
    /// "pod computer in use" with a request that waits for its answer, the video and
    /// voice-over requests, and per player (records with the same number) the colour
    /// and the buttons that were used up.
    ///
    /// FOUND (01.33): "our Pod" and "in use" are two bytes of the one game object, the
    /// flags are the profile's, and the player records hang off the game object: none
    /// of them belongs to a level. The game holds the Pod and the planets level at once.
    /// The rest of a level world (Things, the camera a script wrote, the counts) is the
    /// level's own and is not copied.
    pub fn hand_session_to(&self, other: &mut LevelWorld) {
        other.progression_flags = self.progression_flags;
        other.story_progress.clone_from(&self.story_progress);
        other.our_pod = self.our_pod;
        other.pod_computer_in_use = self.pod_computer_in_use;
        other.pod.host_use_pod_pending = self.pod.host_use_pod_pending;
        other.media.clone_from(&self.media);
        other.profile_colour = self.profile_colour;
        other.profile_stat_colour = self.profile_stat_colour;
        for mine in &self.player_records {
            let theirs = other.player_records.iter_mut();
            let Some(theirs) = theirs.into_iter().find(|r| r.number == mine.number) else {
                continue;
            };
            theirs.colour = mine.colour;
            if let (Some(a), Some(b)) = (mine.pad, theirs.pad.as_mut()) {
                b.use_up(a.held_off);
            }
        }
    }
}

impl LevelWorld {
    /// D381: the level-complete path for the level at `slot` (FOUND, 01.33: 0x1ad654,
    /// which returns at once for slot types 5, 6 and 7, then 0x1aa43c and the add
    /// 0x1a8764). The slot is marked finished; then, when it is a story slot with a link
    /// whose target is still locked, the link {slot, target} is added unless the list
    /// holds it already. The added link, if any.
    ///
    /// Not ported: the add's special case for a target numbered 570 (no story slot is),
    /// and 0x1a88f4's tables of the level's own unlocks (their loader was not found).
    /// INFERRED: the add's lock test (0x1a7efc, group-aware) is [`Self::slot_unlocked`].
    pub fn finish_story_level(&mut self, slot: (i32, i32)) -> Option<PendingLink> {
        if matches!(slot.0, 5..=7) {
            return None;
        }
        let progress = &mut self.story_progress;
        if !progress.completed.contains(&slot) {
            progress.completed.push(slot);
        }
        if slot.1 >= LINKED_SLOT_NUMBERS {
            return None;
        }
        let to = self.story_slots.iter().find(|s| s.id == slot)?.link?;
        if self.slot_unlocked(to) {
            return None;
        }
        let link = PendingLink { from: slot, to };
        let pending = &mut self.story_progress.pending_links;
        if !pending.contains(&link) {
            pending.push(link);
        }
        Some(link)
    }

    /// D381: the slot is open to play: unlocked this session, or a story slot that is
    /// not `InitiallyLocked`. A slot not in the story list counts as unlocked.
    /// D396 (FOUND 0x1a7efc, part 143): a developer group or a DLC pack (SlotID types 4
    /// and 9) with slots in it is locked only while all of them are; one without takes
    /// its own slot's lock (INFERRED: the empty case was not read).
    pub fn slot_unlocked(&self, id: (i32, i32)) -> bool {
        if self.story_progress.unlocked.contains(&id) {
            return true;
        }
        if matches!(id.0, 4 | 9) {
            let mut members = self
                .story_slots
                .iter()
                .filter(|s| s.group == Some(id) && s.id != id)
                .peekable();
            if members.peek().is_some() {
                return members.any(|s| self.slot_unlocked(s.id));
            }
        }
        let slot = self.story_slots.iter().find(|s| s.id == id);
        slot.is_none_or(|s| !s.initially_locked)
    }
}

/// One level of a session ([`tick_session`]): its scripts, the VM they run on and its
/// world.
pub struct SessionLevel<'a> {
    pub scripts: &'a mut LevelScripts,
    pub vm: &'a mut Vm,
    pub host: &'a mut Bindings<LevelWorld>,
}

/// One tick of several levels that are loaded at once, as the game holds the Pod and
/// the planets level: every world takes the host's answer
/// ([`LevelWorld::begin_session_tick`]), then the levels tick in the order given, and
/// after each one its session state goes to all the others
/// ([`LevelWorld::hand_session_to`]). So a flag one level's script sets is seen by the
/// levels after it on the same tick and by the levels before it on the next.
///
/// Before the first tick the caller makes the worlds agree (`hand_session_to` once), and
/// again after it changes one world's session state itself between two ticks.
/// INFERRED: the order of the levels in the game's frame was not read. Each level has
/// its own VM here (a Thing handle is an index into one level world), so script
/// statics and the PodComputer script object are per level; the game has one of each.
pub fn tick_session(levels: &mut [SessionLevel<'_>], budget: u64) -> Vec<StepReport> {
    for level in levels.iter_mut() {
        level.host.world.begin_session_tick();
    }
    let mut reports = Vec::with_capacity(levels.len());
    for i in 0..levels.len() {
        let (before, rest) = levels.split_at_mut(i);
        let Some((level, after)) = rest.split_first_mut() else {
            break;
        };
        reports.push(
            level
                .scripts
                .tick_level_in_session(level.vm, level.host, budget),
        );
        for other in before.iter_mut().chain(after) {
            level.host.world.hand_session_to(&mut other.host.world);
        }
    }
    reports
}
