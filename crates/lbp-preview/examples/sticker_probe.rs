//! Read-only probe (D160): lists STICKERS decals of one level with their owner's kind
//! (shape / render mesh), the owner's world-space front-face extent and the shape's
//! planar material UV range, so the decal UV frame can be checked. Prints numbers only
//! (no game bytes). Usage: sticker_probe SNAPSHOT GAME-DIRS LEVEL-GUID-HEX [MAX-LINES]
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        return Err("sticker_probe SNAPSHOT GAME-DIRS LEVEL-GUID-HEX [MAX-LINES]".into());
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
    let (mut total, mut main, mut printed) = (0usize, 0usize, 0usize);
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    let mut uv_min = [f32::MAX; 2];
    let mut uv_max = [f32::MIN; 2];
    let mut checks = [0usize; 4];
    for t in &report.things {
        total += t.data.sticker_decals;
        if t.data.stickers.is_empty() {
            continue;
        }
        main += t.data.stickers.len();
        let kind = match (
            t.data.shape.is_some(),
            t.data.generated_mesh.is_some(),
            t.data.render_mesh.is_some(),
        ) {
            (true, true, _) => "shape+generated",
            (true, false, true) => "shape+render_mesh",
            (true, false, false) => "shape only",
            (false, _, true) => "render_mesh",
            _ => "other",
        };
        *kinds.entry(kind.into()).or_default() += t.data.stickers.len();
        if kind == "other" {
            *kinds
                .entry(format!("other parts {:?}", t.parts))
                .or_default() += t.data.stickers.len();
        }
        for d in &t.data.stickers {
            for (i, x) in [d.u, d.v].into_iter().enumerate() {
                uv_min[i] = uv_min[i].min(x);
                uv_max[i] = uv_max[i].max(x);
            }
        }
        // Frame check: candidate maps of the decal centre back to local polygon xy
        // (x = u / (|sx| K), y = +-(v - 1) / (|sy| K), K = 1/500) inside the local bbox.
        if let (Some(s), Some(p)) = (&t.data.shape, &t.data.pos) {
            let n = |c: usize| {
                (p.world[c * 4].powi(2) + p.world[c * 4 + 1].powi(2) + p.world[c * 4 + 2].powi(2))
                    .sqrt()
            };
            let (sx, sy) = (n(0) / 500.0, n(1) / 500.0);
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for v in &s.vertices {
                for i in 0..2 {
                    lo[i] = lo[i].min(v[i]);
                    hi[i] = hi[i].max(v[i]);
                }
            }
            let pad = [(hi[0] - lo[0]) * 0.05, (hi[1] - lo[1]) * 0.05];
            let inside = |x: f32, y: f32| {
                x >= lo[0] - pad[0]
                    && x <= hi[0] + pad[0]
                    && y >= lo[1] - pad[1]
                    && y <= hi[1] + pad[1]
            };
            for d in &t.data.stickers {
                let x = d.u / sx;
                checks[0] += 1;
                checks[1] += usize::from(inside(x, (1.0 - d.v) / sy));
                checks[2] += usize::from(inside(x, (d.v - 1.0) / sy));
                checks[3] += usize::from(inside(x, -d.v / sy));
            }
        }
        if printed >= max {
            continue;
        }
        printed += 1;
        // Shape: local vertex bbox, world bbox of the front face, planar UV range.
        let mut extra = String::new();
        if let (Some(s), Some(p)) = (&t.data.shape, &t.data.pos) {
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            let (mut wlo, mut whi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for v in &s.vertices {
                let w = lbp_preview::level_scene::transform_point(&p.world, [v[0], v[1], 0.0]);
                for i in 0..2 {
                    lo[i] = lo[i].min(v[i]);
                    hi[i] = hi[i].max(v[i]);
                    wlo[i] = wlo[i].min(w[i] as f32);
                    whi[i] = whi[i].max(w[i] as f32);
                }
            }
            extra = format!(
                " local=[{:.0},{:.0}]..[{:.0},{:.0}] world=[{:.0},{:.0}]..[{:.0},{:.0}] thick={:.0} uvoff={:?}",
                lo[0],
                lo[1],
                hi[0],
                hi[1],
                wlo[0],
                wlo[1],
                whi[0],
                whi[1],
                s.thickness,
                t.data.generated_mesh.as_ref().map(|g| g.uv_offset)
            );
        }
        println!(
            "thing {} {kind} decals={} mesh={:?}{extra}",
            t.index,
            t.data.stickers.len(),
            t.data
                .render_mesh
                .as_ref()
                .and_then(|r| r.mesh.as_ref())
                .and_then(|m| m.guid)
                .map(|g| format!("{g:08x}")),
        );
        for d in t.data.stickers.iter().take(4) {
            println!(
                "  tex={:?} uv=({:.4},{:.4}) x=({:.4},{:.4}) y=({:.4},{:.4}) color={:#06x} kind={} placed={} scorch={}",
                d.texture.as_ref().and_then(|t| t.guid).map(|g| format!("{g:08x}")),
                d.u,
                d.v,
                d.xvec[0],
                d.xvec[1],
                d.yvec[0],
                d.yvec[1],
                d.color as u16,
                d.kind,
                d.placed_by,
                d.scorch_mark
            );
        }
    }
    println!(
        "sticker decals {total} (main arrays {main}); main by owner kind {kinds:?}; centre u {:.4}..{:.4} v {:.4}..{:.4}",
        uv_min[0], uv_max[0], uv_min[1], uv_max[1]
    );
    println!(
        "shape decal centres inside the owner's local bbox (+5%) of {}: v=1-y {} / v=1+y {} / v=-y {}",
        checks[0], checks[1], checks[2], checks[3]
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
