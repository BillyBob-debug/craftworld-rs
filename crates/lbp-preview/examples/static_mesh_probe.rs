//! D119: read-only probe of every magic-confirmed `static_mesh` resource with the
//! independent `lbp_mesh::static_mesh` reader. Checks: whole payload consumed (stream sizes
//! match), every primitive triangulates, and every vertex a primitive uses lies inside that
//! primitive's stored min/max box (tolerance 1e-3 relative to the box size).
//! Output: a NEW JSON file under ignored `local-analysis/`.
//! Usage: static_mesh_probe SNAPSHOT GAME-DIR[;GAME-DIR] NEW-JSON
//! Counting unit: distinct static-mesh content hash.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: static_mesh_probe SNAPSHOT GAME-DIR NEW-JSON".into());
    }
    let output = Path::new(&args[3]);
    let allowed = Path::new("local-analysis")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if output.exists()
        || !output
            .parent()
            .ok_or("parent required")?
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(allowed)
    {
        return Err("new output under ignored local-analysis required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut by_hash = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "static_mesh" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_insert_with(|| r.map_row.path.clone());
        }
    }
    let mut rows = Vec::new();
    let mut status = BTreeMap::<String, usize>::new();
    for (sha, path) in &by_hash {
        let row = (|| -> Result<serde_json::Value, String> {
            let key = lbp_preview::resolve(&catalog, None, Some(sha))?;
            let bytes = lbp_preview::level_scene::read_resource(
                &source,
                &key,
                &control,
                lbp_mesh::static_mesh::MAX_INPUT,
            )?;
            let m = lbp_mesh::static_mesh::read(&bytes, || false).map_err(|e| e.to_string())?;
            let mut kinds = BTreeMap::<u8, usize>::new();
            let mut triangles = 0usize;
            let mut outside = 0usize;
            let mut failed = Vec::new();
            let mut materials = BTreeSet::new();
            for (i, p) in m.primitives.iter().enumerate() {
                *kinds.entry(p.kind).or_default() += 1;
                if let Some(g) = p.material.as_ref().and_then(|d| d.guid) {
                    materials.insert(format!("{g:08x}"));
                }
                match m.primitive_triangles(i) {
                    Ok(t) => {
                        triangles += t.len();
                        let used: BTreeSet<u32> = t.iter().flatten().copied().collect();
                        for v in used {
                            let q = m.positions[v as usize];
                            let bad = (0..3).any(|a| {
                                let tol = 1e-3 * (p.max[a] - p.min[a]).abs().max(1.);
                                q[a] < p.min[a] - tol || q[a] > p.max[a] + tol
                            });
                            outside += usize::from(bad);
                        }
                    }
                    Err(e) => failed.push(format!("{i}: {e}")),
                }
            }
            Ok(json!({
                "revision": format!("0x{:x}", m.revision),
                "primitives": m.primitives.len(),
                "nodes": m.node_count,
                "kinds": kinds,
                "vertices": m.positions.len(),
                "indices": m.indices.len(),
                "triangles": triangles,
                "vertices_outside_primitive_box": outside,
                "primitive_failures": failed,
                "distinct_materials": materials.len(),
                "lightmaps": m.lightmaps.iter().map(|d| d.as_ref().and_then(|d| d.guid).map(|g| format!("{g:08x}"))).collect::<Vec<_>>(),
            }))
        })();
        let mut v = json!({"sha1": sha, "path": path});
        match row {
            Ok(r) => {
                let ok = r["vertices_outside_primitive_box"] == 0
                    && r["primitive_failures"]
                        .as_array()
                        .is_some_and(Vec::is_empty);
                *status
                    .entry(
                        if ok {
                            "decoded_consistent"
                        } else {
                            "decoded_with_findings"
                        }
                        .into(),
                    )
                    .or_default() += 1;
                v["result"] = r;
            }
            Err(e) => {
                *status.entry(format!("error: {e}")).or_default() += 1;
                v["error"] = json!(e);
            }
        }
        rows.push(v);
    }
    let value = json!({
        "schema_version": 1,
        "milestone": "D119",
        "unit": "distinct static_mesh content hash",
        "status": status,
        "rows": rows,
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &value).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!("static meshes {}; status {status:?}", by_hash.len());
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
