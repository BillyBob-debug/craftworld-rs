//! D71 bounds and mutation tests for the PARAM.SFO identity reader.
//! All bytes are authored here; no original SFO contents.
use lbp_core::ErrorKind;
use lbp_io::sfo::read_identity;

struct Field<'a> {
    key: &'a [u8],
    format: u16,
    value: &'a [u8],
    capacity: u32,
}

/// Builds header (20 bytes), 16-byte index records, key table, value table.
fn sfo(fields: &[Field]) -> Vec<u8> {
    let mut keys = Vec::new();
    let mut key_offsets = Vec::new();
    for field in fields {
        key_offsets.push(keys.len() as u16);
        keys.extend(field.key);
        keys.push(0);
    }
    while keys.len() % 4 != 0 {
        keys.push(0);
    }
    let key_table = 20 + 16 * fields.len();
    let value_table = key_table + keys.len();
    let mut values = Vec::new();
    let mut index = Vec::new();
    for (field, key_offset) in fields.iter().zip(key_offsets) {
        index.extend(key_offset.to_le_bytes());
        index.extend(field.format.to_le_bytes());
        index.extend((field.value.len() as u32).to_le_bytes());
        index.extend(field.capacity.to_le_bytes());
        index.extend((values.len() as u32).to_le_bytes());
        let start = values.len();
        values.extend(field.value);
        values.resize(start + field.capacity as usize, 0);
    }
    let mut bytes = b"\0PSF".to_vec();
    for n in [
        0x101_u32,
        key_table as u32,
        value_table as u32,
        fields.len() as u32,
    ] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(index);
    bytes.extend(keys);
    bytes.extend(values);
    bytes
}

fn text<'a>(key: &'a [u8], value: &'a [u8]) -> Field<'a> {
    Field {
        key,
        format: 0x204,
        value,
        capacity: 16,
    }
}

fn valid() -> Vec<u8> {
    sfo(&[
        Field {
            key: b"ATTRIBUTE",
            format: 0x404,
            value: &[1, 0, 0, 0],
            capacity: 4,
        },
        text(b"TITLE_ID", b"SYNT00001\0"),
        text(b"APP_VER", b"01.00\0"),
        Field {
            key: b"CATEGORY",
            format: 0x4,
            value: b"DG",
            capacity: 4,
        },
    ])
}

fn malformed(bytes: &[u8]) {
    assert_eq!(read_identity(bytes).unwrap_err().kind, ErrorKind::Malformed);
}

fn set_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn synthetic_sfo_reads_identity() {
    let identity = read_identity(&valid()).unwrap();
    assert_eq!(identity.title_id.as_deref(), Some("SYNT00001"));
    assert_eq!(identity.app_version.as_deref(), Some("01.00"));
    assert_eq!(identity.category.as_deref(), Some("DG"));
    assert_eq!(identity.title, None);
}

#[test]
fn empty_and_truncated_header_are_malformed() {
    malformed(&[]);
    let bytes = valid();
    for end in 0..20 {
        malformed(&bytes[..end]);
    }
    for end in 20..bytes.len() {
        assert!(read_identity(&bytes[..end]).is_err(), "end {end}");
    }
}

#[test]
fn zero_entries_is_accepted() {
    let identity = read_identity(&sfo(&[])).unwrap();
    assert!(identity.title_id.is_none());
}

#[test]
fn entry_count_overflow_and_budget() {
    let mut bytes = valid();
    for count in [4097_u32, 0x1000_0000, 0x8000_0000, u32::MAX] {
        set_u32(&mut bytes, 16, count);
        assert_eq!(
            read_identity(&bytes).unwrap_err().kind,
            ErrorKind::LimitExceeded
        );
    }
    // Within budget but the index would overlap the key table.
    let mut bytes = valid();
    set_u32(&mut bytes, 16, 4096);
    malformed(&bytes);
}

