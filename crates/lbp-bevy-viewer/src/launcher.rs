//! D600: the friends' launcher. Double-clicking the viewer with no arguments opens a small
//! menu. The player points it at their own LittleBigPlanet 2 ISO (USA, BCUS98245) and the
//! 01.33 update RPCS3 installed; the launcher copies the files the viewer reads out of the
//! ISO once, builds the asset index from them (the `lbp-cli` scanner, in-process), and
//! starts each mode as a child process of this same executable. Nothing is decrypted and
//! no game file is shipped or modified.
//!
//! The game's compiled scripts sit in the encrypted `patches/patch.edat`, which is never
//! decrypted; modes run on the viewer's own fallbacks where they need them.
mod iso;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// True when the menu should open: no arguments at all, or `--launcher`.
pub fn wanted() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    args.is_empty()
        || args
            .iter()
            .any(|a| a == "--launcher" || a == "--launcher-setup")
}

/// The update the viewer is measured against (docs/update-overlay.md).
const APP_VER: &str = "01.33";

struct Mode {
    name: &'static str,
    blurb: &'static str,
    args: &'static [&'static str],
    /// Not reachable without the game's scripts yet.
    later: bool,
}

// The command lines the lanes' docs use (docs/level-scripts/part-30.md,
// docs/d403-popit-in-the-pod.md, docs/handoff-pc-2026-10-11b-moon-anim.md). Game modes hide
// the explorer's panels. The game's scripts sit in the encrypted patch.edat, so the Pod uses
// the viewer's own camera when none load (D411); Story opens through the search state's
// native stand-in (D353).
const MODES: &[Mode] = &[
    Mode {
        name: "Rookie Test",
        blurb: "Play the first tutorial level as Sackboy.",
        args: &[
            "--level-guid",
            "0001e786",
            "--play-sackboy",
            "--window-16-9",
            "--qa-hide-ui",
        ],
        later: false,
    },
    Mode {
        name: "Sackboy on the Moon",
        blurb: "Sackboy on a blank Moon level, to run and jump around.",
        args: &[
            "--level-path",
            "gamedata/levels/palettes/borders/blank_level_large.bin",
            "--play-sackboy",
            "--window-16-9",
            "--qa-hide-ui",
        ],
        later: false,
    },
    Mode {
        name: "Asset explorer",
        blurb: "Browse the game's models, textures and levels.",
        args: &[],
        later: false,
    },
    Mode {
        name: "The Pod",
        blurb: "Sackboy in the Pod, with Earth and the Moon outside.",
        args: &[
            "--level-guid",
            "00018fdb",
            "--pod-camera",
            "1",
            "--pod-shell",
            "000027b9",
            "--level-scripts",
            "--window-16-9",
            "--preview-lighting",
            "--qa-hide-ui",
        ],
        later: false,
    },
    Mode {
        name: "Popit",
        blurb: "The Pod with the Popit open. Z opens and closes it.",
        args: &[
            "--level-guid",
            "00018fdb",
            "--pod-camera",
            "1",
            "--pod-shell",
            "000027b9",
            "--level-scripts",
            "--window-16-9",
            "--preview-lighting",
            "--qa-hide-ui",
            "--popit-open",
            "--popit-mode",
            "play",
        ],
        later: false,
    },
    Mode {
        name: "Story screen",
        blurb: "The Pod's Story search on Earth, with the first story group.",
        args: &[
            "--level-guid",
            "00018fdb",
            "--pod-camera",
            "1",
            "--pod-shell",
            "000027b9",
            "--level-scripts",
            "--story",
            "--window-16-9",
            "--preview-lighting",
            "--qa-hide-ui",
        ],
        later: false,
    },
];

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("craftworld-rs")
}
fn config_path() -> PathBuf {
    data_dir().join("launcher.txt")
}
fn index_path() -> PathBuf {
    data_dir().join("inventory.json")
}
/// The files copied out of the ISO (a disc root holding PS3_GAME).
fn disc_dir() -> PathBuf {
    data_dir().join("disc")
}

/// Remembered between runs in `%LOCALAPPDATA%/craftworld-rs/launcher.txt`.
#[derive(Default, Clone)]
struct Config {
    iso: String,
    update: String,
    /// The `key()` the copy and index were made from; a change makes them again.
    indexed: String,
}

