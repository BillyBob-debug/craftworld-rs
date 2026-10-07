//! Entirely authored synthetic wrapper/table recipes, no copyrighted fixtures.
use lbp_resource::{inspect, InspectionLimits};
use std::io::{self, Cursor, Read, Seek, SeekFrom};

fn binary(revision: u32, branch: u16, branch_revision: u16, entries: &[(u8, u32)]) -> Vec<u8> {
    let mut data = b"MATb".to_vec();
    data.extend(revision.to_be_bytes());
    if revision < 0x109 {
        return data;
    }
    data.extend(0_u32.to_be_bytes());
    if revision >= 0x189 {
        if revision >= 0x271 {
            data.extend(branch.to_be_bytes());
            data.extend(branch_revision.to_be_bytes());
        }
        if revision >= 0x297 || (revision == 0x272 && branch == 0x4c44 && branch_revision >= 2) {
            data.push(7);
        }
        data.push(1);
    }
    data.extend(b"BODY");
    let offset = data.len() as u32;
    data[8..12].copy_from_slice(&offset.to_be_bytes());
    data.extend((entries.len() as u32).to_be_bytes());
    for (flags, type_id) in entries {
        data.push(*flags);
        if flags & 2 != 0 {
            data.extend(u32::MAX.to_be_bytes());
        }
        if flags & 1 != 0 {
            data.extend([0xaa; 20]);
        }
        data.extend(type_id.to_be_bytes());
    }
    data
}
fn read(data: &[u8]) -> lbp_resource::Inspection {
    inspect(
        &mut Cursor::new(data),
        0,
        data.len() as u64,
        InspectionLimits::default(),
    )
}
#[test]
fn revision_boundaries_and_leerdammer_flags() {
    for revision in [
        0x108, 0x109, 0x131, 0x132, 0x188, 0x189, 0x270, 0x271, 0x297, 951, 952, 0x3f8,
    ] {
        let result = read(&binary(revision, 0, 0, &[]));
        assert_eq!(
            result.status, "metadata_parsed",
            "revision {revision}: {:?}",
            result.reason
        );
        let h = result.header.unwrap();
        assert_eq!(h.compressed, Some(revision >= 0x132));
        assert_eq!(
            h.compression_flags,
            Some(if revision >= 0x297 { 7 } else { 0 })
        );
        assert_eq!(
            result.dependency_status,
            if revision < 0x109 {
                "table_absent_in_revision"
            } else {
                "parsed"
            }
        );
    }
    for branch_revision in [1, 2] {
        let result = read(&binary(0x272, 0x4c44, branch_revision, &[]));
        assert_eq!(result.status, "metadata_parsed");
        assert_eq!(
            result.header.unwrap().compression_flags,
            Some(if branch_revision == 2 { 7 } else { 0 })
        );
    }
    assert_eq!(read(&binary(0x3f9, 0, 0, &[])).status, "unsupported");
    assert_eq!(read(&binary(900, 0x4431, 1, &[])).status, "unsupported");
}
#[test]
fn dependencies_preserve_flags_ids_nulls_duplicates_and_unknown_types() {
    let result = read(&binary(951, 0, 0, &[(3, 1), (3, 1), (0, 123456)]));
    assert_eq!(result.status, "metadata_parsed");
    assert_eq!(result.dependencies.len(), 3);
    assert_eq!(result.dependencies[0].guid.unwrap().0, u32::MAX);
    assert_eq!(result.dependencies[0].sha1.unwrap().0, [0xaa; 20]);
    assert_eq!(result.dependencies[1].index, 1);
    assert_eq!(result.dependencies[2].guid, None);
    assert_eq!(result.dependencies[2].type_id, 123456);
    assert!(result.dependencies[2].known_type.is_none());
    let mut raw = binary(951, 0, 0, &[(3, 1)]);
    let end = raw.len();
    raw[end - 28..end - 4].fill(0);
    let result = read(&raw);
    assert_eq!(result.dependencies[0].guid.unwrap().0, 0);
    assert_eq!(result.dependencies[0].sha1.unwrap().0, [0; 20]);
}
#[test]
fn truncations_bad_ranges_counts_flags_and_budgets() {
    let raw = binary(951, 0, 0, &[(3, 1)]);
    for end in 0..raw.len() {
        assert_eq!(read(&raw[..end]).status, "malformed", "end {end}");
    }
    let mut bad = raw.clone();
    bad[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(read(&bad).status, "malformed");
    let mut bad = raw.clone();
    bad[8..12].copy_from_slice(&12_u32.to_be_bytes());
    assert_eq!(read(&bad).status, "malformed");
    let mut bad = raw.clone();
    bad[17] = 2;
    assert_eq!(read(&bad).status, "malformed");
    let mut bad = raw.clone();
    bad[22..26].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(read(&bad).status, "limit_exceeded");
    let mut bad = raw.clone();
    bad[26] = 4;
    let result = read(&bad);
    assert_eq!(result.status, "unsupported");
    assert!(result.dependencies.is_empty());
    let limits = InspectionLimits {
        max_dependencies: 0,
        ..InspectionLimits::default()
    };
    assert_eq!(
        inspect(&mut Cursor::new(&raw), 0, raw.len() as u64, limits).status,
        "limit_exceeded"
    );
    let limits = InspectionLimits {
        max_metadata_bytes: 8,
        ..InspectionLimits::default()
    };
    let result = inspect(&mut Cursor::new(&raw), 0, raw.len() as u64, limits);
    assert_eq!(result.status, "limit_exceeded");
    assert_eq!(result.metadata_bytes_read, 8);
}
struct Fence {
    bytes: Cursor<Vec<u8>>,
    forbidden: std::ops::Range<u64>,
}
impl Read for Fence {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let start = self.bytes.position();
        assert!(
            start + out.len() as u64 <= self.forbidden.start || start >= self.forbidden.end,
            "payload read"
        );
        self.bytes.read(out)
    }
}
impl Seek for Fence {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        self.bytes.seek(offset)
    }
}
#[test]
fn protected_markers_stop_after_four_bytes_without_false_text_positives() {
    for magic in [b"MATe", b"NPD\0", b"SCE\0", b"\x7fPKG"] {
        let mut source = Fence {
            bytes: Cursor::new([magic.as_slice(), b"SECRET"].concat()),
            forbidden: 4..10,
        };
        let result = inspect(&mut source, 0, 10, InspectionLimits::default());
        assert_eq!(result.status, "blocked_encrypted");
        assert_eq!(result.metadata_bytes_read, 4);
    }
    for text in [b"<cre".as_slice(), b"game"] {
        assert_eq!(read(text).status, "unknown_magic");
    }
}
#[test]
fn resource_range_and_binary_payload_are_never_crossed() {
    assert_eq!(
        inspect(
            &mut Cursor::new(b"MATb"),
            u64::MAX,
            4,
            InspectionLimits::default()
        )
        .status,
        "malformed"
    );
    let raw = binary(951, 0, 0, &[(3, 1)]);
    let mut source = Fence {
        bytes: Cursor::new([b"prefix".as_slice(), &raw, b"suffix"].concat()),
        forbidden: 24..28,
    };
    let result = inspect(
        &mut source,
        6,
        raw.len() as u64,
        InspectionLimits::default(),
    );
    assert_eq!(result.status, "metadata_parsed");
    assert_eq!(result.metadata_bytes_read, raw.len() - 4);
    assert_eq!(
        inspect(&mut source, 6, 8, InspectionLimits::default()).status,
        "malformed"
    );
}
#[test]
fn texture_text_and_static_mesh_scope_are_explicit() {
    for magic in [b"GTF ", b"TEX ", b"GTFs", b"GTFS", b"MATt"] {
        let result = read(magic);
        assert_eq!(result.status, "identified_unparsed");
        assert_eq!(result.metadata_bytes_read, 4);
        assert!(result.header.is_none());
    }
    let mut raw = binary(951, 0, 0, &[(2, 7)]);
    raw[..4].copy_from_slice(b"SMHb");
    let result = read(&raw);
    assert_eq!(result.status, "metadata_partial_static_mesh");
    assert_eq!(result.header.unwrap().compressed, None);
    assert_eq!(result.dependencies.len(), 1);
}
#[test]
fn d73_gate_heads_952_1000_0x3f8_accepted_0x3f9_up_unsupported_not_corrupt() {
    // Gate proof only (D69); layout identical to head 951.
    let base = read(&binary(951, 0, 0, &[(3, 1)]));
    for revision in [952, 1000, 0x3f8] {
        let r = read(&binary(revision, 0, 0, &[(3, 1)]));
        assert_eq!(
            r.status, "metadata_parsed",
            "revision {revision}: {:?}",
            r.reason
        );
        let (h, b) = (r.header.unwrap(), base.header.clone().unwrap());
        assert_eq!(h.compression_flags, b.compression_flags);
        assert_eq!(h.compressed, b.compressed);
        assert_eq!(r.dependency_status, base.dependency_status);
    }
    for revision in [0x3f9, 0x3fa, 0x10000, u32::MAX] {
        let r = read(&binary(revision, 0, 0, &[(3, 1)]));
        assert_eq!(r.status, "unsupported", "revision {revision}");
    }
    // Lower bound unchanged: pre-0x109 revisions still have no dependency table.
    assert_eq!(
        read(&binary(0x108, 0, 0, &[])).dependency_status,
        "table_absent_in_revision"
    );
}
