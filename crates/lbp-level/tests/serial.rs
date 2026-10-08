//! L3 serializer core: authored synthetic recipes only.
mod common;
use common::{rev, W};
use lbp_level::serial::{Limits, Obj, Reader, Stop, Tag};

fn none() -> bool {
    false
}

fn reader(b: &[u8], flags: u8) -> Reader<'_> {
    Reader::new(b, rev(0x3f8, flags), Limits::default(), &none)
}

fn invalid<T: std::fmt::Debug>(r: lbp_level::serial::Result<T>) -> bool {
    matches!(r, Err(Stop::Invalid { .. }))
}

#[test]
fn integers_fixed_and_uleb() {
    let mut w = W::new(0);
    w.u32(0x1234_5678).s32(-2).u64(u64::MAX);
    let mut r = reader(&w.b, 0);
    assert_eq!(r.u32().unwrap(), 0x1234_5678);
    assert_eq!(r.s32().unwrap(), -2);
    assert_eq!(r.u64().unwrap(), u64::MAX);
    assert_eq!(r.remaining(), 0);

    let mut w = W::new(1);
    w.u32(u32::MAX).s32(-1).s32(1).s32(i32::MIN).s32(i32::MAX).u64(u64::MAX).u32(0);
    let mut r = reader(&w.b, 1);
    assert_eq!(r.u32().unwrap(), u32::MAX);
    assert_eq!(r.s32().unwrap(), -1);
    assert_eq!(r.s32().unwrap(), 1);
    assert_eq!(r.s32().unwrap(), i32::MIN);
    assert_eq!(r.s32().unwrap(), i32::MAX);
    assert_eq!(r.u64().unwrap(), u64::MAX);
    assert_eq!(r.u32().unwrap(), 0);
    assert_eq!(r.remaining(), 0);
}

#[test]
fn uleb_overflow_and_truncation_are_invalid() {
    // 33rd bit set in the fifth byte.
    assert!(invalid(reader(&[0xff, 0xff, 0xff, 0xff, 0x10], 1).u32()));
    // Overlong (six bytes) for a 32-bit value.
    assert!(invalid(reader(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x00], 1).u32()));
    // 65th bit.
    let mut b = vec![0xff; 9];
    b.push(0x02);
    assert!(invalid(reader(&b, 1).u64()));
    // Unterminated and truncated fixed.
    assert!(invalid(reader(&[0x80, 0x80], 1).u32()));
    assert!(invalid(reader(&[0, 0, 0], 0).u32()));
    assert!(invalid(reader(&[0; 7], 0).u64()));
}

#[test]
fn matrices_masked_and_full() {
    let t = common::translate(1.0, -2.0, 3.5);
    for flags in [0, 7] {
        let mut w = W::new(flags);
        w.m44(t);
        assert_eq!(w.b.len(), if flags == 7 { 2 + 12 } else { 64 });
        let mut r = reader(&w.b, flags);
        assert_eq!(r.m44().unwrap(), t);
        assert_eq!(r.remaining(), 0);
    }
    // Empty mask = identity, two bytes consumed.
    let mut r = reader(&[0, 0], 4);
    assert_eq!(r.m44().unwrap()[15], 1.0);
    assert_eq!(r.pos(), 2);
    // Mask names more floats than exist.
    assert!(invalid(reader(&[0xff, 0xff, 0, 0, 0, 0], 4).m44()));
}

#[test]
fn int_vectors_planar_and_plain() {
    let v = [1u32, 0x0203, 0xfffe];
    for flags in [0, 7] {
        let mut w = W::new(flags);
        w.int_vector(&v).int_vector(&[]);
        let mut r = reader(&w.b, flags);
        assert_eq!(r.int_vector().unwrap(), v);
        assert_eq!(r.int_vector().unwrap(), Vec::<u32>::new());
        assert_eq!(r.remaining(), 0);
    }
    // Planar layout: count 2, width 2, low plane then high plane.
    let mut r = reader(&[2, 2, 0x01, 0x02, 0x10, 0x20], 7);
    assert_eq!(r.int_vector().unwrap(), [0x1001, 0x2002]);
    // Width above 4 is refused, count beyond remaining bytes is invalid.
    assert!(matches!(reader(&[1, 5, 0, 0, 0, 0, 0], 7).int_vector(), Err(Stop::Unsupported { .. })));
    assert!(invalid(reader(&[3, 2, 0, 0], 7).int_vector()));
    assert!(invalid(reader(&[0, 0, 0, 9, 0, 0, 0, 1], 0).int_vector()));
}

