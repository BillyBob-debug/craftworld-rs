//! Read-only probe (D702): per primitive of one skinned mesh, its material, costume region
//! and vertex bounds in the bind pose and in frame 0 of one clip, with each side of the
//! eye material and the head's width at eye height. Numbers only. Usage:
//! eye_probe SNAPSHOT GAME-DIRS MESH-GUID ANIMATION-GUID EYE-MATERIAL-GUID
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::skinned;
use std::path::Path;

type Bounds = ([f32; 3], [f32; 3]);

fn bounds<'a>(points: impl Iterator<Item = &'a [f32; 3]>) -> Option<Bounds> {
    let mut b: Option<Bounds> = None;
    for p in points {
        let (lo, hi) = b.get_or_insert((*p, *p));
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    b
}

fn show(label: &str, b: Option<Bounds>) {
    if let Some((lo, hi)) = b {
        let size: Vec<_> = (0..3).map(|k| hi[k] - lo[k]).collect();
        let mid: Vec<_> = (0..3).map(|k| (hi[k] + lo[k]) / 2.).collect();
        println!("  {label:<10} size {size:7.2?} centre {mid:7.2?}");
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("eye_probe SNAPSHOT GAME-DIRS MESH-GUID ANIMATION-GUID EYE-MATERIAL".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let guid = |s: &str| u32::from_str_radix(s, 16).map_err(|e| e.to_string());
    let key = lbp_preview::resolve(&catalog, Some(guid(&args[3])?), None)?;
    let grouped = lbp_preview::load_mesh_groups(&source, &catalog, &key, &control)?;
    let anim = skinned::resolve_animation(&catalog, guid(&args[4])?)?;
    let clip = skinned::load_clip(&source, &key, &anim, &control)?;
    let eye = guid(&args[5])?;
    let posed = clip.frame_positions(0).ok_or("no pose")?;
    let g = &grouped.geometry;
    let mut eye_verts = Vec::new();
    for (i, (p, span)) in g
        .metadata
        .primitives
        .iter()
        .zip(&g.primitive_triangles)
        .enumerate()
    {
        let mut verts: Vec<usize> = g.triangles[span.clone()]
            .iter()
            .flatten()
            .map(|&v| v as usize)
            .collect();
        verts.sort_unstable();
        verts.dedup();
        println!(
            "prim {i}: material {:?} region {:#x} verts {}",
            p.material_guid.map(|g| format!("{g:08x}")),
            p.region,
            verts.len()
        );
        show("bind", bounds(verts.iter().map(|&v| &g.positions[v])));
        show(
            "frame 0",
            bounds(verts.iter().filter_map(|&v| posed.get(v))),
        );
        if p.material_guid == Some(eye) {
            eye_verts.extend(verts);
        }
    }
    for (side, label) in [(true, "eye x>0"), (false, "eye x<0")] {
        let pick: Vec<_> = eye_verts
            .iter()
            .filter(|&&v| (g.positions[v][0] > 0.) == side)
            .copied()
            .collect();
        println!("{label}: {} verts", pick.len());
        show("bind", bounds(pick.iter().map(|&v| &g.positions[v])));
        show("frame 0", bounds(pick.iter().filter_map(|&v| posed.get(v))));
    }
    // Each morph's reach on the eye vertices: count moved and the largest offset per axis.
    for m in &clip.skin.morphs {
        let mut big = [0f32; 3];
        let mut moved = 0;
        for &v in &eye_verts {
            let o = m.offsets.get(v).copied().unwrap_or_default();
            if o.iter().any(|c| c.abs() > 1e-4) {
                moved += 1;
            }
            for k in 0..3 {
                big[k] = big[k].max(o[k].abs());
            }
        }
        if moved > 0 {
            println!("morph {:<16} eye verts {moved:3} max {big:6.2?}", m.name);
        }
    }
    // The bones the eye vertices follow, with their chain's local scale in frame 0.
    let pose = clip.frame_pose(0).ok_or("no pose")?;
    let locals = clip.rig.pose_locals(Some(&pose));
    let worlds = clip.rig.worlds_from_trs(&locals);
    let mut used = std::collections::BTreeSet::new();
    for &v in &eye_verts {
        for (j, w) in clip.skin.joints[v].iter().zip(&clip.skin.weights[v]) {
            if *w > 0.01 {
                used.insert(*j as usize);
            }
        }
    }
    let bones = &g.skeleton.bones;
    let mut chain = std::collections::BTreeSet::new();
    for &b in &used {
        let mut i = b as i32;
        while i >= 0 {
            chain.insert(i as usize);
            i = bones[i as usize].parent;
        }
    }
    for &i in &chain {
        let len = |m: &[f32; 16], c: usize| {
            (m[4 * c].powi(2) + m[4 * c + 1].powi(2) + m[4 * c + 2].powi(2)).sqrt()
        };
        let (m, b) = (&worlds[i], &bones[i].skin_pose);
        println!(
            "bone {i:3} {:<22} parent {:3} eye {} local s {:5.3?} world s [{:.3}, {:.3}, {:.3}] bind s [{:.3}, {:.3}, {:.3}]",
            bones[i].name,
            bones[i].parent,
            used.contains(&i),
            locals[i].s,
            len(m, 0),
            len(m, 1),
            len(m, 2),
            len(b, 0),
            len(b, 1),
            len(b, 2)
        );
    }
    // Head width at eye height: every vertex within the eyes' bind y band.
    if let Some((lo, hi)) = bounds(eye_verts.iter().map(|&v| &g.positions[v])) {
        let band: Vec<_> = (0..g.positions.len())
            .filter(|&v| (lo[1]..=hi[1]).contains(&g.positions[v][1]))
            .collect();
        println!("head band ({} verts)", band.len());
        show("bind", bounds(band.iter().map(|&v| &g.positions[v])));
        show("frame 0", bounds(band.iter().filter_map(|&v| posed.get(v))));
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
