//! Original in-memory DDS/TEX recipes; no game bytes or third-party fixtures.
use lbp_texture::{decode, decode_reader, Error};
use std::{cell::Cell, io::Write};
fn dds() -> Vec<u8> {
    let mut b = vec![0; 128];
    b[..4].copy_from_slice(b"DDS ");
    for (p, v) in [
        (4, 124u32),
        (8, 0x81007),
        (12, 2),
        (16, 3),
        (20, 8),
        (76, 32),
        (80, 4),
        (108, 0x1000),
    ] {
        b[p..p + 4].copy_from_slice(&v.to_le_bytes());
    }
    b[84..88].copy_from_slice(b"DXT1");
    b.extend_from_slice(&[0, 248, 0, 0, 0, 0, 0, 0]);
    b
}
fn raw(data: &[u8]) -> Vec<u8> {
    [b"TEX ".as_slice(), &[0, 0, 0, 0], data].concat()
}
fn split(data: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&data[60..]).unwrap();
    let compressed = z.finish().unwrap();
    let mut b = b"TEX \0\x01\0\x02".to_vec();
    b.extend_from_slice(&60u16.to_be_bytes());
    b.extend_from_slice(&60u16.to_be_bytes());
    b.extend_from_slice(&(compressed.len() as u16).to_be_bytes());
    b.extend_from_slice(&((data.len() - 60) as u16).to_be_bytes());
    b.extend_from_slice(&data[..60]);
    b.extend(compressed);
    b
}
#[test]
fn raw_zero_mips_and_split_prefix_match_shared_pixels() {
    let data = dds();
    for b in [raw(&data), split(&data)] {
        let t = decode(&b).unwrap();
        assert_eq!(t.rgba, [255, 0, 0, 255].repeat(6));
        assert_eq!(t.header.mips, 0);
        assert_eq!(t.header.wrapper, "TEX/DDS");
        assert!(t.header.flags.is_none());
        assert_eq!(t.expanded_bytes, 136);
        assert_eq!(t.top_mip_block_bytes, 8);
        assert_eq!(t.unparsed_after_top_mip, 0);
        let h = t.header.dds.unwrap();
        assert_eq!(h.flags, 0x81007);
        assert_eq!(h.caps, [0x1000, 0, 0, 0]);
    }
}
#[test]
fn raw_fields_and_bump_suffix_remain_distinct_from_layout() {
    let mut data = dds();
    data[28..32].copy_from_slice(&3u32.to_le_bytes());
    data[92..96].copy_from_slice(&0xffu32.to_le_bytes());
    data.extend_from_slice(b"opaqueBUMP");
    let t = decode(&split(&data)).unwrap();
    assert!(t.header.no_srgb);
    assert_eq!(t.header.mips, 3);
    assert_eq!(t.unparsed_after_top_mip, 10);
    assert_eq!(t.header.dds.unwrap().masks[0], 255);
    let mut data = dds();
    data[132..136].copy_from_slice(b"BUMP");
    assert!(!decode(&raw(&data)).unwrap().header.no_srgb);
}
#[test]
fn dds_variant_rejections_and_bounds_are_explicit() {
    for (p, v) in [(4, 123u32), (76, 31), (12, 0), (16, 0)] {
        let mut b = dds();
        b[p..p + 4].copy_from_slice(&v.to_le_bytes());
        assert!(matches!(decode(&raw(&b)), Err(Error::Invalid(_))));
    }
    for (p, v) in [
        (24, 2u32),
        (8, 0x800000),
        (112, 0x200),
        (112, 0x400),
        (112, 0x200000),
        (116, 1),
        (120, 1),
        (80, 0),
        (80, 0x44),
    ] {
        let mut b = dds();
        b[p..p + 4].copy_from_slice(&v.to_le_bytes());
        assert!(matches!(decode(&raw(&b)), Err(Error::Unsupported(_))));
    }
    for code in [b"DX10", b"DXT3", b"ABCD"] {
        let mut b = dds();
        b[84..88].copy_from_slice(code);
        assert!(matches!(decode(&raw(&b)), Err(Error::Unsupported(_))));
    }
    for (p, v) in [(12, 4097u32), (16, u32::MAX), (28, u32::MAX)] {
        let mut b = dds();
        b[p..p + 4].copy_from_slice(&v.to_le_bytes());
        assert_eq!(decode(&raw(&b)).unwrap_err(), Error::Budget);
    }
    for n in 0..136 {
        assert!(decode(&raw(&dds()[..n])).is_err(), "DDS prefix {n}");
    }
}
#[test]
fn tex_preflight_short_reads_protection_cancel_and_chunk_failures() {
    assert_eq!(decode(b"TEXe").unwrap_err(), Error::Blocked);
    assert!(matches!(decode(b"TEX "), Err(Error::Invalid(_))));
    let b = split(&dds());
    for n in 0..b.len() {
        assert!(decode(&b[..n]).is_err(), "TEX prefix {n}");
    }
    let mut bad = b.clone();
    bad[15] += 1;
    assert!(matches!(decode(&bad), Err(Error::Invalid(_))));
    let mut bad = b.clone();
    bad.push(0);
    let stored = u16::from_be_bytes([bad[12], bad[13]]) + 1;
    bad[12..14].copy_from_slice(&stored.to_be_bytes());
    assert!(matches!(decode(&bad), Err(Error::Invalid(_))));
    assert!(matches!(
        decode_reader(b.len() as u64, |_, _| Ok(vec![]), || false),
        Err(Error::Invalid(_))
    ));
    let calls = Cell::new(0);
    assert_eq!(
        decode_reader(
            b.len() as u64,
            |p, n| {
                calls.set(calls.get() + 1);
                Ok(b[p as usize..p as usize + n].to_vec())
            },
            || calls.get() >= 1
        )
        .unwrap_err(),
        Error::Cancelled
    );
}
