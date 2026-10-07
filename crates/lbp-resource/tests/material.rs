//! Authored material structures; no original game/shader bytes or reference templates.
use lbp_resource::{binary::Error, material};
use std::{cell::Cell, io::Write};
fn integer(b: &mut Vec<u8>, mut v: u32, compact: bool) {
    if !compact {
        b.extend_from_slice(&v.to_be_bytes());
        return;
    }
    loop {
        let byte = (v & 127) as u8;
        v >>= 7;
        b.push(byte | if v != 0 { 128 } else { 0 });
        if v == 0 {
            break;
        }
    }
}
fn recipe(compact: bool) -> Vec<u8> {
    let mut p = vec![];
    integer(&mut p, 0x10000, compact);
    p.extend_from_slice(&0x7fc00001u32.to_be_bytes()); // Raw NaN bits remain diagnostic.
    p.extend_from_slice(&[255, 17, 254]); // Unknown enums/raw bytes not defaulted.
    for v in [0x80000000u32, 0x3f800000, 0, 0xffffffff] {
        p.extend_from_slice(&v.to_be_bytes());
    }
    for end in 1..=10 {
        integer(&mut p, end, compact);
    }
    integer(&mut p, 16, compact);
    p.extend_from_slice(b"synthetic-opaque");
    for kind in [0, 1, 2, 3, 2, 1, 0, 0] {
        integer(&mut p, u32::MAX, compact);
        p.push(kind);
        if kind & 2 != 0 {
            integer(&mut p, if kind == 3 { 17 } else { 0 }, compact);
        }
        if kind & 1 != 0 {
            p.extend_from_slice(&[if kind == 3 { 0xab } else { 0 }; 20]);
        }
    }
    integer(&mut p, 2, compact);
    p.extend_from_slice(&[1, 255]);
    integer(&mut p, 1, compact);
    p.push(4);
    p.extend_from_slice(b"opaque-tail");
    p
}
fn wire(p: &[u8], compact: bool, mode: u8) -> Vec<u8> {
    let mut body = vec![];
    if mode == 0 {
        body.extend_from_slice(p);
    } else if mode == 3 {
        body.extend_from_slice(&[0, 1, 0, 0]);
        body.extend_from_slice(p);
    } else {
        let stored = if mode == 1 {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(p).unwrap();
            z.finish().unwrap()
        } else {
            p.to_vec()
        };
        body.extend_from_slice(&[0, 1, 0, 1]);
        body.extend_from_slice(&(stored.len() as u16).to_be_bytes());
        body.extend_from_slice(&(p.len() as u16).to_be_bytes());
        body.extend(stored);
    }
    let mut b = b"GMTb".to_vec();
    b.extend_from_slice(&950u32.to_be_bytes());
    b.extend_from_slice(&(18u32 + body.len() as u32).to_be_bytes());
    b.extend_from_slice(&[0; 4]);
    b.push(if compact { 7 } else { 0 });
    b.push(u8::from(mode != 0));
    b.extend(body);
    b.extend_from_slice(&[0; 4]);
    b
}
#[test]
fn raw_stored_zero_and_zlib_diagnostics_preserve_bits_and_nulls() {
    for compact in [false, true] {
        for mode in 0..4 {
            let parsed =
                material::inspect(&wire(&recipe(compact), compact, mode), || false).unwrap();
            let p = parsed.prefix;
            assert_eq!(p.scalars.flags, 0x10000);
            assert_eq!(p.scalars.alpha_test_bits, 0x7fc00001);
            assert_eq!(p.scalars.shadow_cast_raw, 254);
            assert_eq!(p.shader_count, 10);
            assert_eq!(p.uninterpreted_after_shader_bytes, 6);
            assert_eq!(p.unparsed_bytes, 11);
            assert_eq!(
                p.textures[1].sha1.as_deref(),
                Some("0000000000000000000000000000000000000000")
            );
            assert_eq!(p.textures[2].guid, Some(0));
            assert_eq!(p.textures[3].guid, Some(17));
            assert_eq!(
                p.textures[3].sha1.as_deref(),
                Some("abababababababababababababababababababab")
            );
            assert_eq!(p.wrap_s_raw, [1, 255]);
            assert_eq!(p.wrap_t_raw, [4]);
            assert!(p.textures.iter().all(|t| t.flags == u32::MAX));
        }
    }
}
#[test]
fn every_wire_truncation_protection_and_unknown_revision() {
    for compact in [false, true] {
        for mode in 0..4 {
            let b = wire(&recipe(compact), compact, mode);
            for n in 0..b.len() {
                assert!(
                    material::inspect(&b[..n], || false).is_err(),
                    "{compact}/{mode}/{n}"
                );
            }
            // Rebuild a valid envelope around each short payload, so metadata
            // validation cannot mask a padded/overrunning material reader.
            let p = recipe(compact);
            for n in 0..p.len() - 11 {
                assert!(
                    material::inspect(&wire(&p[..n], compact, mode), || false).is_err(),
                    "payload {compact}/{mode}/{n}"
                );
            }
        }
    }
    for magic in [b"GMTe", b"NPD\0", b"SCE\0"] {
        assert_eq!(
            material::inspect(magic, || false).unwrap_err(),
            Error::Blocked
        );
    }
    for rev in [919u32, 952, 0x10000] {
        let mut b = wire(&recipe(false), false, 0);
        b[4..8].copy_from_slice(&rev.to_be_bytes());
        assert!(matches!(
            material::inspect(&b, || false),
            Err(Error::Unsupported(_))
        ));
    }
    let mut b = wire(&recipe(false), false, 0);
    b[14] = 1;
    assert!(material::inspect(&b, || false).is_err());
}
#[test]
fn malformed_intervals_counts_varints_and_descriptors() {
    let p = recipe(false);
    for (offset, value) in [(27, 17u32), (31, 0), (67, u32::MAX)] {
        let mut invalid = p.clone();
        invalid[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        assert!(material::inspect(&wire(&invalid, false, 0), || false).is_err());
    }
    let mut invalid = p.clone();
    invalid[91] = 4; // First descriptor kind.
    assert!(matches!(
        material::inspect(&wire(&invalid, false, 0), || false),
        Err(Error::Unsupported(_))
    ));
    let mut invalid = recipe(true);
    invalid.splice(0..3, [255, 255, 255, 255, 127]);
    assert!(matches!(
        material::inspect(&wire(&invalid, true, 0), || false),
        Err(Error::Invalid(_))
    ));
    let mut invalid = p.clone();
    let wraps = invalid.len() - 11 - 3 - 4 - 2 - 4;
    invalid[wraps..wraps + 4].copy_from_slice(&9u32.to_be_bytes());
    assert_eq!(
        material::inspect(&wire(&invalid, false, 0), || false).unwrap_err(),
        Error::Budget
    );
}
#[test]
fn budgets_cancel_and_unaccounted_chunk_bytes() {
    let mut b = vec![0; material::MAX_INPUT + 1];
    b[..4].copy_from_slice(b"GMTb");
    assert_eq!(material::inspect(&b, || false).unwrap_err(), Error::Budget);
    let b = wire(&recipe(true), true, 1);
    assert_eq!(
        material::inspect(&b, || true).unwrap_err(),
        Error::Cancelled
    );
    let calls = Cell::new(0);
    assert_eq!(
        material::inspect(&b, || {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        })
        .unwrap_err(),
        Error::Cancelled
    );
    let mut b = b;
    b.insert(b.len() - 4, 0);
    let end = (b.len() - 4) as u32;
    b[8..12].copy_from_slice(&end.to_be_bytes());
    assert!(matches!(
        material::inspect(&b, || false),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        lbp_resource::binary::expand(&[0, 0, 0, 0, 1, 2], 1, &|| false).unwrap_err(),
        Error::Budget
    );
    assert_eq!(
        lbp_resource::binary::expand(&[0, 1, 0, 1, 0, 1, 0, 2, 0], 1, &|| false).unwrap_err(),
        Error::Budget
    );
}
