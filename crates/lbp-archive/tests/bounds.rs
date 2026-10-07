//! D71 bounds and mutation tests for MAP and FARC readers.
//! All bytes are authored here; no original game bytes.
use lbp_archive::{overlay_map, read_farc, read_map};
use lbp_core::{ErrorKind, Guid, ReadLimits};
use std::io::Cursor;

const ROW_GUIDS: [u32; 3] = [7, 7, 0];

fn map_rows(rows: &[(&str, u32)]) -> Vec<u8> {
    let mut bytes = 0x100_u32.to_be_bytes().to_vec();
    bytes.extend((rows.len() as u32).to_be_bytes());
    for (path, guid) in rows {
        bytes.extend((path.len() as u32).to_be_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend(1_i64.to_be_bytes());
        bytes.extend(5_u32.to_be_bytes());
        bytes.extend([0x11; 20]);
        bytes.extend(guid.to_be_bytes());
    }
    bytes
}

fn farc_with(payload: &[u8], rows: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = payload.to_vec();
    for (index, (offset, size)) in rows.iter().enumerate() {
        bytes.extend([index as u8; 20]);
        bytes.extend(offset.to_be_bytes());
        bytes.extend(size.to_be_bytes());
    }
    bytes.extend((rows.len() as u32).to_be_bytes());
    bytes.extend(b"FARC");
    bytes
}

fn farc(input: &[u8], limits: ReadLimits) -> lbp_core::Result<lbp_archive::ArchiveIndex> {
    read_farc(&mut Cursor::new(input), limits)
}

fn open_limits() -> ReadLimits {
    ReadLimits {
        max_table_bytes: usize::MAX,
        max_entries: usize::MAX,
        max_path_bytes: usize::MAX,
    }
}

/// Fixed-seed xorshift64; deterministic across runs and platforms.
struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Byte flips, boundary u32 overwrites, truncations and extensions of `valid`.
fn mutate(rng: &mut XorShift, valid: &[u8]) -> Vec<u8> {
    let mut bytes = valid.to_vec();
    for _ in 0..1 + rng.below(4) {
        match rng.below(5) {
            0 if !bytes.is_empty() => {
                let at = rng.below(bytes.len());
                bytes[at] ^= 1 << rng.below(8);
            }
            1 if !bytes.is_empty() => {
                let at = rng.below(bytes.len());
                bytes[at] = rng.next() as u8;
            }
            2 if bytes.len() >= 4 => {
                let at = rng.below(bytes.len() - 3);
                let value =
                    [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX, rng.next() as u32][rng.below(6)];
                bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
            }
            3 => bytes.truncate(rng.below(bytes.len() + 1)),
            _ => {
                let extra = rng.below(8);
                bytes.extend((0..extra).map(|_| rng.next() as u8));
            }
        }
    }
    bytes
}

// ---------------------------------------------------------------- MAP

#[test]
fn map_empty_input_and_header_only() {
    assert!(read_map(&[], ReadLimits::default()).is_err());
    let header = map_rows(&[]);
    assert_eq!(header.len(), 8);
    assert!(read_map(&header, ReadLimits::default())
        .unwrap()
        .rows
        .is_empty());
    for end in 0..header.len() {
        assert!(read_map(&header[..end], ReadLimits::default()).is_err());
    }
}

#[test]
fn map_huge_count_is_rejected_without_allocation() {
    let mut bytes = map_rows(&[("a", 1)]);
    for count in [2_u32, 0x0666_6667, 0x8000_0000, u32::MAX] {
        bytes[4..8].copy_from_slice(&count.to_be_bytes());
        let error = read_map(&bytes, open_limits()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Malformed, "count {count:#x}");
    }
}

#[test]
fn map_path_length_past_end_and_wrapping_are_rejected() {
    let valid = map_rows(&[("abc", 1)]);
    for length in [4_u32, 41, 0x7fff_ffff, u32::MAX - 1, u32::MAX] {
        let mut bad = valid.clone();
        bad[8..12].copy_from_slice(&length.to_be_bytes());
        assert!(read_map(&bad, open_limits()).is_err(), "length {length:#x}");
    }
}

#[test]
fn map_zero_length_path_is_preserved() {
    let parsed = read_map(&map_rows(&[("", 3)]), ReadLimits::default()).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].path, "");
    assert_eq!(parsed.rows[0].guid, Guid(3));
}

#[test]
fn map_non_utf8_path_is_malformed() {
    let mut bytes = map_rows(&[("ab", 1)]);
    bytes[12] = 0xff;
    assert_eq!(
        read_map(&bytes, ReadLimits::default()).unwrap_err().kind,
        ErrorKind::Malformed
    );
}

#[test]
fn map_duplicate_aliases_are_all_preserved_in_order() {
    let bytes = map_rows(&[("same.tex", 7), ("same.tex", 7), ("other.tex", 7)]);
    let parsed = read_map(&bytes, ReadLimits::default()).unwrap();
    assert_eq!(parsed.rows.len(), 3);
    let paths: Vec<_> = parsed.rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, ["same.tex", "same.tex", "other.tex"]);
    let indices: Vec<_> = parsed.rows.iter().map(|r| r.row_index).collect();
    assert_eq!(indices, [0, 1, 2]);
    assert!(parsed.rows.iter().all(|r| r.guid == Guid(7)));
}

