#![allow(clippy::all)]
//! QA probe: decodes textures by GUID to raw RGBA files (`<out>/<guid>.rgba` plus a
//! `<guid>.txt` with width and height). No game bytes are printed.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::{decode_icon, ResourceKey};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("tex_dump SNAPSHOT GAME-DIR OUT-DIR GUID-HEX...");
        std::process::exit(2);
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control).unwrap();
    let source =
        AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control).unwrap();
    std::fs::create_dir_all(&args[3]).unwrap();
    for g in &args[4..] {
        let guid = u32::from_str_radix(g, 16).unwrap();
        if let Ok(loc) = lbp_preview::story::resolve_key(&catalog, ResourceKey::Guid(guid)) {
            if let Ok(t) = lbp_texture::decode_reader(
                loc.size,
                |o, c| {
                    source
                        .read_hex_checked(&loc, o, c, &control)
                        .map(|p| p.bytes)
                        .map_err(|_| lbp_texture::Error::Invalid("read"))
                },
                || false,
            ) {
                let h = &t.header;
                println!(
                    "{g} fmt {:?} {}x{} mips {} flags {:?} remap {:x?} raw {:x?} nosrgb {}",
                    h.pixel_format,
                    h.width,
                    h.height,
                    h.mips,
                    h.flags,
                    h.remap,
                    h.format_raw,
                    h.no_srgb
                );
            }
        }
        match decode_icon(&source, &catalog, ResourceKey::Guid(guid), &control) {
            Ok(i) => {
                std::fs::write(format!("{}/{g}.rgba", args[3]), &i.rgba).unwrap();
                std::fs::write(
                    format!("{}/{g}.txt", args[3]),
                    format!("{} {}\n", i.width, i.height),
                )
                .unwrap();
                println!("{g} {}x{}", i.width, i.height);
            }
            Err(e) => println!("{g} failed: {e}"),
        }
    }
}
