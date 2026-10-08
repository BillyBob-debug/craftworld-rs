//! Authored mesh-prefix recipes; no proprietary fixtures.
use lbp_mesh::{inspect, Error};
use std::io::Write;
fn integer(b: &mut Vec<u8>, mut n: u32, compact: bool) {
    if !compact {
        b.extend_from_slice(&n.to_be_bytes());
        return;
    }
    loop {
        let v = (n & 127) as u8;
        n >>= 7;
        b.push(v | if n > 0 { 128 } else { 0 });
        if n == 0 {
            break;
        }
    }
}
fn recipe(modern: bool, compressed: bool) -> Vec<u8> {
    let mut p = vec![];
    for n in [3, 3, 0, 1, 2, 1, 0] {
        integer(&mut p, n, modern);
    }
    p.resize(p.len() + 512, 0);
    if modern {
        for _ in 0..2 {
            integer(&mut p, 2, true);
            p.extend_from_slice(&[0; 8]);
        }
        p.extend_from_slice(&1f32.to_bits().to_be_bytes());
    }
    for n in [0, 48, 96, 96] {
        integer(&mut p, n, modern);
    }
    for xyz in [[0f32, 0., 0.], [1., 2., 3.], [-1., 4., -2.]] {
        for v in xyz {
            p.extend_from_slice(&v.to_bits().to_be_bytes());
        }
        p.extend_from_slice(&[0; 4]);
    }
    p.extend_from_slice(&[0; 48]);
    integer(&mut p, 24, modern);
    p.extend_from_slice(&[0; 24]);
    integer(&mut p, 6, modern);
    p.extend_from_slice(&[0, 0, 0, 1, 0, 2]);
    p.extend_from_slice(b"unparsed");
    let mut b = b"MSHb".to_vec();
    b.extend_from_slice(&(if modern { 950u32 } else { 567 }).to_be_bytes());
    b.extend_from_slice(&[0; 4]);
    if modern {
        b.extend_from_slice(&[0, 0, 0, 0, 7]);
    }
    b.push(u8::from(compressed));
    if compressed {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&p).unwrap();
        let c = z.finish().unwrap();
        b.extend_from_slice(&[0, 1, 0, 1]);
        b.extend_from_slice(&(c.len() as u16).to_be_bytes());
        b.extend_from_slice(&(p.len() as u16).to_be_bytes());
        b.extend(c);
    } else {
        b.extend(p);
    }
    let end = b.len() as u32;
    b[8..12].copy_from_slice(&end.to_be_bytes());
    b.extend_from_slice(&[0; 4]);
    b
}
#[test]
fn revisions_compact_fields_and_inflation_match() {
    for modern in [false, true] {
        for compressed in [false, true] {
            let m = inspect(&recipe(modern, compressed), || false).unwrap();
            assert_eq!(m.counts, [3, 3, 0, 1, 2, 1, 0]);
            assert_eq!(m.position_bounds, Some([[-1., 0., -2.], [1., 4., 3.]]));
            assert_eq!(m.attribute_bytes, 24);
            assert_eq!(m.index_bytes, 6);
            assert_eq!(m.unparsed_bytes, 8);
            assert_eq!(m.stream_offsets, [0, 48, 96, 96]);
            assert_eq!(m.area_scale_bits.is_some(), modern);
        }
    }
}
#[test]
fn uv_envelopes_revisions_inflation_truncation_protection_and_cancel_are_bounded() {
    for modern in [false, true] {
        for compressed in [false, true] {
            let bytes = recipe(modern, compressed);
            let uv = lbp_mesh::decode_uvs(&bytes, || false).unwrap();
            assert_eq!(uv.channels, vec![vec![[0., 0.]; 3]]);
            for n in 0..bytes.len() {
                assert!(lbp_mesh::decode_uvs(&bytes[..n], || false).is_err());
            }
            assert_eq!(
                lbp_mesh::decode_uvs(&bytes, || true).unwrap_err(),
                Error::Cancelled
            );
        }
    }
    for magic in [b"MSHe", b"NPD\0", b"SCE\0"] {
        assert_eq!(
            lbp_mesh::decode_uvs(magic, || false).unwrap_err(),
            Error::Blocked
        );
    }
}
#[test]
fn all_resource_prefixes_protection_and_cancel() {
    for b in [recipe(false, false), recipe(true, true)] {
        for n in 0..b.len() {
            assert!(inspect(&b[..n], || false).is_err(), "prefix {n}");
        }
    }
    for magic in [b"MSHe", b"NPD\0", b"SCE\0"] {
        assert_eq!(inspect(magic, || false).unwrap_err(), Error::Blocked);
    }
    assert_eq!(
        inspect(&recipe(true, true), || true).unwrap_err(),
        Error::Cancelled
    );
}
#[test]
fn count_varint_flags_and_nonfinite_rejected() {
    let mut b = recipe(true, false);
    b[18..23].copy_from_slice(&[255, 255, 255, 255, 127]);
    assert!(matches!(inspect(&b, || false), Err(Error::Invalid(_))));
    let mut b = recipe(false, false);
    b[13..17].copy_from_slice(&100001u32.to_be_bytes());
    assert_eq!(inspect(&b, || false).unwrap_err(), Error::Budget);
    let mut b = recipe(true, false);
    b[16] = 8;
    assert!(matches!(inspect(&b, || false), Err(Error::Unsupported(_))));
    let mut b = recipe(false, false);
    let position = 13 + 28 + 512 + 16;
    b[position..position + 4].copy_from_slice(&f32::NAN.to_bits().to_be_bytes());
    assert!(matches!(inspect(&b, || false), Err(Error::Unsupported(_))));
}

