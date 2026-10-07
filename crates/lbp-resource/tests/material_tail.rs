//! Authored tail recipes and metadata expectations only, no proprietary fixtures.
use lbp_resource::{
    binary::Error,
    material::{inspect, inspect_graph, inspect_tail, MAX_TAIL_BYTES},
};
use sha2::{Digest, Sha256};
use std::{cell::Cell, io::Write};
struct Builder {
    bytes: Vec<u8>,
    compact: bool,
}
impl Builder {
    fn word(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
    fn int(&mut self, mut value: u32) {
        if !self.compact {
            self.word(value);
            return;
        }
        loop {
            let part = (value & 127) as u8;
            value >>= 7;
            self.bytes.push(part | if value != 0 { 128 } else { 0 });
            if value == 0 {
                break;
            }
        }
    }
    fn animation(&mut self, populated: bool) -> (usize, usize, usize) {
        for bits in [0x7fc00001, 0x80000000, 0, 0x3f800000] {
            self.word(bits);
        }
        let count = self.bytes.len();
        self.int(if populated { 4 } else { 0 });
        if populated {
            for value in [1f32, 2., 3., 4.] {
                self.word(value.to_bits());
            }
        }
        let name = self.bytes.len();
        self.int(3);
        self.bytes.extend_from_slice(b"a\0z");
        let mask = self.bytes.len();
        self.bytes.push(if populated { 5 } else { 255 });
        (count, name, mask)
    }
}
struct Recipe {
    data: Vec<u8>,
    start: usize,
    count: usize,
    values: usize,
    name: usize,
    mask: usize,
}
fn recipe(compact: bool, animations: u32) -> Recipe {
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
    } // Empty opaque blob; no shader fixtures.
    for _ in 0..8 {
        b.int(0);
        b.bytes.push(0);
    }
    for _ in 0..2 {
        b.int(8);
        b.bytes.extend_from_slice(&[255; 8]);
    }
    b.int(0);
    b.int(0); // Empty graph, independently tested by material_graph.rs.
    let start = b.bytes.len();
    b.int(u32::MAX);
    let count = b.bytes.len();
    b.int(animations);
    let (mut values, mut name, mut mask) = (0, 0, 0);
    for i in 0..animations {
        let (v, n, m) = b.animation(i == 0);
        if i == 0 {
            values = v;
            name = n;
            mask = m;
        }
    }
    Recipe {
        data: b.bytes,
        start,
        count,
        values,
        name,
        mask,
    }
}
// Modes: raw, stored chunks, zlib chunks. Every cut rebuilds a valid envelope.
fn wire(data: &[u8], compact: bool, mode: u8) -> Vec<u8> {
    let body = if mode == 0 {
        data.to_vec()
    } else {
        let chunks: Vec<_> = data.chunks(60000).collect();
        let mut body = vec![0, 1];
        body.extend_from_slice(&(chunks.len() as u16).to_be_bytes());
        let mut payload = Vec::new();
        for chunk in chunks {
            let stored = if mode == 1 {
                chunk.to_vec()
            } else {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(chunk).unwrap();
                encoder.finish().unwrap()
            };
            body.extend_from_slice(&(stored.len() as u16).to_be_bytes());
            body.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
            payload.extend(stored);
        }
        body.extend(payload);
        body
    };
    let mut bytes = b"GMTb".to_vec();
    bytes.extend_from_slice(&950u32.to_be_bytes());
    bytes.extend_from_slice(&(18 + body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.push(if compact { 7 } else { 0 });
    bytes.push(u8::from(mode != 0));
    bytes.extend(body);
    bytes.extend_from_slice(&[0; 4]);
    bytes
}
#[test]
fn raw_stored_zlib_and_compact_tail_preserve_bits_digests_and_unknowns() {
    for compact in [false, true] {
        for mode in 0..3 {
            let p = recipe(compact, 2);
            let mut data = p.data.clone();
            data.extend_from_slice(&[0xde, 0xad]);
            let bytes = wire(&data, compact, mode);
            let result = inspect_tail(&bytes, || false).unwrap();
            let t = result.tail;
            assert_eq!(t.sound_enum_raw, u32::MAX);
            assert_eq!(t.parameter_animation_count, 2);
            assert_eq!(t.tail_start, p.start);
            assert_eq!(t.parsed_tail_bytes, p.data.len() - p.start);
            assert_eq!(t.unparsed_bytes, 2);
            assert_eq!(result.inspection.graph.unparsed_bytes, data.len() - p.start);
            assert_eq!(
                format!("{:?}", result.inspection),
                format!("{:?}", inspect_graph(&bytes, || false).unwrap())
            );
            assert_eq!(
                result.inspection.material.prefix.parsed_prefix_bytes,
                inspect(&bytes, || false)
                    .unwrap()
                    .prefix
                    .parsed_prefix_bytes
            );
            let a = &t.parameter_animations[0];
            assert_eq!(a.base_value_bits, [0x7fc00001, 0x80000000, 0, 0x3f800000]);
            assert_eq!(a.name_hex, "61007a");
            assert_eq!(a.components_raw, 5);
            assert_eq!(a.derived_key_count, 2);
            let values: Vec<u8> = [1f32, 2., 3., 4.]
                .into_iter()
                .flat_map(|v| v.to_bits().to_be_bytes())
                .collect();
            assert_eq!(
                a.component_bytes_sha256,
                format!("{:x}", Sha256::digest(values))
            );
            assert_eq!(t.parameter_animations[1].components_raw, 255);
            assert_eq!(t.parameter_animations[1].derived_key_count, 0);
            let empty = recipe(compact, 0);
            let b = wire(&empty.data, compact, mode);
            assert!(inspect_tail(&b, || false)
                .unwrap()
                .tail
                .parameter_animations
                .is_empty());
        }
    }
}
#[test]
fn every_tail_truncation_rejected_while_prefix_and_graph_remain_readable() {
    for compact in [false, true] {
        for mode in 0..3 {
            let p = recipe(compact, 2);
            for n in p.start..p.data.len() {
                let bytes = wire(&p.data[..n], compact, mode);
                assert!(inspect(&bytes, || false).is_ok());
                assert!(inspect_graph(&bytes, || false).is_ok());
                assert!(
                    inspect_tail(&bytes, || false).is_err(),
                    "cut {n}, compact {compact}, mode {mode}"
                );
            }
        }
    }
}
#[test]
fn count_value_name_mask_integer_and_tail_budgets_are_enforced() {
    let p = recipe(false, 2);
    for (offset, value) in [
        (p.count, 129),
        (p.count, u32::MAX),
        (p.values, 4097),
        (p.name, 65),
    ] {
        let mut data = p.data.clone();
        data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        assert_eq!(
            inspect_tail(&wire(&data, false, 0), || false).unwrap_err(),
            Error::Budget
        );
    }
    for mask in [0, 7] {
        let mut data = p.data.clone();
        data[p.mask] = mask;
        assert!(matches!(
            inspect_tail(&wire(&data, false, 0), || false),
            Err(Error::Invalid(_))
        ));
    }
    let packed = recipe(true, 0);
    let mut data = packed.data[..packed.start].to_vec();
    data.extend_from_slice(&[255; 5]);
    assert_eq!(
        inspect_tail(&wire(&data, true, 0), || false).unwrap_err(),
        Error::Invalid("uleb32 overflow")
    );
    let maximum = recipe(true, 128);
    assert_eq!(
        inspect_tail(&wire(&maximum.data, true, 2), || false)
            .unwrap()
            .tail
            .parameter_animation_count,
        128
    );
    let empty = recipe(true, 0);
    let mut data = empty.data;
    data.resize(empty.start + MAX_TAIL_BYTES, 0);
    assert_eq!(
        inspect_tail(&wire(&data, true, 2), || false)
            .unwrap()
            .tail
            .unparsed_bytes,
        MAX_TAIL_BYTES - 6
    );
    data.push(0);
    assert_eq!(
        inspect_tail(&wire(&data, true, 2), || false).unwrap_err(),
        Error::Budget
    );
}
#[test]
fn protection_revision_and_cancellation_before_and_between_tail_records() {
    for marker in [b"GMTe", b"SCE\0", b"NPD\0"] {
        assert_eq!(inspect_tail(marker, || false).unwrap_err(), Error::Blocked);
    }
    let p = recipe(true, 2);
    let bytes = wire(&p.data, true, 0);
    let calls = Cell::new(0);
    inspect_graph(&bytes, || {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let graph_calls = calls.get();
    for threshold in [1, graph_calls + 1, graph_calls + 4] {
        calls.set(0);
        assert_eq!(
            inspect_tail(&bytes, || {
                calls.set(calls.get() + 1);
                calls.get() >= threshold
            })
            .unwrap_err(),
            Error::Cancelled
        );
        assert_eq!(calls.get(), threshold);
    }
    for revision in [919u32, 952] {
        let mut unsupported = bytes.clone();
        unsupported[4..8].copy_from_slice(&revision.to_be_bytes());
        assert!(matches!(
            inspect_tail(&unsupported, || false),
            Err(Error::Unsupported(_))
        ));
    }
}
