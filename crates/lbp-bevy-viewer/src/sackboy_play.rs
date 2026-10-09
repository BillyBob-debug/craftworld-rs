//! `--play-sackboy` (docs/sim-physics.md, Part 4): with a level, the level's shapes as an
//! avian world (lbp-sim, material rules by MAP path) and Sackboy as the kinematic capsule
//! driven by the RPCS3-measured movement rules at 30 Hz. Arrow keys or the left stick walk
//! and change lanes (Up = lane behind, Down = lane in front), Space or the South button
//! jumps; the camera follows him. With `--select-guid` naming the Sackboy mesh he is drawn
//! as that mesh standing on the capsule (feet at its bottom), turned to the walk direction,
//! and `--play-animation` (a walk clip) cycles at the walk speed; otherwise as a placeholder
//! capsule. The level's own dynamic objects are simulated but not redrawn.
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
use lbp_sim::sackboy::{Input, WALK_SPEED};
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

/// The Sackboy mesh standing on the capsule: `upright` turns the raw Z-up model to the
/// display (feet at the origin, level-unit scale); `facing` is the turn about the display Y
/// axis; `phase` the walk clip's frame.
#[derive(Component)]
pub(crate) struct Figure {
    upright: Mat4,
    facing: f32,
    phase: f32,
}

/// True when the viewer was started with `--play-sackboy`.
pub(crate) fn enabled() -> bool {
    std::env::args().any(|a| a == "--play-sackboy")
}

/// Turn about the display Y axis when walking right (+x) or left. The upright model faces
/// the camera (+Z); a quarter turn shows him in profile. INFERRED from the Pod figure's
/// front view; LBP2 may show him three-quarter on (RPCS3 capture to check).
const FACE_RIGHT: f32 = std::f32::consts::FRAC_PI_2;
const FACE_LEFT: f32 = -std::f32::consts::FRAC_PI_2;

/// Spawns the Sackboy mesh (`--select-guid`) as the play figure and, with a clip, its rig
/// and playback (frames chosen by [`show_sackboy`]). Model units are level units (as for
/// the Pod figure), so `k` is the level display scale.
pub(crate) fn spawn_figure(
    v: &mut crate::Viewer,
    (commands, meshes, materials, images): (
        &mut Commands,
        &mut Assets<Mesh>,
        &mut Assets<crate::ViewerMaterial>,
        &mut Assets<Image>,
    ),
    bindposes: &mut Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    k: f32,
    preview: &lbp_preview::GroupedMesh,
    clip: Option<crate::ClipResult>,
) -> Result<String, String> {
    let [lo, hi] =
        crate::adapter_bounds(&preview.geometry.positions).ok_or("Unsupported: empty figure")?;
    // Raw Z-up model: feet are the lowest z, centred in x and y (depth).
    let origin = Mat4::from_translation(Vec3::new(
        -(lo[0] + hi[0]) / 2.,
        -(lo[1] + hi[1]) / 2.,
        -lo[2],
    ));
    let upright =
        Mat4::from_scale(Vec3::splat(k)) * Mat4::from_quat(crate::model_up(true)) * origin;
    let figure = commands
        .spawn((
            Transform::from_matrix(upright),
            Visibility::default(),
            Figure {
                upright,
                facing: FACE_RIGHT,
                phase: 0.,
            },
        ))
        .id();
    let mut evidence = crate::browser::PreviewEvidence::default();
    let mut details = Vec::new();
    let first = v.scene.len();
    for p in crate::prepare(preview, false, &mut evidence, &mut details)? {
        let (mesh, material, image) = crate::upload(p, meshes, materials, images);
        let entity = commands
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                ChildOf(figure),
            ))
            .id();
        v.scene.push((entity, mesh, material, image));
    }
    let mut line = format!(
        "Play figure: model height {:.0} level units (capsule {:.0})",
        hi[2] - lo[2],
        2. * (HALF_SEGMENT + RADIUS)
    );
    let scene: Vec<_> = v.scene[first..]
        .iter()
        .map(|s| (s.0, s.1.clone()))
        .collect();
    match clip {
        None => line += "; no walk clip (--play-animation)",
        Some(Err(e)) => line += &format!("; walk clip not played: {e}"),
        Some(Ok(clip)) => {
            // Raw model units: the identity display frame.
            let frame = ([0.; 3], 1.);
            let gpu = if v.cpu_skinning {
                None
            } else {
                let rig = crate::gpu_rig(
                    commands,
                    meshes,
                    bindposes,
                    &clip.rig,
                    (&clip.skin.joints, &clip.skin.weights),
                    (frame, Quat::IDENTITY),
                    &scene,
                )?;
                commands.entity(rig.root).insert(ChildOf(figure));
                Some(crate::GpuRig {
                    morphs: !clip.skin.morphs.is_empty() && clip.report.animation_morphs > 0,
                    ..rig
                })
            };
            line += &format!(
                "; walk clip {} frames at {} fps",
                clip.report.frames, clip.report.fps
            );
            v.animation_frame = Some(0);
            v.playing = Some(crate::Playing {
                clip,
                frame,
                started: -1.,
                shown: None,
                gpu,
                meshes: scene.into_iter().map(|s| s.1).collect(),
                alt: None,
            });
        }
    }
    Ok(line)
}