#[test]
fn table_offsets_out_of_range_are_malformed() {
    let base = valid();
    let len = base.len() as u32;
    // (offset field, value): key table inside index, value table before keys,
    // value table past end, and wrap-sized values.
    for (field, value) in [
        (8, 0),
        (8, 19),
        (8, 20 + 16 * 4 - 1),
        (12, 0),
        (12, len + 1),
        (12, u32::MAX),
        (8, u32::MAX),
    ] {
        let mut bytes = base.clone();
        set_u32(&mut bytes, field, value);
        malformed(&bytes);
    }
}

#[test]
fn key_and_value_offsets_out_of_range_are_malformed() {
    let base = valid();
    // Record 0 starts at 20: key u16 @20, format @22, len @24, cap @28, off @32.
    let mut bytes = base.clone();
    bytes[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
    malformed(&bytes);
    for (at, value) in [
        (32, u32::MAX),
        (32, 0x8000_0000),
        (28, u32::MAX),
        (24, u32::MAX),
    ] {
        let mut bytes = base.clone();
        set_u32(&mut bytes, at, value);
        malformed(&bytes);
    }
    // Capacity that runs one byte past the file.
    let mut bytes = base;
    let last = 20 + 16 * 3;
    let cap = u32::from_le_bytes(bytes[last + 8..last + 12].try_into().unwrap());
    set_u32(&mut bytes, last + 8, cap + 1);
    malformed(&bytes);
}

#[test]
fn key_without_terminator_is_malformed() {
    let mut bytes = sfo(&[text(b"TITLE_ID", b"X\0")]);
    let keys = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let values = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    bytes[keys..values].fill(b'A');
    malformed(&bytes);
}

#[test]
fn non_utf8_key_is_malformed() {
    malformed(&sfo(&[text(b"TITLE\xff", b"X\0")]));
    malformed(&sfo(&[text(b"\xc3", b"X\0")]));
}

#[test]
fn duplicate_key_is_malformed() {
    malformed(&sfo(&[
        text(b"TITLE_ID", b"A\0"),
        text(b"TITLE_ID", b"B\0"),
    ]));
}

#[test]
fn zero_length_values() {
    // A zero-length unrelated value is fine; a 0x204 identity text needs its NUL.
    let identity = read_identity(&sfo(&[Field {
        key: b"OTHER",
        format: 0x4,
        value: b"",
        capacity: 0,
    }]))
    .unwrap();
    assert!(identity.title_id.is_none());
    malformed(&sfo(&[Field {
        key: b"TITLE_ID",
        format: 0x204,
        value: b"",
        capacity: 0,
    }]));
    let identity = read_identity(&sfo(&[Field {
        key: b"TITLE_ID",
        format: 0x4,
        value: b"",
        capacity: 0,
    }]))
    .unwrap();
    assert_eq!(identity.title_id.as_deref(), Some(""));
}

#[test]
fn identity_text_errors() {
    malformed(&sfo(&[text(b"TITLE", b"\xff\xfe\0")]));
    malformed(&sfo(&[text(b"TITLE", b"a\0b\0")]));
    assert_eq!(
        read_identity(&sfo(&[Field {
            key: b"TITLE",
            format: 0x404,
            value: &[0; 4],
            capacity: 4,
        }]))
        .unwrap_err()
        .kind,
        ErrorKind::Unsupported
    );
}

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

#[test]
fn sfo_mutations_never_panic() {
    let valid = valid();
    let mut rng = XorShift(0x0d71_0000_5eed_0003);
    let mut accepted = 0;
    for _ in 0..4000 {
        let mut bytes = valid.clone();
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
                    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
                }
                3 => bytes.truncate(rng.below(bytes.len() + 1)),
                _ => {
                    let extra = rng.below(8);
                    bytes.extend((0..extra).map(|_| rng.next() as u8));
                }
            }
        }
        if read_identity(&bytes).is_ok() {
            accepted += 1;
        }
    }
    // Sanity: the mutator is not so destructive that nothing parses.
    assert!(accepted > 0);
}
