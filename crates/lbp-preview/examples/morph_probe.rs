//! Read-only probe (D700): each morph target of one skinned mesh with its name, the
//! vertices it moves and its weight in frame 0 of one clip. Numbers and names only.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::skinned;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("morph_probe SNAPSHOT GAME-DIR MESH-GUID ANIMATION-GUID".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control)?;
    let guid = |s: &str| u32::from_str_radix(s, 16).map_err(|e| e.to_string());
    let mesh = lbp_preview::resolve(&catalog, Some(guid(&args[3])?), None)?;
    let anim = skinned::resolve_animation(&catalog, guid(&args[4])?)?;
    let clip = skinned::load_clip(&source, &mesh, &anim, &control)?;
    let pose = skinned::sample_pose(&clip.animation, 0.).ok_or("no pose")?;
    for (i, m) in clip.skin.morphs.iter().enumerate() {
        let moved = m
            .offsets
            .iter()
            .filter(|o| o.iter().any(|v| v.abs() > 1e-4))
            .count();
        let largest = m
            .offsets
            .iter()
            .map(|o| (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt())
            .fold(0f32, f32::max);
        println!(
            "morph {i:2} {:<28} moves {moved:4} max {largest:7.2} weight {:?}",
            m.name,
            pose.morphs.get(i)
        );
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
