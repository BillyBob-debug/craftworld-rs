//! Owned schema-2 snapshot projection shared by UI and headless consumers.
//! No original-format parsing lives here. Wire provenance: docs/resource-metadata.md.
//! Unused envelope fields are ignored; numeric unknown type IDs/raw flags remain intact.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub milestone: String,
    pub partial_metadata: bool,
    pub build: Value,
    #[serde(default)]
    pub build_fingerprint: Option<Fingerprint>,
    #[serde(default)]
    pub map: Option<MapInventory>,
    /// Installed-update MAP applied over `map` (patch rows win per GUID).
    #[serde(default)]
    pub patch_map: Option<MapInventory>,
    pub source_scope: String,
    pub payload_integrity: String,
    pub counts: BTreeMap<String, u64>,
    pub metadata_status_counts: BTreeMap<String, u64>,
    pub confirmed_type_counts: BTreeMap<String, u64>,
    pub dependency_resolution_counts: BTreeMap<String, u64>,
    pub archives: Vec<Archive>,
    pub resources: Vec<Resource>,
    pub resource_inspections: Vec<Location>,
    pub dependency_links: Vec<Link>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Fingerprint {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct MapInventory {
    pub path: String,
    pub revision: u32,
    pub row_count: usize,
    pub sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Archive {
    pub path: String,
    pub file_size: u64,
    pub table_offset: u64,
    pub entry_count: usize,
    pub duplicate_hash_rows: usize,
    pub entries: Vec<ArchiveEntry>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ArchiveEntry {
    pub entry_index: usize,
    pub sha1: String,
    pub offset: u64,
    pub size: u32,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Resource {
    pub map_row: MapRow,
    pub type_hint: Option<Hint>,
    pub confirmed_type: Option<String>,
    pub header_status: String,
    pub dependency_status: String,
    pub map_path_status: String,
    pub resolution: String,
    pub archive_candidates: Vec<ArchiveCandidate>,
    pub loose_candidates: Vec<LooseCandidate>,
}
impl Resource {
    pub fn inspection_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.archive_candidates
            .iter()
            .filter_map(|c| c.inspection_index)
            .chain(
                self.loose_candidates
                    .iter()
                    .filter_map(|c| c.inspection_index),
            )
    }
    pub fn display_type(&self) -> &str {
        self.confirmed_type.as_deref().unwrap_or("unconfirmed")
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct MapRow {
    pub row_index: usize,
    pub path: String,
    pub timestamp_raw: i64,
    pub declared_size: u32,
    pub sha1: String,
    pub guid: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Hint {
    pub name: String,
    pub basis: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ArchiveCandidate {
    pub archive: String,
    pub entry_index: usize,
    pub offset: u64,
    pub size: u32,
    pub declared_size_matches: bool,
    pub inspection_index: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct LooseCandidate {
    pub path: String,
    pub size: u64,
    pub declared_size_matches: bool,
    pub integrity: String,
    pub inspection_index: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Location {
    pub source_path: String,
    pub offset: u64,
    pub size: u64,
    pub sha1: Option<String>,
    pub inspection: Inspection,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Inspection {
    pub status: String,
    pub reason: Option<String>,
    pub magic_hex: String,
    pub confirmed_type: Option<Kind>,
    pub serialization: String,
    pub header: Option<Value>,
    pub dependency_status: String,
    pub dependency_count_raw: Option<u32>,
    pub dependencies: Vec<Dependency>,
    pub metadata_bytes_read: usize,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Kind {
    pub id: u32,
    pub name: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependency {
    pub index: usize,
    pub flags: u8,
    pub guid: Option<String>,
    pub sha1: Option<String>,
    pub type_id: u32,
    pub known_type: Option<Kind>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Link {
    pub inspection_index: usize,
    pub dependency_index: usize,
    pub guid_map_row_indices: Vec<usize>,
    pub sha1_map_row_indices: Vec<usize>,
    pub hash_archive_candidate_count: usize,
    pub status: String,
}
