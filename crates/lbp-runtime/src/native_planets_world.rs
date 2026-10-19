//! D356: the native port of the Pod planets level's world script (`PlanetsWorld`,
//! g00016e0a) and its view object (`PlanetsView`, g0001736c).
//!
//! FOUND (the 01.33 scripts, read from the decrypted overlay for research only): the
//! engine calls `UpdatePlanetsView` once a frame after the planets' own updates. It blends
//! the current view toward the one the pod computer asked for (`SetViewFromRemotePodComputer`),
//! runs the world's state, moves the camera (`UpdatePodCamera`), hands Earth and the Moon
//! their spin offsets, puts the whole level under the view's transform and sets the depth
//! of field. The view objects are kept as members of the world Thing, one per field
//! (`CurrentView.Yaw`, ...): a port has no script objects of its own.
//!
//! D379: the pod introduction (`PodIntroductionThread`) is ported as the world's own
//! state 1: its camera zoom over 451 ticks, then the progression flags. Not ported: the
//! level unlocking cutscene thread (`LevelUnlockingThread`, null: state 2 ends on the
//! next frame; ported since D383), the intro's video and voice-over requests (made
//! since D398 and D1000), the "level created" voice-over and the interactive music.
use lbp_script_vm::{Host, MemberKind, Value};

use crate::native_script::{
    as_bool, as_float, as_int, as_mat44, as_thing, as_vec4, call, site, NativeResult, NativeScript,
    NativeStep, PLANET_THING,
};

/// The script GUID of `PlanetsWorld` (FOUND: the 01.33 patch's `planetsworld.ff` row).
pub const PLANETS_WORLD_SCRIPT: u32 = 0x16e0a;
/// D1000: the video GUID the pod introduction asks the pod computer to start (FOUND).
pub const INTRO_VIDEO: u32 = 139230;
/// D1004 (FOUND, `.init__`): the Pod's interactive music, `FSB` (the bank, `I_POD.FSB`
/// on the disc) and `SettingsFile`.
pub const POD_MUSIC: u32 = 42103;
pub const POD_MUSIC_SETTINGS: u32 = 42108;

const PLANETS_WORLD: &str = "PlanetsWorld";
const DONE: NativeStep = NativeStep::Done(Value::Void);

/// The camera (from, to) while the pod computer is in use, and otherwise (FOUND,
/// constants of the script).
const IN_USE_POS: [f32; 4] = [0., -300., 900., 1.];
const IN_USE_FOC: [f32; 4] = [0., 500., -1500., 1.];
const REST_POS: [f32; 4] = [0., -250., 1130., 1.];
const REST_FOC: [f32; 4] = [0., 100., -1500., 1.];
const UP: [f32; 4] = [0., 1., 0., 0.];
/// The camera (from, to) the pod introduction zooms in from (FOUND, constants of the
/// script); it ends at the rest framing.
const INTRO_FAR_POS: [f32; 4] = [-500., -2000., 2000., 1.];
const INTRO_FAR_FOC: [f32; 4] = [-100., -800., 1000., 1.];
/// `PodIntroductionThread`'s zoom runs 0 to 1 in this many steps, one a tick (FOUND).
const INTRO_STEPS: i32 = 450;

/// The three view objects of the world.
const VIEWS: [&str; 3] = ["CurrentView", "OldView", "DesiredView"];

/// One `PlanetsView`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetsView {
    pub kind: i32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub offset: [f32; 4],
    pub focus: [f32; 4],
    pub dof_min: f32,
    pub dof_max: f32,
    pub fog_bias: f32,
    pub earth_spin_offset: f32,
    pub moon_spin_offset: f32,
    pub focus_planet: i32,
}

impl Default for PlanetsView {
    /// `.init__`: no focus planet, type 46 (no view).
    fn default() -> Self {
        Self {
            kind: 46,
            pitch: 0.,
            yaw: 0.,
            roll: 0.,
            offset: [0.; 4],
            focus: [0.; 4],
            dof_min: 0.,
            dof_max: 0.,
            fog_bias: 0.,
            earth_spin_offset: 0.,
            moon_spin_offset: 0.,
            focus_planet: 0,
        }
    }
}

fn add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + b[i])
}

fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn scaled(a: [f32; 4], s: f32) -> [f32; 4] {
    a.map(|x| x * s)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1. - t) + b * t
}