impl Config {
    fn load() -> Self {
        let mut c = Config::default();
        let text = std::fs::read_to_string(config_path()).unwrap_or_default();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().to_owned();
                match k.trim() {
                    "iso" => c.iso = v,
                    "update" => c.update = v,
                    "indexed" => c.indexed = v,
                    _ => {}
                }
            }
        }
        if c.update.is_empty() {
            c.update = guess_update().unwrap_or_default();
        }
        c
    }
    fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(data_dir()).map_err(|e| format!("Save settings: {e}"))?;
        let text = format!(
            "iso={}\nupdate={}\nindexed={}\n",
            self.iso, self.update, self.indexed
        );
        std::fs::write(config_path(), text).map_err(|e| format!("Save settings: {e}"))
    }
    /// The viewer's `--game-dir` list: the copied disc, then the installed update.
    fn game_dir(&self) -> String {
        format!("{};{}", disc_dir().display(), self.update.trim())
    }
    /// What the copy and index depend on: the ISO (path, size, time) and the update path.
    fn key(&self) -> String {
        let iso = self.iso.trim();
        let stamp = std::fs::metadata(iso)
            .map(|m| {
                let t = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                format!("{}@{t}", m.len())
            })
            .unwrap_or_default();
        format!("{iso}|{stamp}|{}", self.update.trim())
    }
    fn ready(&self) -> bool {
        !self.iso.trim().is_empty() && self.indexed == self.key() && index_path().is_file()
    }
}

/// RPCS3's install folder for the game, where people usually keep RPCS3.
fn guess_update() -> Option<String> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut roots = Vec::new();
    if let Some(home) = env("USERPROFILE") {
        for sub in [
            "Emulators/rpcs3",
            "rpcs3",
            "Desktop/rpcs3",
            "Downloads/rpcs3",
        ] {
            roots.push(home.join(sub));
        }
    }
    for k in ["APPDATA", "LOCALAPPDATA", "ProgramFiles"] {
        if let Some(r) = env(k) {
            roots.push(r.join("rpcs3"));
        }
    }
    roots
        .into_iter()
        .map(|r| r.join("dev_hdd0").join("game").join(iso::TITLE_ID))
        .find(|p| p.join("USRDIR").is_dir())
        .map(|p| p.display().to_string())
}

/// Accepts the game folder itself, RPCS3's folder, or its dev_hdd0.
fn normalise_update(path: &str) -> PathBuf {
    let p = PathBuf::from(path.trim());
    for sub in [&[][..], &["game"], &["dev_hdd0", "game"]] {
        let base = sub.iter().fold(p.clone(), |b, s| b.join(s));
        let game = base.join(iso::TITLE_ID);
        if game.join("USRDIR").is_dir() {
            return game;
        }
    }
    p
}

fn iso_problem(path: &str) -> Option<String> {
    let p = Path::new(path.trim());
    if path.trim().is_empty() {
        return Some("Pick your LittleBigPlanet 2 ISO.".into());
    }
    if !p.is_file() {
        return Some(format!("The ISO doesn't exist: {}", p.display()));
    }
    None
}

fn update_problem(path: &Path) -> Option<String> {
    let sfo = std::fs::read(path.join("PARAM.SFO")).ok();
    let identity = sfo
        .as_deref()
        .and_then(|d| lbp_io::sfo::read_identity(d).ok());
    let Some(id) = identity.filter(|_| path.join("USRDIR").is_dir()) else {
        return Some(format!(
            "No installed game at {}. Install the LBP2 updates in RPCS3, then pick \
             RPCS3's dev_hdd0/game/{} folder.",
            path.display(),
            iso::TITLE_ID
        ));
    };
    if id.title_id.as_deref() != Some(iso::TITLE_ID) {
        return Some(format!(
            "That update is for {}, not {} (USA). Other regions come later.",
            id.title_id.as_deref().unwrap_or("another game"),
            iso::TITLE_ID
        ));
    }
    let version = id.app_version.unwrap_or_default();
    if version != APP_VER {
        return Some(format!(
            "The installed update is {version}; this build needs the full update {APP_VER}. \
             Install every update package in RPCS3."
        ));
    }
    None
}

type Slot<T> = Arc<Mutex<Option<T>>>;

