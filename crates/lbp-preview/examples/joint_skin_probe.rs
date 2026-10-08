//! Read-only probe (D147): bone layout and skin weights of the meshes JOINT Things draw
//! in one level. Prints summary numbers only (no game bytes). Usage:
//! joint_skin_probe SNAPSHOT GAME-DIRS LEVEL-PATH
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::{collections::BTreeMap, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("joint_skin_probe SNAPSHOT GAME-DIRS LEVEL-PATH".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(&args[3]))?;
    let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let things = &report.things;
    let mut per_mesh: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for t in things {
        let (Some(rm), Some(j), None) = (&t.data.render_mesh, &t.data.joint, &t.data.pos) else {
            continue;
        };
        let Some(g) = rm.mesh.as_ref().and_then(|d| d.guid) else {
            continue;
        };
        let pa = j.a.and_then(|a| things[a].data.pos.as_ref());
        let pb = j.b.and_then(|b| things[b].data.pos.as_ref());
        let (Some(pa), Some(pb)) = (pa, pb) else {
            continue;
        };
        let wa = lbp_preview::level_scene::transform_point(&pa.world, j.a_contact);
        let wb = lbp_preview::level_scene::transform_point(&pb.world, j.b_contact);
        let d = (0..3).map(|i| (wa[i] - wb[i]).powi(2)).sum::<f64>().sqrt();
        let rows = per_mesh.entry(g).or_default();
        if rows.len() < 4 {
            rows.push(format!(
                "    type {} len {:.1} dist {:.1} d {:.0?} boneIdx {:?} boneLengths {:?} rs {} angle {}",
                j.joint_type,
                j.length,
                d,
                [wb[0] - wa[0], wb[1] - wa[1], wb[2] - wa[2]],
                j.bone_idx,
                j.bone_lengths,
                j.render_scale,
                j.angle
            ));
        }
    }
    for (g, rows) in per_mesh {
        let key = lbp_preview::resolve(&catalog, Some(g), None)?;
        let (geo, skin) = lbp_preview::level_scene::read_mesh_skin(&source, &key, &control)?;
        println!(
            "mesh {g:08x}: {} vertices, {} bones",
            geo.positions.len(),
            geo.skeleton.bones.len()
        );
        for (i, b) in geo.skeleton.bones.iter().enumerate() {
            let s = b.skin_pose;
            let col =
                |c: usize| (s[4 * c].powi(2) + s[4 * c + 1].powi(2) + s[4 * c + 2].powi(2)).sqrt();
            println!(
                "  bone {i} '{}' parent {} t [{:.2}, {:.2}, {:.2}] scale [{:.3}, {:.3}, {:.3}] x [{:.2},{:.2},{:.2}] y [{:.2},{:.2},{:.2}]",
                b.name,
                b.parent,
                s[12],
                s[13],
                s[14],
                col(0),
                col(1),
                col(2),
                s[0] / col(0),
                s[1] / col(0),
                s[2] / col(0),
                s[4] / col(1),
                s[5] / col(1),
                s[6] / col(1)
            );
        }
        // Per weight combination: vertex count and model-space bounds.
        let mut dom: BTreeMap<String, (usize, [f32; 3], [f32; 3])> = BTreeMap::new();
        for (v, (jt, w)) in geo
            .positions
            .iter()
            .zip(skin.joints.iter().zip(&skin.weights))
        {
            let mut used: Vec<(u8, i32)> = (0..4)
                .filter(|&k| w[k] > 0.)
                .map(|k| (jt[k], (w[k] * 100.).round() as i32))
                .collect();
            used.sort();
            let e = dom
                .entry(format!("{used:?}"))
                .or_insert((0, [f32::MAX; 3], [f32::MIN; 3]));
            e.0 += 1;
            for (c, &x) in v.iter().enumerate() {
                e.1[c] = e.1[c].min(x);
                e.2[c] = e.2[c].max(x);
            }
        }
        let mut list: Vec<_> = dom.into_iter().collect();
        list.sort_by_key(|(_, e)| std::cmp::Reverse(e.0));
        for (k, (n, lo, hi)) in list.iter().take(8) {
            println!("  weights {k}: {n} verts, lo {lo:.1?} hi {hi:.1?}");
        }
        println!("  ({} weight combos)", list.len());
        for r in rows {
            println!("{r}");
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
