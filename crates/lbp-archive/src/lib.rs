//! Bounded readers for the locally corroborated LBP2 MAP 0x100 and FARC variants.
//!
//! Format knowledge: CONFIRMED_FROM_COMMUNITY_CODE and CONFIRMED_FROM_LOCAL_DATA.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9
//! lib/cwlib/src/main/java/cwlib/types/databases/FileDB.java::process
//! lib/cwlib/src/main/java/cwlib/types/archives/FileArchive.java constructor
//! Independent implementation of observed layouts; see docs/formats.md and notices.
//! Unknown revisions/encrypted containers are rejected; no payload decoding/writes.
use lbp_core::{ByteReader, ContentHash, Error, ErrorKind, Guid, ReadLimits, Result};
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Serialize)]
pub struct MapRow {
    pub row_index: usize,
    pub path: String,
    pub timestamp_raw: i64,
    pub declared_size: u32,
    pub sha1: ContentHash,
    pub guid: Guid,
}
#[derive(Debug)]
pub struct FileMap {
    pub revision: u32,
    pub rows: Vec<MapRow>,
}

pub fn read_map(data: &[u8], limits: ReadLimits) -> Result<FileMap> {
    if data.len() > limits.max_table_bytes {
        return Err(Error::limit("MAP byte budget exceeded"));
    }
    let mut reader = ByteReader::new(data);
    let revision = reader.be_u32()?;
    if revision != 0x100 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!("MAP revision {revision:#x}; only 0x100 is supported"),
        ));
    }
    let count = reader.be_u32()? as usize;
    if count > limits.max_entries {
        return Err(Error::limit("MAP entry budget exceeded"));
    }
    if count > reader.remaining() / 40 {
        return Err(Error::malformed("MAP count exceeds minimum row capacity"));
    }
    let mut rows = Vec::with_capacity(count);
    for row_index in 0..count {
        let length = reader.be_u32()? as usize;
        if length > limits.max_path_bytes {
            return Err(Error::limit(format!(
                "MAP row {row_index} path byte budget exceeded"
            )));
        }
        let path = std::str::from_utf8(reader.take(length)?)
            .map_err(|_| Error::malformed(format!("MAP row {row_index} path is not UTF-8")))?
            .to_owned();
        let timestamp_raw = reader.be_i64()?;
        let declared_size = reader.be_u32()?;
        let sha1 = reader.hash()?;
        let guid = Guid(reader.be_u32()?);
        rows.push(MapRow {
            row_index,
            path,
            timestamp_raw,
            declared_size,
            sha1,
            guid,
        });
    }
    if reader.remaining() != 0 {
        return Err(Error::malformed("unexpected bytes after MAP rows"));
    }
    Ok(FileMap { revision, rows })
}

/// Base MAP with an installed update's MAP (`USRDIR/output/brg_patch.map`) applied.
#[derive(Debug)]
pub struct OverlayMap {
    /// Effective rows: unshadowed base rows, then patch rows renumbered after the base.
    pub rows: Vec<MapRow>,
    /// Base rows replaced by a patch row with the same GUID (preserved, not dropped).
    pub shadowed: Vec<MapRow>,
}

/// Each patch row replaces every base row with the same non-null GUID.
/// STRONG_INFERENCE from local BCUS98245 01.33 data (1,014 same-GUID rows with new
/// hashes); no community reader handles the patch MAP. See docs/update-overlay.md.
pub fn overlay_map(base: FileMap, patch: FileMap) -> Result<OverlayMap> {
    if base.revision != patch.revision {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "patch MAP revision differs from base MAP",
        ));
    }
    let offset = base.rows.len();
    let patched: std::collections::BTreeSet<Guid> = patch
        .rows
        .iter()
        .map(|r| r.guid)
        .filter(|g| g.0 != 0)
        .collect();
    let (shadowed, mut rows): (Vec<_>, Vec<_>) = base
        .rows
        .into_iter()
        .partition(|r| patched.contains(&r.guid));
    for mut row in patch.rows {
        row.row_index = row
            .row_index
            .checked_add(offset)
            .ok_or_else(|| Error::limit("MAP row index overflow"))?;
        rows.push(row);
    }
    Ok(OverlayMap { rows, shadowed })
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveEntry {
    pub entry_index: usize,
    pub sha1: ContentHash,
    pub offset: u64,
    pub size: u32,
}
#[derive(Debug)]
pub struct ArchiveIndex {
    pub file_size: u64,
    pub table_offset: u64,
    pub entries: Vec<ArchiveEntry>,
}

pub fn read_farc<R: Read + Seek>(input: &mut R, limits: ReadLimits) -> Result<ArchiveIndex> {
    let file_size = input.seek(SeekFrom::End(0))?;
    if file_size < 8 {
        return Err(Error::malformed("FARC is shorter than its footer"));
    }
    input.seek(SeekFrom::End(-8))?;
    let mut footer = [0; 8];
    input.read_exact(&mut footer)?;
    if &footer[4..] != b"FARC" {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "FARC footer magic absent; no alternate format or decryption attempted",
        ));
    }
    let count = u32::from_be_bytes(footer[..4].try_into().unwrap()) as usize;
    if count > limits.max_entries {
        return Err(Error::limit("FARC entry budget exceeded"));
    }
    let table_bytes = count
        .checked_mul(28)
        .ok_or_else(|| Error::malformed("FARC table size overflow"))?;
    if table_bytes > limits.max_table_bytes {
        return Err(Error::limit("FARC table byte budget exceeded"));
    }
    let table_offset = file_size
        .checked_sub(8)
        .and_then(|n| n.checked_sub(table_bytes as u64))
        .ok_or_else(|| Error::malformed("FARC table begins before file"))?;
    input.seek(SeekFrom::Start(table_offset))?;
    let mut table = vec![0; table_bytes];
    input.read_exact(&mut table)?;
    let mut reader = ByteReader::new(&table);
    let mut entries = Vec::with_capacity(count);
    for entry_index in 0..count {
        let sha1 = reader.hash()?;
        let offset = reader.be_u32()? as u64;
        let size = reader.be_u32()?;
        let end = offset
            .checked_add(u64::from(size))
            .ok_or_else(|| Error::malformed("FARC payload range overflow"))?;
        if end > table_offset {
            return Err(Error::malformed(format!(
                "FARC entry {entry_index} crosses archive table"
            )));
        }
        entries.push(ArchiveEntry {
            entry_index,
            sha1,
            offset,
            size,
        });
    }
    Ok(ArchiveIndex {
        file_size,
        table_offset,
        entries,
    })
}
