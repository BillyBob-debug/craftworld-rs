//! `--play-sackboy` (docs/sim-physics.md, Part 4): with a level, the level's shapes as an
//! avian world (lbp-sim, material rules by MAP path) and Sackboy as the kinematic capsule
//! driven by the RPCS3-measured movement rules at 30 Hz. Arrow keys or the left stick walk
//! and change lanes (Up = lane behind, Down = lane in front), Space or the South button
//! jumps; the camera follows him. He is drawn as a placeholder capsule (the mesh and walk
//! cycle come later), and the level's own dynamic objects are simulated but not redrawn.
//!
//! `--play-sackboy-at X,Y,Z` sets the spawn (level units, capsule centre); otherwise the
//! first CHECKPOINT's POS plus [`SPAWN_OFFSET`]. `--play-sackboy-input STEPS` replaces the pad with a script of
//! `keys:ticks` steps (keys from `l r j b f`, `-` for none; e.g. `r:40,rj:12,-:30`) and prints
//! Sackboy's position every tick, for comparing paths against RPCS3.
use bevy::prelude::*;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::level_scene;
use lbp_sim::avian2d::prelude::*;
use lbp_sim::character::{spawn_sackboy, SackboyBody, SackboyPlugin, HALF_SEGMENT, RADIUS};
use lbp_sim::sackboy::Input;
use lbp_sim::scene::{material_rule_for_path, MaterialRule};
use lbp_sim::{build_scene_with, spawn_scene, to_level, PhysicsScene, SimPlugin};
use std::collections::BTreeMap;
use std::path::Path;

/// Level units to display units, set when the level is drawn (`LevelPlan::normalize`).
#[derive(Resource, Clone, Copy)]
pub(crate) struct LevelDisplay(pub Mat4);

#[derive(Resource)]
struct Play {
    scene: Option<PhysicsScene>,
    spawn: [f32; 3],
    script: Option<Vec<(Input, usize)>>,
    tick: usize,
}

#[derive(Component)]
struct Marker;

/// Sackboy's start relative to the first CHECKPOINT POS. RPCS3, Rookie Test (PC, d-run
/// 2026-10-09): checkpoint (-13335, 6971, -250), Sackboy appears at (-13335.7, 6763, 0) and
/// lands at y 5974. One level only: INFERRED for other levels.
const SPAWN_OFFSET: [f32; 3] = [0., -208., 250.];

/// Camera distance from Sackboy in level units. PLACEHOLDER (not measured in RPCS3).
const FOLLOW_DISTANCE: f32 = 1500.;

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

fn parse_script(text: &str) -> Result<Vec<(Input, usize)>, String> {
    text.split(',')
        .map(|step| {
            let (keys, ticks) = step
                .split_once(':')
                .ok_or_else(|| format!("--play-sackboy-input: {step:?} needs keys:ticks"))?;
            let ticks = ticks
                .trim()
                .parse::<usize>()
                .map_err(|e| format!("--play-sackboy-input: {step:?}: {e}"))?;
            let mut input = Input::default();
            for c in keys.trim().chars() {
                match c {
                    'l' => input.x = -1.,
                    'r' => input.x = 1.,
                    'j' => input.jump = true,
                    'b' => input.back = true,
                    'f' => input.front = true,
                    '-' => {}
                    _ => return Err(format!("--play-sackboy-input: unknown key {c:?}")),
                }
            }
            Ok((input, ticks))
        })
        .collect()
}

/// Reads the level and adds physics and Sackboy when `--play-sackboy` is given.
pub fn install(app: &mut App) -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    if !all.iter().any(|a| a == "--play-sackboy") {
        return Ok(());
    }
    let snapshot = value(&all, "--snapshot").ok_or("--play-sackboy requires --snapshot")?;
    let game = value(&all, "--game-dir").ok_or("--play-sackboy requires --game-dir")?;
    let guid = value(&all, "--level-guid")
        .map(|g| u32::from_str_radix(&g, 16).map_err(|_| "--level-guid requires hex digits"))
        .transpose()?;
    let path = value(&all, "--level-path");
    if guid.is_none() && path.is_none() {
        return Err("--play-sackboy requires --level-guid or --level-path".into());
    }
    let at = value(&all, "--play-sackboy-at")
        .map(|v| {
            let p: Vec<f32> = v
                .split(',')
                .map(|x| x.trim().parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|e| format!("--play-sackboy-at: {e}"))?;
            <[f32; 3]>::try_from(p).map_err(|_| "--play-sackboy-at needs X,Y,Z".to_string())
        })
        .transpose()?;
    let script = value(&all, "--play-sackboy-input")
        .map(|s| parse_script(&s))
        .transpose()?;
    // The level reader recurses deeply: same worker stack as the level scene.
    let (scene, checkpoints, header) = std::thread::Builder::new()
        .stack_size(level_scene::LEVEL_STACK_BYTES)
        .spawn(move || -> Result<_, String> {
            let control = LoadControl::default();
            let catalog = lbp_catalog::load(Path::new(&snapshot), &control)?;
            let source = AttachedSource::attach(Path::new(&game), &catalog, &control)?;
            let mut rules = BTreeMap::<u32, MaterialRule>::new();
            for r in &catalog.snapshot.resources {
                if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
                    let rule = material_rule_for_path(&r.map_row.path);
                    if rule != MaterialRule::Normal {
                        rules.entry(g).or_insert(rule);
                    }
                }
            }
            let (key, identity) = level_scene::resolve_level(&catalog, guid, path.as_deref())?;
            let bytes = level_scene::read_level_bytes(&source, &key, &control)?;
            let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
                .map_err(|e| e.to_string())?;
            let scene = build_scene_with(&report.things, &|d| {
                d.guid
                    .and_then(|g| rules.get(&g).copied())
                    .unwrap_or_default()
            });
            let r = &scene.report;
            let header = format!(
                "Play Sackboy: {identity}; shapes {}: dynamic {}, static {}, not collidable {}, rejected {}; joints {}",
                r.shapes,
                r.dynamic,
                r.static_,
                r.not_collidable,
                r.rejected.values().sum::<usize>(),
                r.joints
            );
            Ok((scene, level_scene::checkpoints(&report.things), header))
        })
        .map_err(|e| format!("play sackboy thread: {e}"))?
        .join()
        .map_err(|_| "play sackboy thread panicked".to_string())??;
    let spawn = match (at, checkpoints.first()) {
        (Some(p), _) => p,
        (None, Some((_, m))) => [
            m[12] + SPAWN_OFFSET[0],
            m[13] + SPAWN_OFFSET[1],
            m[14] + SPAWN_OFFSET[2],
        ],
        (None, None) => return Err("--play-sackboy: no CHECKPOINT; give --play-sackboy-at".into()),
    };
    let points: Vec<String> = checkpoints
        .iter()
        .map(|(t, m)| format!("Thing {t} ({:.0}, {:.0}, {:.0})", m[12], m[13], m[14]))
        .collect();
    println!(
        "{header}; checkpoints [{}]; spawn ({:.0}, {:.0}, {:.0})",
        points.join(", "),
        spawn[0],
        spawn[1],
        spawn[2]
    );
    app.add_plugins((SimPlugin::default(), SackboyPlugin))
        .insert_resource(Play {
            scene: Some(scene),
            spawn,
            script,
            tick: 0,
        })
        .add_systems(Update, (spawn_world, read_pad, show_sackboy).chain())
        .add_systems(FixedFirst, script_input)
        .add_systems(FixedLast, log_scripted);
    Ok(())
}

