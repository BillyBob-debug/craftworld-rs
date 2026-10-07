//! Authored synthetic plans only. No Thing/inventory compatibility implied.
use lbp_resource::{binary::Error, plan};
use sha2::{Digest, Sha256};
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
fn recipe(compact: bool, opaque: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut p = vec![];
    integer(&mut p, u32::MAX, compact); // Deliberate inner/outer mismatch remains raw.
    integer(&mut p, opaque.len() as u32, compact);
    p.extend_from_slice(opaque);
    p.extend_from_slice(tail);
    p
}
fn wire(p: &[u8], revision: u32, compact: bool, mode: u8) -> Vec<u8> {
    let body = if mode == 0 {
        p.to_vec()
    } else if mode == 2 {
        [vec![0, 1, 0, 0], p.to_vec()].concat()
    } else {
        let mut data = if mode == 1 {
            let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
            z.write_all(p).unwrap();
            z.finish().unwrap()
        } else {
            p.to_vec()
        };
        // Equal stored/expanded sizes mark a stored chunk in this format. If a
        // synthetic zlib happens to have that size, choose a distinct valid stream.
        if mode == 1 && data.len() == p.len() {
            let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::none());
            z.write_all(p).unwrap();
            data = z.finish().unwrap();
            assert_ne!(data.len(), p.len());
        }
        [
            vec![0, 1, 0, 1],
            (data.len() as u16).to_be_bytes().to_vec(),
            (p.len() as u16).to_be_bytes().to_vec(),
            data,
        ]
        .concat()
    };
    let mut out = b"PLNb".to_vec();
    out.extend_from_slice(&revision.to_be_bytes());
    out.extend_from_slice(&(18 + body.len() as u32).to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, if compact { 7 } else { 0 }, u8::from(mode != 0)]);
    out.extend(body);
    out.extend_from_slice(&2u32.to_be_bytes());
    for _ in 0..2 {
        out.push(2);
        out.extend_from_slice(&42u32.to_be_bytes());
        out.extend_from_slice(&1u32.to_be_bytes());
    }
    out
}
#[test]
fn raw_inner_zero_blob_all_modes_and_tail_remain_uninterpreted() {
    for revision in [893, 919, 951] {
        for compact in [false, true] {
            for mode in 0..4 {
                for opaque in [b"authored".as_slice(), b""] {
                    let input = wire(
                        &recipe(compact, opaque, b"unknown-inventory"),
                        revision,
                        compact,
                        mode,
                    );
                    let original = input.clone();
                    let m = plan::inspect(&input, || false).unwrap();
                    let p = m.prefix;
                    assert_eq!(input, original);
                    assert_eq!(p.inner_revision_raw_u32, u32::MAX);
                    assert_eq!(p.thing_data_bytes, opaque.len());
                    assert_eq!(p.thing_data_sha256, format!("{:x}", Sha256::digest(opaque)));
                    assert_eq!(p.inventory_tail_bytes, 17);
                    assert_eq!(p.unparsed_bytes, opaque.len() + 17);
                    assert_eq!(p.thing_data_start, p.parsed_prefix_bytes);
                    assert_eq!(p.inventory_tail_start, p.thing_data_start + opaque.len());
                    assert_eq!(m.expanded_bytes, p.inventory_tail_start + 17);
                }
            }
        }
    }
}
#[test]
fn every_prefix_blob_wrapper_truncation_rejects_but_tail_is_not_required() {
    for compact in [false, true] {
        let raw = recipe(compact, b"authored", b"tail");
        let start = plan::inspect(&wire(&raw, 919, compact, 0), || false)
            .unwrap()
            .prefix
            .thing_data_start;
        for n in 0..start + 8 {
            assert!(
                plan::inspect(&wire(&raw[..n], 919, compact, 0), || false).is_err(),
                "prefix {n}"
            );
        }
        let m = plan::inspect(&wire(&raw[..start + 8], 919, compact, 0), || false).unwrap();
        assert_eq!(m.prefix.inventory_tail_bytes, 0); // Not an empty/valid inventory assertion.
        let input = wire(&raw, 919, compact, 1);
        for n in 0..input.len() {
            assert!(plan::inspect(&input[..n], || false).is_err(), "wire {n}");
        }
    }
}
#[test]
fn protected_unknown_branches_flags_and_integer_lengths_are_explicit() {
    for revision in [892, 0x3f9] {
        assert!(matches!(
            plan::inspect(&wire(&recipe(true, b"", b""), revision, true, 0), || false),
            Err(Error::Unsupported(_))
        ));
    }
    for at in [13, 15, 16] {
        let mut input = wire(&recipe(true, b"", b""), 919, true, 0);
        input[at] = 1;
        assert!(matches!(
            plan::inspect(&input, || false),
            Err(Error::Unsupported(_))
        ));
    }
    for marker in [b"PLNe", b"NPD\0", b"SCE\0", b"\x7fPKG"] {
        for input in [marker.to_vec(), [marker.as_slice(), &[0; 64]].concat()] {
            assert_eq!(plan::inspect(&input, || false).unwrap_err(), Error::Blocked);
        }
    }
    let mut p = vec![];
    integer(&mut p, 919, true);
    p.extend_from_slice(&[255, 255, 255, 255, 16]);
    assert!(matches!(
        plan::inspect(&wire(&p, 919, true, 0), || false),
        Err(Error::Invalid(_))
    ));
    p.clear();
    integer(&mut p, 919, true);
    integer(&mut p, u32::MAX, true);
    assert_eq!(
        plan::inspect(&wire(&p, 919, true, 0), || false).unwrap_err(),
        Error::Budget
    );
    let input = wire(&recipe(true, b"", b""), 919, true, 0);
    let mut bad = input.clone();
    let end = u32::from_be_bytes(bad[8..12].try_into().unwrap()) as usize;
    bad[end + 4] = 8;
    assert!(plan::inspect(&bad, || false).is_err());
}
#[test]
fn budgets_cancel_before_inflation_and_during_opaque_hash() {
    let input = [b"PLNb".as_slice(), &vec![0; plan::MAX_INPUT]].concat();
    assert_eq!(plan::inspect(&input, || false).unwrap_err(), Error::Budget);
    assert_eq!(plan::inspect(&[], || true).unwrap_err(), Error::Cancelled);
    let mut chunks = vec![0, 0, 1, 1]; // 257 chunks, declared >16MiB; no body allocation.
    for _ in 0..257 {
        chunks.extend_from_slice(&[0, 0, 255, 255]);
    }
    let mut input = wire(&chunks, 919, true, 0);
    input[17] = 1;
    assert_eq!(plan::inspect(&input, || false).unwrap_err(), Error::Budget);
    let input = wire(&recipe(true, &vec![1; 20000], b"tail"), 919, true, 0);
    let calls = Cell::new(0);
    assert_eq!(
        plan::inspect(&input, || {
            calls.set(calls.get() + 1);
            calls.get() == 5
        })
        .unwrap_err(),
        Error::Cancelled
    );
    assert_eq!(calls.get(), 5);
}
fn long_integer(b: &mut Vec<u8>, mut value: u64, compact: bool) {
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
fn inventory_tail(compact: bool, date: u64) -> Vec<u8> {
    let mut p = vec![];
    long_integer(&mut p, date, compact);
    for value in [
        u32::MAX,
        u32::MAX,
        0,
        0xff00ff00,
        u32::MAX,
        0x80000000,
        42,
        0,
    ] {
        integer(&mut p, value, compact);
    }
    p.extend_from_slice(&[255; 4]); // Deliberately invalid/unknown recursive tag is untouched.
    p
}
#[test]
fn inventory_raw64_unknown32_and_all_envelopes_do_not_normalize_or_recurse() {
    for revision in [893, 919, 951] {
        for compact in [false, true] {
            for mode in 0..4 {
                for date in [0, 1, 2, u64::MAX / 2, 1 << 63, u64::MAX - 1, u64::MAX] {
                    let input = wire(
                        &recipe(compact, b"authored", &inventory_tail(compact, date)),
                        revision,
                        compact,
                        mode,
                    );
                    let result = plan::inspect_inventory(&input, || false).unwrap();
                    let m = result.inventory.unwrap();
                    assert_eq!(m.date_added_encoded_u64_hex, format!("{date:016x}"));
                    assert_eq!(m.slot_type_raw_u32, u32::MAX);
                    assert_eq!(m.slot_number_raw_u32, u32::MAX);
                    assert_eq!(m.highlight_sound_guid_raw_u32, 0);
                    assert_eq!(m.colour_raw_u32, 0xff00ff00);
                    assert_eq!(m.type_flags_raw_u32, u32::MAX);
                    assert_eq!(m.sub_type_raw_u32, 0x80000000);
                    assert_eq!(m.title_key_raw_u32, 42);
                    assert_eq!(m.description_key_raw_u32, 0);
                    assert_eq!(
                        m.inventory_prefix_start,
                        result.plan.prefix.inventory_tail_start
                    );
                    assert_eq!(
                        m.inventory_prefix_end,
                        m.inventory_prefix_start + m.parsed_inventory_prefix_bytes
                    );
                    assert_eq!(m.unparsed_after_inventory_prefix, 4);
                }
            }
        }
    }
}
#[test]
fn inventory_truncation_preserves_base_prefix_and_stops_before_reference() {
    for compact in [false, true] {
        let tail = inventory_tail(compact, u64::MAX);
        for n in 0..tail.len() - 4 {
            let input = wire(&recipe(compact, b"authored", &tail[..n]), 919, compact, 0);
            let result = plan::inspect_inventory(&input, || false).unwrap();
            assert_eq!(result.plan.prefix.thing_data_bytes, 8);
            assert!(
                matches!(result.inventory, Err(Error::Invalid(_))),
                "tail {n}"
            );
        }
        let input = wire(
            &recipe(compact, b"authored", &tail[..tail.len() - 4]),
            919,
            compact,
            0,
        );
        let result = plan::inspect_inventory(&input, || false).unwrap();
        assert_eq!(result.inventory.unwrap().unparsed_after_inventory_prefix, 0);
        assert_eq!(result.plan.prefix.inventory_tail_bytes, tail.len() - 4);
    }
}
#[test]
fn inventory_u64_u32_overflows_are_deeper_errors_without_mutating_plan_prefix() {
    for suffix in [vec![255; 10], [vec![255; 9], vec![2]].concat()] {
        let input = wire(&recipe(true, b"authored", &suffix), 919, true, 0);
        let result = plan::inspect_inventory(&input, || false).unwrap();
        assert_eq!(result.plan.prefix.thing_data_bytes, 8);
        assert!(matches!(result.inventory, Err(Error::Invalid(_))));
    }
    let tail = [vec![0], vec![255, 255, 255, 255, 16]].concat();
    let input = wire(&recipe(true, b"authored", &tail), 919, true, 0);
    let result = plan::inspect_inventory(&input, || false).unwrap();
    assert!(matches!(result.inventory, Err(Error::Invalid(_))));
    assert_eq!(result.plan.prefix.inner_revision_raw_u32, u32::MAX);
}
#[test]
fn inventory_parent_errors_and_stage_cancellation_remain_distinct() {
    let mut input = wire(
        &recipe(true, b"authored", &inventory_tail(true, 0)),
        919,
        true,
        0,
    );
    assert_eq!(
        plan::inspect_inventory(&input, || true).unwrap_err(),
        Error::Cancelled
    );
    let calls = Cell::new(0);
    let result = plan::inspect_inventory(&input, || {
        calls.set(calls.get() + 1);
        calls.get() == 6
    })
    .unwrap();
    assert_eq!(result.inventory.unwrap_err(), Error::Cancelled);
    assert_eq!(result.plan.prefix.thing_data_bytes, 8);
    input[3] = b'e';
    assert_eq!(
        plan::inspect_inventory(&input, || false).unwrap_err(),
        Error::Blocked
    );
    input[3] = b'b';
    input[13] = 1;
    assert!(matches!(
        plan::inspect_inventory(&input, || false),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn d73_widened_gate_accepts_952_and_0x3f8_same_layout_and_rejects_above() {
    // Gate proof only (D69): head 951 layout reused; not payload compatibility on 01.33 files.
    for revision in [951, 952, 1000, 0x3f8] {
        for compact in [false, true] {
            let input = wire(
                &recipe(compact, b"authored", b"unknown-inventory"),
                revision,
                compact,
                0,
            );
            let p = plan::inspect(&input, || false).unwrap().prefix;
            assert_eq!(p.thing_data_bytes, 8, "revision {revision}");
            assert_eq!(p.inventory_tail_bytes, 17);
            let tail = inventory_tail(compact, 1);
            let input = wire(&recipe(compact, b"authored", &tail), revision, compact, 0);
            assert!(plan::inspect_inventory(&input, || false)
                .unwrap()
                .inventory
                .is_ok());
        }
    }
    for revision in [0x3f9, 0x3fa, 0x10000] {
        let input = wire(&recipe(true, b"", b""), revision, true, 0);
        assert!(matches!(
            plan::inspect(&input, || false),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            plan::inspect_inventory(&input, || false),
            Err(Error::Unsupported(_))
        ));
    }
    // Lower bound unchanged.
    let input = wire(&recipe(true, b"", b""), 892, true, 0);
    assert!(matches!(
        plan::inspect(&input, || false),
        Err(Error::Unsupported(_))
    ));
}
