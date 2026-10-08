//! Read-only probe (D152): lists DECORATIONS entries of one level with their owner
//! Thing's POS translation and the stored offset, so the offset frame can be checked.
//! Prints numbers only (no game bytes). Usage:
//! decoration_probe SNAPSHOT GAME-DIRS LEVEL-GUID-HEX [MAX-LINES]
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::path::Path;

fn norm(m: &[f32; 16], c: usize) -> f32 {
    (m[c * 4].powi(2) + m[c * 4 + 1].powi(2) + m[c * 4 + 2].powi(2)).sqrt()
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        return Err("decoration_probe SNAPSHOT GAME-DIRS LEVEL-GUID-HEX [MAX-LINES]".into());
    }
    let max: usize = args
        .get(4)
        .map_or(Ok(40), |s| s.parse())
        .map_err(|e| format!("{e}"))?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let guid =
        u32::from_str_radix(args[3].trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
    let (key, _) = lbp_preview::level_scene::resolve_level(&catalog, Some(guid), None)?;
    let bytes = lbp_preview::level_scene::read_level_bytes(&source, &key, &control)?;
    let report = lbp_level::read(&bytes, lbp_level::Limits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let (mut total, mut decals, mut printed) = (0usize, 0usize, 0usize);
    for t in &report.things {
        decals += t.data.sticker_decals;
        for d in &t.data.decorations {
            total += 1;
            if printed >= max {
                continue;
            }
            printed += 1;
            let p = t
                .data
                .pos
                .as_ref()
                .map(|p| [p.world[12], p.world[13], p.world[14]]);
            let o = &d.offset;
            let mesh = d
                .render_mesh
                .as_ref()
                .and_then(|r| r.mesh.as_ref())
                .and_then(|m| m.guid);
            println!(
                "thing {} rm={} shape={} pos={:?} off_t=[{:.1},{:.1},{:.1}] off_scale=[{:.3},{:.3},{:.3}] mesh={:?} bone={} tri={} bary=({:.3},{:.3}) angle={:.3} scale={:.3} costume={} earth={} rev={} placed={}",
                t.index,
                t.data.render_mesh.is_some(),
                t.data.shape.is_some(),
                p.map(|p| [p[0].round(), p[1].round(), p[2].round()]),
                o[12],
                o[13],
                o[14],
                norm(o, 0),
                norm(o, 1),
                norm(o, 2),
                mesh,
                d.parent_bone,
                d.parent_tri_vert,
                d.bary_u,
                d.bary_v,
                d.angle,
                d.scale,
                d.on_costume_piece,
                d.earth_decoration,
                d.reversed,
                d.placed_by,
            );
        }
    }
    println!("decorations {total}, sticker decals {decals}");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
