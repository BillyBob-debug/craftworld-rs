//! Read-only corpus census of mesh skin and morph streams (lbp_mesh::decode_skin), output
//! stays ignored. Scope: every MAP row whose confirmed type is `mesh`, one read per
//! content hash (same row selection as corpus_skeleton_census). Reports decode outcomes,
//! joints that point past the mesh's bone list while weighted, weight-byte overflow,
//! clamped packed normals, morph counts and morph-name sets (runtime checks S2/S7).
//! No asset-count claim: skinning is not rendered.
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const MAX_MESH_HASHES: usize = 20_000;
const MAX_AGGREGATE_MESH_BYTES: u64 = 1024 * 1024 * 1024;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("corpus_skin_census SNAPSHOT GAME-DIR NEW-JSON".into());
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
        return Err("new ignored output required".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "mesh" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    if by_hash.is_empty() || by_hash.len() > MAX_MESH_HASHES {
        return Err("mesh hash budget".into());
    }
    let mut failed: BTreeMap<String, usize> = BTreeMap::new();
    let (mut decoded, mut skinned, mut mesh_bytes) = (0usize, 0usize, 0u64);
    let (mut joint_past_bones, mut overflow_meshes, mut overflow_vertices) = (0, 0, 0);
    let (mut clamped_meshes, mut clamped_normals, mut single_bone_vertices) = (0, 0, 0usize);
    let mut morph_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut morph_name_sets: BTreeMap<Vec<String>, BTreeSet<String>> = BTreeMap::new();
    let mut rows = vec![];
    for (sha, guids) in &by_hash {
        control.check()?;
        let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Ok(k) => k,
            Err(e) => {
                *failed
                    .entry(lbp_preview::census::reason_key(&e))
                    .or_default() += 1;
                continue;
            }
        };
        if key.size > lbp_mesh::MAX_INPUT as u64 {
            *failed.entry("input budget".into()).or_default() += 1;
            continue;
        }
        mesh_bytes = mesh_bytes
            .checked_add(key.size)
            .ok_or("mesh byte overflow")?;
        if mesh_bytes > MAX_AGGREGATE_MESH_BYTES {
            return Err("aggregate mesh input budget".into());
        }
        let mut data = Vec::with_capacity(key.size as usize);
        while data.len() < key.size as usize {
            let count = (key.size as usize - data.len()).min(4096);
            let page = source
                .read_hex_checked(&key, data.len() as u64, count, &control)?
                .bytes;
            if page.len() != count {
                return Err("short mesh page".into());
            }
            data.extend(page);
        }
        let skin = match lbp_mesh::decode_skin(&data, || control.check().is_err()) {
            Ok(s) => s,
            Err(e) => {
                *failed.entry(e.to_string()).or_default() += 1;
                continue;
            }
        };
        decoded += 1;
        let bones = lbp_mesh::decode_geometry(&data, || control.check().is_err())
            .map(|g| g.skeleton.bones.len())
            .map_err(|e| e.to_string());
        let m = &skin.metadata;
        let past = match (&bones, m.max_weighted_joint) {
            (Ok(n), Some(j)) => usize::from(j) >= *n,
            _ => false,
        };
        if bones.as_ref().is_ok_and(|n| *n > 0) {
            skinned += 1;
        }
        joint_past_bones += usize::from(past);
        overflow_meshes += usize::from(m.weight_overflow_vertices > 0);
        overflow_vertices += m.weight_overflow_vertices;
        clamped_meshes += usize::from(m.clamped_normals > 0);
        clamped_normals += m.clamped_normals;
        single_bone_vertices += m.single_bone_vertices;
        *morph_counts.entry(m.morph_count).or_default() += 1;
        if m.morph_count > 0 {
            morph_name_sets
                .entry(m.morph_names.clone())
                .or_default()
                .insert(sha.clone());
        }
        rows.push(json!({"sha1":sha,"guids":guids,"bones":bones.ok(),"metadata":m,"joint_past_bones":past}));
    }
    source.check_stable()?;
    let mut sets: Vec<_> = morph_name_sets.iter().collect();
    sets.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let sets: Vec<_> = sets
        .iter()
        .take(40)
        .map(|(names, hashes)| json!({"meshes":hashes.len(),"morph_names":names,"mesh_hashes":hashes}))
        .collect();
    let report = json!({
        "schema_version": 1,
        "tool": "corpus_skin_census",
        "scope": "corpus_all_mesh_map_rows",
        "unit": "distinct mesh content hash",
        "budgets": {"max_mesh_hashes": MAX_MESH_HASHES, "max_aggregate_mesh_bytes": MAX_AGGREGATE_MESH_BYTES, "per_mesh_input_bytes": lbp_mesh::MAX_INPUT},
        "totals": {
            "mesh_hashes": by_hash.len(), "decoded": decoded, "failed": failed.values().sum::<usize>(),
            "with_bones": skinned, "weighted_joint_past_bone_list_meshes": joint_past_bones,
            "weight_overflow_meshes": overflow_meshes, "weight_overflow_vertices": overflow_vertices,
            "clamped_normal_meshes": clamped_meshes, "clamped_normals": clamped_normals,
            "single_bone_vertices": single_bone_vertices,
            "distinct_morph_name_sets": morph_name_sets.len(), "mesh_input_bytes": mesh_bytes
        },
        "failed_by_reason": failed,
        "morph_count_histogram": morph_counts,
        "morph_name_sets_top40": sets,
        "meshes": rows
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "skin census: {} mesh hashes, {decoded} decoded, {} failed; {skinned} with bones; weighted joints past bone list {joint_past_bones}; weight overflow {overflow_meshes} meshes / {overflow_vertices} vertices; clamped normals {clamped_meshes} meshes / {clamped_normals}; {} morph name sets",
        by_hash.len(),
        report["totals"]["failed"],
        morph_name_sets.len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
