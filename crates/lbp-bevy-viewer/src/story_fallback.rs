//! D353 (part 100): the Story screen without the game's scripts, and the trace that tests
//! it.
//!
//! The game's scripts exist only in the encrypted patch.edat (D133), so a player with the
//! disc and the update alone has none: the Pod's search state (`PodStateSearch`) never
//! runs and the Story search never opens. [`lbp_runtime::native_script`] holds the native
//! stand-in for that script's Story entry; this module is the viewer's side of it.
//!
//! `--level-scripts-trace PATH` (development) writes one JSON line per tick of the
//! scripted Pod: the search state, the camera a script wrote and every Thing whose
//! display values changed since the line before. A run with the decrypted scripts gives
//! the golden trace the native entry is tested against (kept in ignored local-analysis/;
//! the trace is derived data, no script code).

use std::collections::HashMap;
use std::io::Write as _;

use lbp_runtime::LevelWorld;

/// `--level-scripts-trace PATH`: a JSON line per tick.
pub struct Trace {
    out: std::io::BufWriter<std::fs::File>,
    /// What was last written for a Thing, by (level, Thing).
    last: HashMap<(u8, u32), lbp_runtime::ThingDisplay>,
    /// The drawn menu frame last written.
    menu: Option<String>,
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            c if (c as u32) < 0x20 => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Trace {
    pub fn create(path: &str) -> Result<Self, String> {
        let file = std::fs::File::create(path)
            .map_err(|e| format!("--level-scripts-trace {path}: {e}"))?;
        Ok(Self {
            out: std::io::BufWriter::new(file),
            last: HashMap::new(),
            menu: None,
        })
    }

    /// The line for the tick that just ran: the planets level, then the shell level.
    pub fn tick(&mut self, ticks: u64, level: &LevelWorld, shell: Option<&LevelWorld>) {
        let mut line = format!("{{\"tick\":{ticks},\"frame\":{}", level.pworld.frame);
        line += &format!(",\"story_open\":{}", level.story_open());
        line += &format!(",\"search\":{}", search_json(&level.pod.search));
        line += &format!(",\"camera\":{}", camera_json(level));
        line += &format!(
            ",\"fade\":{}",
            opt(shell.and_then(LevelWorld::pod_transition))
        );
        // The scripts' drawn menu frame, written when it changes.
        if let Some(menu) = level.gooey.drawn_menu() {
            let frame = format!("{:?} {:?}", menu.frame, menu.matrix);
            if self.menu.as_deref() != Some(frame.as_str()) {
                line += &format!(
                    ",\"menu\":{{\"matrix\":{},\"first_built\":{},\"frame\":{}}}",
                    floats(Some(&menu.matrix)),
                    opt(menu.first_built),
                    json_string(&frame)
                );
                self.menu = Some(frame);
            }
        }
        for (i, (name, world)) in [("level", Some(level)), ("shell", shell)]
            .into_iter()
            .enumerate()
        {
            let Some(world) = world else { continue };
            line += &format!(
                ",\"{name}\":[{}]",
                self.things_json(i as u8, world).join(",")
            );
        }
        line.push('}');
        let _ = writeln!(self.out, "{line}");
        let _ = self.out.flush();
    }

    /// The Things whose display values differ from what was written last (from the
    /// level file's on the first line).
    fn things_json(&mut self, level: u8, world: &LevelWorld) -> Vec<String> {
        let mut out = Vec::new();
        for thing in 0..world.things.len() as u32 {
            let Some(now) = world.thing_display(thing) else {
                continue;
            };
            let was = match self.last.get(&(level, thing)) {
                Some(d) => *d,
                None => match world.display_at_load(thing) {
                    Some(d) => d,
                    None => continue,
                },
            };
            if now == was {
                continue;
            }
            self.last.insert((level, thing), now);
            out.push(format!(
                "{{\"thing\":{thing},\"world\":{},\"visibility\":{},\"mesh\":{}}}",
                floats(now.world.as_ref().map(|m| &m[..])),
                opt(now.mesh_visibility),
                opt(now.mesh)
            ));
        }
        out
    }
}

fn floats(v: Option<&[f32]>) -> String {
    match v {
        Some(v) => format!(
            "[{}]",
            v.iter()
                .map(|f| format!("{f}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        None => "null".into(),
    }
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("null".into(), |v| v.to_string())
}

fn search_json(s: &lbp_runtime::PlanetsSearch) -> String {
    format!(
        "{{\"selected_planet\":{},\"cursor_mode\":{},\"browse_mode\":{},\"allowed_browse_modes\":{},\"loader\":{},\"earth_has_control\":{},\"want_search_details_calls\":{},\"publish_cursor_badge_calls\":{},\"error_message_calls\":{},\"default_result\":{},\"cur_result\":{},\"set_search_results_calls\":{},\"info_bubble_since\":{}}}",
        opt(s.selected_planet),
        opt(s.cursor_mode),
        opt(s.browse_mode),
        opt(s.allowed_browse_modes),
        opt(s.loader),
        opt(s.earth_has_control),
        s.want_search_details_calls,
        s.publish_cursor_badge_calls,
        s.error_message_calls,
        opt(s.default_result),
        opt(s.cur_result),
        s.set_search_results_calls,
        opt(s.info_bubble_since),
    )
}

fn camera_json(world: &LevelWorld) -> String {
    match world.script_camera() {
        Some(c) => format!(
            "{{\"eye\":{},\"direction\":{},\"up\":{},\"near\":{},\"far\":{},\"lens\":{},\"fov_y\":{}}}",
            floats(Some(&c.pose.eye)),
            floats(Some(&c.pose.direction)),
            floats(Some(&c.up)),
            c.near,
            c.far,
            c.lens_value,
            c.fov_y
        ),
        None => "null".into(),
    }
}

/// The planets level's Things as the scripted run leaves them once Story has settled
/// (MEASURED, the golden trace: the world Thing 0 at rest from tick 659, Earth 1 and its
/// two parts turned toward the default story group from tick 660, the Moon 4 placed on
/// tick 1). STAND-IN: the world Thing and the Moon are the planets world script's, Earth's
/// turn its planet script's (the Popit lane's native Pod scripts); until those exist the
/// Story screen puts them where the scripts leave them, without the move.
pub const STORY_LEVEL_THINGS: [(u32, [f32; 16]); 5] = [
    (
        0,
        [
            1.5296844, 0.44180164, -1.210321, 0., 0., 1.8787454, 0.6857956, 0., 1.2884353,
            -0.5245254, 1.4369438, 0., 1912.1056, 3423.4658, -9460.587, 1.,
        ],
    ),
    (
        1,
        [
            0.46183985,
            0.050123278,
            0.84126467,
            0.,
            0.08878192,
            0.95107496,
            -0.10540562,
            0.,
            -0.83806705,
            0.12837523,
            0.45243573,
            0.,
            -1250.,
            0.,
            0.,
            1.,
        ],
    ),
    (
        2,
        [
            0.46885338,
            0.050884455,
            0.85404015,
            0.,
            0.09231041,
            0.98887384,
            -0.10959479,
            0.,
            -0.83806694,
            0.12837522,
            0.45243567,
            0.,
            -1250.0015,
            0.0022529147,
            -0.0012144588,
            1.,
        ],
    ),
    (
        3,
        [
            0.46885344,
            0.05088446,
            0.85404027,
            0.,
            0.09231041,
            0.98887384,
            -0.10959479,
            0.,
            -0.83806705,
            0.12837523,
            0.45243573,
            0.,
            -1249.9999,
            -0.00010386089,
            0.00021825377,
            1.,
        ],
    ),
    (
        4,
        [
            2.112109, 0., 0., 0., 0., 2.112109, 0., 0., 0., 0., 2.1121097, 0., 2000., 0., 0., 1.,
        ],
    ),
];

/// The camera the planets world script writes while the pod computer is in use (MEASURED,
/// steady from tick 620 to the end of the run: eye, direction, 1 rad, near 10, far
/// 100,000, level units). STAND-IN for that script (the Popit lane's).
pub const POD_IN_USE_CAMERA: lbp_runtime::DisplayCamera = lbp_runtime::DisplayCamera {
    source: lbp_runtime::CameraSource::Script,
    pose: lbp_runtime::CameraPose {
        eye: [0., -299.48035, 902.39044],
        direction: [0., 0.31498626, -0.94909626],
    },
    up: [0., 1., 0.],
    fov_y_deg: Some(57.29578),
    near: 10.,
    far: 100_000.,
};

/// `--story` without the game's scripts: the Story screen from the first tick, the pod
/// opened by pod.bin's native port and the search state's Story entry run natively
/// ([`lbp_runtime::PodStateSearchStory`]).
#[derive(Default)]
pub struct Fallback {
    pub search: lbp_runtime::PodStateSearchStory,
}

impl Fallback {
    /// After a tick: on the first, the planets placed and Story chosen; on every later
    /// one the open search's update. True on the tick it opens; the caller then sets the
    /// pod computer in use, and pod.bin's native port (D417) opens the pod (D354).
    pub fn after_tick(&mut self, level: &mut LevelWorld) -> bool {
        if self.search.is_open() {
            self.search.update(level);
            return false;
        }
        for (thing, m) in STORY_LEVEL_THINGS {
            level.place_thing(thing, m);
        }
        self.search.open(level);
        true
    }
}
