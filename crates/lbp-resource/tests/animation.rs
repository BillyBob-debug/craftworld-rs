//! Authored in-memory structures, no original animation bytes. Provenance in
//! docs/animation-prefix.md; deliberately raw/unknown values do not imply playback.
use lbp_resource::{animation, binary::Error};
use std::{cell::Cell, io::Write};
fn integer(b: &mut Vec<u8>, mut value: u32, compact: bool) {
    if !compact {
        b.extend_from_slice(&value.to_be_bytes());
        return;
    }
    loop {
        let v = (value & 127) as u8;
        value >>= 7;
        b.push(v | if value != 0 { 128 } else { 0 });
        if value == 0 {
            break;
        }
    }
}
fn recipe(revision: u32, compact: bool) -> Vec<u8> {
    let mut p = vec![];
    for raw in [0x8000u16, 0, 0xffff] {
        p.extend_from_slice(&raw.to_be_bytes());
    }
    p.extend_from_slice(&[255, 255, 17, 254, 13, 0]);
    p.extend_from_slice(&0xffffu16.to_be_bytes());
    if revision > 907 {
        for value in [
            0x7fc00001u32,
            0x80000000,
            0x7f800000,
            0xffffffff,
            0x3f800000,
            1,
            0,
            0xdeadbeef,
        ] {
            p.extend_from_slice(&value.to_be_bytes());
        }
    }
    if revision > 945 {
        p.push(255);
    }
    integer(&mut p, 8, compact);
    p.extend_from_slice(b"authored");
    p.extend_from_slice(b"opaque-tail");
    p
}
fn wire(p: &[u8], revision: u32, compact: bool, mode: u8) -> Vec<u8> {
    let body = if mode == 0 {
        p.to_vec()
    } else if mode == 2 {
        [vec![0, 1, 0, 0], p.to_vec()].concat()
    } else {
        let data = if mode == 1 {
            let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
            z.write_all(p).unwrap();
            z.finish().unwrap()
        } else {
            p.to_vec()
        };
        [
            vec![0, 1, 0, 1],
            (data.len() as u16).to_be_bytes().to_vec(),
            (p.len() as u16).to_be_bytes().to_vec(),
            data,
        ]
        .concat()
    };
    let mut out = b"ANMb".to_vec();
    out.extend_from_slice(&revision.to_be_bytes());
    out.extend_from_slice(&(18 + body.len() as u32).to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, if compact { 7 } else { 0 }, u8::from(mode != 0)]);
    out.extend(body);
    out.extend_from_slice(&[0; 4]);
    out
}
#[test]
fn gates_raw_values_nan_bits_and_all_wrapper_modes_remain_diagnostic() {
    for revision in [888, 907, 908, 945, 946, 951] {
        for compact in [false, true] {
            for mode in 0..4 {
                let p = animation::inspect(
                    &wire(&recipe(revision, compact), revision, compact, mode),
                    || false,
                )
                .unwrap()
                .prefix;
                assert_eq!(
                    (
                        p.frames_raw_u16,
                        p.fps_raw_u16,
                        p.loop_start_raw_u16,
                        p.morph_count_raw_u8
                    ),
                    (0x8000, 0, 0xffff, 255)
                );
                assert_eq!(p.counts, [255, 17, 254, 13, 0, 65535]);
                assert_eq!(
                    p.position_offset_bits,
                    if revision > 907 {
                        Some([0x7fc00001, 0x80000000, 0x7f800000, 0xffffffff])
                    } else {
                        None
                    }
                );
                assert_eq!(p.fat_raw_u8, if revision > 945 { Some(255) } else { None });
                assert_eq!(p.animation_data_bytes, 8);
                assert_eq!(p.animation_data_start, p.parsed_prefix_bytes);
                assert_eq!(p.unparsed_bytes, 19); // Opaque animation8 + uninterpreted tail11.
            }
        }
    }
}
#[test]
fn prefix_and_blob_truncations_reject_without_consuming_tail_records() {
    for compact in [false, true] {
        let raw = recipe(946, compact);
        let start = animation::inspect(&wire(&raw, 946, compact, 0), || false)
            .unwrap()
            .prefix
            .animation_data_start;
        for n in 0..start + 8 {
            assert!(
                animation::inspect(&wire(&raw[..n], 946, compact, 0), || false).is_err(),
                "prefix {n}"
            );
        }
        assert_eq!(
            animation::inspect(&wire(&raw[..start + 8], 946, compact, 0), || false)
                .unwrap()
                .prefix
                .unparsed_bytes,
            8
        );
        let input = wire(&raw, 946, compact, 1);
        for n in 0..input.len() {
            assert!(
                animation::inspect(&input[..n], || false).is_err(),
                "wire {n}"
            );
        }
    }
}
#[test]
fn unsupported_revision_branch_flags_protection_and_length_overflows_are_explicit() {
    for revision in [844, 887, 952] {
        assert!(matches!(
            animation::inspect(&wire(&recipe(946, true), revision, true, 0), || false),
            Err(Error::Unsupported(_))
        ));
    }
    let mut input = wire(&recipe(946, true), 946, true, 0);
    input[13] = 1;
    assert!(matches!(
        animation::inspect(&input, || false),
        Err(Error::Unsupported(_))
    ));
    input[13] = 0;
    input[16] = 1;
    assert!(matches!(
        animation::inspect(&input, || false),
        Err(Error::Unsupported(_))
    ));
    for prefix in [b"ANMe", b"NPD\0", b"SCE\0"] {
        assert_eq!(
            animation::inspect(prefix, || false).unwrap_err(),
            Error::Blocked
        );
        let long = [prefix.as_slice(), &[0; 64]].concat();
        assert_eq!(
            animation::inspect(&long, || false).unwrap_err(),
            Error::Blocked
        );
    }
    let mut raw = recipe(946, true);
    raw.truncate(47);
    raw.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x10]);
    assert!(matches!(
        animation::inspect(&wire(&raw, 946, true, 0), || false),
        Err(Error::Invalid(_))
    ));
    raw.truncate(47);
    integer(&mut raw, u32::MAX, true);
    assert_eq!(
        animation::inspect(&wire(&raw, 946, true, 0), || false).unwrap_err(),
        Error::Budget
    );
}
#[test]
fn input_expansion_budgets_and_cancellation_before_and_after_inflation() {
    let input = wire(&recipe(946, true), 946, true, 1);
    assert_eq!(
        animation::inspect(&input, || true).unwrap_err(),
        Error::Cancelled
    );
    let calls = Cell::new(0);
    assert_eq!(
        animation::inspect(&input, || {
            calls.set(calls.get() + 1);
            calls.get() == 4
        })
        .unwrap_err(),
        Error::Cancelled
    );
    let mut large = b"ANMb".to_vec();
    large.resize(animation::MAX_INPUT + 1, 0);
    assert_eq!(
        animation::inspect(&large, || false).unwrap_err(),
        Error::Budget
    );
    // 257 individually bounded chunk entries exceed the total expansion cap.
    let mut chunks = vec![0, 1, 1, 1];
    for _ in 0..257 {
        chunks.extend_from_slice(&[0, 1, 255, 255]);
    }
    chunks.extend(std::iter::repeat_n(0, 257));
    let mut huge = wire(&[], 946, true, 0);
    huge.truncate(18);
    huge[17] = 1;
    huge[8..12].copy_from_slice(&(18 + chunks.len() as u32).to_be_bytes());
    huge.extend(chunks);
    huge.extend_from_slice(&[0; 4]);
    assert_eq!(
        animation::inspect(&huge, || false).unwrap_err(),
        Error::Budget
    );
}
