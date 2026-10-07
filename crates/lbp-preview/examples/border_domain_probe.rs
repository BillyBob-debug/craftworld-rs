//! Read-only, bounded selected-triangle UV-domain research; no border-color inference.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("border_domain_probe SNAPSHOT GAME-DIR GUID".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(std::path::Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(std::path::Path::new(&args[2]), &catalog, &control)?;
    let guid = u32::from_str_radix(&args[3], 16).map_err(|e| e.to_string())?;
    let key = lbp_preview::resolve(&catalog, Some(guid), None)?;
    if key.size > lbp_mesh::MAX_INPUT as u64 {
        return Err("mesh input budget".into());
    }
    let mut bytes = Vec::new();
    while bytes.len() < key.size as usize {
        let page = source
            .read_hex_checked(
                &key,
                bytes.len() as u64,
                (key.size as usize - bytes.len()).min(4096),
                &control,
            )?
            .bytes;
        if page.len() != (key.size as usize - bytes.len()).min(4096) {
            return Err("short input".into());
        }
        bytes.extend(page);
    }
    let geometry = lbp_mesh::decode_geometry(&bytes, || control.check().is_err())
        .map_err(|e| e.to_string())?;
    if geometry.positions.len() > 100_000 || geometry.triangles.len() > 50_000 {
        return Err("geometry budget".into());
    }
    let uv =
        lbp_mesh::decode_uvs(&bytes, || control.check().is_err()).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for (i, p) in geometry.metadata.primitives.iter().enumerate() {
        let range = geometry.primitive_triangles[i].clone();
        let mut channels = Vec::new();
        for channel in &uv.channels {
            let mut min = [f32::INFINITY; 2];
            let mut max = [f32::NEG_INFINITY; 2];
            for t in &geometry.triangles[range.clone()] {
                control.check()?;
                for &v in t {
                    let point = channel.get(v as usize).ok_or("UV correspondence")?;
                    for axis in 0..2 {
                        if !point[axis].is_finite() {
                            return Err("nonfinite UV".into());
                        }
                        min[axis] = min[axis].min(point[axis]);
                        max[axis] = max[axis].max(point[axis]);
                    }
                }
            }
            channels.push(serde_json::json!({"min":min,"max":max}));
        }
        rows.push(serde_json::json!({"primitive":i,"material_guid":p.material_guid,"triangles":range.len(),"channels":channels}));
    }
    source.check_stable()?;
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"guid":args[3],"scope":"raw UV range over referenced triangle vertices only; no sampling/color claim","groups":rows})
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
