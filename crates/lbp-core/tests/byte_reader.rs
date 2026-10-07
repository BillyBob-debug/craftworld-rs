//! D71 bounds tests for the shared checked cursor used by MAP/FARC readers.
use lbp_core::{ByteReader, ErrorKind};

#[test]
fn empty_input_reads_fail_without_advancing() {
    let mut reader = ByteReader::new(&[]);
    assert_eq!(reader.remaining(), 0);
    assert_eq!(reader.take(0).unwrap(), &[] as &[u8]);
    assert_eq!(reader.be_u32().unwrap_err().kind, ErrorKind::Malformed);
    assert_eq!(reader.position(), 0);
}

#[test]
fn length_overflow_and_past_end_are_malformed() {
    let data = [1_u8, 2, 3, 4, 5];
    let mut reader = ByteReader::new(&data);
    reader.take(2).unwrap();
    for length in [4, usize::MAX - 1, usize::MAX] {
        assert_eq!(reader.take(length).unwrap_err().kind, ErrorKind::Malformed);
        assert_eq!(reader.position(), 2);
    }
    assert_eq!(reader.take(3).unwrap(), &[3, 4, 5]);
    assert_eq!(reader.remaining(), 0);
    assert!(reader.hash().is_err());
    assert!(reader.be_i64().is_err());
}
