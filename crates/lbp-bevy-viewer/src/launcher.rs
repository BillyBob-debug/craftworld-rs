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
    /// The viewer's `--game-dir` list: the disc (the copy out of the ISO, or the player's
    /// own disc folder), then the installed update.
    fn game_dir(&self) -> String {
        let disc = match disc_input(&self.iso) {
            Ok(Disc::Folder(root)) => root,
            _ => disc_dir(),
        };
        format!("{};{}", disc.display(), clean(&self.update))
    }
    /// What the copy and index depend on: the ISO (path, size, time) and the update path.
    fn key(&self) -> String {
        let iso = clean(&self.iso);
        let stamped = match disc_input(&iso) {
            Ok(Disc::Folder(root)) => root.join("PS3_GAME").join("PARAM.SFO"),
            _ => PathBuf::from(&iso),
        };
        let stamp = std::fs::metadata(stamped)
            .map(|m| {
                let t = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                format!("{}@{t}", m.len())
            })
            .unwrap_or_default();
        format!("{iso}|{stamp}|{}", clean(&self.update))
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

/// A pasted path without the quotes Windows' "Copy as path" puts around it.
fn clean(path: &str) -> String {
    path.trim().trim_matches('"').trim().to_owned()
}

/// Accepts the game folder itself, RPCS3's folder, its dev_hdd0, or the game's USRDIR.
fn normalise_update(path: &str) -> PathBuf {
    let p = PathBuf::from(clean(path));
    if p.file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("USRDIR"))
    {
        if let Some(game) = p.parent().filter(|g| g.join("PARAM.SFO").is_file()) {
            return game.to_path_buf();
        }
    }
    for sub in [&[][..], &["game"], &["dev_hdd0", "game"]] {
        let base = sub.iter().fold(p.clone(), |b, s| b.join(s));
        let game = base.join(iso::TITLE_ID);
        if game.join("USRDIR").is_dir() {
            return game;
        }
    }
    p
}

/// What the first box names: the .iso file, or a disc folder holding PS3_GAME (a game
/// RPCS3 boots from a folder instead of an ISO), which is read in place.
#[derive(Debug, PartialEq)]
enum Disc {
    Iso(PathBuf),
    Folder(PathBuf),
}

/// Reads the first box, or says in plain words what is wrong with it and how to fix it.
fn disc_input(path: &str) -> Result<Disc, String> {
    let raw = clean(path);
    if raw.is_empty() {
        return Err(
            "Pick your LittleBigPlanet 2 disc: the .iso file you play in RPCS3, or the \
             game folder that holds PS3_GAME."
                .into(),
        );
    }
    let p = PathBuf::from(&raw);
    if p.is_dir() {
        let disc_root = [Some(p.as_path()), p.parent()]
            .into_iter()
            .flatten()
            .find(|r| r.join("PS3_GAME").join("PARAM.SFO").is_file());
        if let Some(root) = disc_root {
            return Ok(Disc::Folder(root.to_path_buf()));
        }
        if p.join("USRDIR").is_dir() || p.join("dev_hdd0").is_dir() {
            return Err(
                "That's RPCS3's installed game/update folder, which goes in the second box. \
                 The first box needs the disc: the .iso file, or the folder holding PS3_GAME."
                    .into(),
            );
        }
        return Err(format!(
            "There's no PS3_GAME folder in {}. Pick the .iso file itself, or the game \
             folder that holds PS3_GAME.",
            p.display()
        ));
    }
    if !p.is_file() {
        return Err(format!(
            "Can't find {}. Check the path for typos, or use Browse to pick the file.",
            p.display()
        ));
    }
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "zip" | "7z" | "rar" | "gz" | "tar" => Err(
            "That's a compressed archive, not the disc. Extract it first (right-click > \
             Extract All, or 7-Zip), then pick the .iso inside."
                .into(),
        ),
        "pkg" => Err(
            "That's an update package. Install it in RPCS3 (File > Install Packages/Raps/Edats), \
             then pick the game's .iso here and RPCS3's dev_hdd0/game/BCUS98245 below."
                .into(),
        ),
        "rap" | "edat" | "dkey" | "ird" | "sfo" | "bin" | "self" | "elf" => Err(format!(
            "That's a single .{ext} file, not the game disc. Pick the .iso file (about \
             10 GB or more)."
        )),
        _ => Ok(Disc::Iso(p)),
    }
}

