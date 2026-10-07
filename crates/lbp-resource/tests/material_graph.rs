//! Authored graph recipes, no game assets/shaders/unknown-provenance fixtures.
use lbp_resource::{
    binary::Error,
    material::{inspect, inspect_graph},
};
use sha2::{Digest, Sha256};
use std::{cell::Cell, io::Write};
struct Builder {
    bytes: Vec<u8>,
    compact: bool,
}
impl Builder {
    fn int(&mut self, mut n: u32) {
        if !self.compact {
            self.word(n);
            return;
        }
        loop {
            let byte = (n & 127) as u8;
            n >>= 7;
            self.bytes.push(byte | if n > 0 { 128 } else { 0 });
            if n == 0 {
                break;
            }
        }
    }
    fn word(&mut self, n: u32) {
        self.bytes.extend_from_slice(&n.to_be_bytes());
    }
    fn signed(&mut self, n: i32) {
        self.int(if self.compact {
            (n as u32) << 1 ^ (n >> 31) as u32
        } else {
            n as u32
        });
    }
    fn animation(&mut self, populated: bool) -> (usize, usize, usize) {
        for bits in [0x7fc00001, 0x80000000, 0x3f800000, 0] {
            self.word(bits);
        }
        let count = self.bytes.len();
        self.int(if populated { 4 } else { 0 });
        if populated {
            for bits in [1f32, 2., 3., 4.] {
                self.word(bits.to_bits());
            }
        }
        let name = self.bytes.len();
        self.int(3);
        self.bytes.extend_from_slice(b"a\0b");
        let mask = self.bytes.len();
        self.bytes.push(if populated { 5 } else { 255 });
        (count, name, mask)
    }
}
struct Recipe {
    data: Vec<u8>,
    start: usize,
    parameter: usize,
    component_count: usize,
    name: usize,
    mask: usize,
    wires: usize,
}
fn recipe(compact: bool, slot: u32) -> Recipe {
    let mut b = Builder {
        bytes: vec![],
        compact,
    };
    b.int(0);
    b.word(0);
    b.bytes.extend_from_slice(&[0; 3]);
    for _ in 0..4 {
        b.word(0);
    }
    for _ in 0..11 {
        b.int(0);
    } // Opaque blob intentionally empty.
    for _ in 0..8 {
        b.int(0);
        b.bytes.push(0);
    }
    for _ in 0..2 {
        b.int(8);
        b.bytes.extend_from_slice(&[1; 8]);
    }
    let start = b.bytes.len();
    b.int(3);
    let (mut parameter, mut name, mut mask, mut component_count) = (0, 0, 0, 0);
    for (i, kind) in [1, u32::MAX, 0].into_iter().enumerate() {
        b.int(kind);
        if i == 0 {
            parameter = b.bytes.len();
        }
        b.int(8);
        for j in 0..8 {
            b.int(if j == 5 { slot } else { u32::MAX });
        }
        for value in [0x7fc00001, 0x80000000, 0, 0x3f800000] {
            b.word(value);
        }
        b.int(99);
        let (c, n, m) = b.animation(i == 0);
        if i == 0 {
            name = n;
            mask = m;
            component_count = c;
        }
        b.animation(false);
    }
    let wires = b.bytes.len();
    b.int(4);
    for (from, to) in [(-1, 0), (i32::MIN, i32::MAX), (0, 1), (1, 0)] {
        b.signed(from);
        b.signed(to);
        b.bytes.extend_from_slice(&[255, 254, 0, 1, 2, 3, 255]);
    }
    Recipe {
        data: b.bytes,
        start,
        parameter,
        component_count,
        name,
        mask,
        wires,
    }
}
fn wire(p: &[u8], compact: bool, compressed: bool) -> Vec<u8> {
    let mut bytes = b"GMTb".to_vec();
    bytes.extend_from_slice(&950u32.to_be_bytes());
    let body = if compressed {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(p).unwrap();
        let stored = z.finish().unwrap();
        let mut body = vec![0, 1, 0, 1];
        body.extend_from_slice(&(stored.len() as u16).to_be_bytes());
        body.extend_from_slice(&(p.len() as u16).to_be_bytes());
        body.extend(stored);
        body
    } else {
        p.to_vec()
    };
    bytes.extend_from_slice(&(18u32 + body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.push(if compact { 7 } else { 0 });
    bytes.push(u8::from(compressed));
    bytes.extend(body);
    bytes.extend_from_slice(&[0; 4]);
    bytes
}
#[test]
fn raw_compact_compressed_metadata_retains_unknowns_cycles_and_extreme_signed_values() {
    for compact in [false, true] {
        for compressed in [false, true] {
            let p = recipe(compact, 7);
            let b = wire(&p.data, compact, compressed);
            let result = inspect_graph(&b, || false).unwrap();
            let g = result.graph;
            assert_eq!(result.material.prefix.parsed_prefix_bytes, p.start);
            assert_eq!(g.box_count, 3);
            assert_eq!(g.wire_count, 4);
            assert_eq!(g.unparsed_bytes, 0);
            assert_eq!(g.boxes[0].texture_slot_raw, Some(7));
            assert_eq!(g.boxes[0].texture_slot_in_range, Some(true));
            assert_eq!(g.boxes[1].type_raw, u32::MAX);
            assert_eq!(g.boxes[1].texture_slot_raw, None);
            assert_eq!(g.boxes[0].position_bits[0], 0x7fc00001);
            let a = &g.boxes[0].animations[0];
            assert_eq!(a.derived_key_count, 2);
            assert_eq!(a.name_hex, "610062");
            let values: Vec<u8> = [1f32, 2., 3., 4.]
                .into_iter()
                .flat_map(|v| v.to_bits().to_be_bytes())
                .collect();
            assert_eq!(
                a.component_bytes_sha256,
                format!("{:x}", Sha256::digest(&values))
            );
            assert_eq!(g.wires[0].box_from, -1);
            assert!(!g.wires[0].from_in_range);
            assert_eq!(g.wires[1].box_from, i32::MIN);
            assert_eq!(g.wires[1].box_to, i32::MAX);
            assert_eq!(g.wires[2].box_from, 0);
            assert_eq!(g.wires[3].box_to, 0); // Cycle accepted, not traversed.
            assert_eq!(g.wires[0].port_to_raw, 254);
            assert_eq!(g.wires[0].swizzle_raw, [0, 1, 2, 3, 255]);
        }
    }
    let p = recipe(true, 99);
    let g = inspect_graph(&wire(&p.data, true, false), || false)
        .unwrap()
        .graph;
    assert_eq!(g.boxes[0].texture_slot_in_range, Some(false));
}
#[test]
fn every_graph_truncation_in_valid_envelope_and_opaque_tail() {
    for compact in [false, true] {
        for compressed in [false, true] {
            let mut p = recipe(compact, 0);
            for n in p.start..p.data.len() {
                assert!(
                    inspect_graph(&wire(&p.data[..n], compact, compressed), || false).is_err(),
                    "{compact}/{compressed}/{n}"
                );
            }
            p.data.extend_from_slice(b"unparsed");
            let b = wire(&p.data, compact, compressed);
            let plain = inspect(&b, || false).unwrap();
            let parsed = inspect_graph(&b, || false).unwrap();
            assert_eq!(
                plain.prefix.parsed_prefix_bytes,
                parsed.material.prefix.parsed_prefix_bytes
            );
            assert_eq!(
                plain.prefix.unparsed_bytes,
                parsed.graph.parsed_graph_bytes + 8
            );
            assert_eq!(parsed.graph.unparsed_bytes, 8);
        }
    }
}
#[test]
fn unsupported_parameter_count_invalid_animation_and_graph_budgets() {
    let p = recipe(false, 0);
    for (offset, n, expected) in [
        (
            p.parameter,
            7u32,
            Error::Unsupported("material parameter count"),
        ),
        (p.start, 1025, Error::Budget),
        (p.component_count, 4097, Error::Budget),
        (p.name, 65, Error::Budget),
        (p.wires, 4097, Error::Budget),
    ] {
        let mut data = p.data.clone();
        data[offset..offset + 4].copy_from_slice(&n.to_be_bytes());
        assert_eq!(
            inspect_graph(&wire(&data, false, false), || false).unwrap_err(),
            expected
        );
    }
    let mut data = p.data.clone();
    data[p.mask] = 0;
    assert!(matches!(
        inspect_graph(&wire(&data, false, false), || false),
        Err(Error::Invalid(_))
    ));
    data[p.mask] = 7; // Four values cannot form equal tracks for three components.
    assert!(matches!(
        inspect_graph(&wire(&data, false, false), || false),
        Err(Error::Invalid(_))
    ));
    let mut data = p.data;
    data[p.mask] = 1; // A single component with four values remains valid.
    assert_eq!(
        inspect_graph(&wire(&data, false, false), || false)
            .unwrap()
            .graph
            .boxes[0]
            .animations[0]
            .derived_key_count,
        4
    );
}
#[test]
fn empty_graph_preserves_zero_counts_without_invented_edges() {
    for compact in [false, true] {
        let p = recipe(compact, 0);
        let mut data = p.data[..p.start].to_vec();
        data.extend(vec![0; if compact { 2 } else { 8 }]);
        let result = inspect_graph(&wire(&data, compact, false), || false).unwrap();
        assert_eq!(result.graph.box_count, 0);
        assert_eq!(result.graph.wire_count, 0);
        assert!(result.graph.boxes.is_empty() && result.graph.wires.is_empty());
        assert_eq!(result.graph.unparsed_bytes, 0);
    }
}
#[test]
fn cancellation_during_boxes_and_wires_and_protection() {
    for marker in [b"GMTe", b"SCE\0", b"NPD\0"] {
        assert_eq!(inspect_graph(marker, || false).unwrap_err(), Error::Blocked);
    }
    let p = recipe(true, 0);
    let b = wire(&p.data, true, false);
    for threshold in [1, 5, 8, 16] {
        let calls = Cell::new(0);
        assert_eq!(
            inspect_graph(&b, || {
                calls.set(calls.get() + 1);
                calls.get() >= threshold
            })
            .unwrap_err(),
            Error::Cancelled
        );
    }
}
