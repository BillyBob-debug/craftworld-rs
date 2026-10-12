//! Read-only probe (D239): finds 64x64 A8R8G8B8 textures with the Popit tool-icon layout
//! (soft field in R, marker glyphs in B) and writes their R and B channels as raw bytes.
//! Usage: tool_icon_scan SNAPSHOT GAME-DIRS OUT-DIR
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::{decode_icon, ResourceKey};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("tool_icon_scan SNAPSHOT GAME-DIRS OUT-DIR");
        std::process::exit(2);
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control).unwrap();
    let source =
        AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control).unwrap();
    std::fs::create_dir_all(&args[3]).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut hits = 0;
    for r in &catalog.snapshot.resources {
        if r.display_type() != "texture" || !seen.insert(r.map_row.sha1.clone()) {
            continue;
        }
        let Ok(guid) = u32::from_str_radix(r.map_row.guid.trim_start_matches("0x"), 16) else {
            continue;
        };
        let Ok(loc) = lbp_preview::story::resolve_key(&catalog, ResourceKey::Guid(guid)) else {
            continue;
        };
        if loc.size > 120_000 {
            continue;
        }
        let Ok(i) = decode_icon(&source, &catalog, ResourceKey::Guid(guid), &control) else {
            continue;
        };
        if i.width != i.height || i.width > 256 || i.width < 32 {
            continue;
        }
        let px = i.rgba.chunks(4);
        let rmax = px.clone().map(|p| p[0]).max().unwrap_or(0);
        let bmax = px.clone().map(|p| p[2]).max().unwrap_or(0);
        let gmax = px.clone().map(|p| p[1]).max().unwrap_or(0);
        let amin = px.map(|p| p[3]).min().unwrap_or(0);
        if (20..=130).contains(&rmax) && (6..=80).contains(&bmax) && amin == 255 && gmax > 0 {
            std::fs::write(format!("{}/{guid:x}_{}.rgba", args[3], i.width), &i.rgba).unwrap();
            hits += 1;
        }
    }
    println!("hits {hits}");
}
