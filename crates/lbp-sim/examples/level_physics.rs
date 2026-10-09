//! R2 PC check (docs/sim-physics.md): does a level stand still in the sandbox?
//!
//! level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON [--ticks=N]
//!
//! Reads the level(s), builds the physics scene (bodies, static and dynamic shapes,
//! rejected shapes by reason) and runs it headless for N game ticks at 30 Hz (default 90,
//! three seconds). Reports, per level and in total, how far bodies moved from where the
//! level stored them: bodies moved more than 1 and more than 100 world units, the largest
//! move, and bodies whose position became non-finite. Counting unit: SHAPE Thing (scene)
//! and BODY Thing with a collider (motion). Counts only; no game bytes. Output: a NEW JSON
//! file under ignored `local-analysis/`.
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_sim::{build_scene, spawn_scene, PhysicsScene, SimPlugin, ThingIndex, TICK_HZ};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        eprintln!("level_physics: {e}");
        std::process::exit(1);
    }
}

#[derive(Default)]
struct Motion {
    bodies: usize,
    moved_1: usize,
    moved_100: usize,
    non_finite: usize,
    max_move: f32,
}

fn simulate(scene: &PhysicsScene, ticks: usize) -> Motion {
    let start: BTreeMap<usize, [f32; 2]> = scene
        .bodies
        .iter()
        .filter(|b| {
            scene
                .colliders
                .iter()
                .any(|c| c.body.is_some_and(|i| scene.bodies[i].thing == b.thing))
        })
        .map(|b| (b.thing, b.frame.position))
        .collect();
    let owned = scene.clone();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, SimPlugin::default()))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1. / TICK_HZ,
        )))
        .add_systems(Startup, move |mut commands: Commands| {
            spawn_scene(&mut commands, &owned);
        });
    app.finish();
    for _ in 0..ticks {
        app.update();
    }
    let mut m = Motion::default();
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position), With<RigidBody>>();
    for (t, p) in q.iter(app.world()) {
        let Some(s) = start.get(&t.0) else {
            continue;
        };
        m.bodies += 1;
        let d = ((p.x - s[0]).powi(2) + (p.y - s[1]).powi(2)).sqrt();
        if !d.is_finite() {
            m.non_finite += 1;
            continue;
        }
        m.moved_1 += usize::from(d > 1.);
        m.moved_100 += usize::from(d > 100.);
        m.max_move = m.max_move.max(d);
    }
    m
}

fn run() -> Result<(), String> {
    let all: Vec<String> = std::env::args().collect();
    let ticks: usize = all
        .iter()
        .find_map(|a| a.strip_prefix("--ticks="))
        .map_or(Ok(90), str::parse)
        .map_err(|e| format!("--ticks: {e}"))?;
    let args: Vec<&String> = all.iter().filter(|a| !a.starts_with("--ticks=")).collect();
    if args.len() != 5 {
        return Err(
            "usage: level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|--all NEW-JSON [--ticks=N]".into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let mut keys = Vec::new();
    if args[3] == "--all" {
        let mut hashes = BTreeMap::new();
        for r in &catalog.snapshot.resources {
            if r.display_type() == "level" {
                hashes
                    .entry(r.map_row.sha1.to_ascii_lowercase())
                    .or_insert_with(|| r.map_row.path.clone());
            }
        }
        for (h, path) in hashes {
            if let Ok(key) = lbp_preview::resolve(&catalog, None, Some(&h)) {
                keys.push((key, path));
            }
        }
    } else {
        keys.push(lbp_preview::level_scene::resolve_level(
            &catalog,
            None,
            Some(args[3]),
        )?);
    }
    let mut levels = Vec::new();
    let mut total = Motion::default();
    let (mut shapes, mut dynamic, mut static_, mut not_collidable, mut not_read) = (0, 0, 0, 0, 0);
    let mut rejected = BTreeMap::<&str, usize>::new();
    for (key, label) in keys {
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, &key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let Ok(report) = read else {
            not_read += 1;
            continue;
        };
        let scene = build_scene(&report.things);
        let r = &scene.report;
        shapes += r.shapes;
        dynamic += r.dynamic;
        static_ += r.static_;
        not_collidable += r.not_collidable;
        for (k, n) in &r.rejected {
            *rejected.entry(k).or_default() += n;
        }
        let m = simulate(&scene, ticks);
        total.bodies += m.bodies;
        total.moved_1 += m.moved_1;
        total.moved_100 += m.moved_100;
        total.non_finite += m.non_finite;
        total.max_move = total.max_move.max(m.max_move);
        levels.push(serde_json::json!({
            "level": label,
            "stopped": report.stop.is_some(),
            "shapes": r.shapes, "dynamic": r.dynamic, "static": r.static_,
            "not_collidable": r.not_collidable, "rejected": r.rejected,
            "bodies": m.bodies, "moved_over_1": m.moved_1, "moved_over_100": m.moved_100,
            "non_finite": m.non_finite, "max_move": m.max_move,
        }));
    }
    let doc = serde_json::json!({
        "schema_version": 1,
        "unit": "SHAPE Thing (scene); BODY Thing with a collider (motion), world units",
        "ticks": ticks,
        "tick_hz": TICK_HZ,
        "levels": levels.len(),
        "levels_not_read": not_read,
        "shapes": shapes, "dynamic": dynamic, "static": static_,
        "not_collidable": not_collidable, "rejected": rejected,
        "bodies": total.bodies, "moved_over_1": total.moved_1,
        "moved_over_100": total.moved_100, "non_finite": total.non_finite,
        "max_move": total.max_move,
        "per_level": levels,
    });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "levels {} (not read {not_read}); shapes {shapes}: dynamic {dynamic}, static {static_}, \
         not collidable {not_collidable}, rejected {}; after {ticks} ticks: bodies {}, moved >1 {}, \
         >100 {}, non-finite {}, max move {:.1}",
        levels.len(),
        rejected.values().sum::<usize>(),
        total.bodies,
        total.moved_1,
        total.moved_100,
        total.non_finite,
        total.max_move
    );
    Ok(())
}
