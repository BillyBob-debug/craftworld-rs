//! Authored synthetic structures, with no original game bytes.
use lbp_archive::{read_farc, read_map};
use lbp_core::{ErrorKind, ReadLimits};
use std::io::{self, Cursor, Read, Seek, SeekFrom};

fn map(paths: &[&str]) -> Vec<u8> {
    let mut bytes = 0x100_u32.to_be_bytes().to_vec();
    bytes.extend((paths.len() as u32).to_be_bytes());
    for path in paths {
        bytes.extend((path.len() as u32).to_be_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend((-1_i64).to_be_bytes());
        bytes.extend(3_u32.to_be_bytes());
        bytes.extend([0xaa; 20]);
        bytes.extend(u32::MAX.to_be_bytes());
    }
    bytes
}
fn farc(rows: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = b"abc".to_vec();
    for (offset, size) in rows {
        bytes.extend([0xaa; 20]);
        bytes.extend(offset.to_be_bytes());
        bytes.extend(size.to_be_bytes());
    }
    bytes.extend((rows.len() as u32).to_be_bytes());
    bytes.extend(b"FARC");
    bytes
}
#[test]
fn map_preserves_aliases_unsigned_ids_and_raw_timestamps() {
    let m = read_map(&map(&["a.tex", "b.tex"]), ReadLimits::default()).unwrap();
    assert_eq!(m.rows.len(), 2);
    assert_eq!(m.rows[0].sha1, m.rows[1].sha1);
    assert_eq!(m.rows[0].guid.0, u32::MAX);
    assert_eq!(m.rows[1].row_index, 1);
    assert_eq!(m.rows[0].timestamp_raw, -1);
}
#[test]
fn map_rejects_truncation_trailing_bytes_and_unsupported_revision() {
    let bytes = map(&["a.tex"]);
    for end in 0..bytes.len() {
        assert!(read_map(&bytes[..end], ReadLimits::default()).is_err());
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(read_map(&bad, ReadLimits::default()).is_err());
    bad = bytes;
    bad[..4].copy_from_slice(&0x200_u32.to_be_bytes());
    assert_eq!(
        read_map(&bad, ReadLimits::default()).unwrap_err().kind,
        ErrorKind::Unsupported
    );
}
#[test]
fn map_enforces_count_path_and_byte_budgets() {
    let bytes = map(&["a.tex", "b.tex"]);
    for limits in [
        ReadLimits {
            max_entries: 1,
            ..ReadLimits::default()
        },
        ReadLimits {
            max_path_bytes: 4,
            ..ReadLimits::default()
        },
        ReadLimits {
            max_table_bytes: bytes.len() - 1,
            ..ReadLimits::default()
        },
    ] {
        assert_eq!(
            read_map(&bytes, limits).unwrap_err().kind,
            ErrorKind::LimitExceeded
        );
    }
    let mut bad = bytes.clone();
    bad[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(read_map(&bad, ReadLimits::default()).is_err());
    bad = bytes;
    bad[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(read_map(&bad, ReadLimits::default()).is_err());
}
#[test]
fn farc_preserves_duplicates_and_rejects_bad_ranges_and_counts() {
    let bytes = farc(&[(0, 3), (0, 3)]);
    let index = read_farc(&mut Cursor::new(&bytes), ReadLimits::default()).unwrap();
    assert_eq!(index.table_offset, 3);
    assert_eq!(index.entries.len(), 2);
    assert_eq!(index.entries[1].entry_index, 1);
    assert_eq!(index.entries[0].sha1, index.entries[1].sha1);
    for rows in [vec![(0, 4)], vec![(u32::MAX, u32::MAX)]] {
        assert!(read_farc(&mut Cursor::new(farc(&rows)), ReadLimits::default()).is_err());
    }
    for end in 0..bytes.len() {
        assert!(read_farc(&mut Cursor::new(&bytes[..end]), ReadLimits::default()).is_err());
    }
    let limits = ReadLimits {
        max_entries: 1,
        ..ReadLimits::default()
    };
    assert_eq!(
        read_farc(&mut Cursor::new(&bytes), limits)
            .unwrap_err()
            .kind,
        ErrorKind::LimitExceeded
    );
    let limits = ReadLimits {
        max_table_bytes: 55,
        ..ReadLimits::default()
    };
    assert_eq!(
        read_farc(&mut Cursor::new(&bytes), limits)
            .unwrap_err()
            .kind,
        ErrorKind::LimitExceeded
    );
    assert!(read_farc(&mut Cursor::new(b"\0\0\0\x01FARC"), ReadLimits::default()).is_err());
    assert!(
        read_farc(&mut Cursor::new(b"\0\0\0\0FARC"), ReadLimits::default())
            .unwrap()
            .entries
            .is_empty()
    );
}
struct TableOnly(Cursor<Vec<u8>>);
impl Seek for TableOnly {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}
impl Read for TableOnly {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        assert!(
            self.0.position() >= 3,
            "archive payload must not be read in A1"
        );
        self.0.read(data)
    }
}
#[test]
fn farc_reads_only_footer_and_index() {
    let mut source = TableOnly(Cursor::new(farc(&[(0, 3)])));
    assert_eq!(
        read_farc(&mut source, ReadLimits::default())
            .unwrap()
            .entries
            .len(),
        1
    );
}
#[test]
fn patch_map_rows_replace_same_guid_and_keep_shadowed_rows() {
    use lbp_archive::overlay_map;
    use lbp_core::Guid;
    let limits = ReadLimits::default();
    let mut base = read_map(&map(&["a.tex", "b.tex", "c.tex"]), limits).unwrap();
    base.rows[0].guid = Guid(1);
    base.rows[1].guid = Guid(2);
    base.rows[2].guid = Guid(0);
    let mut patch = read_map(&map(&["b2.tex", "z.tex", "n.tex"]), limits).unwrap();
    patch.rows[0].guid = Guid(2);
    patch.rows[1].guid = Guid(9);
    patch.rows[2].guid = Guid(0);
    let merged = overlay_map(base, patch).unwrap();
    let paths: Vec<_> = merged.rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, ["a.tex", "c.tex", "b2.tex", "z.tex", "n.tex"]);
    let indices: Vec<_> = merged.rows.iter().map(|r| r.row_index).collect();
    assert_eq!(indices, [0, 2, 3, 4, 5]);
    assert_eq!(merged.shadowed.len(), 1);
    assert_eq!(merged.shadowed[0].path, "b.tex");
}
