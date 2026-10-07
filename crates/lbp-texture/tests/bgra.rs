//! Authored standard-mask DDS recipes, no game bytes. docs/bgra-preview.md.
use lbp_texture::{decode, decode_checked, decode_reader, Error, PixelFormat};
use std::{cell::Cell, io::Write};
const PIXELS: [[u8; 4]; 6] = [
    [1, 23, 45, 0],
    [67, 89, 101, 127],
    [123, 145, 167, 255],
    [189, 211, 233, 64],
    [254, 32, 16, 128],
    [9, 8, 7, 1],
];
fn put(b: &mut [u8], p: usize, n: u32) {
    b[p..p + 4].copy_from_slice(&n.to_le_bytes());
}
fn dds(pitch: usize) -> Vec<u8> {
    let mut b = vec![0; 128];
    b[..4].copy_from_slice(b"DDS ");
    for (p, n) in [
        (4, 124),
        (8, 0x100f),
        (12, 2),
        (16, 3),
        (20, pitch as u32),
        (76, 32),
        (80, 0x41),
        (88, 32),
        (92, 0x00ff0000),
        (96, 0x0000ff00),
        (100, 0xff),
        (104, 0xff000000),
        (108, 0x1000),
    ] {
        put(&mut b, p, n);
    }
    for row in PIXELS.chunks(3) {
        for p in row {
            b.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
        b.extend(std::iter::repeat_n(0xee, pitch - 12));
    }
    b
}
fn raw(b: &[u8]) -> Vec<u8> {
    [b"TEX \0\0\0\0".as_slice(), b].concat()
}
fn compressed(b: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
    z.write_all(b).unwrap();
    let data = z.finish().unwrap();
    assert_ne!(data.len(), b.len());
    [
        b"TEX \0\x01\0\x01".as_slice(),
        &(data.len() as u16).to_be_bytes(),
        &(b.len() as u16).to_be_bytes(),
        &data,
    ]
    .concat()
}
#[test]
fn distinctive_channels_alpha_padded_rows_raw_and_zlib_match() {
    for pitch in [12, 16] {
        let bytes = dds(pitch);
        for input in [raw(&bytes), compressed(&bytes)] {
            let t = decode(&input).unwrap();
            assert_eq!(t.header.pixel_format, PixelFormat::BGRA8);
            assert_eq!(t.rgba, PIXELS.concat());
            assert_eq!(t.header.mips, 0);
            assert_eq!(t.top_mip_block_bytes, pitch * 2);
            assert_eq!(t.unparsed_after_top_mip, 0);
            assert_eq!(t.header.dds.unwrap().pitch_or_linear_size, pitch as u32);
        }
    }
}
#[test]
fn derived_pitch_legacy_linear_size_and_opaque_tail_preserve_raw_fields() {
    for flags in [0x1007, 0xa1007] {
        let mut bytes = dds(12);
        put(&mut bytes, 8, flags);
        put(&mut bytes, 20, 24);
        put(&mut bytes, 28, 3);
        bytes.extend_from_slice(b"tailBUMP");
        let t = decode(&raw(&bytes)).unwrap();
        assert_eq!(t.rgba, PIXELS.concat());
        assert_eq!(t.header.mips, 3);
        assert!(t.header.no_srgb);
        assert_eq!(t.unparsed_after_top_mip, 8);
        assert_eq!(t.header.dds.unwrap().flags, flags);
    }
}
#[test]
fn unknown_masks_flags_pitch_layouts_and_size_budgets_are_explicit() {
    for (p, n) in [
        (80, 0x40),
        (80, 0x45),
        (88, 24),
        (92, 0xff),
        (96, 0x00ff0000),
        (100, 0xffff),
        (104, 0),
        (112, 0x200),
        (24, 2),
    ] {
        let mut bytes = dds(12);
        put(&mut bytes, p, n);
        assert!(matches!(decode(&raw(&bytes)), Err(Error::Unsupported(_))));
    }
    let mut bytes = dds(12);
    put(&mut bytes, 20, 11);
    assert!(matches!(decode(&raw(&bytes)), Err(Error::Invalid(_))));
    put(&mut bytes, 20, u32::MAX);
    assert_eq!(decode(&raw(&bytes)).unwrap_err(), Error::Budget);
    let mut bytes = dds(12);
    put(&mut bytes, 8, 0x8100f);
    assert!(matches!(decode(&raw(&bytes)), Err(Error::Unsupported(_))));
    put(&mut bytes, 8, 0x81007);
    put(&mut bytes, 20, 23);
    assert!(matches!(decode(&raw(&bytes)), Err(Error::Unsupported(_))));
    let mut bytes = dds(12);
    put(&mut bytes, 16, u32::MAX);
    assert_eq!(decode(&raw(&bytes)).unwrap_err(), Error::Budget);
    let bytes = dds(16);
    for n in 0..bytes.len() {
        assert!(decode(&raw(&bytes[..n])).is_err(), "prefix {n}");
    }
}
#[test]
fn cancellation_between_rows_and_bounded_read_callbacks_remain_enforced() {
    let input = raw(&dds(16));
    let calls = Cell::new(0);
    assert_eq!(
        decode_checked(&input, || {
            calls.set(calls.get() + 1);
            calls.get() == 3
        })
        .unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(calls.get(), 3);
    let decoded = decode_reader(
        input.len() as u64,
        |offset, n| {
            assert!(n <= 4096);
            Ok(input[offset as usize..offset as usize + n].to_vec())
        },
        || false,
    )
    .unwrap();
    assert_eq!(decoded.rgba, PIXELS.concat());
}
