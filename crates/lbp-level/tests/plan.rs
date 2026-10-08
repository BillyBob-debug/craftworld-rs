//! D119 RPlan entry and pre-0x341 plan Things: authored synthetic recipes only.
mod common;
use common::{bit, rev, translate, W};
use lbp_level::{read_plan_payload, Limits, Stop};

fn none() -> bool {
    false
}

const POS: u32 = 4;
const SHAPE: u32 = 0x12;
const METADATA: u32 = 0x16;
const GROUP: u32 = 0x1d;

/// Plan payload: stored revision word, then thingData as a bytearray.
fn plan(head: u32, flags: u8, things: &W) -> W {
    let mut w = W::new(flags);
    w.u32(head).u32(things.b.len() as u32).raw(&things.b);
    w
}

/// Pre-0x341 Thing header (tag already written): marker, UID, three null references,
/// createdBy/changedBy, isStamping, planGUID, hidden, partsRevision, part flags.
fn old_header(w: &mut W, uid: u32, plan_guid: u32, parts: u64) {
    w.u8(0xaa).u32(uid).u32(0).u32(0).u32(0).i16(-1).i16(-1);
    w.u8(0).u32(plan_guid).u8(0).s32(0x2c).u64(parts);
}

#[test]
fn pre_band_plan_thing_reads_local_position_and_old_group() {
    for flags in [0u8, 7] {
        let mut t = W::new(flags);
        t.u32(1).u32(1); // one root, Thing tag 1
        old_header(&mut t, 5, 0x195eb, bit(POS) | bit(GROUP));
        // POS: bone ref, animHash, localPosition, worldPosition.
        t.u32(2)
            .u32(0)
            .u32(0)
            .m44(translate(1., 0., 0.))
            .m44(translate(2., 3., 4.));
        // GROUP (pre-0x341): copyright, creator (36 bytes), plan desc, editable, emitter,
        // lifetime, aliveFrames, pickupAllMembers, mainSelectableObject.
        t.u32(3)
            .u8(1)
            .raw(&[0; 36])
            .u8(0)
            .u8(1)
            .u32(0)
            .u32(0)
            .u32(0)
            .u8(0)
            .u8(1);
        let p = plan(0x336, flags, &t);
        let r = read_plan_payload(&p.b, rev(0x336, flags), Limits::default(), &none);
        assert!(r.stop.is_none(), "{:?}", r.stop);
        assert_eq!(r.trailing_thing_bytes, Some(0));
        assert_eq!(r.stored_revision, Some(0x336));
        assert_eq!(r.roots, vec![Some(0)]);
        let thing = &r.things[0];
        assert_eq!(thing.plan_guid, Some(0x195eb));
        assert_eq!(thing.flags, 0);
        assert_eq!(
            thing.data.pos.as_ref().unwrap().world,
            translate(2., 3., 4.)
        );
        assert_eq!(thing.data.group.as_ref().unwrap().flags, 0);
    }
}

#[test]
fn pre_band_unaudited_part_and_out_of_band_heads_stop_explicitly() {
    let mut t = W::new(0);
    t.u32(1).u32(1);
    old_header(&mut t, 5, 0, bit(SHAPE));
    let p = plan(0x336, 0, &t);
    let r = read_plan_payload(&p.b, rev(0x336, 0), Limits::default(), &none);
    assert!(matches!(
        r.stop,
        Some(Stop::UnsupportedPart { part: "SHAPE", .. })
    ));
    let r = read_plan_payload(&p.b, rev(0x334, 0), Limits::default(), &none);
    assert!(matches!(r.stop, Some(Stop::Unsupported { .. })));
    assert!(r.things.is_empty());
    // Truncated thingData length.
    let mut w = W::new(0);
    w.u32(0x3a5).u32(100).raw(&[0; 4]);
    let r = read_plan_payload(&w.b, rev(0x3a5, 0), Limits::default(), &none);
    assert!(matches!(r.stop, Some(Stop::Invalid { .. })));
}

#[test]
fn in_band_plan_metadata_part() {
    let mut t = W::new(0);
    t.u32(1).u32(1);
    t.thing_header(9, 0, 0, 0, 0x2c, bit(METADATA));
    // METADATA: 4 u32 keys, primaryIndex, fluffCost, type, subType, creationDate, icon,
    // reference(PhotoMetadata) inline, referencable, allowEmit.
    t.u32(2);
    for _ in 0..9 {
        t.u32(0);
    }
    t.res_none();
    t.u32(3)
        .u8(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .raw(&[0; 20])
        .u64(0)
        .u32(0);
    t.u8(1).u8(0);
    let p = plan(0x3a5, 0, &t);
    let r = read_plan_payload(&p.b, rev(0x3a5, 0), Limits::default(), &none);
    assert!(r.stop.is_none(), "{:?}", r.stop);
    assert_eq!(r.trailing_thing_bytes, Some(0));
    assert_eq!(r.things[0].parts, vec!["METADATA"]);
}