/// Checks a disc's PARAM.SFO names the USA LittleBigPlanet 2.
fn title_problem(sfo: &[u8], what: &str) -> Option<String> {
    let id = lbp_io::sfo::read_identity(sfo)
        .ok()
        .and_then(|i| i.title_id);
    match id.as_deref() {
        Some(iso::TITLE_ID) => None,
        Some(other) => Some(format!(
            "{what} is {other}, not the USA LittleBigPlanet 2 ({}). Only the USA disc \
             works for now; other regions come later.",
            iso::TITLE_ID
        )),
        None => Some(format!(
            "{what} has no readable PARAM.SFO: it isn't a PS3 game disc, or it's an \
             encrypted disc dump. Use the ISO (or game folder) that RPCS3 actually plays."
        )),
    }
}

/// The game's archives end in the `FARC` tag. Files that don't were copied out of a disc
/// image whose contents are still encrypted (a raw disc dump), which can't be read.
fn archive_problem(disc_root: &Path) -> Option<String> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>, depth: u32) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 8 {
                walk(&p, found, depth + 1);
            } else if p
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("farc"))
            {
                found.push(p);
            }
        }
    }
    let mut farcs = Vec::new();
    walk(&disc_root.join("PS3_GAME").join("USRDIR"), &mut farcs, 0);
    if farcs.is_empty() {
        return Some(
            "The disc has no game archives (.farc files) in PS3_GAME/USRDIR, so it's \
             incomplete or not the LittleBigPlanet 2 disc."
                .into(),
        );
    }
    let tagged = |p: &Path| -> bool {
        let Ok(mut f) = std::fs::File::open(p) else {
            return false;
        };
        let mut tail = [0u8; 4];
        use std::io::{Seek, SeekFrom};
        f.seek(SeekFrom::End(-4)).is_ok() && f.read_exact(&mut tail).is_ok() && &tail == b"FARC"
    };
    let bad = farcs.iter().filter(|p| !tagged(p)).count();
    (bad > 0).then(|| {
        format!(
            "{bad} of the disc's {} game archives can't be read: their contents are \
             scrambled. That happens with an encrypted disc dump. Use the ISO (or game \
             folder) that RPCS3 actually plays.",
            farcs.len()
        )
    })
}

