//! Authored ANMb keyframe recipes; no original animation bytes. docs/animation-keys.md.
use lbp_resource::{animation, binary::Error};

fn integer(b: &mut Vec<u8>, value: u32, compact: bool) {
    if !compact {
        b.extend_from_slice(&value.to_be_bytes());
        return;
    }
    let mut v = value;
    loop {
        let byte = (v & 127) as u8;
        v >>= 7;
        b.push(byte | if v != 0 { 128 } else { 0 });
        if v == 0 {
            break;
        }
    }
}

fn wire(p: &[u8], revision: u32, compact: bool) -> Vec<u8> {
    let mut out = b"ANMb".to_vec();
    out.extend_from_slice(&revision.to_be_bytes());
    out.extend_from_slice(&(18 + p.len() as u32).to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, if compact { 7 } else { 0 }, 0]);
    out.extend_from_slice(p);
    out.extend_from_slice(&[0; 4]);
    out
}

fn h(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

/// 2 bones, 3 frames (loop start 1), 2 morphs. Animated: rotation of bone 1,
/// position of bone 0, morph 1. One locator key record, one locator.
fn recipe(compact: bool, track_index: u8, pad: usize) -> Vec<u8> {
    let mut blob = vec![];
    for (hash, links) in [
        (0x1111_2220u32, [0xff, 1, 0xff]),
        (0x3333_4440, [0, 0xff, 0xff]),
    ] {
        blob.extend_from_slice(&hash.to_be_bytes());
        blob.extend_from_slice(&links);
        blob.push(0);
    }
    blob.extend_from_slice(&[9, 8, 7, 6]); // locator key record
    blob.extend_from_slice(&[track_index, 0, 1]); // rot [1], pos [0], scale [], morph [1]
    blob.push(0); // align to 2
                  // Rotations: base bone 0 identity, base bone 1 identity, then bone 1 at frames 1, 2.
                  // 0x7fff/0x7fff = 1 on x -> w 0; LSB 1 marks negative w.
    for x in [0i16, 0, 0x7ffe, 0x5a83] {
        blob.extend_from_slice(&x.to_be_bytes());
        blob.extend_from_slice(&[0; 4]);
    }
    // Positions: bases (1,0,0) and (0,2,0), then bone 0 at frames 1, 2: (3,..), (4,..).
    for v in [
        [0x3c00u16, 0, 0],
        [0, 0x4000, 0],
        [0x4200, 0, 0],
        [0x4400, 0, 0],
    ] {
        for c in v {
            blob.extend_from_slice(&h(c));
        }
    }
    for _ in 0..2 {
        for c in [0x3c00u16; 3] {
            blob.extend_from_slice(&h(c)); // scales: bases only
        }
    }
    // Morphs: bases 0.5, 0.25, then morph 1 at frames 1, 2: 1.0, -1.0.
    for v in [0x3800u16, 0x3400, 0x3c00, 0xbc00] {
        blob.extend_from_slice(&h(v));
    }
    blob.extend(std::iter::repeat_n(0, pad));
    let mut p = vec![];
    for v in [3u16, 30, 1] {
        p.extend_from_slice(&v.to_be_bytes());
    }
    p.push(2); // morph count
    p.extend_from_slice(&[2, 1, 1, 0, 1]);
    p.extend_from_slice(&1u16.to_be_bytes()); // locator keys
    p.extend_from_slice(&[0; 32]); // posOffset, posScale (revision > 907)
    p.push(0); // fat (revision > 945)
    integer(&mut p, blob.len() as u32, compact);
    p.extend(blob);
    integer(&mut p, 1, compact); // one locator
    for v in [1.0f32, 2.0, 3.0] {
        p.extend_from_slice(&v.to_be_bytes());
    }
    integer(&mut p, if compact { 4 * 2 } else { 4 }, compact); // s32 length 4 (zigzag)
    p.extend_from_slice(b"foot");
    p.extend_from_slice(&[1, 0xfe]);
    p.extend_from_slice(b"tail");
    wire(&p, 950, compact)
}

#[test]
fn decodes_tables_locators_and_poses() {
    for compact in [false, true] {
        let a = animation::decode(&recipe(compact, 1, 6), || false).unwrap();
        assert_eq!(a.bones[0].parent, -1);
        assert_eq!(a.bones[1].anim_hash, 0x3333_4440);
        assert_eq!(a.locator_keys, [[9, 8, 7, 6]]);
        assert_eq!(
            (a.rotations.len(), a.positions.len(), a.scales.len()),
            (4, 4, 2)
        );
        assert_eq!(a.morphs.len(), 4);
        assert_eq!((a.trailing_bytes, a.clamped_rotations), (6, 0));
        assert_eq!(a.locators[0].name, "foot");
        assert_eq!((a.locators[0].looping, a.locators[0].kind), (1, -2));
        assert_eq!(a.locators[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(a.unparsed_after_locators, 4);

        let p0 = a.pose(0).unwrap();
        assert_eq!(p0.positions, [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0]]);
        assert_eq!(p0.morphs, [0.5, 0.25]);
        assert_eq!(p0.rotations[1], [0.0, 0.0, 0.0, 1.0]);
        let p1 = a.pose(1).unwrap();
        assert_eq!(p1.positions[0], [3.0, 0.0, 0.0]);
        assert_eq!(p1.positions[1], [0.0, 2.0, 0.0]);
        let r = p1.rotations[1];
        assert!((r[0] - 32766.0 / 32767.0).abs() < 1e-6 && r[3] >= 0.0);
        assert_eq!(p1.morphs, [0.5, 1.0]);
        let p2 = a.pose(2).unwrap();
        assert_eq!(p2.positions[0], [4.0, 0.0, 0.0]);
        assert!(p2.rotations[1][3] < 0.0); // LSB of x set
        assert_eq!(p2.morphs, [0.5, -1.0]);
        assert!(a.pose(3).is_none());

        assert_eq!(a.looped_frame(3, true), Some(1));
        assert_eq!(a.looped_frame(-2, true), Some(2));
        assert_eq!(a.looped_frame(7, false), Some(2));
        assert_eq!(a.looped_frame(-4, false), Some(0));
    }
}

#[test]
fn rejects_bad_indices_truncation_and_cancel() {
    assert!(matches!(
        animation::decode(&recipe(true, 2, 0), || false),
        Err(Error::Invalid("animated track index"))
    ));
    let good = recipe(true, 1, 0);
    // Truncating the payload anywhere inside the blob or locators is an error.
    let payload = &good[18..good.len() - 4];
    for keep in [30, 60, payload.len() - 10] {
        assert!(animation::decode(&wire(&payload[..keep], 950, true), || false).is_err());
    }
    assert!(matches!(
        animation::decode(&good, || true),
        Err(Error::Cancelled)
    ));
}
