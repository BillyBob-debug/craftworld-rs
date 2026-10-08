//! D98 synthetic fixtures (hand-built headers and texels only): GCM remap words on
//! uncompressed GTF and swizzled volume textures through the layered entry points.
use lbp_texture::{
    decode, decode_layered_checked, decode_reader_layered, inspect, inspect_layered, Error,
};

/// Raw (count 0) GTF with the given dimension/depth and remap.
fn gtf(format: u8, dimension: u8, w: u16, h: u16, depth: u16, remap: u32, data: &[u8]) -> Vec<u8> {
    let mut b = vec![0; 32];
    b[..4].copy_from_slice(b"GTF ");
    b[4] = format;
    b[5] = 1;
    b[6] = dimension;
    b[8..12].copy_from_slice(&remap.to_be_bytes());
    b[12..14].copy_from_slice(&w.to_be_bytes());
    b[14..16].copy_from_slice(&h.to_be_bytes());
    b[16..18].copy_from_slice(&depth.to_be_bytes());
    b.extend(data);
    b
}

#[test]
fn b8_remap_words_identity_gray_and_alpha_mask() {
    // 2x2 swizzled B8 = linear order for a square of side 2.
    let texels = [0u8, 64, 128, 255];
    let gray: Vec<u8> = texels.iter().flat_map(|&v| [v, v, v, 255]).collect();
    for remap in [0xaae4, 0xa9ff] {
        let t = decode(&gtf(0x81, 2, 2, 2, 1, remap, &texels)).unwrap();
        assert_eq!(t.rgba, gray, "remap {remap:#06x}");
    }
    // 0x02ff: alpha mode remap (from B = the byte), RGB mode zero.
    let t = decode(&gtf(0x81, 2, 2, 2, 1, 0x02ff, &texels)).unwrap();
    assert_eq!(t.header.remap, Some(0x02ff));
    let mask: Vec<u8> = texels.iter().flat_map(|&v| [0, 0, 0, v]).collect();
    assert_eq!(t.rgba, mask);
    // Linear (LN) B8 applies the remap the same way.
    let mut ln = gtf(0xa1, 2, 2, 1, 1, 0x02ff, &[7, 9]);
    ln[20..24].copy_from_slice(&2u32.to_be_bytes());
    assert_eq!(decode(&ln).unwrap().rgba, [0, 0, 0, 7, 0, 0, 0, 9]);
}

#[test]
fn argb_remap_selects_and_constant_modes() {
    // One texel, big-endian A, R, G, B = 10, 20, 30, 40.
    let texel = [10u8, 20, 30, 40];
    let rgba = |remap: u32| decode(&gtf(0x85, 2, 1, 1, 1, remap, &texel)).unwrap().rgba;
    assert_eq!(rgba(0xaae4), [20, 30, 40, 10]);
    // Selects reversed (A<-B, R<-G, G<-R, B<-A), all modes remap.
    assert_eq!(rgba(0xaa1b), [30, 20, 10, 40]);
    // All outputs one / zero regardless of selects.
    assert_eq!(rgba(0x55e4), [255, 255, 255, 255]);
    assert_eq!(rgba(0x00e4), [0, 0, 0, 0]);
    // 0xa9ff on ARGB: RGB from B, alpha one.
    assert_eq!(rgba(0xa9ff), [40, 40, 40, 255]);
    let refused = |remap: u32| {
        assert_eq!(
            inspect(&gtf(0x85, 2, 1, 1, 1, remap, &texel)).unwrap_err(),
            Error::Unsupported("remap/pitch/offset/location/flags")
        )
    };
    refused(0xfee4); // output R mode 3
    refused(0x3aae4); // bits above 15 (remap order)
                      // BC formats keep the identity remap only.
    assert_eq!(
        inspect(&gtf(0x86, 2, 4, 4, 1, 0x02ff, &[0; 8])).unwrap_err(),
        Error::Unsupported("remap/pitch/offset/location/flags")
    );
}

/// Independent expectation for a 4 x 2 x 8 volume (log2 2, 1, 3): bit order
/// x0, y0, z0, x1, z1, z2 (y exhausted after one bit, x after two).
fn index_4x2x8(x: usize, y: usize, z: usize) -> usize {
    (x & 1) | (y & 1) << 1 | (z & 1) << 2 | (x >> 1 & 1) << 3 | (z >> 1 & 1) << 4 | (z >> 2) << 5
}

