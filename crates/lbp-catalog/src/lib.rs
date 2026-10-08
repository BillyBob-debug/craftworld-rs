//! UI-independent, bounded/cancellable snapshot loader and catalog indexes.
//! Reads authored scanner JSON only; never opens game assets or interprets formats.
pub mod attachment;
pub mod census_summary;
mod model;
mod synthetic;
pub use model::*;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Arc,
    },
};
pub use synthetic::demo_snapshot;

pub const MAX_SNAPSHOT_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Clone, Default)]
pub struct LoadControl {
    pub cancelled: Arc<AtomicBool>,
    pub bytes_read: Arc<AtomicU64>,
    pub file_bytes: Arc<AtomicU64>,
    pub phase: Arc<AtomicU8>,
}
impl LoadControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Load cancelled".into())
        } else {
            Ok(())
        }
    }
}
struct ControlledRead<'a> {
    file: File,
    control: &'a LoadControl,
}
impl Read for ControlledRead<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.control.check().map_err(io::Error::other)?;
        let remaining = (MAX_SNAPSHOT_BYTES + 1)
            .saturating_sub(self.control.bytes_read.load(Ordering::Relaxed));
        if remaining == 0 {
            return Err(io::Error::other("Snapshot read budget exceeded"));
        }
        let size = out.len().min(remaining as usize);
        let n = self.file.read(&mut out[..size])?;
        self.control
            .bytes_read
            .fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}