fn update_problem(path: &Path) -> Option<String> {
    if path.as_os_str().is_empty() {
        return Some(format!(
            "Pick the update folder: RPCS3's dev_hdd0/game/{} (install the LBP2 updates \
             in RPCS3 first).",
            iso::TITLE_ID
        ));
    }
    if path.is_file() {
        return Some(format!(
            "The second box needs a folder, not a file: RPCS3's dev_hdd0/game/{}.",
            iso::TITLE_ID
        ));
    }
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
    disc: Disc,
    copy: bool,
    game_dir: String,
    copying: Arc<Mutex<bool>>,
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
) -> Result<(), String> {
    let root = match disc {
        Disc::Iso(iso_path) => {
            if copy {
                *copying.lock().unwrap() = true;
                iso::extract(&iso_path, &disc_dir(), done, total)?;
                *copying.lock().unwrap() = false;
            }
            disc_dir()
        }
        Disc::Folder(root) => {
            let sfo = std::fs::read(root.join("PS3_GAME").join("PARAM.SFO"))
                .map_err(|e| format!("Read the disc's PARAM.SFO: {e}"))?;
            if let Some(problem) = title_problem(&sfo, "That game folder") {
                return Err(problem);
            }
            root
        }
    };
    if let Some(problem) = archive_problem(&root) {
        return Err(problem);
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
    l.cfg.iso = clean(&l.cfg.iso);
    let disc = match disc_input(&l.cfg.iso) {
        Ok(d) => d,
        Err(problem) => {
            l.message = problem;
            l.error = true;
            return;
        }
    };
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
    let copy = matches!(disc, Disc::Iso(_))
        && (old_iso != new_iso || !disc_dir().join("PS3_GAME").is_dir());
    l.cfg.indexed.clear();
    let _ = l.cfg.save();
    let result: Slot<Result<(), String>> = Arc::new(Mutex::new(None));
    let copying = Arc::new(Mutex::new(false));
    let done = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(0));
    let (r, c, d, t) = (result.clone(), copying.clone(), done.clone(), total.clone());
    let game_dir = l.cfg.game_dir();
    std::thread::spawn(move || {
        let out = prepare(disc, copy, game_dir, c, d, t);
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
                    l.message = format!(
                        "Set up stopped: {e}\nSee Troubleshooting in the README on GitHub \
                         (github.com/chilladept/craftworld-rs)."
                    );
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
                    "LittleBigPlanet 2 disc (USA, BCUS98245)",
                    "the .iso file RPCS3 boots, or the game folder holding PS3_GAME",
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
        assert!(disc_input("").unwrap_err().contains("Pick"));
        // The update folder in the disc box, and the update's USRDIR in the update box.
        assert!(disc_input(&game.display().to_string())
            .unwrap_err()
            .contains("second box"));
        std::fs::write(game.join("PARAM.SFO"), b"").unwrap();
        assert_eq!(
            normalise_update(&format!("\"{}\"", game.join("USRDIR").display())),
            game
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disc_box_explains_wrong_picks() {
        let dir = std::env::temp_dir().join(format!("lbp-launcher-disc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let usrdir = dir.join("PS3_GAME/USRDIR");
        std::fs::create_dir_all(&usrdir).unwrap();
        // No PS3_GAME/PARAM.SFO yet: not a disc folder.
        assert!(disc_input(&dir.display().to_string())
            .unwrap_err()
            .contains("no PS3_GAME"));
        std::fs::write(dir.join("PS3_GAME/PARAM.SFO"), b"").unwrap();
        // The folder, PS3_GAME itself, and a quoted pasted path all find the disc root.
        for picked in [
            dir.display().to_string(),
            dir.join("PS3_GAME").display().to_string(),
            format!("\"{}\"", dir.display()),
        ] {
            assert_eq!(disc_input(&picked).unwrap(), Disc::Folder(dir.clone()));
        }
        for (name, says) in [
            ("game.7z", "Extract it first"),
            ("update.pkg", "Install it in RPCS3"),
            ("key.rap", "not the game disc"),
        ] {
            let f = dir.join(name);
            std::fs::write(&f, b"x").unwrap();
            assert!(disc_input(&f.display().to_string())
                .unwrap_err()
                .contains(says));
        }
        let iso_file = dir.join("lbp2.iso");
        std::fs::write(&iso_file, b"x").unwrap();
        assert_eq!(
            disc_input(&iso_file.display().to_string()).unwrap(),
            Disc::Iso(iso_file)
        );
        assert!(disc_input(&dir.join("missing.iso").display().to_string())
            .unwrap_err()
            .contains("Can't find"));
        // Archives: none, then one scrambled, then all tagged.
        assert!(archive_problem(&dir).unwrap().contains("no game archives"));
        std::fs::write(usrdir.join("DATA.FARC"), b"\x01\x02\x03\x04junk").unwrap();
        assert!(archive_problem(&dir)
            .unwrap()
            .contains("encrypted disc dump"));
        std::fs::write(usrdir.join("DATA.FARC"), b"\0\0\0\0FARC").unwrap();
        assert_eq!(archive_problem(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn title_check_names_the_other_disc() {
        let sfo = |id: &[u8]| {
            let mut d = b"\0PSF".to_vec();
            for n in [0x101_u32, 36, 48, 1] {
                d.extend(n.to_le_bytes());
            }
            d.extend(0_u16.to_le_bytes());
            d.extend(0x204_u16.to_le_bytes());
            for n in [10_u32, 10, 0] {
                d.extend(n.to_le_bytes());
            }
            d.extend(b"TITLE_ID\0");
            d.resize(48, 0);
            d.extend(id);
            d.push(0);
            d
        };
        assert_eq!(title_problem(&sfo(b"BCUS98245"), "That ISO"), None);
        let other = title_problem(&sfo(b"BCES00850"), "That ISO").unwrap();
        assert!(other.contains("BCES00850") && other.contains("USA"));
        assert!(title_problem(b"junk", "That ISO")
            .unwrap()
            .contains("PARAM.SFO"));
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