fn spawn_world(mut commands: Commands, mut play: ResMut<Play>) {
    let Some(scene) = play.scene.take() else {
        return;
    };
    spawn_scene(&mut commands, &scene);
    let [x, y, z] = play.spawn;
    spawn_sackboy(&mut commands, [x, y], z);
}

fn read_pad(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    play: Res<Play>,
    mut sackboy: Query<&mut SackboyBody>,
) {
    if play.script.is_some() {
        return;
    }
    let pad = pads.iter().next();
    let held = |k: KeyCode, b: GamepadButton| keys.pressed(k) || pad.is_some_and(|p| p.pressed(b));
    let mut x = pad.map_or(0., |p| p.left_stick().x);
    if x.abs() < 0.3 {
        x = 0.;
    }
    if held(KeyCode::ArrowLeft, GamepadButton::DPadLeft) {
        x = -1.;
    }
    if held(KeyCode::ArrowRight, GamepadButton::DPadRight) {
        x = 1.;
    }
    let input = Input {
        x,
        jump: held(KeyCode::Space, GamepadButton::South),
        back: held(KeyCode::ArrowUp, GamepadButton::DPadUp),
        front: held(KeyCode::ArrowDown, GamepadButton::DPadDown),
    };
    for mut body in &mut sackboy {
        body.input = input;
    }
}

/// Scripted input for this tick (none once the script ends).
fn script_input(play: Res<Play>, mut sackboy: Query<&mut SackboyBody>) {
    let Some(script) = &play.script else {
        return;
    };
    let mut left = play.tick;
    let input = script.iter().find_map(|(input, ticks)| {
        if left < *ticks {
            Some(*input)
        } else {
            left -= ticks;
            None
        }
    });
    for mut body in &mut sackboy {
        body.input = input.unwrap_or_default();
    }
}

/// With a script, Sackboy's position after each tick (1-based).
fn log_scripted(
    mut play: ResMut<Play>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
) {
    if play.script.is_none() {
        return;
    }
    let Ok((body, pos, slab)) = sackboy.single() else {
        return;
    };
    play.tick += 1;
    let p = to_level(pos.0);
    println!(
        "sackboy tick {} x {:.1} y {:.1} z {:.1} vx {:.2} vy {:.2} grounded {}",
        play.tick, p[0], p[1], slab.z, body.controller.vx, body.controller.vy, body.grounded
    );
}

fn show_sackboy(
    mut commands: Commands,
    display: Option<Res<LevelDisplay>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sackboy: Query<(&Position, &lbp_sim::ZSlab), With<SackboyBody>>,
    mut marker: Query<&mut Transform, With<Marker>>,
    mut orbit: ResMut<crate::Orbit>,
) {
    let (Some(display), Ok((pos, slab))) = (display, sackboy.single()) else {
        return;
    };
    let p = to_level(pos.0);
    let at = display.0.transform_point3(Vec3::new(p[0], p[1], slab.z));
    match marker.single_mut() {
        Ok(mut t) => t.translation = at,
        Err(_) => {
            let s = display.0.x_axis.length();
            commands.spawn((
                Marker,
                Mesh3d(meshes.add(Capsule3d::new(RADIUS * s, 2. * HALF_SEGMENT * s))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.85, 0.55, 0.3),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(at),
            ));
            // Close in on him once. PLACEHOLDER: the game camera's distance is not measured.
            orbit.distance = FOLLOW_DISTANCE * s;
        }
    }
    orbit.target = at;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_script_steps() {
        let s = parse_script("r:40, rj:12,-:30,bl:1").unwrap();
        assert_eq!(s.len(), 4);
        assert_eq!((s[0].0.x, s[0].1), (1., 40));
        assert!(s[1].0.jump && s[1].0.x == 1.);
        assert_eq!(s[2].0, Input::default());
        assert!(s[3].0.back && s[3].0.x == -1.);
        assert!(parse_script("x:3").is_err());
        assert!(parse_script("r").is_err());
    }
}
