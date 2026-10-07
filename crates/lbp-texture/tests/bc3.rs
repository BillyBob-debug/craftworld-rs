//! Authored BC3 recipes, public block specification; no original asset bytes.
use lbp_texture::{decode, decode_checked, decode_reader, Error, PixelFormat};
use std::{cell::Cell, io::Write};
fn block(a: u8, b: u8, color0: u16, color1: u16) -> Vec<u8> {
    let indices = (0..16).fold(0u64, |v, i| v | ((i % 8) as u64) << (i * 3));
    [
        vec![a, b],
        indices.to_le_bytes()[..6].to_vec(),
        color0.to_le_bytes().to_vec(),
        color1.to_le_bytes().to_vec(),
        0xe4e4e4e4u32.to_le_bytes().to_vec(),
    ]
    .concat()
}
fn gtf(w: u16, h: u16, data: &[u8]) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(b"GTF ");
    b[4] = 0x88;
    b[5] = 1;
    b[6] = 2;
    b[8..12].copy_from_slice(&0xaae4u32.to_be_bytes());
    b[12..14].copy_from_slice(&w.to_be_bytes());
    b[14..16].copy_from_slice(&h.to_be_bytes());
    b[17] = 1;
    b.extend_from_slice(data);
    b
}
fn tex(w: u32, h: u32, data: &[u8], compressed: bool) -> Vec<u8> {
    let mut dds = vec![0; 128];
    dds[..4].copy_from_slice(b"DDS ");
    for (p, v) in [
        (4, 124u32),
        (8, 0x81007),
        (12, h),
        (16, w),
        (20, 16),
        (76, 32),
        (80, 4),
        (108, 0x1000),
    ] {
        dds[p..p + 4].copy_from_slice(&v.to_le_bytes());
    }
    dds[84..88].copy_from_slice(b"DXT5");
    dds.extend_from_slice(data);
    if !compressed {
        return [b"TEX \0\0\0\0".as_slice(), &dds].concat();
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&dds).unwrap();
    let c = z.finish().unwrap();
    [
        b"TEX \0\x01\0\x01".as_slice(),
        &(c.len() as u16).to_be_bytes(),
        &(dds.len() as u16).to_be_bytes(),
        &c,
    ]
    .concat()
}
#[test]
fn both_alpha_modes_equality_and_packed_indices() {
    for (a, b, expected) in [
        (255, 0, [255, 0, 218, 182, 145, 109, 72, 36]),
        (0, 255, [0, 255, 51, 102, 153, 204, 0, 255]),
        (17, 17, [17, 17, 17, 17, 17, 17, 0, 255]),
        (23, 11, [23, 11, 21, 19, 17, 16, 14, 12]),
    ] {
        let data = block(a, b, 0x07e0, 0xf800);
        for input in [
            gtf(4, 4, &data),
            tex(4, 4, &data, false),
            tex(4, 4, &data, true),
        ] {
            let t = decode(&input).unwrap();
            assert_eq!(t.header.pixel_format, PixelFormat::BC3);
            assert_eq!(t.top_mip_block_bytes, 16);
            let colors = [[0, 255, 0], [255, 0, 0], [85, 170, 0], [170, 85, 0]];
            for (i, p) in t.rgba.as_chunks::<4>().0.iter().enumerate() {
                assert_eq!(p[..3], colors[i % 4]);
                assert_eq!(p[3], expected[i % 8]);
            }
        }
    }
}
#[test]
fn partial_blocks_orientation_and_tail() {
    let data = [
        block(255, 255, 0xf800, 0xf800),
        block(255, 255, 0x07e0, 0x07e0),
        block(255, 255, 0x001f, 0x001f),
        block(255, 255, 0xffff, 0xffff),
        b"tail".to_vec(),
    ]
    .concat();
    for input in [gtf(7, 5, &data), tex(7, 5, &data, true)] {
        let t = decode(&input).unwrap();
        assert_eq!(t.top_mip_block_bytes, 64);
        assert_eq!(t.unparsed_after_top_mip, 4);
        for (x, y, c) in [
            (0, 0, [255, 0, 0, 255]),
            (4, 0, [0, 255, 0, 255]),
            (0, 4, [0, 0, 255, 255]),
            (4, 4, [255; 4]),
        ] {
            assert_eq!(&t.rgba[(y * 7 + x) * 4..(y * 7 + x + 1) * 4], &c);
        }
    }
}
#[test]
fn all_prefixes_and_decompression_errors() {
    let data = block(255, 0, 0xf800, 0x07e0);
    for input in [
        gtf(4, 4, &data),
        tex(4, 4, &data, false),
        tex(4, 4, &data, true),
    ] {
        for n in 0..input.len() {
            assert!(decode(&input[..n]).is_err(), "prefix {n}");
        }
    }
    let mut b = tex(4, 4, &data, true);
    b[11] += 1;
    assert!(matches!(decode(&b), Err(Error::Invalid(_))));
    let mut b = tex(4, 4, &data, true);
    b.push(0);
    let size = u16::from_be_bytes([b[8], b[9]]) + 1;
    b[8..10].copy_from_slice(&size.to_be_bytes());
    assert!(matches!(decode(&b), Err(Error::Invalid(_))));
}
#[test]
fn budgets_protection_and_block_row_cancellation() {
    let data = block(255, 0, 0xf800, 0x07e0).repeat(2);
    let b = gtf(4, 8, &data);
    let calls = Cell::new(0);
    let err = decode_checked(&b, || {
        calls.set(calls.get() + 1);
        calls.get() >= 3
    })
    .unwrap_err();
    assert_eq!(err, Error::Cancelled);
    assert_eq!(decode(b"GTFe").unwrap_err(), Error::Blocked);
    assert_eq!(decode(b"TEXe").unwrap_err(), Error::Blocked);
    assert_eq!(
        decode(&tex(4096, 4096, &data, false)).unwrap_err(),
        Error::Budget
    );
    let mut b = gtf(4, 4, &data);
    b[4] = 0xa8;
    let reads = Cell::new(0);
    assert!(matches!(
        decode_reader(
            b.len() as u64,
            |p, n| {
                reads.set(reads.get() + 1);
                Ok(b[p as usize..p as usize + n].to_vec())
            },
            || false
        ),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(reads.get(), 1);
}
