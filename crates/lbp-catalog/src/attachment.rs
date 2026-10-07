//! B2 source binding. Reuses existing SFO/MAP/FARC readers, never decodes payloads.
//! Format provenance: docs/formats.md, A1 readers and their pinned Toolkit citations.
//! Binding/stability/hex policy is project-authored; see docs/source-attachment.md.
use crate::{Catalog, LoadControl};
use lbp_archive::{read_farc, read_map};
use lbp_core::{Error, ReadLimits};
use lbp_io::{
    sfo,
    source::{map_relative, Source, SourceFile},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::atomic::Ordering,
    time::SystemTime,
};

pub const HEX_PAGE_BYTES: usize = 4096;
const ATTACH_READ_LIMIT: u64 = 128 * 1024 * 1024;
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    size: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
}
fn record_stamp(
    stamps: &mut BTreeMap<String, Stamp>,
    path: String,
    stamp: Stamp,
) -> Result<(), String> {
    if stamps.get(&path).is_some_and(|old| old != &stamp) {
        return Err("Source changed during attachment validation".into());
    }
    stamps.insert(path, stamp);
    Ok(())
}
impl Stamp {
    fn of(file: &File) -> Result<Self, String> {
        let m = file.metadata().map_err(|e| e.to_string())?;
        Ok(Self {
            size: m.len(),
            modified: m.modified().ok(),
            created: m.created().ok(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocationKey {
    pub path: String,
    pub offset: u64,
    pub size: u64,
}
#[derive(Clone)]
struct Approved {
    hashes: BTreeSet<String>,
    blocked: bool,
}
pub struct AttachedSource {
    source: Source,
    stamps: BTreeMap<String, Stamp>,
    approved: HashMap<LocationKey, Approved>,
    pub validation_bytes: u64,
    pub archive_count: usize,
    pub entry_count: usize,
}
pub struct HexPage {
    pub relative_offset: u64,
    pub absolute_offset: u64,
    pub resource_size: u64,
    pub bytes: Vec<u8>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn cancelled(control: &LoadControl) -> Result<(), Error> {
    control.check().map_err(Error::malformed)
}
fn source_file<'a>(source: &'a Source, path: &str) -> Result<&'a SourceFile, String> {
    source.required(path).map_err(|e| e.to_string())
}
fn budget(control: &LoadControl, n: u64) -> Result<(), String> {
    control.check()?;
    let total = control
        .bytes_read
        .fetch_add(n, Ordering::Relaxed)
        .checked_add(n)
        .ok_or("Attachment read overflow")?;
    if total > ATTACH_READ_LIMIT {
        return Err("Attachment metadata read budget exceeded".into());
    }
    Ok(())
}
fn small(
    source: &Source,
    path: &str,
    max: usize,
    control: &LoadControl,
    stamps: &mut BTreeMap<String, Stamp>,
) -> Result<Vec<u8>, String> {
    let entry = source_file(source, path)?;
    if entry.size > max as u64 {
        return Err("Attachment file exceeds metadata byte budget".into());
    }
    let mut file = source.open(entry).map_err(|e| e.to_string())?;
    let before = Stamp::of(&file)?;
    let mut out = Vec::new();
    let mut first = [0; 4];
    let n = usize::try_from(entry.size.min(4)).unwrap();
    file.read_exact(&mut first[..n])
        .map_err(|e| e.to_string())?;
    budget(control, n as u64)?;
    if lbp_resource::is_protected_prefix(&first[..n]) {
        return Err("Protected metadata input; stopped at prefix".into());
    }
    out.extend_from_slice(&first[..n]);
    let mut buf = [0; 65536];
    loop {
        control.check()?;
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        budget(control, n as u64)?;
        if out.len().checked_add(n).is_none_or(|v| v > max) {
            return Err("Attachment file grew beyond budget".into());
        }
        out.extend_from_slice(&buf[..n]);
    }
    if before != Stamp::of(&file)? || out.len() as u64 != entry.size {
        return Err("Metadata input changed while validating".into());
    }
    record_stamp(stamps, entry.path.clone(), before)?;
    Ok(out)
}
struct CheckedFile<'a> {
    file: File,
    control: &'a LoadControl,
}
impl Read for CheckedFile<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        self.control.check().map_err(std::io::Error::other)?;
        let remaining =
            ATTACH_READ_LIMIT.saturating_sub(self.control.bytes_read.load(Ordering::Relaxed));
        if remaining == 0 {
            return Err(std::io::Error::other(
                "Attachment metadata read budget exceeded",
            ));
        }
        let size = out.len().min(65536).min(remaining as usize);
        let n = self.file.read(&mut out[..size])?;
        budget(self.control, n as u64).map_err(std::io::Error::other)?;
        Ok(n)
    }
}
impl Seek for CheckedFile<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.control.check().map_err(std::io::Error::other)?;
        self.file.seek(pos)
    }
}