impl PlanetsView {
    /// `SetPlanetsView__i`: the view `kind`, focused on Earth's or the Moon's place.
    /// FOUND: every view starts from the defaults (no turn, offset (0, 0, -15000), DOF
    /// 40000-50000) and the focus planet is not reset (a view the table does not name
    /// keeps the old one). Views whose rows match are grouped as the script groups them.
    pub fn set(&mut self, kind: i32, earth: [f32; 4], moon: [f32; 4], is_4to3: bool) {
        let fpt = self.focus_planet;
        *self = Self {
            kind,
            offset: [0., 0., -15000., 0.],
            focus: [0., 0., 0., 1.],
            dof_min: 40000.,
            dof_max: 50000.,
            focus_planet: fpt,
            ..Self::default()
        };
        let wide = if is_4to3 { 200. } else { 0. };
        // (offset x, y, z), yaw, pitch, roll, focus (0 origin, 1 Earth, 2 Moon), focus
        // planet type, DOF min, max, fog bias, Earth spin offset, Moon spin offset.
        type Row = ([f32; 3], f32, f32, f32, u8, i32, f32, f32, f32, f32, f32);
        let row: Option<Row> = match kind {
            0 => Some((
                [0., 0., -15000.],
                -0.07,
                0.15,
                0.,
                0,
                0,
                40000.,
                50000.,
                10000.,
                0.,
                0.,
            )),
            1 => Some((
                [-700., 400., -5000.],
                -0.07,
                0.35,
                0.,
                0,
                0,
                6000.,
                8000.,
                0.,
                0.,
                0.,
            )),
            2 => Some((
                [-750., 0., -1800.],
                0.3,
                0.35,
                0.,
                0,
                0,
                6000.,
                8000.,
                0.,
                0.,
                0.,
            )),
            3 => Some((
                [0., 0., -4000.],
                0.7,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                0.,
                0.,
            )),
            39 => Some((
                [-1000., -35., -4000.],
                0.9,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                0.,
                0.,
            )),
            4 | 6 | 29 => Some((
                [0., 0., -3750.],
                0.7,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                -0.7,
                0.,
            )),
            5 | 7 => Some((
                [0., 0., -2000.],
                0.7,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                -0.7,
                0.,
            )),
            30 => Some((
                [0., 0., -1750.],
                0.7,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                -0.7,
                0.,
            )),
            8 => Some((
                [-1550., -75., -3750.],
                1.1,
                0.32,
                0.,
                1,
                1,
                4500.,
                5500.,
                0.,
                -0.8,
                0.,
            )),
            9 => Some((
                [1850., 0., -4000.],
                2.0,
                0.32,
                0.,
                1,
                0,
                3200.,
                4000.,
                0.,
                -2.2,
                0.,
            )),
            10 | 31 => Some((
                [0., 100., -1200.],
                -1.0,
                0.3,
                0.,
                2,
                2,
                2500.,
                3500.,
                0.,
                0.,
                1.0,
            )),
            11 | 32 => Some((
                [0., 100., -600.],
                -1.0,
                0.3,
                0.,
                2,
                2,
                1900.,
                2900.,
                0.,
                0.,
                1.0,
            )),
            12 => Some((
                [-2400., 0., -1500.],
                1.4,
                0.35,
                0.,
                1,
                1,
                2000.,
                3000.,
                0.,
                -0.3,
                0.,
            )),
            13 => Some((
                [-2400., -600., -1500.],
                1.4,
                0.35,
                -0.2,
                1,
                1,
                1800.,
                2000.,
                0.,
                -0.3,
                0.,
            )),
            14 => Some((
                [-3800., 0., -1500.],
                1.5,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            36 => Some((
                [-4400., -800., -1500.],
                1.6,
                0.35,
                0.3,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            37 => Some((
                [-4700., 800., -1500.],
                1.6,
                0.35,
                -0.3,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            15 | 38 => Some((
                [-5200., -900., -2000.],
                1.8,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            16 => Some((
                [-7000., -900., -2500.],
                1.9,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            17 => Some((
                [-4700., 0., -1500.],
                1.5,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            18 => Some((
                [-2150. + wide, 0., -1500.],
                1.4,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            19 => Some((
                [-2300., 300., -1700.],
                1.5,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            20 => Some((
                [-4600., 200., -1500.],
                1.5,
                0.35,
                0.,
                1,
                1,
                500.,
                1000.,
                0.,
                -0.3,
                0.,
            )),
            21 => Some((
                [-1300., 100., -500.],
                -1.4,
                0.3,
                0.,
                2,
                2,
                1500.,
                2000.,
                0.,
                0.,
                2.5,
            )),
            22 => Some((
                [-1050. + wide, 100., -500.],
                -1.4,
                0.3,
                0.,
                2,
                2,
                500.,
                1000.,
                0.,
                0.,
                2.5,
            )),
            23 => Some((
                [-1200., 400., -700.],
                -1.3,
                0.3,
                0.,
                2,
                2,
                500.,
                1000.,
                0.,
                0.,
                2.5,
            )),
            24 => Some((
                [-1000., 0., -3000.],
                -0.7,
                0.35,
                0.,
                1,
                0,
                1500.,
                2500.,
                0.,
                0.,
                1.0,
            )),
            25 => Some((
                [-750. + wide, 0., -3000.],
                -0.7,
                0.35,
                0.,
                1,
                0,
                1500.,
                2500.,
                0.,
                0.,
                1.0,
            )),
            26 => Some((
                [200., 0., -3000.],
                -0.7,
                0.35,
                0.,
                1,
                0,
                1500.,
                2500.,
                0.,
                0.,
                1.0,
            )),
            27 => Some((
                [1000., -50., -3500.],
                0.5,
                0.35,
                0.,
                1,
                1,
                3600.,
                4400.,
                0.,
                -0.55,
                0.,
            )),
            28 => Some((
                [300., 100., -1200.],
                -1.0,
                0.3,
                0.,
                2,
                2,
                2500.,
                3500.,
                0.,
                0.,
                0.85,
            )),
            33 => Some((
                [-1500., 400., -1600.],
                0.25,
                0.35,
                0.,
                0,
                0,
                800.,
                1000.,
                0.,
                0.,
                0.,
            )),
            34 => Some((
                [-800., 600., -1600.],
                0.3,
                0.35,
                0.,
                0,
                0,
                800.,
                1000.,
                0.,
                0.,
                0.,
            )),
            35 => Some((
                [-800., 600., -1625.],
                0.3,
                0.35,
                0.,
                0,
                0,
                800.,
                1000.,
                0.,
                0.,
                0.,
            )),
            40 | 45 => Some((
                [0., 800., -4000.],
                0.5,
                0.2,
                0.,
                1,
                0,
                3600.,
                4400.,
                0.,
                0.,
                0.,
            )),
            41 | 43 => Some((
                [1000., 800., -3900.],
                0.7,
                0.2,
                0.,
                1,
                0,
                3600.,
                4400.,
                0.,
                0.,
                0.,
            )),
            42 | 44 => Some((
                [1000., 800., -3950.],
                0.7,
                0.2,
                0.,
                1,
                0,
                3600.,
                4400.,
                0.,
                0.,
                0.,
            )),
            _ => None,
        };
        let Some((o, yaw, pitch, roll, focus, fpt, dmin, dmax, fog, es, ms)) = row else {
            return;
        };
        self.offset = [o[0], o[1], o[2], 0.];
        self.yaw = yaw;
        self.pitch = pitch;
        self.roll = roll;
        self.focus = match focus {
            1 => earth,
            2 => moon,
            _ => [0., 0., 0., 1.],
        };
        self.focus_planet = fpt;
        self.dof_min = dmin;
        self.dof_max = dmax;
        self.fog_bias = fog;
        self.earth_spin_offset = es;
        self.moon_spin_offset = ms;
    }

    /// `CopyFromTransition`: `a` to `b` by `t`; the type and focus planet are `b`'s once
    /// `t` reaches 1 (and in between when both have the same; else none and 46).
    pub fn between(a: &Self, b: &Self, t: f32) -> Self {
        let both = |x: i32, y: i32, none: i32| match () {
            () if t <= 0. => x,
            () if t >= 1. => y,
            () if x == y => y,
            () => none,
        };
        Self {
            kind: both(a.kind, b.kind, 46),
            pitch: lerp(a.pitch, b.pitch, t),
            yaw: lerp(a.yaw, b.yaw, t),
            roll: lerp(a.roll, b.roll, t),
            offset: std::array::from_fn(|i| lerp(a.offset[i], b.offset[i], t)),
            focus: std::array::from_fn(|i| lerp(a.focus[i], b.focus[i], t)),
            dof_min: lerp(a.dof_min, b.dof_min, t),
            dof_max: lerp(a.dof_max, b.dof_max, t),
            fog_bias: lerp(a.fog_bias, b.fog_bias, t),
            earth_spin_offset: lerp(a.earth_spin_offset, b.earth_spin_offset, t),
            moon_spin_offset: lerp(a.moon_spin_offset, b.moon_spin_offset, t),
            focus_planet: both(a.focus_planet, b.focus_planet, 0),
        }
    }

    /// `GetTransitionSpeed`: the view blend's step a frame.
    pub fn transition_speed(a: &Self, b: &Self) -> f32 {
        let between = |i, j| (a.kind == i && b.kind == j) || (a.kind == j && b.kind == i);
        if between(34, 2) {
            return 0.1;
        }
        if between(34, 35) || between(41, 42) || between(43, 44) {
            return 0.25;
        }
        let (x, y) = (a.focus_planet, b.focus_planet);
        if x != 0 && y != 0 {
            if x == y {
                return 0.1;
            }
            return 0.02;
        }
        0.05
    }

    fn get(host: &mut dyn Host, this: u32, name: &str) -> Self {
        let f =
            |host: &mut dyn Host, field: &str| get_member(host, this, &format!("{name}.{field}"));
        let float = |host: &mut dyn Host, field: &str| f(host, field).map_or(0., |v| as_float(&v));
        let int =
            |host: &mut dyn Host, field: &str, d: i32| f(host, field).map_or(d, |v| as_int(&v));
        let vec =
            |host: &mut dyn Host, field: &str| f(host, field).map_or([0.; 4], |v| as_vec4(&v));
        Self {
            kind: int(host, "Type", 46),
            pitch: float(host, "Pitch"),
            yaw: float(host, "Yaw"),
            roll: float(host, "Roll"),
            offset: vec(host, "Offset"),
            focus: vec(host, "FocusPos"),
            dof_min: float(host, "DofMin"),
            dof_max: float(host, "DofMax"),
            fog_bias: float(host, "FogBias"),
            earth_spin_offset: float(host, "EarthSpinOffset"),
            moon_spin_offset: float(host, "MoonSpinOffset"),
            focus_planet: int(host, "FocusPlanetType", 0),
        }
    }

    fn put(&self, host: &mut dyn Host, this: u32, name: &str) -> Result<(), String> {
        let fields = [
            ("Type", Value::Int(self.kind)),
            ("Pitch", Value::Float(self.pitch)),
            ("Yaw", Value::Float(self.yaw)),
            ("Roll", Value::Float(self.roll)),
            ("Offset", Value::Vec4(self.offset)),
            ("FocusPos", Value::Vec4(self.focus)),
            ("DofMin", Value::Float(self.dof_min)),
            ("DofMax", Value::Float(self.dof_max)),
            ("FogBias", Value::Float(self.fog_bias)),
            ("EarthSpinOffset", Value::Float(self.earth_spin_offset)),
            ("MoonSpinOffset", Value::Float(self.moon_spin_offset)),
            ("FocusPlanetType", Value::Int(self.focus_planet)),
        ];
        for (field, v) in fields {
            set_member(host, this, &format!("{name}.{field}"), v)?;
        }
        Ok(())
    }
}

fn get_member(host: &mut dyn Host, this: u32, field: &str) -> Result<Value, String> {
    host.get_member(
        MemberKind::Thing,
        &Value::Thing(Some(this)),
        PLANETS_WORLD,
        field,
    )
}

fn set_member(host: &mut dyn Host, this: u32, field: &str, v: Value) -> Result<(), String> {
    host.set_member(
        MemberKind::Thing,
        &Value::Thing(Some(this)),
        PLANETS_WORLD,
        field,
        v,
    )
}

fn float(host: &mut dyn Host, this: u32, field: &str) -> f32 {
    get_member(host, this, field).map_or(0., |v| as_float(&v))
}

fn vec4(host: &mut dyn Host, this: u32, field: &str) -> [f32; 4] {
    get_member(host, this, field).map_or([0.; 4], |v| as_vec4(&v))
}

fn int(host: &mut dyn Host, this: u32, field: &str) -> i32 {
    get_member(host, this, field).map_or(0, |v| as_int(&v))
}

fn math(host: &mut dyn Host, function: &str, args: &[Value]) -> Result<Value, String> {
    call(host, "Math", function, args)
}

fn mat(host: &mut dyn Host, function: &str, arg: Value) -> Result<[f32; 16], String> {
    Ok(as_mat44(&math(host, function, &[arg])?))
}

/// D1004 (FOUND, `UpdateInteractiveMusic__`): the stereo pairs' volumes by the view type.
/// Pair 3 is always 1. Views 1, 2, 33 to 35 and 45 add pair 0; the other listed views
/// add pair 0 and one more; view 0 and views the script does not list add none.
pub fn music_pairs(view: i32) -> [f32; 6] {
    let mut v = [0., 0., 0., 1., 0., 0.];
    let more = match view {
        1 | 2 | 33..=35 | 45 => None,
        6 | 7 => Some(4),
        3..=5 | 8 | 9 | 12..=20 | 24..=27 | 29 | 30 | 36 | 37 | 39..=44 => Some(2),
        10 | 11 | 21..=23 | 28 | 31 | 32 => Some(1),
        _ => return v,
    };
    v[0] = 1.;
    if let Some(p) = more {
        v[p] = 1.;
    }
    v
}

/// The native port of `PlanetsWorld`.
pub struct PlanetsWorldScript;

/// `PlanetsWorld` (g00016e0a).
pub static PLANETS_WORLD_PORT: PlanetsWorldScript = PlanetsWorldScript;

impl PlanetsWorldScript {
    /// D1004: `UpdateInteractiveMusic__`, each frame from `UpdatePlanetsView__`. It stops
    /// the music in the attract loop, sets the Pod's music while the current music is
    /// another (a whole-second delay from 1 to 8), and when the view or whether music is
    /// heard changed, sets the six stereo pairs' volumes by the view ([`music_pairs`]);
    /// not heard, as in view 0.
    fn update_interactive_music(host: &mut dyn Host, this: u32) -> Result<(), String> {
        let flag = |host: &mut dyn Host, field: &str| {
            matches!(get_member(host, this, field), Ok(Value::Bool(true)))
        };
        let ask = |host: &mut dyn Host, class: &str, function: &str, args: &[Value]| {
            call(host, class, function, args).map(|v| matches!(v, Value::Bool(true)))
        };
        let leader = call(host, "Game", "GetLocalLeadersPlayerNumber__", &[])?;
        if ask(host, "Input", "IsInAttractionLoop__i", &[leader])? {
            if flag(host, "IMusicPlaying") {
                call(host, "Audio", "StopInteractiveMusic__", &[])?;
                set_member(host, this, "IMusicPlaying", Value::Bool(false))?;
                set_member(host, this, "IMusicInit", Value::Bool(false))?;
            }
            return Ok(());
        }
        let fsb = int(host, this, "FSB");
        if as_int(&call(host, "Audio", "GetCurrMusicGuid__", &[])?) != fsb {
            let between = [Value::Float(1.49), Value::Float(8.49)];
            let delay = as_float(&math(host, "NonDeterministicRandomBetween__ff", &between)?);
            let args = [
                Value::Int(fsb),
                Value::Int(int(host, this, "SettingsFile")),
                Value::Float(1.),
                Value::Float(delay as i32 as f32),
                Value::Int(0),
                Value::Thing(None),
            ];
            call(host, "Audio", "SetInteractiveMusic__ggffiQ5Thing", &args)?;
            set_member(host, this, "IMusicPlaying", Value::Bool(true))?;
            set_member(host, this, "IMusicInit", Value::Bool(true))?;
        }
        let view = int(host, this, "ViewType");
        if int(host, this, "IMusicPlanetsView") != view {
            set_member(host, this, "IMusicInit", Value::Bool(true))?;
            set_member(host, this, "IMusicPlanetsView", Value::Int(view))?;
        }
        let user_music_off = ask(host, "Audio", "UserMusicIsStopped__", &[])?
            || ask(host, "Audio", "UserMusicIsPaused__", &[])?;
        let enabled = user_music_off && ask(host, "Audio", "IsInteractiveMusicAudible__", &[])?;
        if flag(host, "IMusicEnabled") != enabled {
            set_member(host, this, "IMusicInit", Value::Bool(true))?;
            set_member(host, this, "IMusicEnabled", Value::Bool(enabled))?;
        }
        if flag(host, "IMusicInit") {
            let view = if enabled {
                int(host, this, "IMusicPlanetsView")
            } else {
                0
            };
            for (pair, volume) in music_pairs(view).into_iter().enumerate() {
                let args = [
                    Value::Int(fsb),
                    Value::Int(pair as i32),
                    Value::Float(volume),
                    Value::Float(-1.),
                ];
                call(
                    host,
                    "Audio",
                    "SetInteractiveMusicStereoPairVolume__giff",
                    &args,
                )?;
            }
        }
        set_member(host, this, "IMusicInit", Value::Bool(false))
    }

    /// `.init__` and `OnCreate__`: the fields' first values and the three views (each
    /// made by `PlanetsView..ctor__`, view 0).
    fn create(host: &mut dyn Host, this: u32) -> Result<NativeStep, String> {
        site(host, PLANETS_WORLD, "OnCreate__");
        let fields = [
            ("FSB", Value::Int(POD_MUSIC as i32)),
            ("SettingsFile", Value::Int(POD_MUSIC_SETTINGS as i32)),
            ("IMusicPlanetsView", Value::Int(0)),
            ("IMusicInit", Value::Bool(true)),
            ("IMusicEnabled", Value::Bool(false)),
            ("IMusicPlaying", Value::Bool(false)),
            ("State", Value::Int(0)),
            ("WantLevelCreatedDialogue", Value::Bool(false)),
            ("HaveDonePodIntro", Value::Bool(false)),
            ("cur_pos", Value::Vec4(REST_POS)),
            ("cur_foc", Value::Vec4(REST_FOC)),
            ("cur_up", Value::Vec4(UP)),
            ("cur_vel", Value::Vec4([0.; 4])),
            ("cur_foc_vel", Value::Vec4([0.; 4])),
            ("t", Value::Float(0.)),
            ("badge_lerp", Value::Float(0.)),
            ("ViewType", Value::Int(0)),
            ("LocalViewType", Value::Int(0)),
            ("ViewTransition", Value::Float(1.)),
            ("ExtraPlanetZoom", Value::Float(0.)),
            ("ExtraPlanetZoomVel", Value::Float(0.)),
        ];
        for (field, v) in fields {
            set_member(host, this, field, v)?;
        }
        Self::make_views(host, this, 0)?;
        Ok(DONE)
    }

    /// The three views made as `PlanetsView..ctor__` makes them (`SetPlanetsView__i`).
    /// Without the crafteroid objects (no scripts) a view cannot be set before the
    /// engine hands the world its planets, so they are made again on the first frame
    /// that has them (`ViewsMade`, a member of the port only).
    fn make_views(host: &mut dyn Host, this: u32, kind: i32) -> Result<(), String> {
        let mut made = true;
        for name in VIEWS {
            let mut view = PlanetsView::default();
            made &= Self::set_planets_view(host, this, &mut view, kind)?;
            view.put(host, this, name)?;
        }
        set_member(host, this, "ViewsMade", Value::Bool(made))
    }

    /// The place of a crafteroid's Thing (1 Earth, 2 the Moon): `PodComputer.GetPlanets`,
    /// `Planets.GetCrafteroid__i`, `Planet.GetThing__`, its world matrix's last column.
    /// A run without the scripts has no crafteroid objects; the port then takes the
    /// Things the engine hands `UpdatePlanetsView` (INFERRED the same Things: the
    /// runtime's crafteroids are the Earth and OrbitingMoon Things, lib.rs).
    fn crafteroid_place(
        host: &mut dyn Host,
        this: u32,
        n: i32,
    ) -> Result<Option<[f32; 4]>, String> {
        let mut thing = Self::crafteroid_thing(host, n);
        if thing.is_none() {
            let field = if n == 1 { "EarthThing" } else { "MoonThing" };
            thing = get_member(host, this, field)
                .ok()
                .and_then(|v| as_thing(&v));
        }
        let Some(thing) = thing else {
            return Ok(None);
        };
        let m = as_mat44(&call(
            host,
            "Thing",
            "GetWorldPosition__",
            &[Value::Thing(Some(thing))],
        )?);
        Ok(Some([m[12], m[13], m[14], m[15]]))
    }

    /// The crafteroid's Thing by the script's calls; none when a call has nothing to
    /// answer with (no pod computer or planets object without the scripts).
    fn crafteroid_thing(host: &mut dyn Host, n: i32) -> Option<u32> {
        let pc = call(host, "PodComputer", "GetPodComputer__", &[]).ok()?;
        let planets = call(host, "PodComputer", "GetPlanets__", &[pc]).ok()?;
        let planet = call(
            host,
            "Planets",
            "GetCrafteroid__i",
            &[planets, Value::Int(n)],
        )
        .ok()?;
        as_thing(&call(host, "Planet", "GetThing__", &[planet]).ok()?)
    }

    /// `PlanetsView.SetPlanetsView__i` on one of the world's views: nothing written
    /// without Earth or the Moon (FOUND).
    fn set_planets_view(
        host: &mut dyn Host,
        this: u32,
        view: &mut PlanetsView,
        kind: i32,
    ) -> Result<bool, String> {
        site(host, "PlanetsView", "SetPlanetsView__i");
        let Some(earth) = Self::crafteroid_place(host, this, 1)? else {
            return Ok(false);
        };
        let Some(moon) = Self::crafteroid_place(host, this, 2)? else {
            return Ok(false);
        };
        let is_4to3 = match kind {
            18 | 22 | 25 => as_bool(&call(host, "System", "Is4to3__", &[])?),
            _ => false,
        };
        view.set(kind, earth, moon, is_4to3);
        Ok(true)
    }

    /// `SetViewFromRemotePodComputer__i`: the pod computer asks for a view; a new one
    /// starts a blend from the current view. The desired view is set again either way.
    pub fn set_view_from_remote(host: &mut dyn Host, this: u32, kind: i32) -> Result<(), String> {
        site(host, PLANETS_WORLD, "SetViewFromRemotePodComputer__i");
        if int(host, this, "ViewType") != kind {
            set_member(host, this, "ViewType", Value::Int(kind))?;
            set_member(host, this, "ViewTransition", Value::Float(0.))?;
            PlanetsView::get(host, this, "CurrentView").put(host, this, "OldView")?;
        }
        let mut desired = PlanetsView::get(host, this, "DesiredView");
        Self::set_planets_view(host, this, &mut desired, kind)?;
        desired.put(host, this, "DesiredView")?;
        Ok(())
    }

    /// D1005: `OnEnterPod__`. It clears the state, resets the interactive music
    /// (`ResetPodInteractiveMusic__`), restores the view type kept by `OnExitPod__` and
    /// snaps all three views to it. D1007: `WantLevelCreatedDialogue` is set when the
    /// previous level's slot is of type 2 (a user level stored locally) and the previous
    /// level is valid.
    pub fn on_enter_pod(host: &mut dyn Host, this: u32) -> Result<(), String> {
        site(host, PLANETS_WORLD, "OnEnterPod__");
        set_member(host, this, "State", Value::Int(0))?;
        Self::reset_interactive_music(host, this)?;
        let previous = call(host, "Game", "GetPreviousLevelSlotID__", &[])?;
        let kind = host.get_member(MemberKind::RefPtr, &previous, "SlotID", "Type")?;
        let want =
            as_int(&kind) == 2 && as_bool(&call(host, "Game", "IsPreviousLevelValid__", &[])?);
        set_member(host, this, "WantLevelCreatedDialogue", Value::Bool(want))?;
        let kind = int(host, this, "LocalViewType");
        set_member(host, this, "ViewType", Value::Int(kind))?;
        set_member(host, this, "ViewTransition", Value::Float(1.))?;
        let mut desired = PlanetsView::get(host, this, "DesiredView");
        Self::set_planets_view(host, this, &mut desired, kind)?;
        desired.put(host, this, "DesiredView")?;
        desired.put(host, this, "CurrentView")?;
        desired.put(host, this, "OldView")
    }

    /// D1005: `ResetPodInteractiveMusic__`: the pair volumes are set again next frame.
    fn reset_interactive_music(host: &mut dyn Host, this: u32) -> Result<(), String> {
        site(host, PLANETS_WORLD, "ResetPodInteractiveMusic__");
        set_member(host, this, "IMusicInit", Value::Bool(true))?;
        set_member(host, this, "IMusicEnabled", Value::Bool(false))?;
        set_member(host, this, "IMusicPlaying", Value::Bool(false))
    }

    /// `UpdateState__`: a cutscene state ends once its thread has finished. The pod
    /// introduction is the port's own (`IntroRunning`); D383: so is the level unlocking
    /// thread (`Host::level_unlocking_tick`).
    fn update_state(host: &mut dyn Host, this: u32) -> Result<(), String> {
        site(host, PLANETS_WORLD, "UpdateState__");
        // The pod computer object is fetched and not used (none without the scripts).
        let _ = call(host, "PodComputer", "GetPodComputer__", &[]);
        let state = int(host, this, "State");
        let running = match state {
            1 => Self::intro_tick(host, this)?,
            2 => host.level_unlocking_tick()?,
            _ => false,
        };
        if state == 2 && !running {
            // The thread's `OnFinish__` (D381; links it did not play unlock there,
            // PLACEHOLDER).
            call(host, "Game", "ClearPendingLinks__", &[])?;
        }
        if matches!(state, 1 | 2) && !running {
            Self::set_state(host, this, 0)?;
        }
        if int(host, this, "State") != 0 {
            return Ok(());
        }
        let flag = call(
            host,
            "Profile",
            "IsLBP2GameProgressionFlagSet__i",
            &[Value::Int(2)],
        )?;
        if !as_bool(&flag) && !as_bool(&call(host, "Game", "IsNonInteractive__", &[])?) {
            return Self::set_state(host, this, 1);
        }
        if as_int(&call(host, "Game", "GetNumPendingLinks__", &[])?) > 0 {
            return Self::set_state(host, this, 2);
        }
        // `WantLevelCreatedDialogue` is only set by `OnEnterPod__` (D1005, D1007).
        Ok(())
    }

    fn set_state(host: &mut dyn Host, this: u32, s: i32) -> Result<(), String> {
        let _ = call(host, "PodComputer", "GetPodComputer__", &[]);
        set_member(host, this, "State", Value::Int(s))?;
        if s == 1 {
            // The thread's first run, at the end of the step that started it: zoom 0.
            set_member(host, this, "IntroStep", Value::Int(0))?;
            set_member(host, this, "IntroZoom", Value::Float(0.))?;
            set_member(host, this, "IntroRunning", Value::Bool(true))?;
            // D398 (FOUND, `PodIntroductionThread.Run__` before its zoom loop): the line
            // `pod_1st_time`, or `pod_1st_time_alt` when the player has LBP1's Pod,
            // interrupt and replay both set. D1000 (FOUND, the same): first the pod
            // computer's `StartVideo__g` with the video 139230, which is
            // `Video.StartVideo__Q5Thinggb(null, guid, false)` kept as its video handle
            // (script 16e6e8c9). D1003: it starts when its Bink file was read, else the
            // handle is -1.
            let video = [
                Value::Thing(None),
                Value::Int(INTRO_VIDEO as i32),
                Value::Bool(false),
            ];
            let handle = call(host, "Video", "StartVideo__Q5Thinggb", &video)?;
            let handle = if let Value::Int(h) = handle { h } else { -1 };
            set_member(host, this, "IntroVideo", Value::Int(handle))?;
            let alt = call(host, "Profile", "HasPlayedLBP1AndUnlockedPod__", &[])?;
            let line = if as_bool(&alt) {
                "pod_1st_time_alt"
            } else {
                "pod_1st_time"
            };
            let args = [
                Value::Str(line.into()),
                Value::Bool(true),
                Value::Bool(true),
            ];
            call(host, "Audio", "RequestVoiceOver__Q7StringAbb", &args)?;
        }
        Ok(())
    }

    /// One tick of `PodIntroductionThread`, resumed before the world's update: the zoom
    /// steps by 1/450 to 1, then the thread waits while a voice-over or the pod
    /// computer's video plays (the line plays for its bank length, D397; the video for
    /// its Bink file's, D1003: the pod computer's `UpdateVideo__` each tick, which sets
    /// the handle to -1 once `Video.UpdateVideo__ib` answers false) and sets the
    /// progression flags 3 (8188 when LBP1's Moon is unlocked and this is not a demo or
    /// kiosk build, 8064 in the PSN demo). Whether it is still running. The thread's
    /// voice-over line (D398) and its video (D1000) are requested when it starts.
    fn intro_tick(host: &mut dyn Host, this: u32) -> Result<bool, String> {
        if !get_member(host, this, "IntroRunning").is_ok_and(|v| as_bool(&v)) {
            return Ok(false);
        }
        let step = int(host, this, "IntroStep") + 1;
        if step <= INTRO_STEPS {
            set_member(host, this, "IntroStep", Value::Int(step))?;
            let zoom = step as f32 / INTRO_STEPS as f32;
            set_member(host, this, "IntroZoom", Value::Float(zoom))?;
            return Ok(true);
        }
        let ask = |host: &mut dyn Host, class, function| {
            call(host, class, function, &[]).is_ok_and(|v| as_bool(&v))
        };
        let video = int(host, this, "IntroVideo");
        if video != -1 {
            let args = [Value::Int(video), Value::Bool(false)];
            let playing = call(host, "Video", "UpdateVideo__ib", &args)?;
            if !as_bool(&playing) {
                set_member(host, this, "IntroVideo", Value::Int(-1))?;
            }
        }
        if ask(host, "Audio", "IsVoiceOverPlaying__") || int(host, this, "IntroVideo") != -1 {
            return Ok(true);
        }
        let set = |host: &mut dyn Host, mask: i32| {
            let args = [Value::Int(mask)];
            call(host, "Profile", "SetLBP2GameProgressionFlag__i", &args).map(|_| ())
        };
        set(host, 3)?;
        let demo = ask(host, "Game", "IsPSNDemoBuild__");
        if ask(host, "Profile", "HasPlayedLBP1AndUnlockedMoon__")
            && !demo
            && !ask(host, "Game", "IsKioskBuild__")
        {
            set(host, 8188)?;
        }
        if demo {
            set(host, 8064)?;
        }
        set_member(host, this, "IntroRunning", Value::Bool(false))?;
        Ok(false)
    }

    /// `UpdatePodCamera__Q11PlanetThingbb`: the camera eases (`Math.Smooth3D`, 10, 100)
    /// toward the pod computer's framing while it is in use, else the Pod's; snapped on
    /// the first frame. The players' framing and the loading badge zoom are ported as the
    /// script has them (their natives answer nothing here).
    fn update_pod_camera(host: &mut dyn Host, this: u32, snap: bool) -> Result<(), String> {
        site(host, PLANETS_WORLD, "UpdatePodCamera__Q11PlanetThingbb");
        let mut snap = snap;
        let mut up = UP;
        let (mut pos, mut foc);
        if as_bool(&call(host, "PodComputer", "GetPodComputerInUse__", &[])?) {
            (pos, foc) = (IN_USE_POS, IN_USE_FOC);
            if as_int(&call(host, "HUD", "GetCinemaModeCount__", &[])?) != 0 {
                pos[2] -= 100.;
                foc[1] += 50.;
            }
        } else {
            (pos, foc) = (REST_POS, REST_FOC);
            if !as_bool(&call(host, "Video", "IsVideoPlaying__", &[])?) {
                let n = as_int(&call(host, "Game", "GetNumAvatars__", &[])?);
                let mut acc = [0.; 4];
                for i in 0..n {
                    let i = [Value::Int(i)];
                    let p = as_vec4(&call(
                        host,
                        "Game",
                        "GetPoppetPosFromTemporaryIndex__i",
                        &i,
                    )?);
                    let mode = as_int(&call(
                        host,
                        "Game",
                        "GetPoppetModeFromTemporaryIndex__i",
                        &i,
                    )?);
                    let sub = as_int(&call(
                        host,
                        "Game",
                        "GetPoppetSubModeFromTemporaryIndex__i",
                        &i,
                    )?);
                    let zero = p.iter().map(|x| x * x).sum::<f32>() <= 0.;
                    if !zero && (mode == 1 || (mode == 2 && sub != 16)) {
                        acc[0] += p[0];
                        acc[1] += p[1];
                        acc[2] += 1.;
                    }
                }
                if acc[2] > 0. {
                    foc[0] = acc[0] / acc[2];
                    foc[1] = acc[1] / acc[2];
                }
            }
        }
        // State 1 frames the introduction's zoom, from far below the planets to the rest
        // framing, set without easing.
        if int(host, this, "State") == 1 {
            let z = float(host, this, "IntroZoom");
            let mix = |near, far| add(scaled(near, z), scaled(far, 1. - z));
            (pos, foc, up) = (
                mix(REST_POS, INTRO_FAR_POS),
                mix(REST_FOC, INTRO_FAR_FOC),
                UP,
            );
            snap = true;
        }
        let mut badge = float(host, this, "badge_lerp");
        badge = if as_bool(&call(host, "PodComputer", "GetCrashZoom__", &[])?) {
            badge + 0.03
        } else {
            0.
        };
        set_member(host, this, "badge_lerp", Value::Float(badge))?;
        let (mut cur_pos, mut cur_foc, mut cur_up) = (
            vec4(host, this, "cur_pos"),
            vec4(host, this, "cur_foc"),
            vec4(host, this, "cur_up"),
        );
        if badge > 0. {
            let l = badge.min(1.);
            let mb = as_mat44(&call(host, "PodComputer", "GetLoadingBadgeMat__", &[])?);
            let col = |c: usize| [mb[c * 4], mb[c * 4 + 1], mb[c * 4 + 2], mb[c * 4 + 3]];
            let badge_w = col(3);
            let pw = sub(col(3), scaled(col(2), 200.));
            let nz = as_vec4(&math(host, "normalize3__r", &[Value::Vec4(col(2))])?);
            let target = sub(pw, scaled(scaled(nz, 1. - l), 4000.));
            let lerp_v =
                |host: &mut dyn Host, a: [f32; 4], b: [f32; 4]| -> Result<[f32; 4], String> {
                    let args = [Value::Float(l), Value::Vec4(a), Value::Vec4(b)];
                    Ok(as_vec4(&math(host, "lerp__frr", &args)?))
                };
            foc = lerp_v(host, foc, badge_w)?;
            pos = lerp_v(host, pos, target)?;
            pos[1] = pos[1].max(-100.);
            let ub = [(badge * 0.25).sin(), (badge * 0.25).cos(), 0., 0.];
            up = lerp_v(host, up, ub)?;
            let f = (0.1 + l * 0.5).min(1.);
            cur_pos = add(cur_pos, scaled(sub(pos, cur_pos), f));
            cur_foc = add(cur_foc, scaled(sub(foc, cur_foc), f));
            cur_up = add(cur_up, scaled(sub(up, cur_up), f));
        } else if snap {
            (cur_pos, cur_foc, cur_up) = (pos, foc, up);
        } else {
            let smooth = |host: &mut dyn Host, now: [f32; 4], vel: [f32; 4], to: [f32; 4]| {
                let args = [
                    Value::Vec4(now),
                    Value::Vec4(vel),
                    Value::Vec4(to),
                    Value::Float(10.),
                    Value::Float(100.),
                ];
                math(host, "Smooth3D__rrrff", &args).map(|v| as_vec4(&v))
            };
            let (vel, foc_vel) = (vec4(host, this, "cur_vel"), vec4(host, this, "cur_foc_vel"));
            let vel = smooth(host, cur_pos, vel, pos)?;
            let foc_vel = smooth(host, cur_foc, foc_vel, foc)?;
            set_member(host, this, "cur_vel", Value::Vec4(vel))?;
            set_member(host, this, "cur_foc_vel", Value::Vec4(foc_vel))?;
            cur_pos = add(cur_pos, vel);
            cur_foc = add(cur_foc, foc_vel);
            cur_up = add(cur_up, scaled(sub(up, cur_up), 0.1));
        }
        set_member(host, this, "cur_pos", Value::Vec4(cur_pos))?;
        set_member(host, this, "cur_foc", Value::Vec4(cur_foc))?;
        set_member(host, this, "cur_up", Value::Vec4(cur_up))?;
        let args = [
            Value::Vec4(cur_pos),
            Value::Vec4(cur_foc),
            Value::Vec4(cur_up),
            Value::Float(10.),
            Value::Float(100_000.),
            Value::Float(0.),
            Value::Float(1.),
        ];
        call(host, "Cam", "SetCameraFromToUp__rrrffff", &args)?;
        if snap {
            call(host, "Cam", "ClearMotionBlur__", &[])?;
        }
        Ok(())
    }

    /// `PlanetsView.GetTransform__rf`: the level's matrix for a view, scaled 2 about the
    /// camera's eye: `translate(eye) scale(2) translate(-eye) translate(0, -zoom/4, zoom)
    /// rotateZ(roll) rotateX(pitch) translate(offset) rotateY(yaw) translate(-focus)`.
    fn transform(
        host: &mut dyn Host,
        v: &PlanetsView,
        eye: [f32; 4],
        zoom: f32,
    ) -> Result<[f32; 16], String> {
        use crate::transform::mul;
        let translate =
            |host: &mut dyn Host, p: [f32; 4]| mat(host, "translate__r", Value::Vec4(p));
        let neg = |p: [f32; 4]| p.map(|x| -x);
        let mut m = translate(host, [0., -zoom * 0.25, zoom, 0.])?;
        m = mul(&m, &mat(host, "rotateZ__f", Value::Float(v.roll))?);
        m = mul(&m, &mat(host, "rotateX__f", Value::Float(v.pitch))?);
        m = mul(&m, &translate(host, v.offset)?);
        m = mul(&m, &mat(host, "rotateY__f", Value::Float(v.yaw))?);
        m = mul(&m, &translate(host, neg(v.focus))?);
        let mut s = translate(host, eye)?;
        s = mul(&s, &mat(host, "scale__r", Value::Vec4([2., 2., 2., 1.]))?);
        s = mul(&s, &translate(host, neg(eye))?);
        Ok(mul(&s, &m))
    }

    /// `UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb`: once a frame.
    fn update_planets_view(
        host: &mut dyn Host,
        this: u32,
        b4: bool,
        earth: Option<u32>,
        moon: Option<u32>,
        snap: bool,
        b17: bool,
    ) -> Result<NativeStep, String> {
        site(
            host,
            PLANETS_WORLD,
            "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb",
        );
        set_member(host, this, "EarthThing", Value::Thing(earth))?;
        set_member(host, this, "MoonThing", Value::Thing(moon))?;
        let made = get_member(host, this, "ViewsMade").is_ok_and(|v| as_bool(&v));
        if !made {
            let kind = int(host, this, "ViewType");
            Self::make_views(host, this, kind)?;
        }
        // The pod computer state's extra zoom (`GetExtraPlanetZoom`, a script method of
        // the state), asked of the host (D360: 500 in a type 4 or 9 group search; the
        // natively run pod computer has no script object to ask).
        let mut target_zoom = 0.;
        if b17 && !b4 {
            if let Ok(pc) = call(host, "PodComputer", "GetPodComputer__", &[]) {
                let _ = call(host, "PodComputer", "GetState__", &[pc]);
            }
            target_zoom = call(host, "PodComputer", "HostExtraPlanetZoom__", &[])
                .map_or(0., |v| as_float(&v));
        }
        let desired = PlanetsView::get(host, this, "DesiredView");
        if snap {
            set_member(host, this, "ViewTransition", Value::Float(1.))?;
            desired.put(host, this, "CurrentView")?;
            set_member(host, this, "ExtraPlanetZoom", Value::Float(target_zoom))?;
        } else {
            let old = PlanetsView::get(host, this, "OldView");
            let speed = PlanetsView::transition_speed(&old, &desired);
            let t = (float(host, this, "ViewTransition") + speed).min(1.);
            set_member(host, this, "ViewTransition", Value::Float(t))?;
            let args = [Value::Float(0.), Value::Float(1.), Value::Float(t)];
            let s = as_float(&math(host, "SmoothStep__fff", &args)?);
            PlanetsView::between(&old, &desired, s).put(host, this, "CurrentView")?;
            let args = [
                Value::Float(float(host, this, "ExtraPlanetZoom")),
                Value::Float(float(host, this, "ExtraPlanetZoomVel")),
                Value::Float(target_zoom),
                Value::Float(10.),
                Value::Float(100.),
            ];
            let r = as_vec4(&math(host, "Smooth__fffff", &args)?);
            set_member(host, this, "ExtraPlanetZoom", Value::Float(r[0]))?;
            set_member(host, this, "ExtraPlanetZoomVel", Value::Float(r[1]))?;
        }
        Self::update_state(host, this)?;
        Self::update_pod_camera(host, this, snap)?;
        let current = PlanetsView::get(host, this, "CurrentView");
        for (thing, spin) in [
            (earth, current.earth_spin_offset),
            (moon, current.moon_spin_offset),
        ] {
            let planet = Value::Thing(thing);
            host.set_member(
                MemberKind::Thing,
                &planet,
                PLANET_THING,
                "SpinOffset",
                Value::Float(spin),
            )?;
        }
        let eye = vec4(host, this, "cur_pos");
        let zoom = float(host, this, "ExtraPlanetZoom");
        let m = Self::transform(host, &current, eye, zoom)?;
        call(
            host,
            "Thing",
            "SetWorldPosition__m",
            &[Value::Thing(Some(this)), Value::Mat44(m)],
        )?;
        Self::update_interactive_music(host, this)?;
        let args = [
            Value::Float(current.dof_min * 2.),
            Value::Float(current.dof_max * 2.),
            Value::Float(current.fog_bias * 2.),
        ];
        call(host, "Cam", "SetDOFAndFog__fff", &args)?;
        let blending = float(host, this, "ViewTransition") < 1.;
        Ok(NativeStep::Done(Value::Bool(blending)))
    }
}

/// The argument the VM staged at byte offset `offset`.
fn arg(args: &[(u16, Value)], offset: u16) -> Option<&Value> {
    args.iter().find(|(o, _)| *o == offset).map(|(_, v)| v)
}

impl NativeScript for PlanetsWorldScript {
    fn class(&self) -> &'static str {
        PLANETS_WORLD
    }
    fn guid(&self) -> Option<u32> {
        Some(PLANETS_WORLD_SCRIPT)
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
        let flag = |o| arg(args, o).is_some_and(as_bool);
        let planet = |o| arg(args, o).and_then(as_thing);
        Some(match function {
            "UpdatePlanetsView__bQ11PlanetThingQ11PlanetThingbb" => Self::update_planets_view(
                host,
                thing,
                flag(4),
                planet(8),
                planet(12),
                flag(16),
                flag(17),
            ),
            "SetViewFromRemotePodComputer__i" => {
                let kind = arg(args, 4).map_or(0, as_int);
                Self::set_view_from_remote(host, thing, kind).map(|()| DONE)
            }
            "GetCurrentViewType__" => {
                Ok(NativeStep::Done(Value::Int(int(host, thing, "ViewType"))))
            }
            "OnExitPod__" => {
                let v = int(host, thing, "ViewType");
                set_member(host, thing, "LocalViewType", Value::Int(v)).map(|()| DONE)
            }
            "OnEnterPod__" => Self::on_enter_pod(host, thing).map(|()| DONE),
            "ResetPodInteractiveMusic__" => {
                Self::reset_interactive_music(host, thing).map(|()| DONE)
            }
            "OnScriptReloaded__" => Ok(DONE),
            _ => return None,
        })
    }
}
