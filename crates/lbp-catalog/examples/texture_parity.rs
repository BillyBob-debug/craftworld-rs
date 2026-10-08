//! Read-only supported-texture comparison to pinned external oracle; metadata/digests only.
use lbp_catalog::{
    attachment::{AttachedSource, LocationKey},
    LoadControl,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: texture_parity SNAPSHOT GAME-DIR C1-ORACLE NEW-JSON".into());
    }
    let out = Path::new(&args[4]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if out.exists()
        || !out
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new ignored output required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let oracle: Value =
        serde_json::from_reader(std::fs::File::open(&args[3]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if oracle["toolkit_commit"] != "d6a2ef80b5a09999f08f23a803b516096055e5c9" {
        return Err("oracle pin differs".into());
    }
    let mut rows = Vec::new();
    let mut mismatches = 0;
    let mut compared = 0;
    let mut failures = 0;
    for sample in oracle["samples"]
        .as_array()
        .ok_or("oracle sample array absent")?
    {
        let sha = sample["sha1"].as_str().ok_or("sample hash absent")?;
        let l = catalog
            .snapshot
            .resource_inspections
            .iter()
            .find(|l| l.sha1.as_deref() == Some(sha))
            .ok_or("sample location absent")?;
        let key = LocationKey {
            path: l.source_path.clone(),
            offset: l.offset,
            size: l.size,
        };
        // D95: layered decode; a cube map compares face 0 (+X), the image Toolkit returns.
        let texture = lbp_texture::decode_reader_layered(
            key.size,
            |offset, count| {
                source
                    .read_hex_checked(&key, offset, count, &control)
                    .map(|p| p.bytes)
                    .map_err(lbp_texture::Error::Input)
            },
            || false,
        );
        match texture {
            Ok(l) => {
                let (layout, faces_decoded) = (l.layout, l.faces_decoded);
                let t = l.texture;
                let digest = format!("{:x}", Sha256::digest(&t.rgba));
                let dds_equal = if let Some(header) = &t.header.dds {
                    let raw = serde_json::to_value(header).map_err(|e| e.to_string())?;
                    Some(
                        raw.as_object()
                            .ok_or("DDS object absent")?
                            .iter()
                            .all(|(k, v)| sample["dds"][k] == *v),
                    )
                } else {
                    None
                };
                let equal = sample["top_mip_rgba_sha256"] == digest
                    && sample["image_width"] == t.header.width
                    && sample["image_height"] == t.header.height
                    && (if t.header.dds.is_some() {
                        &sample["dds"]["mips"]
                    } else {
                        &sample["gtf"]["mips"]
                    }) == &json!(t.header.mips)
                    && sample["no_srgb"] == t.header.no_srgb
                    && dds_equal.unwrap_or(true);
                let transparent_pixels = t
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[3] == 0)
                    .count();
                compared += 1;
                mismatches += usize::from(!equal);
                rows.push(json!({"sha1":sha,"status":"decoded","texture":t,"rgba_sha256":digest,"oracle_equal":equal,"dds_fields_equal":dds_equal,"transparent_pixels":transparent_pixels,"layout":layout,"faces_decoded":faces_decoded}));
            }
            Err(e) => {
                let skipped = matches!(
                    e,
                    lbp_texture::Error::Unsupported(_)
                        | lbp_texture::Error::Blocked
                        | lbp_texture::Error::Budget
                );
                failures += usize::from(!skipped);
                rows.push(json!({"sha1":sha,"status":if skipped {"not_supported"}else{"failed"},"reason":e.to_string()}));
            }
        }
    }
    source.check_stable()?;
    let report = json!({"schema_version":1,"compared":compared,"mismatches":mismatches,"failures":failures,"samples":rows});
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut f, &report).map_err(|e| e.to_string())?;
    f.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "Texture parity: {compared} supported samples, {mismatches} pixel/metadata mismatches"
    );
    if mismatches > 0 || failures > 0 || compared == 0 {
        return Err("pixel parity not established".into());
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