#[derive(Default)]
enum Job {
    #[default]
    Idle,
    Preparing {
        result: Slot<Result<(), String>>,
        copying: Arc<Mutex<bool>>,
        done: Arc<AtomicU64>,
        total: Arc<AtomicU64>,
        started: Instant,
    },
}

struct Running {
    name: &'static str,
    /// Set once the mode has exited; `Err` carries the tail of its log.
    result: Slot<Result<(), String>>,
}

#[derive(Resource)]
struct Launcher {
    cfg: Config,
    message: String,
    error: bool,
    job: Job,
    running: Vec<Running>,
    pass_flags: Vec<OsString>,
}

/// `--launcher-setup ISO UPDATE`: the Set up button without the window (for checks and
/// for players who prefer a console); saves the same settings the menu uses.
fn setup_cli(args: &[String]) -> Result<(), String> {
    let i = args.iter().position(|a| a == "--launcher-setup").unwrap();
    let (Some(iso_path), Some(update)) = (args.get(i + 1), args.get(i + 2)) else {
        return Err("Usage: --launcher-setup ISO UPDATE_FOLDER".into());
    };
    let mut l = Launcher {
        cfg: Config {
            iso: iso_path.clone(),
            update: update.clone(),
            indexed: Config::load().indexed,
        },
        message: String::new(),
        error: false,
        job: Job::Idle,
        running: Vec::new(),
        pass_flags: Vec::new(),
    };
    start_setup(&mut l);
    println!("{}", l.message);
    while matches!(l.job, Job::Preparing { .. }) {
        std::thread::sleep(Duration::from_millis(200));
        poll(&mut l);
    }
    println!("{}", l.message);
    if l.error {
        Err(l.message)
    } else {
        Ok(())
    }
}

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--launcher-setup") {
        if let Err(e) = setup_cli(&args) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let cfg = Config::load();
    let pass_flags = std::env::args_os()
        .skip(1)
        .filter(|a| a == "--no-activate")
        .collect();
    let message = if cfg.ready() {
        "Pick something to play.".to_owned()
    } else if cfg.iso.is_empty() {
        "First time here: pick your own LittleBigPlanet 2 ISO and its RPCS3 update, then Set up."
            .to_owned()
    } else {
        "Your game files changed; press Set up again.".to_owned()
    };
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
        .insert_resource(Launcher {
            cfg,
            message,
            error: false,
            job: Job::Idle,
            running: Vec::new(),
            pass_flags,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "craftworld-rs launcher".into(),
                resolution: (680u32, 560u32).into(),
                visible: false,
                focused: crate::activate_on_show(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, crate::place_on_second_monitor)
        .add_systems(EguiPrimaryContextPass, menu)
        .run();
}

fn pick(current: &str, file: bool) -> Option<String> {
    #[cfg(windows)]
    {
        let mut dialog = rfd::FileDialog::new();
        let here = Path::new(current);
        let start = if file { here.parent() } else { Some(here) };
        if let Some(dir) = start.filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let picked = if file {
            dialog.add_filter("Disc image", &["iso", "ISO"]).pick_file()
        } else {
            dialog.pick_folder()
        };
        picked.map(|p| p.display().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (current, file);
        None
    }
}

fn path_row(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String, file: bool) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new(label).strong());
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(ui.available_width() - 80.0);
        changed |= ui.add(edit).changed();
        if cfg!(windows) && ui.button("Browse...").clicked() {
            if let Some(p) = pick(value, file) {
                *value = p;
                changed = true;
            }
        }
    });
    changed
}

/// Copies the disc files out of the ISO (when the ISO changed) and builds the index.
fn prepare(
    iso_path: PathBuf,
    copy: bool,
    game_dir: String,
    copying: Arc<Mutex<bool>>,
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
) -> Result<(), String> {
    if copy {
        *copying.lock().unwrap() = true;
        iso::extract(&iso_path, &disc_dir(), done, total)?;
        *copying.lock().unwrap() = false;
    }
    // The scanner never overwrites, so the launcher removes its own previous index first.
    let index = index_path();
    let _ = std::fs::remove_file(&index);
    let args: Vec<OsString> = vec![
        "--game-dir".into(),
        game_dir.into(),
        "--json".into(),
        index.into_os_string(),
    ];
    lbp_cli::run(args)
        .map(|_partial| ())
        .map_err(|e| e.to_string())
}

