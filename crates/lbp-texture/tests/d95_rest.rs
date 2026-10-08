//! D95 synthetic fixtures (hand-built headers and texels only): B8 pixel budget, BC1 cube
//! maps through the layered entry points, volume and remap refusals.
use lbp_texture::{
    decode, decode_checked, decode_layered_checked, decode_reader, decode_reader_layered, inspect,
    inspect_layered, Error, MAX_INPUT, MAX_RGBA,
};
use std::io::Write;

/// GTF header; `chunks` = None stores `data` raw (count 0), Some(n) zlib-compresses it in
/// chunks of `n` expanded bytes.
#[allow(clippy::too_many_arguments)]
fn gtf(
    format: u8,
    mips: u8,
    dimension: u8,
    cube: u8,
    w: u16,
    h: u16,
    depth: u16,
    remap: u32,
    data: &[u8],
    chunks: Option<usize>,
) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(b"GTF ");
    b[4] = format;
    b[5] = mips;
    b[6] = dimension;
    b[7] = cube;
    b[8..12].copy_from_slice(&remap.to_be_bytes());
    b[12..14].copy_from_slice(&w.to_be_bytes());
    b[14..16].copy_from_slice(&h.to_be_bytes());
    b[16..18].copy_from_slice(&depth.to_be_bytes());
    let Some(n) = chunks else {
        b.extend(data);
        return b;
    };
    let parts: Vec<&[u8]> = data.chunks(n).collect();
    b[29] = 1;
    b[30..32].copy_from_slice(&(parts.len() as u16).to_be_bytes());
    let mut payload = Vec::new();
    for part in &parts {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(part).unwrap();
        let c = z.finish().unwrap();
        b.extend_from_slice(&(c.len() as u16).to_be_bytes());
        b.extend_from_slice(&(part.len() as u16).to_be_bytes());
        payload.extend(c);
    }
    b.extend(payload);
    b
}

/// One BC1 block with both endpoints equal: every texel is the RGB565 `color`.
fn solid_block(color: u16) -> [u8; 8] {
    let c = color.to_le_bytes();
    [c[0], c[1], c[0], c[1], 0, 0, 0, 0]
}

const FACE_COLORS: [u16; 6] = [0xf800, 0x07e0, 0x001f, 0xffff, 0xf81f, 0x0000];
const FACE_RGBA: [[u8; 4]; 6] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 255, 255],
    [255, 0, 255, 255],
    [0, 0, 0, 255],
];

/// 8x8 BC1 cube with 2 mips: chain = 4 blocks + 1 block = 40 bytes, stride 128.
fn cube_payload() -> Vec<u8> {
    let mut data = Vec::new();
    for color in FACE_COLORS {
        let mut face = Vec::new();
        for _ in 0..5 {
            face.extend(solid_block(color));
        }
        face.resize(128, 0xee); // padding is never read
        data.extend(face);
    }
    data
}

#[test]
fn b8_4096_square_decodes_within_byte_budgets() {
    // Top mip only, 16 MiB of B8 in 512 zlib chunks (the real files carry 13 mips).
    let mut texels = vec![0u8; 4096 * 4096];
    texels[0] = 9; // swizzle index 0 = (0, 0)
    texels[1] = 7; // swizzle index 1 = (1, 0)
    texels[2] = 5; // swizzle index 2 = (0, 1)
    let b = gtf(0x81, 1, 2, 0, 4096, 4096, 1, 0xaae4, &texels, Some(32768));
    assert!(b.len() <= MAX_INPUT);
    let t = decode(&b).unwrap();
    assert_eq!((t.header.width, t.header.height), (4096, 4096));
    assert_eq!(t.rgba.len(), MAX_RGBA);
    assert_eq!(&t.rgba[..8], &[9, 9, 9, 255, 7, 7, 7, 255]);
    assert_eq!(&t.rgba[4096 * 4..4096 * 4 + 4], &[5, 5, 5, 255]);
    // The raw (count 0) form of the same image exceeds the 16 MiB input budget.
    let raw = gtf(0x81, 1, 2, 0, 4096, 4096, 1, 0xaae4, &texels, None);
    assert_eq!(decode(&raw).unwrap_err(), Error::Budget);
    // Multi-byte formats keep the 4 Mi pixel budget; sides stay capped at 4096.
    for (format, w, h) in [(0x85, 4096, 2048), (0x86, 4096, 2048), (0x81, 8192, 2048)] {
        let b = gtf(format, 1, 2, 0, w, h, 1, 0xaae4, &[], None);
        assert_eq!(inspect(&b).unwrap_err(), Error::Budget);
    }
}

