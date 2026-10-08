//! Read-only Rd probe: binds one animation to one skinned mesh and prints the binding
//! report plus per-frame bounds and largest vertex displacement from the bind pose.
//! A headless check before opening the viewer; prints counts only, no game bytes.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::skinned;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("skinned_clip_probe SNAPSHOT GAME-DIR MESH-GUID ANIMATION-GUID".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control)?;
    let guid = |s: &str| u32::from_str_radix(s, 16).map_err(|e| e.to_string());
    let mesh = lbp_preview::resolve(&catalog, Some(guid(&args[3])?), None)?;
    let anim = skinned::resolve_animation(&catalog, guid(&args[4])?)?;
    let clip = skinned::load_clip(&source, &mesh, &anim, &control)?;
    println!("{:?}", clip.report);
    let frames = i64::from(clip.report.frames.max(1));
    for frame in (0..frames).step_by((frames as usize / 8).max(1)) {
        let posed = clip.frame_positions(frame).ok_or("no pose")?;
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        let mut moved = 0f32;
        let mut nonfinite = 0usize;
        for (p, b) in posed.iter().zip(&clip.bind_positions) {
            if p.iter().any(|v| !v.is_finite()) {
                nonfinite += 1;
                continue;
            }
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            moved = moved.max((0..3).map(|i| (p[i] - b[i]).powi(2)).sum::<f32>().sqrt());
        }
        println!(
            "frame {frame}: bounds {lo:?}..{hi:?}; max displacement {moved}; nonfinite {nonfinite}"
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
