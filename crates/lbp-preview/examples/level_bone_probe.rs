//! Read-only probe (D120): characterizes RENDER_MESH Things without their own POS in one
//! level. Prints summary counts only (no game bytes). Usage:
//! level_bone_probe SNAPSHOT GAME-DIRS LEVEL-PATH [NEW-JOINTS-JSON]   (one level)
//! level_bone_probe SNAPSHOT GAME-DIRS --all                          (corpus summary)
//! NEW-JOINTS-JSON (ignored local-analysis/ only): per JOINT Thing, reader fields, contact
//! points and the D120 placement, for comparison with the LevelOracle scene dump.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::{collections::BTreeMap, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        return Err("level_bone_probe SNAPSHOT GAME-DIRS LEVEL-PATH|--all [NEW-JSON]".into());
    }
    let json_out = args.get(4).map(Path::new);
    if let Some(out) = json_out {
        let allowed = Path::new("local-analysis")
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let parent = out
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if out.exists() || !parent.starts_with(allowed) {
            return Err("new ignored output required".into());
        }
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    if args[3] == "--all" {
        return census(&catalog, &source, &control);
    }
    let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(&args[3]))?;
    let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let things = &report.things;
    let mut hist: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |k: String| *hist.entry(k).or_default() += 1;
    let mut mesh_cache: BTreeMap<u32, Option<std::rc::Rc<lbp_mesh::Geometry>>> = BTreeMap::new();
    let mut printed: BTreeMap<u32, usize> = BTreeMap::new();
    for t in things {
        let Some(rm) = &t.data.render_mesh else {
            continue;
        };
        let has_pos = t.data.pos.is_some();
        let guid = rm.mesh.as_ref().and_then(|d| d.guid);
        let geo = guid.and_then(|g| {
            mesh_cache
                .entry(g)
                .or_insert_with(|| {
                    lbp_preview::resolve(&catalog, Some(g), None)
                        .and_then(|k| {
                            lbp_preview::load_mesh(&source, &catalog, &k, false, &control)
                        })
                        .ok()
                        .map(|m| std::rc::Rc::new(m.geometry))
                })
                .clone()
        });
        let nb = geo.as_ref().map(|g| g.skeleton.bones.len());
        if has_pos {
            // Toolkit StaticMeshBuilder places bone 0 by POS x invSkinPose(bone 0); D113
            // draws POS alone. Count where that differs (bone-0 skin pose not identity).
            let off = geo
                .as_ref()
                .and_then(|g| g.skeleton.bones.first())
                .map(|b| {
                    let id = [
                        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
                    ];
                    b.skin_pose
                        .iter()
                        .zip(id)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0f32, f32::max)
                });
            if off.is_some_and(|o| o > 1e-3) && printed.insert(guid.unwrap_or(0), 0).is_none() {
                let g = geo.as_ref().unwrap();
                println!(
                    "POS thing {} mesh {:08x} bone0 '{}' skin {:?} POS {:?}",
                    t.index,
                    guid.unwrap_or(0),
                    g.skeleton.bones[0].name,
                    g.skeleton.bones[0].skin_pose,
                    t.data.pos.as_ref().unwrap().world
                );
            }
            add(format!(
                "pos: bone0 skin pose identity within 1e-3: {:?}",
                off.map(|o| o <= 1e-3)
            ));
        }
        let tag = if has_pos { "pos" } else { "nopos" };
        if !has_pos {
            if let Some(j) = &t.data.joint {
                let pa = j.a.and_then(|a| things[a].data.pos.as_ref());
                let pb = j.b.and_then(|b| things[b].data.pos.as_ref());
                add(format!(
                    "joint: guid={:08x} type={} bone_idx={:?} a_pos={} b_pos={} hide={} rs={}",
                    guid.unwrap_or(0),
                    j.joint_type,
                    j.bone_idx,
                    pa.is_some(),
                    pb.is_some(),
                    j.hide_in_play_mode,
                    j.render_scale
                ));
                let n = printed.entry(guid.unwrap_or(0)).or_insert(0usize);
                *n += 1;
                if *n <= 2 {
                    println!(
                        "JOINT thing {} guid {:08x} {:?}",
                        t.index,
                        guid.unwrap_or(0),
                        j
                    );
                    if let (Some(pa), Some(pb)) = (pa, pb) {
                        println!("  A world {:?}\n  B world {:?}", pa.world, pb.world);
                        let wa = lbp_preview::level_scene::transform_point(&pa.world, j.a_contact);
                        let wb = lbp_preview::level_scene::transform_point(&pb.world, j.b_contact);
                        let d = ((wa[0] - wb[0]).powi(2)
                            + (wa[1] - wb[1]).powi(2)
                            + (wa[2] - wb[2]).powi(2))
                        .sqrt();
                        println!("  A contact world {wa:?} B contact world {wb:?} dist {d}");
                        for (k, &bi) in j.bone_idx.iter().enumerate() {
                            let owner = if k == 0 { j.a } else { j.b };
                            if let Some(rm2) =
                                owner.and_then(|o| things[o].data.render_mesh.as_ref())
                            {
                                println!(
                                    "  endpoint {k} boneIdx {bi} owner bone_things {:?}",
                                    rm2.bone_things
                                );
                            }
                        }
                    }
                    if let Some(g) = &geo {
                        for (bi, bn) in g.skeleton.bones.iter().enumerate() {
                            println!(
                                "  bone {bi} '{}' parent {} skin {:?}",
                                bn.name, bn.parent, bn.skin_pose
                            );
                        }
                        let mut lo = [f32::INFINITY; 3];
                        let mut hi = [f32::NEG_INFINITY; 3];
                        for p in &g.positions {
                            for i in 0..3 {
                                lo[i] = lo[i].min(p[i]);
                                hi[i] = hi[i].max(p[i]);
                            }
                        }
                        println!("  mesh bounds {lo:?} {hi:?}");
                    }
                }
            } else {
                add("nopos without JOINT".into());
            }
            let in_world = things.iter().any(|w| {
                w.data
                    .world
                    .as_ref()
                    .is_some_and(|wd| wd.things.contains(&Some(t.index)))
            });
            let refs: Vec<String> = things
                .iter()
                .filter(|o| o.index != t.index)
                .flat_map(|o| {
                    let mut v = vec![];
                    if o.parent == Some(t.index) {
                        v.push(format!("parent-of:{}", o.parts.join("+")));
                    }
                    if o.group_head == Some(t.index) {
                        v.push("grouphead-of".to_string());
                    }
                    if o.data
                        .pos
                        .as_ref()
                        .is_some_and(|p| p.bone_of == Some(t.index))
                    {
                        v.push("bone_of".into());
                    }
                    if o.data
                        .render_mesh
                        .as_ref()
                        .is_some_and(|r| r.bone_things.contains(&Some(t.index)))
                    {
                        v.push("bone_thing".into());
                    }
                    if o.data
                        .group
                        .as_ref()
                        .is_some_and(|g| g.emitter == Some(t.index))
                    {
                        v.push("group-emitter".into());
                    }
                    if o.old_emitter == Some(t.index) {
                        v.push("old_emitter".into());
                    }
                    v
                })
                .collect();
            add(format!(
                "nopos detail: guid={:08x} parts={} gh={} emitter={} plan={} flags={} in_world={} refs={:?} anim={}",
                guid.unwrap_or(0),
                t.parts.join("+"),
                t.group_head.is_some(),
                t.old_emitter.is_some(),
                t.plan_guid.is_some(),
                t.flags,
                in_world,
                refs,
                rm.anim.is_some()
            ));
        }
        add(format!(
            "{tag}: bone_things={} mesh_bones={:?} parent={} parent_pos={}",
            rm.bone_things.len(),
            nb,
            t.parent.is_some(),
            t.parent.is_some_and(|p| things[p].data.pos.is_some())
        ));
        for (i, b) in rm.bone_things.iter().enumerate() {
            let Some(b) = b else {
                add(format!("{tag}: bone[{}] null", i.min(1)));
                continue;
            };
            let bt = &things[*b];
            let Some(bp) = &bt.data.pos else {
                add(format!("{tag}: bone[{}] no POS", i.min(1)));
                continue;
            };
            add(format!(
                "{tag}: bone[{}] self={} bone_of_self={} hash_match={:?} parts={}",
                i.min(1),
                *b == t.index,
                bp.bone_of == Some(t.index),
                geo.as_ref()
                    .and_then(|g| g.skeleton.bones.get(i))
                    .map(|bn| bn.anim_hash as i32 == bp.anim_hash),
                bt.parts.join("+")
            ));
            if !has_pos && i == 0 {
                if let Some(g) = &geo {
                    let bn = &g.skeleton.bones[0];
                    println!(
                        "thing {} uid {:?} guid {:08x} bones {} root parent {} skin {:?} inv {:?} bone0 world {:?}",
                        t.index,
                        t.uid,
                        guid.unwrap_or(0),
                        g.skeleton.bones.len(),
                        bn.parent,
                        bn.skin_pose,
                        bn.inv_skin_pose,
                        bp.world
                    );
                }
            }
        }
    }
    for (k, n) in hist {
        println!("{n:6}  {k}");
    }
    if let Some(out) = json_out {
        let mut rows = vec![];
        for t in things {
            let Some(j) = &t.data.joint else { continue };
            let contact = |e: Option<usize>, c: [f32; 3]| {
                e.and_then(|i| things[i].data.pos.as_ref())
                    .map(|p| lbp_preview::level_scene::transform_point(&p.world, c))
            };
            let geo = t
                .data
                .render_mesh
                .as_ref()
                .and_then(|rm| rm.mesh.as_ref())
                .and_then(|d| d.guid)
                .and_then(|g| mesh_cache.get(&g).cloned().flatten());
            let placement =
                (t.data.pos.is_none() && t.data.render_mesh.is_some()).then(|| match &geo {
                    None => serde_json::json!({"error": "mesh not loaded"}),
                    Some(g) => {
                        let bind: Vec<[f32; 16]> =
                            g.skeleton.bones.iter().map(|b| b.skin_pose).collect();
                        let mut lo = [f32::INFINITY; 3];
                        let mut hi = [f32::NEG_INFINITY; 3];
                        for p in &g.positions {
                            for i in 0..3 {
                                lo[i] = lo[i].min(p[i]);
                                hi[i] = hi[i].max(p[i]);
                            }
                        }
                        match lbp_preview::level_scene::joint_world(
                            things,
                            j,
                            &bind,
                            Some([lo, hi]),
                        ) {
                            Ok((m, k)) => serde_json::json!({"world": m, "kind": format!("{k:?}")}),
                            Err(e) => serde_json::json!({"error": e}),
                        }
                    }
                });
            rows.push(serde_json::json!({
                "tag": t.tag, "uid": t.uid,
                "a_tag": j.a.map(|i| things[i].tag), "b_tag": j.b.map(|i| things[i].tag),
                "a_contact": j.a_contact, "b_contact": j.b_contact,
                "length": j.length, "angle": j.angle,
                "settings_guid": j.settings.as_ref().and_then(|d| d.guid),
                "bone_idx": j.bone_idx, "bone_lengths": j.bone_lengths, "type": j.joint_type,
                "render_scale": j.render_scale, "hide_in_play_mode": j.hide_in_play_mode,
                "contact_a_world": contact(j.a, j.a_contact),
                "contact_b_world": contact(j.b, j.b_contact),
                "placement": placement,
            }));
        }
        let body =
            serde_json::json!({"schema": "d120-joint-rows-v1", "level": args[3], "joints": rows});
        let text = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out)
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut f, text.as_bytes()).map_err(|e| e.to_string())?;
        println!("joint rows: {}", rows.len());
    }
    Ok(())
}

