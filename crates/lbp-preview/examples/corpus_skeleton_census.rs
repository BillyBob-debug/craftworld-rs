//! Read-only corpus census of stored mesh bones (lbp_mesh::Skeleton), output stays ignored.
//! Scope: every MAP row whose confirmed type is `mesh`, one load per content hash
//! (same row selection as corpus_mesh_census). Reports decode outcomes, bone counts,
//! SkeletonCheck consistency findings (links, cycles, child lists, name hash, bind
//! inverse error), compressed-matrix masks, and skeleton signatures (ordered anim
//! hashes) shared by several meshes, which is how the Sackboy body and its costume
//! pieces are expected to group. No asset-count claim: bones are not rendered.
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
/// A bind pose whose skin x inverse differs from identity by more than this is listed.
const INVERSE_TOLERANCE: f32 = 1e-3;

#[derive(Default)]
struct Signature {
    bone_count: usize,
    names: Vec<String>,
    hashes: BTreeSet<String>,
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("corpus_skeleton_census SNAPSHOT GAME-DIR NEW-JSON".into());
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
    let (mut decoded, mut with_bones, mut total_bones, mut mesh_bytes) = (0usize, 0, 0, 0u64);
    let (mut bad_links, mut cycles, mut child_mismatch, mut hash_mismatch_bones) = (0, 0, 0, 0);
    let mut failed: BTreeMap<String, usize> = BTreeMap::new();
    let mut bone_counts: BTreeMap<usize, usize> = BTreeMap::new();
    let mut root_counts: BTreeMap<usize, usize> = BTreeMap::new();
    let mut masks: BTreeMap<String, usize> = BTreeMap::new();
    let mut mirror_lengths_match = 0usize;
    let mut inverse_outliers = vec![];
    let mut signatures: BTreeMap<Vec<u32>, Signature> = BTreeMap::new();
    let mut rows = vec![];
    for (sha, guids) in &by_hash {
        control.check()?;
        let loaded = lbp_preview::resolve(&catalog, None, Some(sha)).and_then(|key| {
            mesh_bytes = mesh_bytes
                .checked_add(key.size)
                .ok_or("mesh byte overflow")?;
            if mesh_bytes > MAX_AGGREGATE_MESH_BYTES {
                return Err("aggregate mesh input budget".into());
            }
            lbp_preview::load_mesh(&source, &catalog, &key, false, &control)
        });
        let geometry = match loaded {
            Ok(m) => m.geometry,
            Err(e) => {
                *failed
                    .entry(lbp_preview::census::reason_key(&e))
                    .or_default() += 1;
                continue;
            }
        };
        decoded += 1;
        let s = &geometry.skeleton;
        if s.bones.is_empty() {
            continue;
        }
        with_bones += 1;
        total_bones += s.bones.len();
        let c = s.check();
        *bone_counts.entry(s.bones.len()).or_default() += 1;
        *root_counts.entry(c.roots).or_default() += 1;
        bad_links += usize::from(!c.links_in_range);
        cycles += usize::from(c.links_in_range && !c.acyclic);
        child_mismatch += usize::from(c.acyclic && !c.child_lists_match_parents);
        hash_mismatch_bones += c.bone_count - c.name_hash_matches;
        mirror_lengths_match += usize::from(
            s.mirror_bones.len() == s.bones.len() && s.mirror_flip_types.len() == s.bones.len(),
        );
        for b in &s.bones {
            for m in [b.skin_pose_mask, b.inv_skin_pose_mask] {
                *masks.entry(format!("{m:04x}")).or_default() += 1;
            }
        }
        if c.max_inverse_error.is_none_or(|e| e > INVERSE_TOLERANCE) {
            inverse_outliers.push(json!({"sha1":sha,"max_inverse_error":c.max_inverse_error}));
        }
        let sig = signatures
            .entry(s.bones.iter().map(|b| b.anim_hash).collect())
            .or_default();
        if sig.names.is_empty() {
            sig.bone_count = s.bones.len();
            sig.names = s.bones.iter().map(|b| b.name.clone()).collect();
        }
        sig.hashes.insert(sha.clone());
        rows.push(json!({"sha1":sha,"guids":guids,"bones":s.bones.len(),"check":c,
            "mirror_morphs":s.mirror_morphs.len(),"morph_count":geometry.metadata.prefix.counts[6]}));
    }
    source.check_stable()?;
    let mut shared: Vec<_> = signatures
        .iter()
        .filter(|(_, s)| s.hashes.len() > 1)
        .map(|(k, s)| (s.hashes.len(), k, s))
        .collect();
    shared.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let shared: Vec<_> = shared
        .iter()
        .take(40)
        .map(|(n, _, s)| json!({"meshes":n,"bone_count":s.bone_count,"bone_names":s.names,"mesh_hashes":s.hashes}))
        .collect();
    let report = json!({
        "schema_version": 1,
        "tool": "corpus_skeleton_census",
        "scope": "corpus_all_mesh_map_rows",
        "scope_note": "MAP rows with confirmed type mesh, one load per content hash; meshes over the 50,000-triangle preview budget count as failed",
        "unit": "distinct mesh content hash",
        "budgets": {"max_mesh_hashes": MAX_MESH_HASHES, "max_aggregate_mesh_bytes": MAX_AGGREGATE_MESH_BYTES, "inverse_tolerance": INVERSE_TOLERANCE},
        "totals": {
            "mesh_hashes": by_hash.len(), "decoded": decoded, "failed": failed.values().sum::<usize>(),
            "with_bones": with_bones, "bones": total_bones,
            "links_out_of_range": bad_links, "parent_cycles": cycles,
            "child_list_mismatch": child_mismatch, "bones_name_hash_mismatch": hash_mismatch_bones,
            "inverse_outliers": inverse_outliers.len(), "mirror_tables_full_length": mirror_lengths_match,
            "distinct_skeleton_signatures": signatures.len(), "mesh_input_bytes": mesh_bytes
        },
        "failed_by_reason": failed,
        "bone_count_histogram": bone_counts,
        "root_count_histogram": root_counts,
        "matrix_mask_histogram": masks,
        "inverse_outliers": inverse_outliers,
        "shared_signatures_top40": shared,
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
        "skeleton census: {} mesh hashes, {decoded} decoded, {} failed; {with_bones} with bones ({total_bones} bones); links out of range {bad_links}, cycles {cycles}, child-list mismatches {child_mismatch}, name-hash mismatched bones {hash_mismatch_bones}, inverse outliers {}; {} skeleton signatures",
        by_hash.len(),
        report["totals"]["failed"],
        report["totals"]["inverse_outliers"],
        signatures.len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
