//! A2 directory metadata scanner: original files are inputs, never output targets.
use lbp_io::{sfo, source};

use lbp_archive::{read_farc, read_map, ArchiveEntry, MapRow};
use lbp_core::{ContentHash, Error, ErrorKind, Guid, ReadLimits, Result};
use lbp_resource::{inspect, type_hint, Inspection, InspectionLimits, TypeHint};
use serde::Serialize;
use sha2::{Digest, Sha256};
use source::{Source, SourceFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const HELP: &str = "LBP2 A2 asset scanner (read-only; no payload decoding)\n\nUsage: lbp-cli --game-dir PATH [--json NEW_FILE] [--tables-only]\n\nInput: disc root containing PS3_GAME, or PS3_GAME directory.\nOutput parent must exist and be outside the selected game directory. No overwrite.\n\nLimits: --max-table-bytes N (67108864), --max-entries N (1000000),\n        --max-files N (1000000; directories/skipped entries count too),\n        --metadata-bytes N (1048576 per resource), --dependency-limit N (65536),\n        --scan-read-budget N (268435456 resource metadata bytes total).\n\nMAP 0x100, FARC, bounded clear binary headers/dependencies; type hints remain\nseparate from magic-confirmed types. --tables-only disables metadata reads.\nProtected data is blocked. Texture/static-mesh payloads, ISO and GUI are deferred.\nExit: 0 catalog complete within supported scope; 1 fatal; 2 partial metadata\n(malformed/I/O/limit diagnostics preserved in JSON). Unknown/unsupported/blocked\nresources and missing candidates are counted without treating them as corrupt.\n";
struct Options {
    game_dir: PathBuf,
    json: Option<PathBuf>,
    limits: ReadLimits,
    max_files: usize,
    tables_only: bool,
    inspection_limits: InspectionLimits,
    scan_read_budget: usize,
}
fn options() -> Result<Option<Options>> {
    let mut args = env::args_os().skip(1);
    let mut game_dir = None;
    let mut json = None;
    let mut limits = ReadLimits::default();
    let mut max_files = 1_000_000;
    let mut tables_only = false;
    let mut inspection_limits = InspectionLimits::default();
    let mut scan_read_budget = 256 * 1024 * 1024;
    let mut seen = BTreeSet::new();
    while let Some(arg) = args.next() {
        let key = arg
            .to_str()
            .ok_or_else(|| Error::malformed("argument flag is not UTF-8"))?;
        if key == "--help" || key == "-h" {
            print!("{HELP}");
            return Ok(None);
        }
        if key == "--tables-only" {
            if !seen.insert(key.to_owned()) {
                return Err(Error::malformed("duplicate --tables-only"));
            }
            tables_only = true;
            continue;
        }
        if !matches!(
            key,
            "--game-dir"
                | "--json"
                | "--max-table-bytes"
                | "--max-entries"
                | "--max-files"
                | "--metadata-bytes"
                | "--dependency-limit"
                | "--scan-read-budget"
        ) {
            return Err(Error::malformed(format!(
                "unknown argument {key}; use --help"
            )));
        }
        if !seen.insert(key.to_owned()) {
            return Err(Error::malformed(format!("duplicate argument {key}")));
        }
        let value = args
            .next()
            .ok_or_else(|| Error::malformed(format!("{key} requires a value")))?;
        match key {
            "--game-dir" => game_dir = Some(PathBuf::from(value)),
            "--json" => json = Some(PathBuf::from(value)),
            _ => {
                let number = value
                    .to_str()
                    .and_then(|v| v.parse::<usize>().ok())
                    .filter(|n| *n > 0)
                    .ok_or_else(|| {
                        Error::malformed(format!("{key} requires a positive integer"))
                    })?;
                match key {
                    "--max-table-bytes" => limits.max_table_bytes = number,
                    "--max-entries" => limits.max_entries = number,
                    "--max-files" => max_files = number,
                    "--metadata-bytes" => inspection_limits.max_metadata_bytes = number,
                    "--dependency-limit" => inspection_limits.max_dependencies = number,
                    _ => scan_read_budget = number,
                }
            }
        }
    }
    Ok(Some(Options {
        game_dir: game_dir.ok_or_else(|| Error::malformed("--game-dir is required; use --help"))?,
        json,
        limits,
        max_files,
        tables_only,
        inspection_limits,
        scan_read_budget,
    }))
}

#[derive(Serialize)]
struct Fingerprint {
    path: String,
    size: u64,
    sha256: String,
}
#[derive(Serialize)]
struct MapInventory {
    path: String,
    revision: u32,
    row_count: usize,
    sha256: String,
}
#[derive(Serialize)]
struct ArchiveInventory {
    path: String,
    file_size: u64,
    table_offset: u64,
    entry_count: usize,
    duplicate_hash_rows: usize,
    entries: Vec<ArchiveEntry>,
}
#[derive(Clone, Serialize)]
struct ArchiveLocation {
    archive: String,
    entry_index: usize,
    offset: u64,
    size: u32,
    declared_size_matches: bool,
    inspection_index: Option<usize>,
}
#[derive(Clone, Serialize)]
struct LooseLocation {
    path: String,
    size: u64,
    declared_size_matches: bool,
    integrity: &'static str,
    inspection_index: Option<usize>,
}
#[derive(Serialize)]
struct InspectedLocation {
    source_path: String,
    offset: u64,
    size: u64,
    sha1: Option<ContentHash>,
    inspection: Inspection,
}
#[derive(Serialize)]
struct DependencyLink {
    inspection_index: usize,
    dependency_index: usize,
    guid_map_row_indices: Vec<usize>,
    sha1_map_row_indices: Vec<usize>,
    hash_archive_candidate_count: usize,
    status: &'static str,
}
#[derive(Serialize)]
struct ResourceRow {
    map_row: MapRow,
    type_hint: Option<TypeHint>,
    confirmed_type: Option<String>,
    header_status: &'static str,
    dependency_status: &'static str,
    map_path_status: &'static str,
    resolution: &'static str,
    archive_candidates: Vec<ArchiveLocation>,
    loose_candidates: Vec<LooseLocation>,
}
#[derive(Serialize)]
struct DuplicateGuid {
    guid: Guid,
    map_row_indices: Vec<usize>,
}
#[derive(Default, Serialize)]
struct Counts {
    filesystem_files: usize,
    filesystem_directories: usize,
    skipped_filesystem_entries: usize,
    archives: usize,
    archive_entries: usize,
    unique_archive_hashes: usize,
    duplicate_archive_hash_rows: usize,
    map_rows: usize,
    unique_map_hashes: usize,
    duplicate_guid_rows: usize,
    map_rows_with_archive_candidates: usize,
    map_rows_without_archive_candidates: usize,
    map_rows_with_loose_candidates: usize,
    map_rows_missing_in_selected_directory: usize,
    ambiguous_resource_rows: usize,
    unsafe_map_paths: usize,
    declared_size_mismatch_candidates: usize,
    archive_hashes_without_map_rows: usize,
    header_inspected_resources: usize,
    uninspected_resources: usize,
    rows_with_unknown_extension_hint: usize,
    metadata_bytes_read: usize,
    dependency_entries: usize,
}
#[derive(Serialize)]
struct Inventory {
    schema_version: u32,
    milestone: &'static str,
    partial_metadata: bool,
    metadata_status_counts: BTreeMap<String, usize>,
    confirmed_type_counts: BTreeMap<String, usize>,
    resource_inspections: Vec<InspectedLocation>,
    inspection_budgets: BTreeMap<&'static str, usize>,
    dependency_links: Vec<DependencyLink>,
    dependency_resolution_counts: BTreeMap<String, usize>,
    build: sfo::BuildIdentity,
    build_fingerprint: Fingerprint,
    source_scope: &'static str,
    payload_integrity: &'static str,
    map: MapInventory,
    counts: Counts,
    map_extension_counts: BTreeMap<String, usize>,
    duplicate_guids: Vec<DuplicateGuid>,
    archives: Vec<ArchiveInventory>,
    resources: Vec<ResourceRow>,
    filesystem_files: Vec<SourceFile>,
    skipped_filesystem_entries: Vec<source::SkippedEntry>,
}

fn read_small(source: &Source, file: &SourceFile, max: usize) -> Result<Vec<u8>> {
    if file.size > max as u64 {
        return Err(Error::limit(format!("{} exceeds byte budget", file.path)));
    }
    let input = source.open(file)?;
    let cap = max
        .checked_add(1)
        .ok_or_else(|| Error::limit("byte budget overflow"))?;
    let mut bytes = Vec::new();
    input.take(cap as u64).read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(Error::limit(format!(
            "{} grew beyond byte budget",
            file.path
        )));
    }
    if bytes.len() as u64 != file.size {
        return Err(Error::malformed(format!(
            "{} changed size during scan",
            file.path
        )));
    }
    Ok(bytes)
}
fn fingerprint(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn scan(source: Source, options: &Options) -> Result<Inventory> {
    let limits = options.limits;
    let mut resource_inspections = Vec::new();
    let mut remaining_budget = options.scan_read_budget;
    let sfo_file = source.required("PARAM.SFO")?;
    let sfo_bytes = read_small(&source, sfo_file, 1024 * 1024)?;
    let build = sfo::read_identity(&sfo_bytes)?;
    let build_fingerprint = Fingerprint {
        path: sfo_file.path.clone(),
        size: sfo_file.size,
        sha256: fingerprint(&sfo_bytes),
    };
    let map_file = source.required("USRDIR/OUTPUT/BLURAYGUIDS.MAP")?;
    let map_bytes = read_small(&source, map_file, limits.max_table_bytes)?;
    let mut mapping = read_map(&map_bytes, limits)?;
    let map = MapInventory {
        path: map_file.path.clone(),
        revision: mapping.revision,
        row_count: mapping.rows.len(),
        sha256: fingerprint(&map_bytes),
    };
    let mut archives = Vec::new();
    let mut locations: BTreeMap<ContentHash, Vec<ArchiveLocation>> = BTreeMap::new();
    let mut total_archive_entries = 0_usize;
    for file in source.files.iter().filter(|f| {
        f.path.to_ascii_lowercase().starts_with("usrdir/")
            && f.path.to_ascii_lowercase().ends_with(".farc")
    }) {
        let mut input = source.open(file)?;
        let mut index = read_farc(&mut input, limits)?;
        if index.file_size != file.size {
            return Err(Error::malformed("archive changed size during scan"));
        }
        total_archive_entries = total_archive_entries
            .checked_add(index.entries.len())
            .ok_or_else(|| Error::limit("total archive count overflow"))?;
        if total_archive_entries > limits.max_entries {
            return Err(Error::limit("combined archive entry budget exceeded"));
        }
        let unique = index
            .entries
            .iter()
            .map(|r| r.sha1)
            .collect::<BTreeSet<_>>()
            .len();
        for entry in &index.entries {
            let inspection_index = if options.tables_only {
                None
            } else {
                let mut limits = options.inspection_limits;
                limits.max_metadata_bytes = limits.max_metadata_bytes.min(remaining_budget);
                let inspection = inspect(&mut input, entry.offset, u64::from(entry.size), limits);
                remaining_budget -= inspection.metadata_bytes_read;
                let i = resource_inspections.len();
                resource_inspections.push(InspectedLocation {
                    source_path: file.path.clone(),
                    offset: entry.offset,
                    size: u64::from(entry.size),
                    sha1: Some(entry.sha1),
                    inspection,
                });
                Some(i)
            };
            locations
                .entry(entry.sha1)
                .or_default()
                .push(ArchiveLocation {
                    archive: file.path.clone(),
                    entry_index: entry.entry_index,
                    offset: entry.offset,
                    size: entry.size,
                    declared_size_matches: false,
                    inspection_index,
                });
        }
        index
            .entries
            .sort_by_key(|e| (e.sha1, e.offset, e.entry_index));
        archives.push(ArchiveInventory {
            path: file.path.clone(),
            file_size: index.file_size,
            table_offset: index.table_offset,
            entry_count: index.entries.len(),
            duplicate_hash_rows: index.entries.len() - unique,
            entries: index.entries,
        });
    }
    if archives.is_empty() {
        return Err(Error::malformed("no .FARC archives under USRDIR"));
    }
    let mut loose: BTreeMap<String, Vec<&SourceFile>> = BTreeMap::new();
    for file in &source.files {
        loose
            .entry(file.path.to_ascii_lowercase())
            .or_default()
            .push(file);
    }
    let mut loose_inspections = BTreeMap::new();
    if !options.tables_only {
        let wanted: BTreeSet<_> = mapping
            .rows
            .iter()
            .filter_map(|r| source::map_relative(&r.path))
            .map(|p| p.to_ascii_lowercase())
            .collect();
        for file in &source.files {
            if !wanted.contains(&file.path.to_ascii_lowercase())
                || file.path.to_ascii_lowercase().ends_with(".farc")
            {
                continue;
            }
            let mut input = source.open(file)?;
            let mut limits = options.inspection_limits;
            limits.max_metadata_bytes = limits.max_metadata_bytes.min(remaining_budget);
            let inspection = inspect(&mut input, 0, file.size, limits);
            remaining_budget -= inspection.metadata_bytes_read;
            loose_inspections.insert(file.path.clone(), resource_inspections.len());
            resource_inspections.push(InspectedLocation {
                source_path: file.path.clone(),
                offset: 0,
                size: file.size,
                sha1: None,
                inspection,
            });
        }
    }
    let map_hashes: BTreeSet<_> = mapping.rows.iter().map(|r| r.sha1).collect();
    let mut guids: BTreeMap<Guid, Vec<usize>> = BTreeMap::new();
    for row in &mapping.rows {
        guids.entry(row.guid).or_default().push(row.row_index);
    }
    let duplicate_guids: Vec<_> = guids
        .into_iter()
        .filter(|(_, rows)| rows.len() > 1)
        .map(|(guid, map_row_indices)| DuplicateGuid {
            guid,
            map_row_indices,
        })
        .collect();
    let mut counts = Counts {
        filesystem_files: source.files.len(),
        filesystem_directories: source.directory_count,
        skipped_filesystem_entries: source.skipped.len(),
        archives: archives.len(),
        archive_entries: total_archive_entries,
        unique_archive_hashes: locations.len(),
        duplicate_archive_hash_rows: total_archive_entries - locations.len(),
        map_rows: mapping.rows.len(),
        unique_map_hashes: map_hashes.len(),
        duplicate_guid_rows: duplicate_guids
            .iter()
            .map(|d| d.map_row_indices.len() - 1)
            .sum(),
        archive_hashes_without_map_rows: locations
            .keys()
            .filter(|h| !map_hashes.contains(h))
            .count(),
        uninspected_resources: map_hashes.len(),
        ..Counts::default()
    };
    let inspected_hashes: BTreeSet<_> = locations
        .iter()
        .filter(|(_, items)| {
            items.iter().any(|c| {
                c.inspection_index
                    .is_some_and(|i| resource_inspections[i].inspection.metadata_bytes_read >= 4)
            })
        })
        .map(|(hash, _)| *hash)
        .filter(|hash| map_hashes.contains(hash))
        .collect();
    // Include actual loose-file inspections in distinct MAP-hash coverage, without hashing their payloads.
    let mut inspected_hashes = inspected_hashes;
    for row in &mapping.rows {
        if let Some(path) = source::map_relative(&row.path) {
            if loose.get(&path.to_ascii_lowercase()).is_some_and(|files| {
                files.iter().any(|f| {
                    loose_inspections.get(&f.path).is_some_and(|i| {
                        resource_inspections[*i].inspection.metadata_bytes_read >= 4
                    })
                })
            }) {
                inspected_hashes.insert(row.sha1);
            }
        }
    }
    counts.header_inspected_resources = inspected_hashes.len();
    counts.uninspected_resources = map_hashes.len() - inspected_hashes.len();
    counts.metadata_bytes_read = options.scan_read_budget - remaining_budget;
    let mut metadata_status_counts = BTreeMap::new();
    let mut confirmed_type_counts = BTreeMap::new();
    let mut partial_metadata = false;
    for item in &resource_inspections {
        let inspection = &item.inspection;
        *metadata_status_counts
            .entry(inspection.status.to_owned())
            .or_insert(0) += 1;
        if let Some(kind) = inspection.confirmed_type {
            *confirmed_type_counts
                .entry(kind.name.to_owned())
                .or_insert(0) += 1;
        }
        counts.dependency_entries += inspection.dependencies.len();
        partial_metadata |= matches!(
            inspection.status,
            "malformed" | "io_error" | "limit_exceeded"
        );
    }
    let mut map_extension_counts = BTreeMap::new();
    let mut by_guid: BTreeMap<Guid, Vec<&MapRow>> = BTreeMap::new();
    let mut by_hash: BTreeMap<ContentHash, Vec<&MapRow>> = BTreeMap::new();
    for row in &mapping.rows {
        by_guid.entry(row.guid).or_default().push(row);
        by_hash.entry(row.sha1).or_default().push(row);
    }
    let mut dependency_links = Vec::new();
    let mut dependency_resolution_counts = BTreeMap::new();
    for (inspection_index, item) in resource_inspections.iter().enumerate() {
        for dep in &item.inspection.dependencies {
            // Toolkit stream.guid maps zero to null; SHA1.EMPTY is likewise null.
            // Raw zero bytes remain in Dependency, while catalog linkage treats them as null.
            let guid = dep.guid.filter(|g| g.0 != 0);
            let hash = dep.sha1.filter(|h| h.0 != [0; 20]);
            let guid_rows = guid.and_then(|g| by_guid.get(&g));
            let hash_rows = hash.and_then(|h| by_hash.get(&h));
            let hash_archive_candidate_count =
                hash.and_then(|h| locations.get(&h)).map_or(0, Vec::len);
            let has_guid = guid_rows.is_some();
            let has_hash = hash_rows.is_some() || hash_archive_candidate_count > 0;
            let mismatch = guid_rows
                .zip(hash)
                .is_some_and(|(rows, h)| !rows.iter().any(|r| r.sha1 == h));
            let status = if mismatch {
                "guid_hash_disagree_in_selected_catalog"
            } else if has_guid && has_hash {
                "guid_and_hash_catalog_candidates"
            } else if has_guid {
                "guid_catalog_candidate"
            } else if has_hash {
                "hash_catalog_candidate"
            } else if guid.is_none() && hash.is_none() {
                "null_reference"
            } else {
                "unresolved_in_selected_catalog"
            };
            *dependency_resolution_counts
                .entry(status.to_owned())
                .or_insert(0) += 1;
            dependency_links.push(DependencyLink {
                inspection_index,
                dependency_index: dep.index,
                guid_map_row_indices: guid_rows
                    .map(|rows| rows.iter().map(|r| r.row_index).collect())
                    .unwrap_or_default(),
                sha1_map_row_indices: hash_rows
                    .map(|rows| rows.iter().map(|r| r.row_index).collect())
                    .unwrap_or_default(),
                hash_archive_candidate_count,
                status,
            });
        }
    }
    let mut resources = Vec::with_capacity(mapping.rows.len());
    mapping.rows.sort_by(|a, b| {
        (a.sha1, a.guid, &a.path, a.row_index).cmp(&(b.sha1, b.guid, &b.path, b.row_index))
    });
    for row in mapping.rows {
        let extension = Path::new(&row.path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{}", s.to_ascii_lowercase()))
            .unwrap_or_else(|| "(none)".into());
        *map_extension_counts.entry(extension).or_insert(0) += 1;
        let hint = type_hint(&row.path);
        if hint.is_none() {
            counts.rows_with_unknown_extension_hint += 1;
        }
        let mut archive_candidates = locations.get(&row.sha1).cloned().unwrap_or_default();
        for candidate in &mut archive_candidates {
            candidate.declared_size_matches = candidate.size == row.declared_size;
        }
        let safe = source::map_relative(&row.path);
        if safe.is_none() {
            counts.unsafe_map_paths += 1;
        }
        let loose_candidates: Vec<_> = safe
            .as_ref()
            .and_then(|p| loose.get(&p.to_ascii_lowercase()))
            .into_iter()
            .flatten()
            .map(|file| LooseLocation {
                path: file.path.clone(),
                size: file.size,
                declared_size_matches: file.size == u64::from(row.declared_size),
                integrity: "not_hashed",
                inspection_index: loose_inspections.get(&file.path).copied(),
            })
            .collect();
        if archive_candidates.is_empty() {
            counts.map_rows_without_archive_candidates += 1;
        } else {
            counts.map_rows_with_archive_candidates += 1;
        }
        if !loose_candidates.is_empty() {
            counts.map_rows_with_loose_candidates += 1;
        }
        counts.declared_size_mismatch_candidates += archive_candidates
            .iter()
            .filter(|c| !c.declared_size_matches)
            .count()
            + loose_candidates
                .iter()
                .filter(|c| !c.declared_size_matches)
                .count();
        let resolution = match archive_candidates.len() + loose_candidates.len() {
            0 => {
                counts.map_rows_missing_in_selected_directory += 1;
                "missing_in_selected_directory"
            }
            1 if !archive_candidates.is_empty() => "archive_candidate",
            1 => "loose_candidate",
            _ => {
                counts.ambiguous_resource_rows += 1;
                "multiple_candidates_no_precedence_chosen"
            }
        };
        let indices: Vec<_> = archive_candidates
            .iter()
            .filter_map(|c| c.inspection_index)
            .chain(loose_candidates.iter().filter_map(|c| c.inspection_index))
            .collect();
        let metadata =
            if archive_candidates.len() + loose_candidates.len() == 1 && indices.len() == 1 {
                Some(&resource_inspections[indices[0]].inspection)
            } else {
                None
            };
        let absent_status = if options.tables_only {
            "not_inspected"
        } else if archive_candidates.len() + loose_candidates.len() > 1 {
            "ambiguous_not_selected"
        } else if resolution == "missing_in_selected_directory" {
            "missing"
        } else {
            "container_not_inspected"
        };
        resources.push(ResourceRow {
            type_hint: hint,
            confirmed_type: metadata
                .and_then(|m| m.confirmed_type)
                .map(|t| t.name.to_owned()),
            header_status: metadata.map(|m| m.status).unwrap_or(absent_status),
            dependency_status: metadata
                .map(|m| m.dependency_status)
                .unwrap_or(absent_status),
            map_path_status: if safe.is_some() {
                "relative_path_candidate"
            } else {
                "unsafe_not_resolved_as_filesystem_path"
            },
            resolution,
            archive_candidates,
            loose_candidates,
            map_row: row,
        });
    }
    Ok(Inventory {
        schema_version: 2,
        milestone: if options.tables_only {
            "A1_directory_tables"
        } else {
            "A2_resource_metadata"
        },
        partial_metadata,
        metadata_status_counts,
        confirmed_type_counts,
        resource_inspections,
        inspection_budgets: BTreeMap::from([
            (
                "per_resource_bytes",
                options.inspection_limits.max_metadata_bytes,
            ),
            (
                "dependencies_per_resource",
                options.inspection_limits.max_dependencies,
            ),
            ("total_metadata_bytes", options.scan_read_budget),
        ]),
        dependency_links,
        dependency_resolution_counts,
        build,
        build_fingerprint,
        source_scope: "selected_directory_only_no_update_overlay_assumed",
        payload_integrity: "archive_identifiers_read_only_payload_hashes_not_verified",
        map,
        counts,
        map_extension_counts,
        duplicate_guids,
        archives,
        resources,
        filesystem_files: source.files,
        skipped_filesystem_entries: source.skipped,
    })
}

fn publish_json(destination: &Path, inventory: &Inventory) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| Error::malformed("output parent absent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = BufWriter::new(temporary.as_file_mut());
        serde_json::to_writer_pretty(&mut writer, inventory)
            .map_err(|e| Error::new(ErrorKind::Io, format!("serialize JSON: {e}")))?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    temporary.as_file_mut().sync_all()?;
    temporary.persist_noclobber(destination).map_err(|e| {
        Error::new(
            ErrorKind::Io,
            format!("publish JSON without overwrite: {}", e.error),
        )
    })?;
    Ok(())
}
fn run() -> Result<bool> {
    let Some(options) = options()? else {
        return Ok(false);
    };
    let source = Source::discover(&options.game_dir, options.max_files)?;
    let output = options
        .json
        .as_ref()
        .map(|p| source::check_output(p, &source))
        .transpose()?;
    let inventory = scan(source, &options)?;
    if let Some(path) = &output {
        publish_json(path, &inventory)?;
    }
    let c = &inventory.counts;
    println!(
        "LBP2 directory inventory: {} (no payload decoding)",
        inventory.milestone
    );
    println!(
        "Build: {}  APP_VER {}  VERSION {}",
        inventory.build.title_id.as_deref().unwrap_or("unknown"),
        inventory.build.app_version.as_deref().unwrap_or("unknown"),
        inventory.build.disc_version.as_deref().unwrap_or("unknown")
    );
    println!(
        "MAP: {} rows, {} distinct hashes, {} duplicate GUID rows (revision {:#x})",
        c.map_rows, c.unique_map_hashes, c.duplicate_guid_rows, inventory.map.revision
    );
    println!(
        "FARC: {} archives, {} entries, {} distinct hashes",
        c.archives, c.archive_entries, c.unique_archive_hashes
    );
    println!(
        "MAP rows: {} with archive candidates; {} outside archives",
        c.map_rows_with_archive_candidates, c.map_rows_without_archive_candidates
    );
    println!(
        "Selected-directory loose matches: {}; missing candidates: {}; ambiguous rows: {}",
        c.map_rows_with_loose_candidates,
        c.map_rows_missing_in_selected_directory,
        c.ambiguous_resource_rows
    );
    println!(
        "Uninspected distinct resources: {}; extension hints are separate from confirmed types",
        c.uninspected_resources
    );
    println!(
        "Metadata statuses: {:?}; dependencies: {}; metadata bytes: {}; partial: {}",
        inventory.metadata_status_counts,
        c.dependency_entries,
        c.metadata_bytes_read,
        inventory.partial_metadata
    );
    println!(
        "Size mismatch candidates: {}; skipped filesystem entries: {}",
        c.declared_size_mismatch_candidates, c.skipped_filesystem_entries
    );
    if let Some(path) = output {
        println!("JSON: {}", path.display());
    }
    Ok(inventory.partial_metadata)
}
fn main() -> ExitCode {
    match run() {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(2),
        Err(error) => {
            eprintln!("lbp-cli: {error}");
            ExitCode::FAILURE
        }
    }
}