/// Every in-band level: RENDER_MESH Things without POS, by JOINT presence/type, plus
/// contact-point geometry checks (summary counts only).
fn census(
    catalog: &lbp_catalog::Catalog,
    source: &AttachedSource,
    control: &LoadControl,
) -> Result<(), String> {
    let mut hashes = std::collections::BTreeSet::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "level" {
            hashes.insert(r.map_row.sha1.to_ascii_lowercase());
        }
    }
    let mut hist: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |k: String| *hist.entry(k).or_default() += 1;
    let mut worst_xy: BTreeMap<i32, f64> = BTreeMap::new();
    let mut levels = 0;
    for h in &hashes {
        let Ok(key) = lbp_preview::resolve(catalog, None, Some(h)) else {
            continue;
        };
        let Ok(bytes) = lbp_preview::level_scene::read_level_bytes(source, &key, control) else {
            continue;
        };
        let Ok(report) = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false) else {
            continue;
        };
        if report.stop.is_some() || report.things.is_empty() {
            continue;
        }
        levels += 1;
        let things = &report.things;
        for t in things {
            let Some(_rm) = &t.data.render_mesh else {
                continue;
            };
            if t.data.pos.is_some() {
                if t.data.joint.is_some() {
                    add("pos+joint".into());
                }
                continue;
            }
            let Some(j) = &t.data.joint else {
                let b0 = _rm.bone_things.first().copied().flatten();
                let b0pos = b0.and_then(|b| things[b].data.pos.as_ref());
                add(format!(
                    "nopos, no JOINT, parent={} parent_pos={} bones={} b0_is_self={} b0_is_parent={} b0_pos={} b0_bone_of_self={} b0_parts={} parts={} level={}",
                    t.parent.is_some(),
                    t.parent.is_some_and(|p| things[p].data.pos.is_some()),
                    _rm.bone_things.len(),
                    b0 == Some(t.index),
                    b0.is_some() && b0 == t.parent,
                    b0pos.is_some(),
                    b0pos.is_some_and(|p| p.bone_of == Some(t.index)),
                    b0.map_or(String::new(), |b| things[b].parts.join("+")),
                    t.parts.join("+"),
                    catalog.snapshot.resources.iter().find(|r| r.map_row.sha1.eq_ignore_ascii_case(h)).map_or("", |r| r.map_row.path.as_str())
                ));
                continue;
            };
            let pa = j.a.and_then(|a| things[a].data.pos.as_ref());
            let pb = j.b.and_then(|b| things[b].data.pos.as_ref());
            let (Some(pa), Some(pb)) = (pa, pb) else {
                add(format!("joint type {} endpoint without POS", j.joint_type));
                continue;
            };
            let wa = lbp_preview::level_scene::transform_point(&pa.world, j.a_contact);
            let wb = lbp_preview::level_scene::transform_point(&pb.world, j.b_contact);
            let dxy = ((wa[0] - wb[0]).powi(2) + (wa[1] - wb[1]).powi(2)).sqrt();
            let d = (dxy.powi(2) + (wa[2] - wb[2]).powi(2)).sqrt();
            let dz = (wa[2] - wb[2]).abs();
            let w = worst_xy.entry(j.joint_type).or_insert(0.);
            if j.joint_type >= 7 {
                *w = w.max(dxy);
                add(format!(
                    "type {} angular: dxy<1 {} dz {:.0}",
                    j.joint_type,
                    dxy < 1.,
                    dz
                ));
            } else {
                let rel = if j.length > 0. {
                    (d - f64::from(j.length)).abs() / f64::from(j.length)
                } else {
                    f64::NAN
                };
                add(format!(
                    "type {} linear: |dist-length|/length<1% {} <10% {} dz {:.0} boneIdx {:?}",
                    j.joint_type,
                    rel < 0.01,
                    rel < 0.1,
                    dz,
                    j.bone_idx
                ));
            }
        }
    }
    println!("levels read complete: {levels}");
    for (k, n) in hist {
        println!("{n:6}  {k}");
    }
    println!("angular max in-plane contact distance by type: {worst_xy:?}");
    Ok(())
}

fn main() {
    let r = std::thread::Builder::new()
        .stack_size(lbp_preview::level_scene::LEVEL_STACK_BYTES)
        .spawn(run)
        .expect("thread")
        .join()
        .expect("join");
    if let Err(e) = r {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