#[test]
fn stored_raw_and_strict_chunk_endings() {
    let plain = recipe(true, false);
    let end = u32::from_be_bytes(plain[8..12].try_into().unwrap()) as usize;
    let data = &plain[18..end];
    for stored in [false, true] {
        let mut b = plain[..18].to_vec();
        b[17] = 1;
        b.extend_from_slice(&[0, 1, 0, u8::from(stored)]);
        if stored {
            b.extend_from_slice(&(data.len() as u16).to_be_bytes());
            b.extend_from_slice(&(data.len() as u16).to_be_bytes());
        }
        b.extend_from_slice(data);
        let end = b.len() as u32;
        b[8..12].copy_from_slice(&end.to_be_bytes());
        b.extend_from_slice(&[0; 4]);
        assert_eq!(
            inspect(&b, || false).unwrap().position_bytes_sha256,
            inspect(&plain, || false).unwrap().position_bytes_sha256
        );
    }
    let mut b = recipe(true, true);
    b[25] += 1;
    assert!(matches!(inspect(&b, || false), Err(Error::Invalid(_))));
    let mut b = recipe(true, true);
    let end = u32::from_be_bytes(b[8..12].try_into().unwrap()) as usize;
    b.insert(end, 0);
    b[8..12].copy_from_slice(&((end + 1) as u32).to_be_bytes());
    let size = u16::from_be_bytes(b[22..24].try_into().unwrap()) + 1;
    b[22..24].copy_from_slice(&size.to_be_bytes());
    assert!(matches!(inspect(&b, || false), Err(Error::Invalid(_))));
}

#[test]
fn complete_geometry_tail_traversal_and_primitive_ranges() {
    for modern in [false, true] {
        let mut b = recipe(modern, false);
        let old_end = u32::from_be_bytes(b[8..12].try_into().unwrap()) as usize;
        b.truncate(old_end - 8);
        let prefix_end = b.len();
        integer(&mut b, 1, modern);
        for _ in 0..2 {
            integer(&mut b, 0, modern);
            b.push(0);
        }
        for n in [0, 2, 0, 3, 7] {
            integer(&mut b, n, modern);
        }
        integer(&mut b, 1, modern);
        b.extend_from_slice(&[0; 32]);
        for _ in 0..5 {
            integer(&mut b, 0, modern);
        }
        b.extend_from_slice(&vec![0; if modern { 4 } else { 128 }]);
        b.extend_from_slice(&[0; 32]);
        integer(&mut b, 0, modern);
        integer(&mut b, 0, modern);
        b.extend_from_slice(&[0; 56]);
        for _ in 0..3 {
            integer(&mut b, 0, modern);
        }
        b.push(5);
        b.extend_from_slice(b"tail");
        let end = b.len() as u32;
        b[8..12].copy_from_slice(&end.to_be_bytes());
        b.extend_from_slice(&[0; 4]);
        let g = lbp_mesh::decode_geometry(&b, || false).unwrap();
        assert_eq!(g.triangles, [[0, 1, 2]]);
        assert_eq!(g.metadata.bone_count, 1);
        assert_eq!(g.skeleton.bones.len(), 1);
        assert_eq!(
            g.skeleton.bones[0].skin_pose_mask,
            if modern { 0 } else { u16::MAX }
        );
        assert_eq!(g.metadata.unparsed_after_geometry, 4);
        assert_eq!(g.metadata.primitives[0].region, 7);
        for n in prefix_end..b.len() - 8 {
            let mut truncated = b[..n].to_vec();
            let end = truncated.len() as u32;
            truncated[8..12].copy_from_slice(&end.to_be_bytes());
            truncated.extend_from_slice(&[0; 4]);
            assert!(
                lbp_mesh::decode_geometry(&truncated, || false).is_err(),
                "geometry prefix {n}"
            );
        }
    }
}

#[test]
fn primitive_partitions_preserve_empty_overlap_degenerates_and_strip_resets() {
    for topology in [5, 6] {
        for indices in [[0u16, 1, 2], [0, 0, 1]] {
            let mut b = recipe(true, false);
            let end = u32::from_be_bytes(b[8..12].try_into().unwrap()) as usize;
            b.truncate(end - 8);
            let index_start = b.len() - 6;
            for (i, value) in indices.iter().enumerate() {
                b[index_start + i * 2..index_start + i * 2 + 2]
                    .copy_from_slice(&value.to_be_bytes());
            }
            integer(&mut b, 4, true);
            // Same source range twice; short strip and empty primitive keep ownership.
            for (first, count) in [
                (0, 3),
                (3, 0),
                (0, 3),
                (1, if topology == 6 { 2 } else { 0 }),
            ] {
                for _ in 0..2 {
                    integer(&mut b, 0, true);
                    b.push(0);
                }
                for value in [0, 2, first, count, 0] {
                    integer(&mut b, value, true);
                }
            }
            for _ in 0..4 {
                integer(&mut b, 0, true);
            }
            b.push(topology);
            let end = b.len() as u32;
            b[8..12].copy_from_slice(&end.to_be_bytes());
            b.extend_from_slice(&[0; 4]);
            let g = lbp_mesh::decode_geometry(&b, || false).unwrap();
            assert_eq!(g.primitive_triangles, [0..1, 1..1, 1..2, 2..2]);
            let triangle = indices.map(u32::from);
            assert_eq!(g.triangles, [triangle, triangle]);
            assert_eq!(
                lbp_mesh::decode_geometry(&b, || true).unwrap_err(),
                Error::Cancelled
            );
        }
    }
}
