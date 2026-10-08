//! D91 synthetic uncompressed GTF fixtures (hand-built headers and texels only).
use lbp_texture::{decode, inspect, Error, PixelFormat};
use std::io::Write;
fn gtf(format: u8, w: u16, h: u16, remap: u32, pitch: u32, data: &[u8], zlib: bool) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(b"GTF ");
    b[4] = format;
    b[5] = 1;
    b[6] = 2;
    b[8..12].copy_from_slice(&remap.to_be_bytes());
    b[12..14].copy_from_slice(&w.to_be_bytes());
    b[14..16].copy_from_slice(&h.to_be_bytes());
    b[17] = 1;
    b[20..24].copy_from_slice(&pitch.to_be_bytes());
    if zlib {
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
/// Big-endian A,R,G,B texel whose red byte is `id` so positions are traceable.
fn argb(id: u8) -> [u8; 4] {
    [200, id, 2 * id, 255 - id]
}
fn rgba_of(id: u8) -> [u8; 4] {
    [id, 2 * id, 255 - id, 200]
}
/// Stored texels in the given order of (x, y) positions; red byte encodes 10*y + x.
fn stored(order: &[(u8, u8)]) -> Vec<u8> {
    order.iter().flat_map(|(x, y)| argb(10 * y + x)).collect()
}
fn expected(w: u8, h: u8) -> Vec<u8> {
    (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| rgba_of(10 * y + x)))
        .collect()
}
#[test]
fn a8r8g8b8_square_swizzle_order() {
    // 4x4 Morton order, written out by hand.
    let order = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (2, 0),
        (3, 0),
        (2, 1),
        (3, 1),
        (0, 2),
        (1, 2),
        (0, 3),
        (1, 3),
        (2, 2),
        (3, 2),
        (2, 3),
        (3, 3),
    ];
    let t = decode(&gtf(0x85, 4, 4, 0xaae4, 0, &stored(&order), false)).unwrap();
    assert_eq!(t.header.pixel_format, PixelFormat::A8R8G8B8);
    assert_eq!(t.top_mip_block_bytes, 64);
    assert_eq!(t.rgba, expected(4, 4));
}
#[test]
fn a8r8g8b8_wide_and_tall_swizzle_order() {
    // 4x2: 2x2 Morton squares placed left to right.
    let wide = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (2, 0),
        (3, 0),
        (2, 1),
        (3, 1),
    ];
    let t = decode(&gtf(0x85, 4, 2, 0xaae4, 0, &stored(&wide), true)).unwrap();
    assert_eq!(t.rgba, expected(4, 2));
    // 2x4: 2x2 Morton squares stacked top to bottom.
    let tall = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (0, 2),
        (1, 2),
        (0, 3),
        (1, 3),
    ];
    let t = decode(&gtf(0x85, 2, 4, 0xaae4, 0, &stored(&tall), false)).unwrap();
    assert_eq!(t.rgba, expected(2, 4));
    // 8x1 degenerates to linear order; extra mip bytes are retained as tail.
    let mut row = stored(&(0..8).map(|x| (x, 0)).collect::<Vec<_>>());
    row.extend([9; 16]);
    let t = decode(&gtf(0x85, 8, 1, 0xaae4, 0, &row, false)).unwrap();
    assert_eq!(t.rgba, expected(8, 1));
    assert_eq!(t.unparsed_after_top_mip, 16);
}
#[test]
fn a8r8g8b8_linear_rows_skip_pitch_padding() {
    // 0xa5 = 0x85 | LN; 2x2, pitch 12 leaves 4 padding bytes after each row.
    let mut data = Vec::new();
    data.extend(argb(0));
    data.extend(argb(1));
    data.extend([0xee; 4]);
    data.extend(argb(10));
    data.extend(argb(11));
    let t = decode(&gtf(0xa5, 2, 2, 0xaae4, 12, &data, false)).unwrap();
    assert_eq!(t.header.format_raw, Some(0xa5));
    assert_eq!(t.top_mip_block_bytes, 20);
    assert_eq!(t.rgba, expected(2, 2));
    // Last row shorter than pitch*h but complete: accepted; one byte short: truncated.
    assert_eq!(
        decode(&gtf(0xa5, 2, 2, 0xaae4, 12, &data[..19], false)).unwrap_err(),
        Error::Invalid("truncated top mip data")
    );
}
#[test]
fn b8_is_opaque_gray_under_identity_or_bbbb_remap() {
    // 2x2 swizzled = linear order for a square of side 2.
    for remap in [0xaae4, 0xa9ff] {
        let t = decode(&gtf(0x81, 2, 2, remap, 0, &[0, 64, 128, 255], true)).unwrap();
        assert_eq!(t.header.pixel_format, PixelFormat::B8);
        assert_eq!(t.header.remap, Some(remap));
        assert_eq!(
            t.rgba,
            [0, 0, 0, 255, 64, 64, 64, 255, 128, 128, 128, 255, 255, 255, 255, 255]
        );
    }
    // 4x2 B8 swizzle: memory order (0,0),(1,0),(0,1),(1,1),(2,0),(3,0),(2,1),(3,1).
    let t = decode(&gtf(
        0x81,
        4,
        2,
        0xaae4,
        0,
        &[0, 1, 10, 11, 2, 3, 12, 13],
        false,
    ))
    .unwrap();
    let gray: Vec<u8> = t.rgba.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
    assert_eq!(gray, [0, 1, 2, 3, 10, 11, 12, 13]);
    // LN B8 with pitch padding.
    let t = decode(&gtf(0xa1, 1, 2, 0xa9ff, 4, &[7, 0, 0, 0, 9], false)).unwrap();
    assert_eq!(t.rgba, [7, 7, 7, 255, 9, 9, 9, 255]);
}
#[test]
fn refusals_and_bounds() {
    let px = [0u8; 64];
    let unsupported =
        |b: Vec<u8>, why| assert_eq!(inspect(&b).unwrap_err(), Error::Unsupported(why));
    let fmt = "pixel format or layout flags";
    let flags = "remap/pitch/offset/location/flags";
    unsupported(gtf(0xc5, 4, 4, 0xaae4, 0, &px, false), fmt); // UN flag
    unsupported(gtf(0xe5, 4, 4, 0xaae4, 16, &px, false), fmt); // LN + UN
    unsupported(gtf(0xa6, 4, 4, 0xaae4, 8, &px, false), fmt); // LN BC1
    unsupported(gtf(0x83, 4, 4, 0xaae4, 0, &px, false), fmt); // A4R4G4B4
    unsupported(gtf(0x81, 4, 4, 0x02ff, 0, &px, false), flags); // alpha-only remap
    unsupported(gtf(0x85, 4, 4, 0xa9ff, 0, &px, false), flags); // BBBB on ARGB
    unsupported(gtf(0x85, 4, 4, 0xaae4, 16, &px, false), flags); // swizzled + pitch
    unsupported(
        gtf(0x85, 3, 2, 0xaae4, 0, &px, false),
        "non-power-of-two swizzled GTF",
    );
    let mut volume = gtf(0x85, 4, 4, 0xaae4, 0, &px, false);
    volume[6] = 3;
    unsupported(volume.clone(), "cube/volume/dimension");
    volume[17] = 4; // depth 4: a real volume (D95 reason)
    unsupported(volume, "volume texture: slice layout UNKNOWN");
    let mut cube = gtf(0x81, 4, 4, 0xaae4, 0, &px, false);
    cube[7] = 1;
    unsupported(cube, "cube map (2D decode)");
    // LN non-power-of-two is fine (linear rows), short pitch is malformed.
    assert!(decode(&gtf(0xa5, 3, 1, 0xaae4, 12, &px[..12], false)).is_ok());
    assert_eq!(
        inspect(&gtf(0xa5, 4, 2, 0xaae4, 15, &px, false)).unwrap_err(),
        Error::Invalid("GTF linear pitch shorter than row")
    );
    // Untrusted pitch: product beyond the expanded budget is refused before indexing.
    assert_eq!(
        decode(&gtf(0xa5, 4, 4096, 0xaae4, u32::MAX, &px, false)).unwrap_err(),
        Error::Budget
    );
    // Truncated swizzled payload.
    assert_eq!(
        decode(&gtf(0x85, 4, 4, 0xaae4, 0, &px[..63], false)).unwrap_err(),
        Error::Invalid("truncated top mip data")
    );
    // Pixel budget: 4096x2048 A8R8G8B8 exceeds MAX_PIXELS; B8 has its own D95 budget.
    assert_eq!(
        inspect(&gtf(0x85, 4096, 2048, 0xaae4, 0, &px, false)).unwrap_err(),
        Error::Budget
    );
    assert!(inspect(&gtf(0x81, 4096, 2048, 0xaae4, 0, &px, false)).is_ok());
}
