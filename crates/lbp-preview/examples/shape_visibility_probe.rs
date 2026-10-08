//! Read-only probe (D153): classifies SHAPE Things that the level scene does not draw, per
//! level. Prints summary counts only (no game bytes). Usage:
//! shape_visibility_probe SNAPSHOT GAME-DIRS LEVEL-GUID-HEX [LEVEL-GUID-HEX ...]
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_level::shape_mesh;
use std::{collections::BTreeMap, path::Path};

fn guid(d: &Option<lbp_level::serial::Descriptor>) -> String {
    match d {
        None => "null".into(),
        Some(d) => match (d.guid, d.sha1) {
            (Some(g), _) => format!("g{g:08x}"),
            (None, Some(_)) => "sha1".into(),
            _ => "empty".into(),
        },
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        return Err("shape_visibility_probe SNAPSHOT GAME-DIRS GUID...".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    for g in &args[3..] {
        let g = u32::from_str_radix(g, 16).map_err(|e| e.to_string())?;
        let (key, name) = lbp_preview::level_scene::resolve_level(&catalog, Some(g), None)?;
        let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
        let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
            .map_err(|e| e.to_string())?;
        let things = &report.things;
        let mut hist: BTreeMap<String, usize> = BTreeMap::new();
        let mut detail: BTreeMap<String, usize> = BTreeMap::new();
        let (mut total, mut drawn) = (0, 0);
        for t in things {
            let Some(s) = &t.data.shape else { continue };
            total += 1;
            let r = shape_mesh::extrude_thing(t);
            let reason = match &r {
                Ok(_) => {
                    drawn += 1;
                    continue;
                }
                Err(e) => e.reason(),
            };
            let rm = t.data.render_mesh.is_some();
            let parent_rm = t
                .parent
                .and_then(|p| things.get(p))
                .is_some_and(|p| p.data.render_mesh.is_some());
            let root = |mut i: usize| {
                for _ in 0..64 {
                    match things[i].parent {
                        Some(p) if p < things.len() => i = p,
                        _ => break,
                    }
                }
                i
            };
            let my_root = root(t.index);
            let tree_rm = things.iter().any(|o| {
                o.index != t.index && o.data.render_mesh.is_some() && root(o.index) == my_root
            });
            let tree_gm = things.iter().any(|o| {
                o.index != t.index && o.data.generated_mesh.is_some() && root(o.index) == my_root
            });
            let class = if reason != "no_generated_mesh" {
                reason.to_string()
            } else if rm {
                "no_gm: own RENDER_MESH (collision hull)".into()
            } else if tree_rm {
                "no_gm: no mesh, same parent tree has RENDER_MESH".into()
            } else if tree_gm {
                "no_gm: no mesh, same parent tree has GENERATED_MESH only".into()
            } else if t.parent.is_none() {
                "no_gm: no mesh, root Thing, no mesh in tree".into()
            } else {
                "no_gm: no mesh, child, no mesh in tree".into()
            };
            *hist.entry(class.clone()).or_default() += 1;
            if rm || tree_rm {
                // Is the mesh that stands for this collider placeable (POS or JOINT)?
                let placeable = things.iter().any(|o| {
                    o.data.render_mesh.is_some()
                        && root(o.index) == my_root
                        && (o.data.pos.is_some() || o.data.joint.is_some())
                });
                *hist
                    .entry(format!("  (mesh in tree placeable: {placeable})"))
                    .or_default() += 1;
                continue;
            }
            let pos = t
                .data
                .pos
                .as_ref()
                .map(|p| [p.world[12], p.world[13], p.world[14]]);
            let joined = things.iter().any(|o| {
                o.data.joint.as_ref().is_some_and(|j| {
                    j.a.is_some_and(|a| root(a) == my_root)
                        || j.b.is_some_and(|b| root(b) == my_root)
                })
            });
            let near = pos.and_then(|p| {
                things
                    .iter()
                    .filter(|o| o.data.render_mesh.is_some())
                    .filter_map(|o| {
                        let q = o.data.pos.as_ref()?.world;
                        let d = ((q[12] - p[0]).powi(2) + (q[13] - p[1]).powi(2)).sqrt();
                        Some((d, o.plan_guid))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            });
            println!(
                "    collider thing {} verts {} thick {} pos {:?} joined {joined} nearest mesh {:?}",
                t.index, s.vertices.len(), s.thickness, pos, near
            );
            let plan_rm = t.plan_guid.is_some_and(|pg| {
                things
                    .iter()
                    .any(|o| o.plan_guid == Some(pg) && o.data.render_mesh.is_some())
            });
            let others: Vec<_> = t
                .parts
                .iter()
                .filter(|p| !matches!(**p, "POS" | "SHAPE" | "BODY"))
                .copied()
                .collect();
            *detail
                .entry(format!(
                    "{class} | plan_rm {plan_rm} | mat {} | beh {:#x} flags {:#x} | parent_rm {parent_rm} | plan {} | parts {}",
                    guid(&s.material),
                    s.behavior,
                    s.flags,
                    t.plan_guid.map(|p| format!("{p:08x}")).unwrap_or("-".into()),
                    others.join(",")
                ))
                .or_default() += 1;
        }
        println!("== {g:08x} {name}: SHAPE {total}, drawn {drawn}");
        for (k, n) in &hist {
            println!("  {n:5}  {k}");
        }
        println!("  -- detail");
        let mut d: Vec<_> = detail.into_iter().collect();
        d.sort_by_key(|a| std::cmp::Reverse(a.1));
        for (k, n) in d.iter().take(25) {
            println!("  {n:5}  {k}");
        }
    }
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