#[test]
fn bc1_cube_decodes_six_faces_only_through_layered_entry_points() {
    let b = gtf(0x86, 2, 2, 1, 8, 8, 1, 0xaae4, &cube_payload(), Some(300));
    assert_eq!(
        decode(&b).unwrap_err(),
        Error::Unsupported("cube map (2D decode)")
    );
    assert_eq!(
        inspect(&b).unwrap_err(),
        Error::Unsupported("cube map (2D decode)")
    );
    assert!(inspect_layered(&b).unwrap().1);
    let l = decode_layered_checked(&b, || false).unwrap();
    assert_eq!(
        (l.layout, l.faces_decoded, l.face_stride),
        ("cube", 6, Some(128))
    );
    assert_eq!(l.extra_faces.len(), 5);
    let solid = |rgba: &[u8], c: [u8; 4]| {
        assert_eq!(rgba.len(), 8 * 8 * 4);
        assert!(rgba.as_chunks::<4>().0.iter().all(|p| *p == c));
    };
    solid(&l.texture.rgba, FACE_RGBA[0]);
    for (face, rgba) in l.extra_faces.iter().enumerate() {
        solid(rgba, FACE_RGBA[face + 1]);
    }
    assert_eq!(l.texture.expanded_bytes, 768);
    assert_eq!(l.texture.unparsed_after_top_mip, 768 - (5 * 128 + 32));
    // Paged reader: the 2D reader refuses at header preflight, the layered one decodes.
    let read = |o: u64, n: usize| Ok(b[o as usize..o as usize + n].to_vec());
    assert_eq!(
        decode_reader(b.len() as u64, read, || false).unwrap_err(),
        Error::Unsupported("cube map (2D decode)")
    );
    let paged = decode_reader_layered(b.len() as u64, read, || false).unwrap();
    assert_eq!(paged.extra_faces, l.extra_faces);
    // Cancellation reaches the per-face block loops.
    assert_eq!(
        decode_layered_checked(&b, || true).unwrap_err(),
        Error::Cancelled
    );
}

#[test]
fn cube_layout_and_budget_refusals() {
    let payload = cube_payload();
    // Payload one byte short or long of six aligned faces: layout not established.
    for data in [&payload[..767], &[payload.as_slice(), &[0]].concat()[..]] {
        let b = gtf(0x86, 2, 2, 1, 8, 8, 1, 0xaae4, data, None);
        assert_eq!(
            decode_layered_checked(&b, || false).unwrap_err(),
            Error::Unsupported("cube payload is not six 128-byte-aligned mip chains")
        );
    }
    let refuse = |b: Vec<u8>, why| {
        assert_eq!(
            decode_layered_checked(&b, || false).unwrap_err(),
            Error::Unsupported(why)
        )
    };
    refuse(
        gtf(0x86, 1, 2, 1, 8, 4, 1, 0xaae4, &payload, None),
        "non-square cube map",
    );
    refuse(
        gtf(0x81, 1, 2, 1, 4, 4, 1, 0xaae4, &payload, None),
        "uncompressed cube map",
    );
    refuse(
        gtf(0x86, 1, 2, 2, 4, 4, 1, 0xaae4, &payload, None),
        "cube/volume/dimension",
    );
    // 2048 BC1 cube: six RGBA faces (96 MiB) exceed MAX_RGBA before any face is decoded.
    let top = vec![0u8; 2048 * 2048 / 2];
    let b = gtf(0x86, 1, 2, 1, 2048, 2048, 1, 0xaae4, &top, None);
    assert_eq!(
        decode_layered_checked(&b, || false).unwrap_err(),
        Error::Budget
    );
    // 2D through the layered entry point is the ordinary decode.
    let flat = gtf(0x86, 1, 2, 0, 4, 4, 1, 0xaae4, &solid_block(0xf800), None);
    let l = decode_layered_checked(&flat, || false).unwrap();
    assert_eq!((l.layout, l.faces_decoded, l.face_stride), ("2d", 1, None));
    assert!(l.extra_faces.is_empty());
    assert_eq!(
        l.texture.rgba,
        decode_checked(&flat, || false).unwrap().rgba
    );
}

#[test]
fn volume_and_alpha_only_remap_stay_refused() {
    let texels = vec![0u8; 16 * 16 * 16 * 4];
    let volume = gtf(0x85, 1, 3, 0, 16, 16, 16, 0xaae4, &texels, None);
    for result in [
        decode(&volume).map(|_| ()),
        decode_layered_checked(&volume, || false).map(|_| ()),
    ] {
        assert_eq!(
            result.unwrap_err(),
            Error::Unsupported("volume texture: slice layout UNKNOWN")
        );
    }
    let b8 = gtf(0x81, 1, 2, 0, 4, 4, 1, 0x02ff, &[0; 16], None);
    assert_eq!(
        decode_layered_checked(&b8, || false).unwrap_err(),
        Error::Unsupported("remap/pitch/offset/location/flags")
    );
}