fn start_setup(l: &mut Launcher) {
    if let Some(problem) = iso_problem(&l.cfg.iso) {
        l.message = problem;
        l.error = true;
        return;
    }
    let update = normalise_update(&l.cfg.update);
    if let Some(problem) = update_problem(&update) {
        l.message = problem;
        l.error = true;
        return;
    }
    l.cfg.update = update.display().to_string();
    if let Err(e) = std::fs::create_dir_all(data_dir()) {
        l.message = format!("Can't create {}: {e}", data_dir().display());
        l.error = true;
        return;
    }
    // Copy again only when the ISO itself changed (its part of the key).
    let old_iso = l
        .cfg
        .indexed
        .split('|')
        .take(2)
        .collect::<Vec<_>>()
        .join("|");
    let new_key = l.cfg.key();
    let new_iso = new_key.split('|').take(2).collect::<Vec<_>>().join("|");
    let copy = old_iso != new_iso || !disc_dir().join("PS3_GAME").is_dir();
    l.cfg.indexed.clear();
    let _ = l.cfg.save();
    let result: Slot<Result<(), String>> = Arc::new(Mutex::new(None));
    let copying = Arc::new(Mutex::new(false));
    let done = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(0));
    let (r, c, d, t) = (result.clone(), copying.clone(), done.clone(), total.clone());
    let iso_path = PathBuf::from(l.cfg.iso.trim());
    let game_dir = l.cfg.game_dir();
    std::thread::spawn(move || {
        let out = prepare(iso_path, copy, game_dir, c, d, t);
        *r.lock().unwrap() = Some(out);
    });
    l.job = Job::Preparing {
        result,
        copying,
        done,
        total,
        started: Instant::now(),
    };
    l.message = if copy {
        "Copying the game files out of your ISO (about 1.2 GB, once).".into()
    } else {
        "Building the asset index from your files.".into()
    };
    l.error = false;
}

fn launch(l: &mut Launcher, mode: &'static Mode) {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            l.message = format!("Can't find this program's own path: {e}");
            l.error = true;
            return;
        }
    };
    let mut cmd = Command::new(exe);
    cmd.arg("--snapshot")
        .arg(index_path())
        .arg("--game-dir")
        .arg(l.cfg.game_dir())
        .args(mode.args)
        .args(&l.pass_flags)
        .current_dir(data_dir())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    match cmd.spawn() {
        Ok(mut child) => {
            let result: Slot<Result<(), String>> = Arc::new(Mutex::new(None));
            let out = result.clone();
            std::thread::spawn(move || {
                // Bevy logs to stderr; it is drained here and its tail kept for errors.
                let mut err = String::new();
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_string(&mut err);
                }
                let r = match child.wait() {
                    Ok(status) if status.success() => Ok(()),
                    Ok(status) => {
                        let lines: Vec<&str> = err.lines().collect();
                        let tail = lines[lines.len().saturating_sub(4)..].join("\n");
                        Err(format!("exited with {status}\n{tail}"))
                    }
                    Err(e) => Err(e.to_string()),
                };
                *out.lock().unwrap() = Some(r);
            });
            l.running.push(Running {
                name: mode.name,
                result,
            });
            l.message = format!("Starting {}. Its window opens in a moment.", mode.name);
            l.error = false;
        }
        Err(e) => {
            l.message = format!("Couldn't start {}: {e}", mode.name);
            l.error = true;
        }
    }
}

fn poll(l: &mut Launcher) {
    if let Job::Preparing {
        result, started, ..
    } = &l.job
    {
        let finished = result.lock().unwrap().take();
        if let Some(r) = finished {
            let secs = started.elapsed().as_secs();
            match r {
                Ok(()) => {
                    l.cfg.indexed = l.cfg.key();
                    l.message = match l.cfg.save() {
                        Ok(()) => format!("Ready in {secs}s. Pick something to play."),
                        Err(e) => e,
                    };
                    l.error = false;
                }
                Err(e) => {
                    l.message = format!("Set up stopped: {e}");
                    l.error = true;
                }
            }
            l.job = Job::Idle;
        }
    }
    let mut i = 0;
    while i < l.running.len() {
        let finished = l.running[i].result.lock().unwrap().take();
        match finished {
            Some(r) => {
                let name = l.running.remove(i).name;
                if let Err(e) = r {
                    l.message = format!("{name} stopped: {e}");
                    l.error = true;
                }
            }
            None => i += 1,
        }
    }
}

