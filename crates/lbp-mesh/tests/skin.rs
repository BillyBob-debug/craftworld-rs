//! Authored skin/morph stream recipes; no proprietary fixtures.
use lbp_mesh::{decode_skin, Error};

fn uleb(b: &mut Vec<u8>, mut n: u32) {
    loop {
        let v = (n & 127) as u8;
        n >>= 7;
        b.push(v | if n > 0 { 128 } else { 0 });
        if n == 0 {
            break;
        }
    }
}

/// Modern (rev 950, flags 7) uncompressed MSHb with two vertices, the given stream and
/// morph counts, stream 1 rows as given, and morph stream rows filled per morph.
fn recipe(streams: u32, morphs: u32, skin_rows: [[u8; 16]; 2]) -> Vec<u8> {
    let mut p = vec![];
    for n in [2, 0, 0, 0, streams, 0, morphs] {
        uleb(&mut p, n);
    }
    let mut names = [0u8; 512];
    names[..5].copy_from_slice(b"Smile");
    names[16..20].copy_from_slice(b"Blnk");
    p.extend_from_slice(&names);
    for _ in 0..2 {
        uleb(&mut p, 0);
    }
    p.extend_from_slice(&1f32.to_be_bytes());
    for _ in 0..streams + 2 {
        uleb(&mut p, 0);
    }
    p.extend_from_slice(&[0; 32]); // stream 0: two zero positions
    if streams > 1 {
        for row in skin_rows {
            p.extend_from_slice(&row);
        }
    }
    for s in 2..streams {
        for v in 0..2u32 {
            for c in 0..3 {
                p.extend_from_slice(&((s * 10 + v * 3 + c) as f32).to_be_bytes());
            }
            p.extend_from_slice(&0x3ffu32.to_be_bytes()); // normal (1, 0, 0)
        }
    }
    uleb(&mut p, 0); // attributes
    uleb(&mut p, 0); // indices
    let mut b = b"MSHb".to_vec();
    b.extend_from_slice(&950u32.to_be_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&[0, 0, 0, 0, 7]);
    b.push(0);
    b.extend(p);
    let end = b.len() as u32;
    b[8..12].copy_from_slice(&end.to_be_bytes());
    b.extend_from_slice(&[0; 4]);
    b
}

#[test]
fn joints_weights_normals_and_morphs() {
    // Vertex 0: weights bytes [w2=10, w1=20, w0=200] -> (200,20,10,25)/255, joints 1,2,3,4,
    // normal x=0x7ff (1,0,0). Vertex 1: single-bone 0xFF case, joint 9.
    let v0 = [10, 20, 200, 1, 0, 0x07, 0xff, 2, 0, 0, 0, 3, 0x80, 0, 0, 4];
    let v1 = [0, 0, 0xff, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let s = decode_skin(&recipe(4, 2, [v0, v1]), || false).unwrap();
    assert_eq!(s.joints, [[1, 2, 3, 4], [9, 0, 0, 0]]);
    assert_eq!(
        s.weights[0],
        [200.0 / 255.0, 20.0 / 255.0, 10.0 / 255.0, 25.0 / 255.0]
    );
    assert_eq!(s.weights[1], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(s.normals[0][0], 1.0);
    assert_eq!(s.normals[1], [0.0, 0.0, 1.0]);
    assert_eq!(s.tangents[0], [0.0, 0.0, -1.0]); // bytes 12..15, bit 23 set
    assert_eq!(s.smooth_normals[0], [0.0, 0.0, 1.0]);
    let m = &s.metadata;
    assert_eq!((m.vertex_count, m.stream_count, m.morph_count), (2, 4, 2));
    assert_eq!(m.max_weighted_joint, Some(9));
    assert_eq!((m.single_bone_vertices, m.weight_overflow_vertices), (1, 0));
    assert_eq!(m.morph_names, ["Smile", "Blnk"]);
    assert_eq!(s.morphs[1].name, "Blnk");
    assert_eq!(s.morphs[0].offsets[1], [23.0, 24.0, 25.0]);
    assert_eq!(s.morphs[1].offsets[0], [30.0, 31.0, 32.0]);
    assert_eq!(s.morphs[0].normals[0], [1.0, 0.0, 0.0]);
}

#[test]
fn overflowing_weights_are_kept_and_counted() {
    let v = [100, 100, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let s = decode_skin(&recipe(2, 0, [v, v]), || false).unwrap();
    assert_eq!(s.metadata.weight_overflow_vertices, 2);
    assert!(s.weights[0][3] < 0.0);
    assert!(s.morphs.is_empty() && s.metadata.morph_names.is_empty());
}

#[test]
fn missing_streams_and_cancel() {
    let z = [0u8; 16];
    assert!(matches!(
        decode_skin(&recipe(1, 0, [z, z]), || false),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        decode_skin(&recipe(3, 2, [z, z]), || false),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        decode_skin(&recipe(2, 0, [z, z]), || true),
        Err(Error::Cancelled)
    ));
}
