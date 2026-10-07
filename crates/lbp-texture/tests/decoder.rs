use lbp_texture::{decode, decode_checked, inspect, Error};
use std::io::Write;
fn wrapper(w: u16, h: u16, data: &[u8], compressed: bool) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(b"GTF ");
    b[4] = 0x86;
    b[5] = 1;
    b[6] = 2;
    b[8..12].copy_from_slice(&0xaae4u32.to_be_bytes());
    b[12..14].copy_from_slice(&w.to_be_bytes());
    b[14..16].copy_from_slice(&h.to_be_bytes());
    b[17] = 1;
    if compressed {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(data).unwrap();
        let c = z.finish().unwrap();
        b[29] = 1;
        b[31] = 1;
        b.extend_from_slice(&(c.len() as u16).to_be_bytes());
        b.extend_from_slice(&(data.len() as u16).to_be_bytes());
        b.extend(c);
    } else {
        b.extend(data);
    }
    b
}
fn block(a: u16, b: u16, indices: u32) -> Vec<u8> {
    [
        a.to_le_bytes().as_slice(),
        b.to_le_bytes().as_slice(),
        indices.to_le_bytes().as_slice(),
    ]
    .concat()
}
#[test]
fn palette_order_and_transparency() {
    let t = decode(&wrapper(4, 4, &block(0xf800, 0x07e0, 0xe4e4e4e4), false)).unwrap();
    assert_eq!(
        &t.rgba[..16],
        &[255, 0, 0, 255, 0, 255, 0, 255, 170, 85, 0, 255, 85, 170, 0, 255]
    );
    for (a, b) in [(0x07e0, 0xf800), (0x07e0, 0x07e0)] {
        let t = decode(&wrapper(1, 1, &block(a, b, 3), false)).unwrap();
        assert_eq!(t.rgba, [0, 0, 0, 0]);
    }
}
#[test]
fn endpoint_low_bits_and_rounding() {
    let t = decode(&wrapper(4, 1, &block(0x18e3, 0, 0xe4), false)).unwrap();
    assert_eq!(
        &t.rgba[..16],
        &[25, 28, 25, 255, 0, 0, 0, 255, 16, 18, 16, 255, 8, 9, 8, 255]
    );
}
#[test]
fn rectangular_partial_blocks_and_orientation() {
    let blocks = [
        block(0xf800, 0, 0),
        block(0x07e0, 0, 0),
        block(0x001f, 0, 0),
        block(0xffff, 0, 0),
    ]
    .concat();
    let t = decode(&wrapper(7, 5, &blocks, true)).unwrap();
    assert_eq!(t.rgba.len(), 7 * 5 * 4);
    for (x, y, c) in [
        (0, 0, [255, 0, 0, 255]),
        (6, 0, [0, 255, 0, 255]),
        (0, 4, [0, 0, 255, 255]),
        (6, 4, [255; 4]),
    ] {
        assert_eq!(&t.rgba[(y * 7 + x) * 4..(y * 7 + x + 1) * 4], &c);
    }
}
#[test]
fn stored_mixed_chunks_and_unknown_tail() {
    let data = block(0xf800, 0, 0);
    let compressed = wrapper(1, 1, &data, true);
    // Compressed block then a stored opaque tail; do not assume what the tail means.
    let mut b2 = compressed[..32].to_vec();
    b2[31] = 2;
    b2.extend_from_slice(&compressed[32..36]);
    b2.extend_from_slice(&[0, 3, 0, 3]);
    b2.extend_from_slice(&compressed[36..]);
    b2.extend_from_slice(b"xyz");
    let t = decode(&b2).unwrap();
    assert_eq!(t.unparsed_after_top_mip, 3);
    assert_eq!(t.rgba, [255, 0, 0, 255]);
    let mut stored = wrapper(1, 1, &[], false);
    stored[31] = 1;
    stored.extend_from_slice(&[0, 8, 0, 8]);
    stored.extend(data);
    assert_eq!(decode(&stored).unwrap().rgba, [255, 0, 0, 255]);
}
#[test]
fn reader_preflight_limits_and_mid_read_cancellation() {
    use std::cell::Cell;
    let calls = Cell::new(0);
    let unsupported = wrapper(1, 1, &block(0xf800, 0, 0), false);
    let mut unsupported = unsupported;
    unsupported[4] = 0x87;
    assert!(matches!(
        lbp_texture::decode_reader(
            unsupported.len() as u64,
            |p, n| {
                calls.set(calls.get() + 1);
                Ok(unsupported[p as usize..p as usize + n].to_vec())
            },
            || false
        ),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(calls.get(), 1);
    let mut data = block(0xf800, 0, 0);
    data.resize(10000, 0);
    let b = wrapper(1, 1, &data, false);
    calls.set(0);
    assert_eq!(
        lbp_texture::decode_reader(
            b.len() as u64,
            |p, n| {
                assert!(n <= 4096);
                calls.set(calls.get() + 1);
                Ok(b[p as usize..p as usize + n].to_vec())
            },
            || calls.get() >= 2
        )
        .unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(calls.get(), 2);
    let mut b = wrapper(1, 1, &[], false);
    b[30..32].copy_from_slice(&513u16.to_be_bytes());
    for _ in 0..513 {
        b.extend_from_slice(&[0, 1, 255, 255]);
    }
    b.resize(b.len() + 513, 0);
    assert_eq!(decode(&b).unwrap_err(), Error::Budget);
}
#[test]
fn truncation_lengths_and_trailing_zlib_are_rejected() {
    let mut b = wrapper(4, 4, &block(0xf800, 0, 0), true);
    for n in 0..b.len() {
        assert!(decode(&b[..n]).is_err(), "prefix {n}");
    }
    b[35] = 9;
    assert!(matches!(decode(&b), Err(Error::Invalid(_))));
    b[35] = 8;
    b.push(0);
    let s = u16::from_be_bytes([b[32], b[33]]) + 1;
    b[32..34].copy_from_slice(&s.to_be_bytes());
    assert!(matches!(decode(&b), Err(Error::Invalid(_))));
}
#[test]
fn unsupported_protection_budgets_and_cancel() {
    assert_eq!(inspect(b"GTFe").unwrap_err(), Error::Blocked);
    assert_eq!(inspect(b"NPD\0").unwrap_err(), Error::Blocked);
    let base = wrapper(1, 1, &block(0xf800, 0, 0), false);
    for (p, value) in [(4, 0x87), (6, 3), (7, 1), (19, 4), (20, 1), (24, 1), (8, 1)] {
        let mut b = base.clone();
        b[p] = value;
        assert!(matches!(decode(&b), Err(Error::Unsupported(_))));
    }
    assert_eq!(
        decode_checked(&base, || true).unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(
        decode(&wrapper(4096, 4096, &[], false)).unwrap_err(),
        Error::Budget
    );
    assert!(matches!(
        decode(&wrapper(0, 1, &[], false)),
        Err(Error::Invalid(_))
    ));
}