impl AttachedSource {
    pub fn attach(path: &Path, catalog: &Catalog, control: &LoadControl) -> Result<Self, String> {
        control.check()?;
        control.phase.store(3, Ordering::Relaxed);
        control.bytes_read.store(0, Ordering::Relaxed);
        let snapshot = &catalog.snapshot;
        let fingerprint = snapshot.build_fingerprint.as_ref().ok_or(
            "Snapshot lacks SFO fingerprint; regenerate with scanner (Demo cannot attach)",
        )?;
        let map = snapshot
            .map
            .as_ref()
            .ok_or("Snapshot lacks MAP fingerprint; regenerate with scanner")?;
        if fingerprint.path != "PARAM.SFO"
            || !map
                .path
                .eq_ignore_ascii_case("USRDIR/OUTPUT/BLURAYGUIDS.MAP")
        {
            return Err("Snapshot identity/index paths are unsupported".into());
        }
        let source = Source::discover_checked(path, 1_000_000, || cancelled(control))
            .map_err(|e| e.to_string())?;
        let mut stamps = BTreeMap::new();
        let sfo_bytes = small(
            &source,
            &fingerprint.path,
            1024 * 1024,
            control,
            &mut stamps,
        )?;
        if sfo_bytes.len() as u64 != fingerprint.size
            || !digest(&sfo_bytes).eq_ignore_ascii_case(&fingerprint.sha256)
        {
            return Err("Source SFO fingerprint differs from snapshot".into());
        }
        let identity = sfo::read_identity(&sfo_bytes).map_err(|e| e.to_string())?;
        if serde_json::to_value(identity).map_err(|e| e.to_string())? != snapshot.build {
            return Err("Source build identity differs from snapshot".into());
        }
        let map_bytes = small(
            &source,
            &map.path,
            ReadLimits::default().max_table_bytes,
            control,
            &mut stamps,
        )?;
        if !digest(&map_bytes).eq_ignore_ascii_case(&map.sha256) {
            return Err("Source MAP fingerprint differs from snapshot".into());
        }
        let parsed = read_map(&map_bytes, ReadLimits::default()).map_err(|e| e.to_string())?;
        if parsed.revision != map.revision
            || parsed.rows.len() != map.row_count
            || parsed.rows.len() != snapshot.resources.len()
        {
            return Err("Source MAP layout/count differs from snapshot".into());
        }
        for row in parsed.rows {
            control.check()?;
            let r = catalog
                .row_position
                .get(&row.row_index)
                .and_then(|i| snapshot.resources.get(*i))
                .ok_or("Snapshot MAP row absent")?;
            let saved = &r.map_row;
            if row.path != saved.path
                || row.timestamp_raw != saved.timestamp_raw
                || row.declared_size != saved.declared_size
                || !row.guid.to_string().eq_ignore_ascii_case(&saved.guid)
                || !row.sha1.to_string().eq_ignore_ascii_case(&saved.sha1)
            {
                return Err("Snapshot MAP fields differ from source".into());
            }
        }
        let mut approved = HashMap::new();
        let mut entry_count = 0;
        for archive in &snapshot.archives {
            control.check()?;
            let entry = source_file(&source, &archive.path)?;
            if entry.size != archive.file_size {
                return Err("Source archive size differs from snapshot".into());
            }
            let mut file = source.open(entry).map_err(|e| e.to_string())?;
            let before = Stamp::of(&file)?;
            let mut prefix = [0; 4];
            file.read_exact(&mut prefix).map_err(|e| e.to_string())?;
            budget(control, 4)?;
            // Container signatures only: an archive's first payload can itself be an encrypted wrapper.
            if prefix == *b"NPD\0" || prefix == *b"SCE\0" || prefix == [0x7f, b'P', b'K', b'G'] {
                return Err("Protected container; stopped at prefix".into());
            }
            let mut checked = CheckedFile { file, control };
            let parsed =
                read_farc(&mut checked, ReadLimits::default()).map_err(|e| e.to_string())?;
            if parsed.file_size != archive.file_size
                || parsed.table_offset != archive.table_offset
                || parsed.entries.len() != archive.entries.len()
            {
                return Err("Source FARC table layout differs from snapshot".into());
            }
            for (row, saved) in parsed.entries.iter().zip(&archive.entries) {
                control.check()?;
                if row.entry_index != saved.entry_index
                    || row.offset != saved.offset
                    || row.size != saved.size
                    || !row.sha1.to_string().eq_ignore_ascii_case(&saved.sha1)
                {
                    return Err("Source FARC entry differs from snapshot".into());
                }
                let key = LocationKey {
                    path: entry.path.to_ascii_lowercase(),
                    offset: row.offset,
                    size: u64::from(row.size),
                };
                let existing = approved.entry(key).or_insert(Approved {
                    hashes: BTreeSet::new(),
                    blocked: false,
                });
                existing.hashes.insert(saved.sha1.to_ascii_lowercase());
            }
            if before != Stamp::of(&checked.file)? {
                return Err("Archive changed while validating".into());
            }
            record_stamp(&mut stamps, entry.path.clone(), before)?;
            entry_count += parsed.entries.len();
        }
        for r in &snapshot.resources {
            control.check()?;
            for candidate in &r.archive_candidates {
                let archive = snapshot
                    .archives
                    .iter()
                    .find(|a| a.path == candidate.archive)
                    .ok_or("Candidate archive absent")?;
                let entry = archive
                    .entries
                    .get(candidate.entry_index)
                    .ok_or("Candidate archive entry absent")?;
                if candidate.offset != entry.offset
                    || candidate.size != entry.size
                    || !entry.sha1.eq_ignore_ascii_case(&r.map_row.sha1)
                {
                    return Err("Snapshot candidate differs from validated FARC/MAP".into());
                }
                if let Some(i) = candidate.inspection_index {
                    let location = snapshot
                        .resource_inspections
                        .get(i)
                        .ok_or("Candidate inspection absent")?;
                    if !location
                        .source_path
                        .eq_ignore_ascii_case(&candidate.archive)
                        || location.offset != candidate.offset
                        || location.size != u64::from(candidate.size)
                        || location
                            .sha1
                            .as_ref()
                            .is_none_or(|h| !h.eq_ignore_ascii_case(&r.map_row.sha1))
                    {
                        return Err("Candidate points to a different inspection location".into());
                    }
                }
            }
            for candidate in &r.loose_candidates {
                let normalized =
                    map_relative(&r.map_row.path).ok_or("Unsafe MAP loose candidate")?;
                if !normalized.eq_ignore_ascii_case(&candidate.path) {
                    return Err("Loose candidate is not its MAP path".into());
                }
                let entry = source_file(&source, &candidate.path)?;
                if entry.size != candidate.size {
                    return Err("Loose candidate size differs from snapshot".into());
                }
                if let Some(i) = candidate.inspection_index {
                    let location = snapshot
                        .resource_inspections
                        .get(i)
                        .ok_or("Loose inspection absent")?;
                    if !location.source_path.eq_ignore_ascii_case(&candidate.path)
                        || location.offset != 0
                        || location.size != candidate.size
                    {
                        return Err(
                            "Loose candidate points to a different inspection location".into()
                        );
                    }
                }
                let file = source.open(entry).map_err(|e| e.to_string())?;
                record_stamp(&mut stamps, entry.path.clone(), Stamp::of(&file)?)?;
                approved
                    .entry(LocationKey {
                        path: entry.path.to_ascii_lowercase(),
                        offset: 0,
                        size: entry.size,
                    })
                    .or_insert(Approved {
                        hashes: BTreeSet::new(),
                        blocked: false,
                    });
            }
        }
        for location in &snapshot.resource_inspections {
            control.check()?;
            let key = LocationKey {
                path: location.source_path.to_ascii_lowercase(),
                offset: location.offset,
                size: location.size,
            };
            let value = approved
                .get_mut(&key)
                .ok_or("Snapshot inspection has no validated location")?;
            if !value.hashes.is_empty()
                && location
                    .sha1
                    .as_ref()
                    .is_none_or(|s| !value.hashes.contains(&s.to_ascii_lowercase()))
            {
                return Err("Inspection hash differs from archive entry".into());
            }
            value.blocked |= location.inspection.status == "blocked_encrypted";
        }
        let attached = Self {
            source,
            stamps,
            approved,
            validation_bytes: control.bytes_read.load(Ordering::Relaxed),
            archive_count: snapshot.archives.len(),
            entry_count,
        };
        attached.check_stable_checked(control)?;
        Ok(attached)
    }
    pub fn root(&self) -> String {
        self.source.game_root.to_string_lossy().into_owned()
    }
    fn open_stable(&self, path: &str) -> Result<File, String> {
        let entry = source_file(&self.source, path)?;
        let file = self.source.open(entry).map_err(|e| e.to_string())?;
        if self.stamps.get(&entry.path) != Some(&Stamp::of(&file)?) {
            return Err(
                "Attached source changed; detach and regenerate/revalidate snapshot".into(),
            );
        }
        Ok(file)
    }
    pub fn check_stable(&self) -> Result<(), String> {
        self.check_stable_checked(&LoadControl::default())
    }
    fn check_stable_checked(&self, control: &LoadControl) -> Result<(), String> {
        for path in self.stamps.keys() {
            control.check()?;
            self.open_stable(path)?;
        }
        Ok(())
    }
    pub fn read_hex(
        &self,
        key: &LocationKey,
        relative: u64,
        count: usize,
    ) -> Result<HexPage, String> {
        self.read_hex_checked(key, relative, count, &LoadControl::default())
    }
    pub fn read_hex_checked(
        &self,
        key: &LocationKey,
        relative: u64,
        count: usize,
        control: &LoadControl,
    ) -> Result<HexPage, String> {
        control.check()?;
        if count == 0 || count > HEX_PAGE_BYTES {
            return Err("Hex page size must be 1..4096".into());
        }
        let lookup = LocationKey {
            path: key.path.to_ascii_lowercase(),
            offset: key.offset,
            size: key.size,
        };
        let approved = self
            .approved
            .get(&lookup)
            .ok_or("Location was not validated during attachment")?;
        if approved.blocked {
            return Err("Protected resource; no byte preview is permitted".into());
        }
        if relative > key.size {
            return Err("Hex offset lies outside selected resource".into());
        }
        self.check_stable_checked(control)?;
        let mut file = self.open_stable(&key.path)?;
        let before = Stamp::of(&file)?;
        let end = key
            .offset
            .checked_add(key.size)
            .ok_or("Resource range overflow")?;
        if end > before.size {
            return Err("Resource range exceeds source file".into());
        }
        let mut prefix = [0; 4];
        let prefix_size = key.size.min(4) as usize;
        file.seek(SeekFrom::Start(key.offset))
            .map_err(|e| e.to_string())?;
        file.read_exact(&mut prefix[..prefix_size])
            .map_err(|e| e.to_string())?;
        if lbp_resource::is_protected_prefix(&prefix[..prefix_size]) {
            return Err("Protected marker; stopped after prefix, no byte preview".into());
        }
        let absolute = key
            .offset
            .checked_add(relative)
            .ok_or("Hex offset overflow")?;
        let size = (key.size - relative).min(count as u64) as usize;
        let mut bytes = vec![0; size];
        file.seek(SeekFrom::Start(absolute))
            .map_err(|e| e.to_string())?;
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        if before != Stamp::of(&file)? {
            return Err("Source changed while reading hex page".into());
        }
        self.check_stable_checked(control)?;
        Ok(HexPage {
            relative_offset: relative,
            absolute_offset: absolute,
            resource_size: key.size,
            bytes,
        })
    }
}
