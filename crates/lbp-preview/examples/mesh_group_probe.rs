//! Read-only probe (D160): prints each primitive of one mesh with its material GUID,
//! triangle count and preview material outcome (route label or explicit error).
//! Prints numbers and labels only (no game bytes). Usage:
//! mesh_group_probe SNAPSHOT GAME-DIRS MESH-GUID-HEX
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("mesh_group_probe SNAPSHOT GAME-DIRS MESH-GUID-HEX".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let guid =
        u32::from_str_radix(args[3].trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
    let key = lbp_preview::resolve(&catalog, Some(guid), None)?;
    let grouped = lbp_preview::load_mesh_groups(&source, &catalog, &key, &control)?;
    for (i, (p, b)) in grouped
        .geometry
        .metadata
        .primitives
        .iter()
        .zip(&grouped.bindings)
        .enumerate()
    {
        println!(
            "prim {i}: material {:?} hash {:?} kind {} flags {} first {} count {} -> {}",
            p.material_guid.map(|g| format!("{g:08x}")),
            p.material_hash,
            p.material_kind,
            p.material_flags,
            p.first_index,
            p.index_count,
            match b {
                Ok(m) => format!("OK {}", m.label()),
                Err(e) => format!("ERR {e}"),
            }
        );
        // Graph summary for neutral groups: box types/params, wires, texture slots.
        if b.is_err() {
            let mkey = lbp_preview::resolve(&catalog, p.material_guid, None)?;
            let bytes = lbp_preview::level_scene::read_resource(&source, &mkey, &control, 1 << 24)?;
            let g = lbp_resource::material::inspect_graph(&bytes, || false)
                .map_err(|e| e.to_string())?;
            for (k, t) in g.material.prefix.textures.iter().enumerate() {
                println!(
                    "  slot {k}: guid {:?} wrap {:?}/{:?}",
                    t.guid.map(|g| format!("{g:08x}")),
                    g.material.prefix.wrap_s_raw.get(k),
                    g.material.prefix.wrap_t_raw.get(k)
                );
            }
            for (k, x) in g.graph.boxes.iter().enumerate() {
                println!(
                    "  box {k}: type {} sub {} params {:?} floats {:?}",
                    x.type_raw,
                    x.subtype_raw,
                    x.parameters_raw,
                    x.parameters_raw.map(f32::from_bits)
                );
            }
            for w in &g.graph.wires {
                println!(
                    "  wire {}:{} -> {}:{} swz {:?}",
                    w.box_from, w.port_from_raw, w.box_to, w.port_to_raw, w.swizzle_raw
                );
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