#[test]
fn overlay_keeps_duplicate_base_aliases_as_shadowed_and_patch_duplicates() {
    let limits = ReadLimits::default();
    let base = read_map(
        &map_rows(&[
            ("a.tex", ROW_GUIDS[0]),
            ("a2.tex", ROW_GUIDS[1]),
            ("n.tex", 0),
        ]),
        limits,
    )
    .unwrap();
    let patch = read_map(
        &map_rows(&[("p.tex", 7), ("p.tex", 7), ("z.tex", 0)]),
        limits,
    )
    .unwrap();
    let merged = overlay_map(base, patch).unwrap();
    let shadowed: Vec<_> = merged.shadowed.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(shadowed, ["a.tex", "a2.tex"]);
    let paths: Vec<_> = merged.rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, ["n.tex", "p.tex", "p.tex", "z.tex"]);
    let indices: Vec<_> = merged.rows.iter().map(|r| r.row_index).collect();
    assert_eq!(indices, [2, 3, 4, 5]);
}

#[test]
fn map_mutations_never_panic() {
    let valid = map_rows(&[("a.tex", 1), ("dir/b.tex", 1), ("", 0)]);
    assert_eq!(
        read_map(&valid, ReadLimits::default()).unwrap().rows.len(),
        3
    );
    let mut rng = XorShift(0x0d71_0000_5eed_0001);
    for iteration in 0..4000 {
        let bytes = mutate(&mut rng, &valid);
        let limits = if iteration % 2 == 0 {
            ReadLimits::default()
        } else {
            open_limits()
        };
        if let Ok(parsed) = read_map(&bytes, limits) {
            assert_eq!(parsed.revision, 0x100);
            for (index, row) in parsed.rows.iter().enumerate() {
                assert_eq!(row.row_index, index);
            }
        }
    }
}

// ---------------------------------------------------------------- FARC

#[test]
fn farc_empty_and_short_inputs_are_rejected() {
    assert_eq!(
        farc(&[], ReadLimits::default()).unwrap_err().kind,
        ErrorKind::Malformed
    );
    for end in 1..8 {
        assert!(farc(&b"\0\0\0\0FARC"[..end], ReadLimits::default()).is_err());
    }
    // Footer present but magic altered.
    assert_eq!(
        farc(b"\0\0\0\0FARX", ReadLimits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Unsupported
    );
}

#[test]
fn farc_count_larger_than_file_is_rejected_without_overflow() {
    for count in [1_u32, 0x0924_9249, 0x0924_924a, 0x8000_0000, u32::MAX] {
        let mut bytes = farc_with(b"xy", &[]);
        let at = bytes.len() - 8;
        bytes[at..at + 4].copy_from_slice(&count.to_be_bytes());
        assert!(farc(&bytes, open_limits()).is_err(), "count {count:#x}");
        assert!(
            farc(&bytes, ReadLimits::default()).is_err(),
            "count {count:#x}"
        );
    }
}

#[test]
fn farc_ranges_past_table_and_wrapping_are_rejected() {
    let payload = [0_u8; 16];
    for (offset, size) in [
        (0, 17),
        (16, 1),
        (17, 0),
        (u32::MAX, 0),
        (u32::MAX, 1),
        (1, u32::MAX),
        (u32::MAX, u32::MAX),
        (0x8000_0000, 0x8000_0000),
    ] {
        let bytes = farc_with(&payload, &[(offset, size)]);
        assert_eq!(
            farc(&bytes, ReadLimits::default()).unwrap_err().kind,
            ErrorKind::Malformed,
            "offset {offset:#x} size {size:#x}"
        );
    }
}

#[test]
fn farc_zero_length_and_boundary_entries_are_accepted() {
    let payload = [0_u8; 16];
    let bytes = farc_with(&payload, &[(0, 0), (16, 0), (0, 16), (15, 1)]);
    let index = farc(&bytes, ReadLimits::default()).unwrap();
    assert_eq!(index.table_offset, 16);
    assert_eq!(index.file_size, bytes.len() as u64);
    let ranges: Vec<_> = index.entries.iter().map(|e| (e.offset, e.size)).collect();
    assert_eq!(ranges, [(0, 0), (16, 0), (0, 16), (15, 1)]);
}

#[test]
fn farc_duplicate_and_overlapping_entries_are_preserved() {
    let payload = [0_u8; 8];
    let mut bytes = farc_with(&payload, &[(0, 4), (0, 4), (2, 6)]);
    // Make all hashes identical.
    for row in 0..3 {
        let at = 8 + row * 28;
        bytes[at..at + 20].copy_from_slice(&[0x5a; 20]);
    }
    let index = farc(&bytes, ReadLimits::default()).unwrap();
    assert_eq!(index.entries.len(), 3);
    assert!(index
        .entries
        .iter()
        .all(|e| e.sha1 == index.entries[0].sha1));
    let indices: Vec<_> = index.entries.iter().map(|e| e.entry_index).collect();
    assert_eq!(indices, [0, 1, 2]);
}

#[test]
fn farc_mutations_never_panic() {
    let valid = farc_with(b"payload-bytes", &[(0, 7), (7, 6), (0, 0), (0, 13)]);
    assert_eq!(
        farc(&valid, ReadLimits::default()).unwrap().entries.len(),
        4
    );
    let mut rng = XorShift(0x0d71_0000_5eed_0002);
    for iteration in 0..4000 {
        let bytes = mutate(&mut rng, &valid);
        let limits = if iteration % 2 == 0 {
            ReadLimits::default()
        } else {
            open_limits()
        };
        if let Ok(index) = farc(&bytes, limits) {
            assert_eq!(index.file_size, bytes.len() as u64);
            let table_end = index.table_offset + 28 * index.entries.len() as u64 + 8;
            assert_eq!(table_end, index.file_size);
            for entry in &index.entries {
                assert!(entry.offset + u64::from(entry.size) <= index.table_offset);
            }
        }
    }
}
