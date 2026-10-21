//! R2 PC check (docs/sim-physics.md): does a level stand still in the sandbox?
//!
//! level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|guid:HEX|--all NEW-JSON [--ticks=N]
//!
//! Reads the level(s), builds the physics scene (bodies, static and dynamic shapes,
//! rejected shapes by reason) and runs it headless for N game ticks at 30 Hz (default 90,
//! three seconds). Reports, per level and in total, how far bodies moved from where the
//! level stored them: bodies moved more than 1 and more than 100 world units, the largest
//! move, and bodies whose position became non-finite. Fixed materials: physical materials
//! whose MAP path names dark matter (`dark_matter`/`darkmatter`, case-insensitive) make
//! their body static; the report lists the paths matched and the most used shape materials
//! so the rule can be checked. Counting unit: SHAPE Thing (scene)
//! and BODY Thing with a collider (motion). Counts only; no game bytes. Output: a NEW JSON
//! file under ignored `local-analysis/`.
use avian2d::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_sim::{
    build_scene_with, scene::MaterialRule, spawn_scene, to_level, PhysicsScene, SimPlugin,
    ThingIndex, ZSlab, TICK_HZ,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

fn main() {
    // The level reader recurses natively: run on a thread with the level-loader stack.
    let worker = std::thread::Builder::new()
        .stack_size(lbp_preview::level_scene::LEVEL_STACK_BYTES)
        .spawn(run)
        .expect("spawn the level_physics worker");
    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("level_physics: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("level_physics: worker panicked");
            std::process::exit(1);
        }
    }
}

#[derive(Default)]
struct Motion {
    bodies: usize,
    moved_1: usize,
    moved_100: usize,
    non_finite: usize,
    max_move: f32,
    /// Bodies that moved more than 100: Thing and displacement.
    moved: Vec<(usize, [f32; 2])>,
    /// First tick (1-based) each body was more than 1 unit from its start.
    first_moved: BTreeMap<usize, usize>,
    /// Bodies seen asleep, and the first tick each was awake again after that.
    asleep: std::collections::BTreeSet<usize>,
    /// What a downward ray from each dynamic body meets after tick 1 (same slab, own
    /// colliders skipped).
    below: BTreeMap<usize, &'static str>,
    woke: BTreeMap<usize, usize>,
}

/// Ray this far below each dynamic body's origin (world units).
const BELOW_RAY: f32 = 1000.;

