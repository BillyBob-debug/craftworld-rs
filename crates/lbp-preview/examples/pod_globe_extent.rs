//! Read-only probe (P2m): a level model's world-space extent under two placement rules,
//! to test whether the Pod globe's size gap comes from placing every vertex by bone 0.
//! Prints summary numbers only (no game bytes). Usage:
//! pod_globe_extent SNAPSHOT GAME-DIRS LEVEL-PATH THING-INDEX
//! - "bone 0": every vertex at POS(Thing) x invSkinPose(bone 0) (D120, what we draw);
//! - "per bone": each vertex at POS(boneThings[j]) x invSkinPose(j) for its strongest
//!   joint j (rigid skinning; INFERRED to be what the game draws for boneThings).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::skinned::mul;
use std::path::Path;

type Mat4 = [f32; 16];

fn point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}

fn axis_scales(m: &Mat4) -> [f32; 3] {
    std::array::from_fn(|c| (0..3).map(|r| m[c * 4 + r].powi(2)).sum::<f32>().sqrt())
}

struct Extent {
    lo: [f32; 3],
    hi: [f32; 3],
}

impl Extent {
    fn new() -> Self {
        Self {
            lo: [f32::INFINITY; 3],
            hi: [f32::NEG_INFINITY; 3],
        }
    }
    fn add(&mut self, p: [f32; 3]) {
        for (k, v) in p.into_iter().enumerate() {
            self.lo[k] = self.lo[k].min(v);
            self.hi[k] = self.hi[k].max(v);
        }
    }
    fn centre(&self) -> [f32; 3] {
        std::array::from_fn(|k| (self.lo[k] + self.hi[k]) / 2.)
    }
    fn half(&self) -> [f32; 3] {
        std::array::from_fn(|k| (self.hi[k] - self.lo[k]) / 2.)
    }
}

fn summary(name: &str, points: &[[f32; 3]]) {
    let mut e = Extent::new();
    points.iter().for_each(|&p| e.add(p));
    let c = e.centre();
    let d: Vec<f32> = points
        .iter()
        .map(|p| (0..3).map(|k| (p[k] - c[k]).powi(2)).sum::<f32>().sqrt())
        .collect();
    let max = d.iter().copied().fold(0f32, f32::max);
    let mean = d.iter().sum::<f32>() / d.len().max(1) as f32;
    let h = e.half();
    println!(
        "{name}: world half-extent ({:.1}, {:.1}, {:.1}), centre ({:.1}, {:.1}, {:.1}), radius from centre max {max:.1} mean {mean:.1}",
        h[0], h[1], h[2], c[0], c[1], c[2]
    );
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("pod_globe_extent SNAPSHOT GAME-DIRS LEVEL-PATH THING-INDEX".into());
    }
    let index: usize = args[4].parse().map_err(|_| "THING-INDEX: a number")?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, None, Some(&args[3]))?;
    let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let things = &report.things;
    let thing = things
        .iter()
        .find(|t| t.index == index)
        .ok_or("no Thing with that index")?;
    let rm = thing
        .data
        .render_mesh
        .as_ref()
        .ok_or("Thing has no RENDER_MESH")?;
    let pos_of = |i: usize| {
        things
            .iter()
            .find(|t| t.index == i)
            .and_then(|t| t.data.pos.as_ref())
            .map(|p| p.world)
    };
    let own = thing.data.pos.as_ref().ok_or("Thing has no POS")?.world;
    let guid = rm
        .mesh
        .as_ref()
        .and_then(|d| d.guid)
        .ok_or("RENDER_MESH has no GUID")?;
    let mesh_key = lbp_preview::resolve(&catalog, Some(guid), None)?;
    let (geo, skin) = lbp_preview::level_scene::read_mesh_skin(&source, &mesh_key, &control)?;
    let bones = &geo.skeleton.bones;
    let b0 = bones.first().ok_or("mesh has no bones")?;
    println!(
        "Thing {index}: mesh {guid:08x}, {} vertices, {} bones, boneThings {:?}",
        geo.positions.len(),
        bones.len(),
        rm.bone_things
    );
    println!("  POS axis scales {:?}", axis_scales(&own));
    let per_bone: Vec<Option<Mat4>> = bones
        .iter()
        .enumerate()
        .map(|(j, b)| {
            let world = rm.bone_things.get(j).copied().flatten().and_then(pos_of)?;
            let m = mul(&world, &b.inv_skin_pose);
            println!(
                "  bone {j} {}: boneThing POS scales {:?}, invSkinPose scales {:?}, product {:?}",
                b.name,
                axis_scales(&world),
                axis_scales(&b.inv_skin_pose),
                axis_scales(&m)
            );
            Some(m)
        })
        .collect();
    let rule0 = mul(&own, &b0.inv_skin_pose);
    let model: Vec<[f32; 3]> = geo.positions.clone();
    let (mut a, mut b, mut missing) = (Vec::new(), Vec::new(), 0usize);
    for (i, &p) in model.iter().enumerate() {
        a.push(point(&rule0, p));
        let j = skin
            .joints
            .get(i)
            .zip(skin.weights.get(i))
            .map(|(js, ws)| {
                let k = (0..4).max_by(|&x, &y| ws[x].total_cmp(&ws[y])).unwrap_or(0);
                js[k] as usize
            })
            .unwrap_or(0);
        match per_bone.get(j).copied().flatten() {
            Some(m) => b.push(point(&m, p)),
            None => {
                missing += 1;
                b.push(point(&rule0, p));
            }
        }
    }
    summary("model units (raw)", &model);
    summary("bone 0 rule (drawn)", &a);
    summary("per bone rule", &b);
    println!("  vertices without a bone Thing (kept on bone 0 rule): {missing}");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