fn menu(mut contexts: EguiContexts, mut l: ResMut<Launcher>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let l = &mut *l;
    poll(l);
    let busy = !matches!(l.job, Job::Idle);
    let mut root = egui::Ui::new(
        ctx.clone(),
        "launcher_root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.heading("craftworld-rs");
        ui.label(
            "A fan-made LittleBigPlanet 2 engine in Rust. It runs on your own copy of the game.",
        );
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(!busy, |ui| {
                let mut changed = path_row(
                    ui,
                    "LittleBigPlanet 2 ISO (USA, BCUS98245)",
                    "the .iso RPCS3 boots",
                    &mut l.cfg.iso,
                    true,
                );
                changed |= path_row(
                    ui,
                    "Update 1.33, installed in RPCS3",
                    "RPCS3's dev_hdd0/game/BCUS98245 folder",
                    &mut l.cfg.update,
                    false,
                );
                if changed {
                    let _ = l.cfg.save();
                }
                ui.horizontal(|ui| {
                    if ui.button("Set up").clicked() {
                        start_setup(l);
                    }
                    if busy {
                        ui.spinner();
                    }
                });
            });
        });
        ui.add_space(6.0);
        let colour = if l.error {
            egui::Color32::from_rgb(240, 110, 100)
        } else {
            egui::Color32::from_rgb(200, 210, 220)
        };
        ui.colored_label(colour, &l.message);
        if let Job::Preparing {
            copying,
            done,
            total,
            started,
            ..
        } = &l.job
        {
            if *copying.lock().unwrap() {
                let t = total.load(Ordering::Relaxed).max(1);
                let d = done.load(Ordering::Relaxed);
                ui.add(egui::ProgressBar::new(d as f32 / t as f32).show_percentage());
            } else {
                ui.label(format!(
                    "Indexing... {}s (copying finished)",
                    started.elapsed().as_secs()
                ));
            }
        }
        ui.add_space(8.0);
        let ready = l.cfg.ready() && !busy;
        for mode in MODES {
            ui.horizontal(|ui| {
                let button = egui::Button::new(mode.name);
                let on = ready && !mode.later;
                if ui
                    .add_enabled(on, button.min_size(egui::vec2(170.0, 28.0)))
                    .clicked()
                {
                    launch(l, mode);
                }
                ui.label(mode.blurb);
            });
        }
        if !l.running.is_empty() {
            ui.add_space(6.0);
            let names: Vec<&str> = l.running.iter().map(|r| r.name).collect();
            ui.label(format!("Running: {}", names.join(", ")));
        }
        ui.add_space(8.0);
        ui.small(format!(
            "The copied files, the index and these settings live in {}. Your ISO and RPCS3 \
             folders are only read.",
            data_dir().display()
        ));
    });
    // Keep polling the background jobs while nothing else wakes the window.
    ctx.request_repaint_after(Duration::from_millis(250));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_dir_is_the_copied_disc_then_the_update() {
        let c = Config {
            update: " D:/rpcs3/dev_hdd0/game/BCUS98245 ".into(),
            ..Config::default()
        };
        let disc = disc_dir().display().to_string();
        assert_eq!(
            c.game_dir(),
            format!("{disc};D:/rpcs3/dev_hdd0/game/BCUS98245")
        );
    }

    #[test]
    fn update_folder_is_found_from_rpcs3_or_its_dev_hdd0() {
        let dir = std::env::temp_dir().join(format!("lbp-launcher-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = dir.join("dev_hdd0/game/BCUS98245");
        std::fs::create_dir_all(game.join("USRDIR")).unwrap();
        for picked in [&dir, &dir.join("dev_hdd0"), &game] {
            assert_eq!(normalise_update(&picked.display().to_string()), game);
        }
        assert!(update_problem(&game).unwrap().contains("No installed game"));
        assert!(iso_problem("").unwrap().contains("Pick"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_open_mode_names_a_level_or_is_the_explorer() {
        for m in MODES.iter().filter(|m| !m.later) {
            let level = m
                .args
                .iter()
                .any(|a| *a == "--level-guid" || *a == "--level-path");
            assert!(level || m.name == "Asset explorer", "{}", m.name);
        }
    }
}