#[test]
fn counts_are_bounded_by_bytes_and_budget() {
    let mut w = W::new(1);
    w.u32(3).raw(&[0; 6]);
    assert_eq!(reader(&w.b, 1).count(2).unwrap(), 3);
    assert!(invalid(reader(&w.b, 1).count(3)));
    let limits = Limits {
        max_array: 2,
        ..Limits::default()
    };
    let mut r = Reader::new(&w.b, rev(0x3f8, 1), limits, &none);
    assert!(matches!(r.count(1), Err(Stop::Budget { .. })));
}

#[test]
fn resource_descriptors() {
    let mut w = W::new(1);
    w.res_none().res_guid(0x1234);
    w.u32(5).u8(3).u32(9).raw(&[0xab; 20]);
    w.u8(1).raw(&[0xcd; 20]); // descriptor form: no flags word
    let mut r = reader(&w.b, 1);
    assert_eq!(r.resource(false).unwrap(), None);
    let d = r.resource(false).unwrap().unwrap();
    assert_eq!((d.flags, d.guid, d.sha1), (Some(0), Some(0x1234), None));
    let d = r.resource(false).unwrap().unwrap();
    assert_eq!((d.flags, d.guid, d.sha1), (Some(5), Some(9), Some([0xab; 20])));
    let d = r.resource(true).unwrap().unwrap();
    assert_eq!((d.flags, d.kind_byte, d.guid), (None, 1, None));
    assert_eq!(r.remaining(), 0);
    // Head <= 0x22e has no flags word.
    let b = [2u8, 0, 0, 0, 7];
    let mut r = Reader::new(&b, rev(0x22e, 0), Limits::default(), &none);
    assert_eq!(r.resource(false).unwrap().unwrap().guid, Some(7));
    // Truncated hash.
    assert!(invalid(reader(&[0, 1, 0xab], 1).resource(false)));
}

#[test]
fn reference_tags_null_new_seen_and_conflicts() {
    let b = [0u8, 5, 5, 5];
    let mut r = reader(&b, 1);
    assert_eq!(r.tag("Thing", |_| Ok(Obj::Thing(0))).unwrap(), Tag::Null);
    assert_eq!(r.tag("Thing", |_| Ok(Obj::Thing(0))).unwrap(), Tag::New(5));
    assert_eq!(r.tag("Thing", |_| unreachable!()).unwrap(), Tag::Seen(Obj::Thing(0)));
    assert!(invalid(r.tag("SHAPE", |_| unreachable!())));
    let limits = Limits {
        max_tags: 1,
        ..Limits::default()
    };
    let b = [1u8, 2];
    let mut r = Reader::new(&b, rev(0x3f8, 1), limits, &none);
    r.tag("Thing", |_| Ok(Obj::Thing(0))).unwrap();
    assert!(matches!(r.tag("Thing", |_| Ok(Obj::Thing(1))), Err(Stop::Budget { .. })));
}

#[test]
fn depth_budget_and_cancel() {
    let limits = Limits {
        max_depth: 2,
        ..Limits::default()
    };
    let mut r = Reader::new(&[], rev(0x3f8, 1), limits, &none);
    r.enter().unwrap();
    r.enter().unwrap();
    assert!(matches!(r.enter(), Err(Stop::Budget { .. })));
    r.leave();
    r.enter().unwrap();
    assert_eq!(r.max_depth_seen, 2);
    let yes = || true;
    let r = Reader::new(&[], rev(0x3f8, 1), Limits::default(), &yes);
    assert_eq!(r.check_cancel(), Err(Stop::Cancelled));
}