pub fn load(path: &Path, control: &LoadControl) -> Result<Catalog, String> {
    control.check()?;
    let file = File::open(path).map_err(|e| format!("Open snapshot: {e}"))?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_SNAPSHOT_BYTES {
        return Err("Choose a regular snapshot file no larger than 256 MiB".into());
    }
    control.file_bytes.store(meta.len(), Ordering::Relaxed);
    control.bytes_read.store(0, Ordering::Relaxed);
    control.phase.store(0, Ordering::Relaxed);
    let reader = BufReader::with_capacity(64 * 1024, ControlledRead { file, control });
    let snapshot: Snapshot =
        serde_json::from_reader(reader).map_err(|e| format!("Read schema-2 snapshot: {e}"))?;
    if control.bytes_read.load(Ordering::Relaxed) > MAX_SNAPSHOT_BYTES {
        return Err("Snapshot grew beyond read budget".into());
    }
    Catalog::build(snapshot, control)
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Filter {
    pub query: String,
    pub archive: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
}
pub struct Catalog {
    pub snapshot: Snapshot,
    pub row_position: HashMap<usize, usize>,
    pub by_guid: HashMap<String, Vec<usize>>,
    pub by_hash: HashMap<String, Vec<usize>>,
    pub inspection_resources: Vec<Vec<usize>>,
    pub outgoing: Vec<Vec<usize>>,
    pub reverse: Vec<Vec<usize>>,
    pub kinds: Vec<String>,
    search_text: Vec<String>,
}
fn hex(value: &str, width: usize) -> Result<(), String> {
    if value.len() != width || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Err(format!("Invalid {width}-digit identifier"))
    } else {
        Ok(())
    }
}
fn count(snapshot: &Snapshot, key: &str, actual: usize) -> Result<(), String> {
    if snapshot.counts.get(key).copied() != Some(actual as u64) {
        return Err(format!("Snapshot count {key} disagrees with its records"));
    }
    Ok(())
}
impl Catalog {
    pub fn build(snapshot: Snapshot, control: &LoadControl) -> Result<Self, String> {
        control.check()?;
        control.phase.store(1, Ordering::Relaxed);
        if snapshot.schema_version != 2
            || !matches!(
                snapshot.milestone.as_str(),
                "A1_directory_tables" | "A2_resource_metadata"
            )
        {
            return Err("Unsupported snapshot schema/milestone; use the schema-2 scanner".into());
        }
        if snapshot.resources.len() > 1_000_000
            || snapshot.resource_inspections.len() > 1_000_000
            || snapshot.dependency_links.len() > 2_000_000
        {
            return Err("Catalog entry budget exceeded".into());
        }
        count(&snapshot, "map_rows", snapshot.resources.len())?;
        count(&snapshot, "archives", snapshot.archives.len())?;
        let entries: usize = snapshot.archives.iter().map(|a| a.entries.len()).sum();
        if entries > 1_000_000 {
            return Err("Archive entry budget exceeded".into());
        }
        count(&snapshot, "archive_entries", entries)?;
        let mut row_position = HashMap::new();
        let mut by_guid: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_hash: HashMap<String, Vec<usize>> = HashMap::new();
        let mut inspection_resources = vec![Vec::new(); snapshot.resource_inspections.len()];
        let mut search_text = Vec::with_capacity(snapshot.resources.len());
        for (i, r) in snapshot.resources.iter().enumerate() {
            if i % 1024 == 0 {
                control.check()?;
            }
            hex(&r.map_row.guid, 8)?;
            hex(&r.map_row.sha1, 40)?;
            if row_position.insert(r.map_row.row_index, i).is_some() {
                return Err("Duplicate MAP row index in snapshot".into());
            }
            by_guid
                .entry(r.map_row.guid.to_ascii_lowercase())
                .or_default()
                .push(i);
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .push(i);
            let mut used = HashSet::new();
            for index in r.inspection_indices() {
                let targets = inspection_resources
                    .get_mut(index)
                    .ok_or("Candidate inspection index out of range")?;
                if used.insert(index) {
                    targets.push(i);
                }
            }
            search_text.push(
                format!(
                    "{} {} {} {} {} {} {}",
                    r.map_row.path,
                    r.map_row.guid,
                    r.map_row.sha1,
                    r.display_type(),
                    r.type_hint.as_ref().map_or("", |h| &h.name),
                    r.header_status,
                    r.resolution
                )
                .to_ascii_lowercase(),
            );
        }
        for archive in &snapshot.archives {
            if archive.entry_count != archive.entries.len()
                || archive.table_offset > archive.file_size
            {
                return Err("Archive snapshot bounds/count disagree".into());
            }
            let mut indices = HashSet::new();
            for entry in &archive.entries {
                control.check()?;
                hex(&entry.sha1, 40)?;
                if !indices.insert(entry.entry_index)
                    || entry
                        .offset
                        .checked_add(u64::from(entry.size))
                        .is_none_or(|end| end > archive.table_offset)
                {
                    return Err("Archive snapshot has invalid entry index/range".into());
                }
            }
        }
        let mut dependencies = 0usize;
        for (i, location) in snapshot.resource_inspections.iter().enumerate() {
            if i % 1024 == 0 {
                control.check()?;
            }
            if let Some(hash) = &location.sha1 {
                hex(hash, 40)?;
            }
            if location.inspection.dependencies.len() > 65536 {
                return Err("Snapshot dependency budget exceeded".into());
            }
            dependencies = dependencies
                .checked_add(location.inspection.dependencies.len())
                .ok_or("Dependency count overflow")?;
            for (index, dep) in location.inspection.dependencies.iter().enumerate() {
                if dep.index != index {
                    return Err("Dependency indices are not contiguous".into());
                }
                if let Some(guid) = &dep.guid {
                    hex(guid, 8)?;
                }
                if let Some(hash) = &dep.sha1 {
                    hex(hash, 40)?;
                }
            }
        }
        count(&snapshot, "dependency_entries", dependencies)?;
        if dependencies != snapshot.dependency_links.len() {
            return Err("Dependency/link count disagrees".into());
        }
        let mut outgoing = vec![Vec::new(); snapshot.resource_inspections.len()];
        let mut reverse = vec![Vec::new(); snapshot.resources.len()];
        let mut link_keys = HashSet::new();
        let mut target_references = 0usize;
        for (i, link) in snapshot.dependency_links.iter().enumerate() {
            if i % 1024 == 0 {
                control.check()?;
            }
            target_references = target_references
                .checked_add(link.guid_map_row_indices.len())
                .and_then(|n| n.checked_add(link.sha1_map_row_indices.len()))
                .ok_or("Reference count overflow")?;
            if target_references > 2_000_000 {
                return Err("Reference target budget exceeded".into());
            }
            let location = snapshot
                .resource_inspections
                .get(link.inspection_index)
                .ok_or("Link inspection out of range")?;
            if link.dependency_index >= location.inspection.dependencies.len()
                || !link_keys.insert((link.inspection_index, link.dependency_index))
            {
                return Err("Link dependency index is invalid or duplicated".into());
            }
            outgoing[link.inspection_index].push(i);
            let mut targets = BTreeSet::new();
            for row in link
                .guid_map_row_indices
                .iter()
                .chain(&link.sha1_map_row_indices)
            {
                let position = *row_position.get(row).ok_or("Link target MAP row absent")?;
                targets.insert(position);
            }
            for target in targets {
                reverse[target].push(i);
            }
        }
        control.check()?;
        control.phase.store(2, Ordering::Relaxed);
        let kinds = snapshot
            .resources
            .iter()
            .map(|r| r.display_type().to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(Self {
            snapshot,
            row_position,
            by_guid,
            by_hash,
            inspection_resources,
            outgoing,
            reverse,
            kinds,
            search_text,
        })
    }
    pub fn filtered(&self, filter: &Filter) -> Vec<usize> {
        let query = filter.query.trim().to_ascii_lowercase();
        self.snapshot
            .resources
            .iter()
            .enumerate()
            .filter(|(i, r)| {
                if let Some(archive) = &filter.archive {
                    if !r.archive_candidates.iter().any(|c| &c.archive == archive) {
                        return false;
                    }
                }
                if let Some(kind) = &filter.kind {
                    if r.display_type() != kind {
                        return false;
                    }
                }
                if let Some(status) = &filter.status {
                    if &r.header_status != status {
                        return false;
                    }
                }
                if let Some(id) = query.strip_prefix("guid:") {
                    return r
                        .map_row
                        .guid
                        .eq_ignore_ascii_case(id.trim().trim_start_matches("0x"));
                }
                if let Some(id) = query.strip_prefix("hash:") {
                    return r.map_row.sha1.eq_ignore_ascii_case(id.trim());
                }
                self.search_text[*i].contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn targets(&self, link_index: usize) -> Vec<usize> {
        let link = &self.snapshot.dependency_links[link_index];
        link.guid_map_row_indices
            .iter()
            .chain(&link.sha1_map_row_indices)
            .filter_map(|row| self.row_position.get(row).copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_interrupts_subsequent_reads_and_read_budget_is_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("authored.txt");
        std::fs::write(&path, b"synthetic bytes").unwrap();
        let control = LoadControl::default();
        let mut reader = ControlledRead {
            file: File::open(&path).unwrap(),
            control: &control,
        };
        assert_eq!(reader.read(&mut [0; 2]).unwrap(), 2);
        control.cancel();
        assert!(reader
            .read(&mut [0; 2])
            .unwrap_err()
            .to_string()
            .contains("cancelled"));
        let control = LoadControl::default();
        control
            .bytes_read
            .store(MAX_SNAPSHOT_BYTES + 1, Ordering::Relaxed);
        let mut reader = ControlledRead {
            file: File::open(path).unwrap(),
            control: &control,
        };
        assert!(reader
            .read(&mut [0; 2])
            .unwrap_err()
            .to_string()
            .contains("budget"));
    }
}
