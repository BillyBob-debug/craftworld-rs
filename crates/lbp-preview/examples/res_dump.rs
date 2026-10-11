#![allow(clippy::all)]
//! QA probe: writes the stored bytes of resources by GUID to `<out>/<guid>.bin`.
//! No game bytes are printed.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_preview::story::{resolve_key, ResourceKey};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("res_dump SNAPSHOT GAME-DIR OUT-DIR GUID-HEX...");
        std::process::exit(2);
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control).unwrap();
    let source =
        AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control).unwrap();
    std::fs::create_dir_all(&args[3]).unwrap();
    for g in &args[4..] {
        let guid = u32::from_str_radix(g, 16).unwrap();
        let loc = match resolve_key(&catalog, ResourceKey::Guid(guid)) {
            Ok(l) => l,
            Err(e) => {
                println!("{g} failed: {e}");
                continue;
            }
        };
        let mut out = Vec::new();
        let mut off = 0u64;
        while off < loc.size {
            let n = (loc.size - off).min(4096) as usize;
            match source.read_hex_checked(&loc, off, n, &control) {
                Ok(p) => {
                    off += p.bytes.len() as u64;
                    if p.bytes.is_empty() {
                        break;
                    }
                    out.extend_from_slice(&p.bytes);
                }
                Err(e) => {
                    println!("{g} read failed: {e:?}");
                    break;
                }
            }
        }
        std::fs::write(format!("{}/{g}.bin", args[3]), &out).unwrap();
        println!("{g} {} bytes", out.len());
    }
}