#[test]
fn argb_volume_decodes_slices_in_3d_swizzle_order() {
    let (w, h, d) = (4usize, 2usize, 8usize);
    let mut data = vec![0u8; w * h * d * 4];
    for z in 0..d {
        for y in 0..h {
            for x in 0..w {
                let i = index_4x2x8(x, y, z) * 4;
                data[i..i + 4].copy_from_slice(&[200, x as u8, y as u8, z as u8]);
            }
        }
    }
    let b = gtf(0x85, 3, w as u16, h as u16, d as u16, 0xaae4, &data);
    assert!(inspect_layered(&b).unwrap().1);
    let l = decode_layered_checked(&b, || false).unwrap();
    assert_eq!(
        (l.layout, l.faces_decoded, l.face_stride),
        ("volume", 8, None)
    );
    assert_eq!(l.extra_faces.len(), d - 1);
    assert_eq!(l.texture.top_mip_block_bytes, w * h * d * 4);
    assert_eq!(l.texture.unparsed_after_top_mip, 0);
    let slices: Vec<&Vec<u8>> = std::iter::once(&l.texture.rgba)
        .chain(l.extra_faces.iter())
        .collect();
    for (z, slice) in slices.iter().enumerate() {
        assert_eq!(slice.len(), w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let p = (y * w + x) * 4;
                assert_eq!(slice[p..p + 4], [x as u8, y as u8, z as u8, 200]);
            }
        }
    }
    // Paged reader gives the same slices; cancellation reaches the slice loop.
    let read = |o: u64, n: usize| Ok(b[o as usize..o as usize + n].to_vec());
    let paged = decode_reader_layered(b.len() as u64, read, || false).unwrap();
    assert_eq!(paged.extra_faces, l.extra_faces);
    assert_eq!(
        decode_layered_checked(&b, || true).unwrap_err(),
        Error::Cancelled
    );
}

#[test]
fn cubic_volume_matches_xyz_interleave_and_b8_remap() {
    // 2x2x2 B8: index = x | y << 1 | z << 2; remap 0x02ff gives alpha masks.
    let data: Vec<u8> = (0..8).map(|i| i * 10).collect();
    let b = gtf(0x81, 3, 2, 2, 2, 0x02ff, &data);
    let l = decode_layered_checked(&b, || false).unwrap();
    assert_eq!(
        l.texture.rgba,
        [0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0, 30]
    );
    assert_eq!(
        l.extra_faces,
        [vec![0, 0, 0, 40, 0, 0, 0, 50, 0, 0, 0, 60, 0, 0, 0, 70]]
    );
}

#[test]
fn volume_refusals_and_budgets() {
    let texels = vec![0u8; 4 * 4 * 4 * 4];
    let refuse = |b: Vec<u8>, why| {
        assert_eq!(
            decode_layered_checked(&b, || false).unwrap_err(),
            Error::Unsupported(why)
        )
    };
    // 2D entry points.
    assert_eq!(
        inspect(&gtf(0x85, 3, 4, 4, 4, 0xaae4, &texels)).unwrap_err(),
        Error::Unsupported("volume texture (2D decode)")
    );
    // BC and linear volumes have no established layout.
    let bc = "compressed or linear volume texture";
    refuse(gtf(0x86, 3, 4, 4, 4, 0xaae4, &texels), bc);
    let mut ln = gtf(0xa5, 3, 4, 4, 4, 0xaae4, &texels);
    ln[20..24].copy_from_slice(&16u32.to_be_bytes());
    refuse(ln, bc);
    refuse(
        gtf(0x85, 3, 4, 4, 3, 0xaae4, &texels),
        "non-power-of-two swizzled GTF",
    );
    // Cube byte set on a volume header stays refused.
    let mut cube = gtf(0x85, 3, 4, 4, 4, 0xaae4, &texels);
    cube[7] = 1;
    refuse(cube, "cube/volume/dimension");
    // One byte short of the 4x4x4 top mip.
    assert_eq!(
        decode_layered_checked(&gtf(0x85, 3, 4, 4, 4, 0xaae4, &texels[..255]), || false)
            .unwrap_err(),
        Error::Invalid("truncated top mip data")
    );
    // 256 x 256 x 128 ARGB = 8 Mi pixels over the 4 Mi budget; checked before any read.
    assert_eq!(
        inspect_layered(&gtf(0x85, 3, 256, 256, 128, 0xaae4, &[])).unwrap_err(),
        Error::Budget
    );
    // 4096 x 4096 x 32768 B8: refused by the pixel budget before any payload read.
    assert_eq!(
        inspect_layered(&gtf(0x81, 3, 4096, 4096, 32768, 0xaae4, &[])).unwrap_err(),
        Error::Budget
    );
}