fn below_bodies(
    spatial: &SpatialQuery,
    bodies: &Query<(Entity, &ThingIndex, &RigidBody, &Position)>,
    colliders: &Query<(Entity, &ColliderOf, &ZSlab, &ThingIndex)>,
) -> BTreeMap<usize, &'static str> {
    let mut out = BTreeMap::new();
    for (e, t, rb, pos) in bodies {
        if *rb != RigidBody::Dynamic {
            continue;
        }
        let own: Vec<ZSlab> = colliders
            .iter()
            .filter(|c| c.1.body == e)
            .map(|c| *c.2)
            .collect();
        let mut best: Option<(f32, Entity)> = None;
        spatial.ray_hits_callback(
            pos.0,
            Dir2::NEG_Y,
            BELOW_RAY * lbp_sim::UNIT_SCALE,
            true,
            &SpatialQueryFilter::default(),
            |h| {
                if let Ok((_, of, slab, _)) = colliders.get(h.entity) {
                    if of.body != e
                        && own.iter().any(|s| s.overlaps(slab))
                        && best.is_none_or(|b| h.distance < b.0)
                    {
                        best = Some((h.distance, h.entity));
                    }
                }
                true
            },
        );
        let what = match best {
            None => "nothing within 1000",
            Some((_, c)) => {
                let (_, of, _, _) = colliders.get(c).expect("hit is a collider");
                match bodies.get(of.body).map(|b| *b.2) {
                    Ok(RigidBody::Dynamic) => "dynamic body",
                    _ => "static",
                }
            }
        };
        out.insert(t.0, what);
    }
    out
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
    let mut m = Motion::default();
    for tick in 1..=ticks {
        app.update();
        if tick == 1 {
            m.below = app
                .world_mut()
                .run_system_once(move |spatial: SpatialQuery,
                      bodies: Query<(Entity, &ThingIndex, &RigidBody, &Position)>,
                      colliders: Query<(Entity, &ColliderOf, &ZSlab, &ThingIndex)>| {
                    below_bodies(&spatial, &bodies, &colliders)
                })
                .unwrap_or_default();
        }
        let mut q = app
            .world_mut()
            .query_filtered::<(&ThingIndex, &Position, Has<Sleeping>), With<RigidBody>>();
        for (t, p, sleeping) in q.iter(app.world()) {
            if sleeping {
                m.asleep.insert(t.0);
            } else if m.asleep.contains(&t.0) {
                m.woke.entry(t.0).or_insert(tick);
            }
            if m.first_moved.contains_key(&t.0) {
                continue;
            }
            if let Some(s) = start.get(&t.0) {
                let p = to_level(p.0);
                if (p[0] - s[0]).powi(2) + (p[1] - s[1]).powi(2) > 1. {
                    m.first_moved.insert(t.0, tick);
                }
            }
        }
    }
    let mut q = app
        .world_mut()
        .query_filtered::<(&ThingIndex, &Position), With<RigidBody>>();
    for (t, p) in q.iter(app.world()) {
        let Some(s) = start.get(&t.0) else {
            continue;
        };
        m.bodies += 1;
        let p = to_level(p.0);
        let d = ((p[0] - s[0]).powi(2) + (p[1] - s[1]).powi(2)).sqrt();
        if !d.is_finite() {
            m.non_finite += 1;
            continue;
        }
        m.moved_1 += usize::from(d > 1.);
        m.moved_100 += usize::from(d > 100.);
        if d > 100. {
            m.moved.push((t.0, [p[0] - s[0], p[1] - s[1]]));
        }
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
            "usage: level_physics SNAPSHOT GAME-DIRS LEVEL-PATH|guid:HEX|--all NEW-JSON [--ticks=N]".into(),
        );
    }
    let output = Path::new(args[4]);
    if !output.starts_with("local-analysis") || output.exists() {
        return Err("output must be a new file under local-analysis/".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(args[2]), &catalog, &control)?;
    let mut paths = BTreeMap::<u32, String>::new();
    for r in &catalog.snapshot.resources {
        if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
            paths.entry(g).or_insert_with(|| r.map_row.path.clone());
        }
    }
    // Material rules by MAP path (INFERRED until RMaterial is read): dark matter and
    // infinite-mass materials are fixed, hologram does not collide.
    let rule_of_path = |p: &str| {
        let p = p.to_ascii_lowercase().replace(' ', "_");
        if !p.ends_with(".mat") {
            MaterialRule::Normal
        } else if p.contains("dark_matter")
            || p.contains("darkmatter")
            || p.contains("infinite_mass")
        {
            MaterialRule::Fixed
        } else if p.contains("hologram") {
            MaterialRule::NoCollide
        } else {
            MaterialRule::Normal
        }
    };
    let rules: BTreeMap<u32, MaterialRule> = paths
        .iter()
        .map(|(g, p)| (*g, rule_of_path(p)))
        .filter(|(_, r)| *r != MaterialRule::Normal)
        .collect();
    let fixed_guids: std::collections::BTreeSet<u32> = rules
        .iter()
        .filter(|(_, r)| **r == MaterialRule::Fixed)
        .map(|(g, _)| *g)
        .collect();
    let no_collide_guids: Vec<u32> = rules
        .iter()
        .filter(|(_, r)| **r == MaterialRule::NoCollide)
        .map(|(g, _)| *g)
        .collect();
    let fixed = |d: &lbp_level::serial::Descriptor| {
        d.guid
            .and_then(|g| rules.get(&g).copied())
            .unwrap_or_default()
    };
    let material_name = |d: &lbp_level::serial::Descriptor| match d.guid {
        Some(g) => paths.get(&g).cloned().unwrap_or_else(|| format!("g{g:x}")),
        None => "by hash".to_string(),
    };
    let mut materials = BTreeMap::<String, usize>::new();
    let (mut fixed_bodies, mut nested_bodies) = (0, 0);
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
            args[3]
                .strip_prefix("guid:")
                .and_then(|g| u32::from_str_radix(g, 16).ok()),
            (!args[3].starts_with("guid:")).then_some(args[3].as_str()),
        )?);
    }
    let mut levels = Vec::new();
    let mut total = Motion::default();
    let (mut shapes, mut dynamic, mut static_, mut not_collidable, mut not_read) = (0, 0, 0, 0, 0);
    let mut rejected = BTreeMap::<&str, usize>::new();
    let mut filled = BTreeMap::<&str, usize>::new();
    let (mut joints, mut rejected_joints) = (0, BTreeMap::<&str, usize>::new());
    let mut anchored = 0;
    let (mut touching_pairs, mut touching_pairs_dynamic) = (0usize, 0usize);
    // D1154: followers, and those whose parent's point is off their body's origin.
    let (mut followers, mut followers_offset) = (0usize, 0usize);
    // D1156: configuration-5 spin tweaks, and those off their body's origin.
    let (mut aimers, mut aimers_offset) = (0usize, 0usize);
    // D1157: tag synchronisers: senders, receivers, and those riding a body.
    let (mut senders, mut receivers, mut syncs_on_body) = (0usize, 0usize, 0usize);
    // D1158: stick tweaks (spin 4, move 6 and 7) and those with input port 1 or 2 wired.
    let (mut sticks, mut sticks_wired) = (0usize, 0usize);
    // D1159: spin and move tweaks with a turn on their microchip boards (+0x220), and flipped spins.
    let (mut chip_turned, mut chip_flipped) = (0usize, 0usize);
    let (mut move_mirrored, mut move_flipped) = (0usize, 0usize);
    let mut input_scaled = 0usize;
    let mut follow_cross = 0usize;
    let mut aim_lead = 0usize;
    let mut panicked = Vec::new();
    // Why bodies move: the same keys over every dynamic body and over those that moved >100.
    let mut why = BTreeMap::<&str, (BTreeMap<String, usize>, BTreeMap<String, usize>)>::new();
    let count = keys.len();
    for (n, (key, label)) in keys.into_iter().enumerate() {
        // Progress on stderr so a crash names the level it was in.
        eprintln!("level {}/{count}: {label}", n + 1);
        let read =
            lbp_preview::level_scene::read_level_bytes(&source, &key, &control).and_then(|b| {
                lbp_level::read(&b, lbp_level::Limits::default(), &|| false)
                    .map_err(|e| e.to_string())
            });
        let Ok(report) = read else {
            not_read += 1;
            continue;
        };
        for t in &report.things {
            if let Some(m) = t.data.shape.as_ref().and_then(|s| s.material.as_ref()) {
                *materials.entry(material_name(m)).or_default() += 1;
            }
        }
        let scene = build_scene_with(&report.things, &fixed);
        // D1153 census: collider pairs on different bodies whose xy boxes overlap and whose
        // depth ranges only touch (within SLAB_TOUCH_EPSILON), split by whether one is dynamic.
        let boxes: Vec<([f32; 2], [f32; 2])> = scene
            .colliders
            .iter()
            .map(|c| {
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                let (s, k) = c.frame.angle.sin_cos();
                for p in c.triangles.iter().flatten() {
                    let w = [
                        c.frame.position[0] + k * p[0] - s * p[1],
                        c.frame.position[1] + s * p[0] + k * p[1],
                    ];
                    for i in 0..2 {
                        lo[i] = lo[i].min(w[i]);
                        hi[i] = hi[i].max(w[i]);
                    }
                }
                (lo, hi)
            })
            .collect();
        let dynamic_of =
            |c: &lbp_sim::scene::SceneCollider| c.body.is_some_and(|b| !scene.bodies[b].fixed);
        let (mut touching, mut touching_dynamic) = (0usize, 0usize);
        for (i, a) in scene.colliders.iter().enumerate() {
            for (j, b) in scene.colliders.iter().enumerate().skip(i + 1) {
                let sum = a.half_thickness + b.half_thickness;
                let dz = (a.z - b.z).abs();
                if a.body.is_some() && a.body == b.body
                    || dz >= sum
                    || dz < sum - lbp_sim::SLAB_TOUCH_EPSILON
                {
                    continue;
                }
                let ((alo, ahi), (blo, bhi)) = (boxes[i], boxes[j]);
                if (0..2).all(|k| alo[k] <= bhi[k] && blo[k] <= ahi[k]) {
                    touching += 1;
                    touching_dynamic += usize::from(dynamic_of(a) || dynamic_of(b));
                }
            }
        }
        touching_pairs += touching;
        touching_pairs_dynamic += touching_dynamic;
        let (mut follow, mut follow_offset) = (0usize, 0usize);
        for m in scene
            .bodies
            .iter()
            .flat_map(|b| &b.moves)
            .filter(|m| m.configuration == 11)
        {
            follow += 1;
            follow_offset += usize::from(m.at[0].hypot(m.at[1]) > 1.);
        }
        followers += follow;
        followers_offset += follow_offset;
        let (mut aim, mut aim_offset) = (0usize, 0usize);
        for t in scene
            .bodies
            .iter()
            .flat_map(|b| &b.spins)
            .filter(|t| t.configuration == 5)
        {
            aim += 1;
            aim_offset += usize::from(t.at[0].hypot(t.at[1]) > 1.);
        }
        aimers += aim;
        aimers_offset += aim_offset;
        let send = scene.syncs.iter().filter(|y| y.kind == 0).count();
        let receive = scene.syncs.iter().filter(|y| y.kind == 1).count();
        let sync_body = scene.syncs.iter().filter(|y| y.body.is_some()).count();
        senders += send;
        receivers += receive;
        let ports: std::collections::BTreeSet<(usize, i32)> = report
            .things
            .iter()
            .filter_map(|t| t.data.switch.as_ref())
            .flat_map(|s| s.outputs.iter().flatten())
            .flat_map(|o| o.targets.iter().filter_map(|&(t, p)| Some((t?, p))))
            .collect();
        let stick_things: Vec<usize> = scene
            .bodies
            .iter()
            .flat_map(|b| {
                b.spins
                    .iter()
                    .filter(|t| t.configuration == 4)
                    .map(|t| t.thing)
                    .chain(
                        b.moves
                            .iter()
                            .filter(|m| matches!(m.configuration, 6 | 7))
                            .map(|m| m.thing),
                    )
            })
            .collect();
        let stick = stick_things.len();
        let stick_wired = stick_things
            .iter()
            .filter(|&&t| ports.contains(&(t, 1)) || ports.contains(&(t, 2)))
            .count();
        sticks += stick;
        sticks_wired += stick_wired;
        let turned = scene
            .bodies
            .iter()
            .map(|b| {
                b.spins.iter().filter(|t| t.angle != 0.).count()
                    + b.moves.iter().filter(|m| m.angle != 0.).count()
            })
            .sum::<usize>();
        let flipped = scene
            .bodies
            .iter()
            .map(|b| b.spins.iter().filter(|t| t.flip).count())
            .sum::<usize>();
        let moves = scene.bodies.iter().flat_map(|b| b.moves.iter());
        let mirrored = moves.clone().filter(|m| m.mirror).count();
        let move_flip = moves.filter(|m| m.flip).count();
        chip_turned += turned;
        chip_flipped += flipped;
        move_mirrored += mirrored;
        // D1161: tweaks whose input scale 0xcbed4 follows their input (behavior 2).
        let scaled = scene
            .bodies
            .iter()
            .map(|b| {
                b.spins.iter().filter(|t| t.behavior == 2).count()
                    + b.moves.iter().filter(|m| m.behavior == 2).count()
            })
            .sum::<usize>();
        input_scaled += scaled;
        // D1163: followers with decel × range 0, which pack row 2 only as followers.
        let cross = scene
            .bodies
            .iter()
            .flat_map(|b| &b.moves)
            .filter(|m| m.configuration == 11 && m.decel * m.range == 0.)
            .count();
        follow_cross += cross;
        // D1164: keyed aimers with a tag of their key on a dynamic body, which they lead.
        let led = scene
            .bodies
            .iter()
            .flat_map(|b| &b.spins)
            .filter(|t| t.configuration == 5)
            .filter(|t| {
                scene
                    .tags
                    .iter()
                    .any(|g| Some(g.key) == t.key && g.body.is_some_and(|b| !scene.bodies[b].fixed))
            })
            .count();
        aim_lead += led;
        move_flipped += move_flip;
        syncs_on_body += sync_body;
        let r = &scene.report;
        fixed_bodies += r.fixed_bodies;
        nested_bodies += r.nested_bodies;
        shapes += r.shapes;
        dynamic += r.dynamic;
        static_ += r.static_;
        not_collidable += r.not_collidable;
        for (k, n) in &r.rejected {
            *rejected.entry(k).or_default() += n;
        }
        for (k, n) in &r.filled {
            *filled.entry(k).or_default() += n;
        }
        joints += r.joints;
        anchored += r.anchored_joint_ends;
        for (k, n) in &r.rejected_joints {
            *rejected_joints.entry(k).or_default() += n;
        }
        let run =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| simulate(&scene, ticks)));
        let Ok(m) = run else {
            panicked.push(label.clone());
            continue;
        };
        let moved: BTreeMap<usize, [f32; 2]> = m.moved.iter().copied().collect();
        for (bi, b) in scene.bodies.iter().enumerate() {
            if b.fixed {
                continue;
            }
            let t = &report.things[b.thing];
            let shapes: Vec<_> = scene
                .colliders
                .iter()
                .filter(|c| c.body == Some(bi))
                .collect();
            if shapes.is_empty() {
                continue;
            }
            let material = report.things[shapes[0].thing]
                .data
                .shape
                .as_ref()
                .and_then(|s| s.material.as_ref())
                .map_or("none".to_string(), material_name);
            let jointed = scene
                .joints
                .iter()
                .any(|j| [j.a, j.b].contains(&lbp_sim::scene::Endpoint::Body(bi)));
            let first = match m.first_moved.get(&b.thing) {
                None => "never",
                Some(1) => "1",
                Some(2..=5) => "2-5",
                Some(6..=30) => "6-30",
                Some(_) => "31+",
            };
            let bucket = |t: usize| match t {
                0..=5 => "woke 1-5",
                6..=30 => "woke 6-30",
                _ => "woke 31+",
            };
            let sleep = match (m.asleep.contains(&b.thing), m.woke.get(&b.thing)) {
                (false, _) => "never asleep",
                (true, None) => "asleep to the end",
                (true, Some(&t)) => bucket(t),
            };
            let keys = [
                (
                    "below_at_start",
                    m.below
                        .get(&b.thing)
                        .copied()
                        .unwrap_or("not a body")
                        .to_string(),
                ),
                ("first_moved_tick", first.to_string()),
                ("sleep", sleep.to_string()),
                ("frozen", b.frozen.to_string()),
                ("material", material),
                ("parts", t.parts.join("+")),
                ("jointed", jointed.to_string()),
                ("shapes", shapes.len().min(5).to_string()),
                ("has_parent", t.parent.is_some().to_string()),
                (
                    "direction",
                    match moved.get(&b.thing) {
                        Some(d) if d[1] < -100. => "down",
                        Some(_) => "other",
                        None => "stayed",
                    }
                    .to_string(),
                ),
            ];
            for (k, v) in keys {
                let e = why.entry(k).or_default();
                *e.0.entry(v.clone()).or_default() += 1;
                if moved.contains_key(&b.thing) {
                    *e.1.entry(v).or_default() += 1;
                }
            }
        }
        total.bodies += m.bodies;
        total.moved_1 += m.moved_1;
        total.moved_100 += m.moved_100;
        total.non_finite += m.non_finite;
        total.max_move = total.max_move.max(m.max_move);
        levels.push(serde_json::json!({
            "level": label,
            "stopped": report.stop.is_some(),
            "shapes": r.shapes, "dynamic": r.dynamic, "static": r.static_,
            "not_collidable": r.not_collidable, "rejected": r.rejected, "filled": r.filled,
            "flat_samples": r.flat_samples,
            "joints": r.joints, "rejected_joints": r.rejected_joints,
            "anchored_joint_ends": r.anchored_joint_ends,
            "fixed_bodies": r.fixed_bodies, "nested_bodies": r.nested_bodies,
            "bodies": m.bodies, "moved_over_1": m.moved_1, "moved_over_100": m.moved_100,
            "non_finite": m.non_finite, "max_move": m.max_move,
            "touching_pairs": touching, "touching_pairs_dynamic": touching_dynamic,
            "followers": follow, "followers_offset": follow_offset,
            "aimers": aim, "aimers_offset": aim_offset,
            "senders": send, "receivers": receive, "syncs_on_body": sync_body,
            "sticks": stick, "sticks_wired": stick_wired,
            "chip_turned": turned, "chip_flipped": flipped,
            "move_mirrored": mirrored, "move_flipped": move_flip,
            "input_scaled": scaled, "follow_cross": cross, "aim_lead": led,
        }));
    }
    let mut top: Vec<(&String, &usize)> = materials.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    top.truncate(40);
    let fixed_paths: Vec<&String> = fixed_guids.iter().filter_map(|g| paths.get(g)).collect();
    let mut most_moved: Vec<(&str, u64)> = levels
        .iter()
        .filter_map(|l| Some((l["level"].as_str()?, l["moved_over_100"].as_u64()?)))
        .filter(|l| l.1 > 0)
        .collect();
    most_moved.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    most_moved.truncate(15);
    let doc = serde_json::json!({
        "levels_most_moved_top15": most_moved,
        "schema_version": 1,
        "unit": "SHAPE Thing (scene); BODY Thing with a collider (motion), world units",
        "ticks": ticks,
        "tick_hz": TICK_HZ,
        "levels": levels.len(),
        "levels_not_read": not_read,
        "shapes": shapes, "dynamic": dynamic, "static": static_,
        "not_collidable": not_collidable, "rejected": rejected, "filled": filled,
        "joints": joints, "rejected_joints": rejected_joints,
        "touching_pairs": touching_pairs, "touching_pairs_dynamic": touching_pairs_dynamic,
        "followers": followers, "followers_offset": followers_offset,
        "aimers": aimers, "aimers_offset": aimers_offset,
        "senders": senders, "receivers": receivers, "syncs_on_body": syncs_on_body,
        "sticks": sticks, "sticks_wired": sticks_wired,
        "chip": {
            "turned": chip_turned, "flipped": chip_flipped,
            "move_mirrored": move_mirrored, "move_flipped": move_flipped,
            "input_scaled": input_scaled, "follow_cross": follow_cross, "aim_lead": aim_lead,
        },
        "anchored_joint_ends": anchored,
        "fixed_bodies": fixed_bodies, "nested_bodies": nested_bodies,
        "fixed_material_paths": fixed_paths,
        "no_collide_material_paths": no_collide_guids.iter().filter_map(|g| paths.get(g)).collect::<Vec<_>>(),
        "levels_panicked": panicked,
        "why_moved": why.iter().map(|(k, (all, moved))| {
            let top = |m: &BTreeMap<String, usize>| {
                let mut v: Vec<_> = m.iter().collect();
                v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                v.truncate(20);
                v.into_iter().map(|(k, n)| serde_json::json!([k, n])).collect::<Vec<_>>()
            };
            (k.to_string(), serde_json::json!({"dynamic_bodies": top(all), "moved_over_100": top(moved)}))
        }).collect::<serde_json::Map<_, _>>(),
        "shape_materials_top40": top,
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
        "levels {} (not read {not_read}, panicked {}); shapes {shapes}: dynamic {dynamic}, static {static_}, \
         not collidable {not_collidable}, rejected {}, filled {}; joints {joints} (rejected {}, anchored ends {anchored}); \
         fixed bodies {fixed_bodies} ({} fixed materials), nested bodies {nested_bodies}; \
         after {ticks} ticks: bodies {}, moved >1 {}, >100 {}, non-finite {}, max move {:.1}",
        levels.len(),
        panicked.len(),
        rejected.values().sum::<usize>(),
        filled.values().sum::<usize>(),
        rejected_joints.values().sum::<usize>(),
        fixed_paths.len(),
        total.bodies,
        total.moved_1,
        total.moved_100,
        total.non_finite,
        total.max_move
    );
    Ok(())
}