/// The walk clip's next frame position: advances at the clip rate scaled by walking speed
/// (full rate at [`WALK_SPEED`]) while grounded and moving; otherwise back to frame 0.
/// INFERRED pacing (RPCS3 stride timing to check).
fn walk_phase(phase: f32, dt: f32, fps: f32, vx: f32, grounded: bool) -> f32 {
    let speed = (vx.abs() / WALK_SPEED).min(1.);
    if grounded && speed > 0.05 {
        phase + dt * fps * speed
    } else {
        0.
    }
}

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

#[allow(clippy::too_many_arguments)]
fn show_sackboy(
    mut commands: Commands,
    display: Option<Res<LevelDisplay>>,
    time: Res<Time>,
    mut v: ResMut<crate::Viewer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sackboy: Query<(&SackboyBody, &Position, &lbp_sim::ZSlab)>,
    mut marker: Query<&mut Transform, (With<Marker>, Without<Figure>)>,
    mut figure: Query<(&mut Transform, &mut Figure)>,
    mut orbit: ResMut<crate::Orbit>,
    mut framed: Local<bool>,
) {
    let (Some(display), Ok((body, pos, slab))) = (display, sackboy.single()) else {
        return;
    };
    let p = to_level(pos.0);
    let at = display.0.transform_point3(Vec3::new(p[0], p[1], slab.z));
    let s = display.0.x_axis.length();
    if let Ok((mut t, mut f)) = figure.single_mut() {
        let vx = body.controller.vx;
        if vx > 0.5 {
            f.facing = FACE_RIGHT;
        } else if vx < -0.5 {
            f.facing = FACE_LEFT;
        }
        let feet =
            display
                .0
                .transform_point3(Vec3::new(p[0], p[1] - (HALF_SEGMENT + RADIUS), slab.z));
        *t = Transform::from_matrix(
            Mat4::from_translation(feet) * Mat4::from_rotation_y(f.facing) * f.upright,
        );
        if let Some(fps) = v
            .playing
            .as_ref()
            .map(|p| f32::from(p.clip.report.fps.max(1)))
        {
            f.phase = walk_phase(f.phase, time.delta_secs(), fps, vx, body.grounded);
            v.animation_frame = Some(f.phase as i64);
        }
    } else if let Ok(mut t) = marker.single_mut() {
        t.translation = at;
    } else {
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
    }
    if !*framed {
        *framed = true;
        // Close in on him once. PLACEHOLDER: the game camera's distance is not measured.
        orbit.distance = FOLLOW_DISTANCE * s;
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

    #[test]
    fn walk_cycle_paces_with_speed() {
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, true), 19.);
        assert_eq!(walk_phase(4., 0.5, 30., -WALK_SPEED / 2., true), 11.5);
        assert_eq!(walk_phase(4., 0.5, 30., WALK_SPEED, false), 0.);
        assert_eq!(walk_phase(4., 0.5, 30., 0., true), 0.);
    }
}
