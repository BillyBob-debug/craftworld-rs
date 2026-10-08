//! Read-only corpus census of ANMb keyframe decoding (lbp_resource::animation::decode),
//! output stays ignored. Scope: every MAP row whose confirmed type is `animation`, one
//! read per content hash. Reports decode outcomes and the open questions of runtime
//! checks S4-S6: frames/fps, the `fat` byte and posOffset/posScale bit patterns,
//! trailing blob bytes (padding vs wider keys), clamped rotations, locator keys and
//! locator names, and bone-hash sets (compare with corpus_skeleton_census signatures).
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_resource::animation;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Write,
    path::Path,
};

const MAX_HASHES: usize = 50_000;
const MAX_AGGREGATE_BYTES: u64 = 1024 * 1024 * 1024;

fn bump<K: Ord>(map: &mut BTreeMap<K, usize>, key: K) {
    *map.entry(key).or_default() += 1;
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("corpus_animation_census SNAPSHOT GAME-DIR NEW-JSON".into());
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
        if r.display_type() == "animation" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    if by_hash.is_empty() || by_hash.len() > MAX_HASHES {
        return Err("animation hash budget".into());
    }
    let (mut decoded, mut bytes_read, mut clamped, mut with_locator_keys) = (0usize, 0u64, 0, 0);
    let mut failed: BTreeMap<String, usize> = BTreeMap::new();
    let mut frames: BTreeMap<u16, usize> = BTreeMap::new();
    let mut fps: BTreeMap<u16, usize> = BTreeMap::new();
    let mut fat: BTreeMap<String, usize> = BTreeMap::new();
    let mut offset_scale: BTreeMap<String, usize> = BTreeMap::new();
    let mut trailing: BTreeMap<usize, usize> = BTreeMap::new();
    let mut locator_names: BTreeMap<String, usize> = BTreeMap::new();
    let mut bone_counts: BTreeMap<usize, usize> = BTreeMap::new();
    let mut bone_sets: BTreeMap<Vec<u32>, BTreeSet<String>> = BTreeMap::new();
    let mut unparsed_tail: BTreeMap<usize, usize> = BTreeMap::new();
    let mut rows = vec![];
    for (sha, guids) in &by_hash {
        control.check()?;
        let key = match lbp_preview::resolve(&catalog, None, Some(sha)) {
            Ok(k) => k,
            Err(e) => {
                bump(&mut failed, lbp_preview::census::reason_key(&e));
                continue;
            }
        };
        if key.size > animation::MAX_INPUT as u64 {
            bump(&mut failed, "input budget".into());
            continue;
        }
        bytes_read = bytes_read.checked_add(key.size).ok_or("byte overflow")?;
        if bytes_read > MAX_AGGREGATE_BYTES {
            return Err("aggregate input budget".into());
        }
        let mut data = Vec::with_capacity(key.size as usize);
        while data.len() < key.size as usize {
            let count = (key.size as usize - data.len()).min(4096);
            let page = source
                .read_hex_checked(&key, data.len() as u64, count, &control)?
                .bytes;
            if page.len() != count {
                return Err("short page".into());
            }
            data.extend(page);
        }
        let a = match animation::decode(&data, || control.check().is_err()) {
            Ok(a) => a,
            Err(e) => {
                bump(&mut failed, e.to_string());
                continue;
            }
        };
        decoded += 1;
        let p = &a.inspection.prefix;
        bump(&mut frames, p.frames_raw_u16);
        bump(&mut fps, p.fps_raw_u16);
        bump(
            &mut fat,
            p.fat_raw_u8.map_or("absent".into(), |v| v.to_string()),
        );
        bump(
            &mut offset_scale,
            match (p.position_offset_bits, p.position_scale_bits) {
                (Some(o), Some(s)) => format!("{o:08x?} {s:08x?}"),
                _ => "absent".into(),
            },
        );
        bump(&mut trailing, a.trailing_bytes);
        bump(&mut unparsed_tail, a.unparsed_after_locators);
        bump(&mut bone_counts, a.bones.len());
        clamped += a.clamped_rotations;
        with_locator_keys += usize::from(!a.locator_keys.is_empty());
        for l in &a.locators {
            bump(&mut locator_names, l.name.clone());
        }
        bone_sets
            .entry(a.bones.iter().map(|b| b.anim_hash).collect())
            .or_default()
            .insert(sha.clone());
        rows.push(json!({"sha1":sha,"guids":guids,"revision":p.revision,"frames":p.frames_raw_u16,
            "fps":p.fps_raw_u16,"loop_start":p.loop_start_raw_u16,"bones":a.bones.len(),
            "tracks":[a.rot_animated.len(),a.pos_animated.len(),a.scale_animated.len(),a.morph_animated.len()],
            "morph_count":p.morph_count_raw_u8,"locator_keys":a.locator_keys.len(),
            "locators":a.locators.len(),"trailing_bytes":a.trailing_bytes,
            "clamped_rotations":a.clamped_rotations}));
    }
    source.check_stable()?;
    let mut sets: Vec<_> = bone_sets.iter().collect();
    sets.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let sets: Vec<_> = sets
        .iter()
        .take(20)
        .map(|(hashes, files)| {
            json!({"animations":files.len(),"bone_count":hashes.len(),
                "anim_hashes":hashes.iter().map(|h| format!("{h:08x}")).collect::<Vec<_>>()})
        })
        .collect();
    let report = json!({
        "schema_version": 1,
        "tool": "corpus_animation_census",
        "scope": "corpus_all_animation_map_rows",
        "unit": "distinct animation content hash",
        "totals": {"animation_hashes": by_hash.len(), "decoded": decoded,
            "failed": failed.values().sum::<usize>(), "clamped_rotations": clamped,
            "with_locator_keys": with_locator_keys, "distinct_bone_sets": bone_sets.len(),
            "input_bytes": bytes_read},
        "failed_by_reason": failed,
        "frames_histogram": frames,
        "fps_histogram": fps,
        "fat_histogram": fat,
        "position_offset_scale_bits_histogram": offset_scale,
        "trailing_bytes_histogram": trailing,
        "unparsed_after_locators_histogram": unparsed_tail,
        "bone_count_histogram": bone_counts,
        "locator_names": locator_names,
        "bone_sets_top20": sets,
        "animations": rows
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, &report).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    println!(
        "animation census: {} hashes, {decoded} decoded, {} failed; trailing-byte values {:?}; fat values {:?}; {} bone sets",
        by_hash.len(),
        report["totals"]["failed"],
        trailing.keys().collect::<Vec<_>>(),
        fat.keys().collect::<Vec<_>>(),
        bone_sets.len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
