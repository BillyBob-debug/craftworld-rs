//! L4 Thing header/part flags and L5 subset: authored synthetic recipes only.
mod common;
use common::{bit, rev, translate, W};
use lbp_level::parts::ShapeColor;
use lbp_level::thing::present_parts;
use lbp_level::{read_payload, LevelReport, Limits, Stop};

fn none() -> bool {
    false
}

const BODY: u32 = 0;
const WORLD: u32 = 2;
const RENDER_MESH: u32 = 3;
const POS: u32 = 4;
const TRIGGER: u32 = 5;
const SHAPE: u32 = 0x12;
const CREATOR_ANIM_BIT: u32 = 0x29;

fn run(w: &W, head: u32) -> LevelReport {
    read_payload(&w.b, rev(head, w.flags), Limits::default(), &none)
}

#[test]
fn part_flags_creator_anim_shift_and_history() {
    // partsRevision 0x3f: bit 0x29 is CREATOR_ANIM, bit 0x2a is TRANSITION (index 0x29).
    let (p, un) = present_parts(0x3f8, bit(0) | bit(0x28) | bit(0x29) | bit(0x2a) | bit(0x36), 0x3f);
    assert_eq!(p, ["BODY", "POCKET_ITEM", "CREATOR_ANIM", "TRANSITION", "STREAMING_HINT"]);
    assert_eq!(un, 0);
    // partsRevision 0x2f (CONTROLINATOR): no shift; POPPET_POWERUP (history 0x30) absent.
    let (p, un) = present_parts(0x3e2, bit(0x26) | bit(0x27) | bit(0x29), 0x2f);
    assert_eq!(p, ["CONTROLINATOR"]);
    assert_eq!(un, bit(0x27) | bit(0x29));
    // Deprecated indices 0x36..0x3d never appear in the LBP2 band.
    let (p, un) = present_parts(0x3f8, bit(0x3d), 0x2c);
    assert!(p.is_empty());
    assert_eq!(un, bit(0x3d));
}

/// World Thing (BODY, WORLD) whose things array holds A (RENDER_MESH, POS; parent C
/// read inline), C (back-reference) and B (SHAPE, CREATOR_ANIM zero).
fn level(flags: u8) -> W {
    let mut w = W::new(flags);
    w.u32(1).raw(&[0x11; 20]); // cross-play hashes (head > 0x3e6)
    w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(BODY) | bit(WORLD));
    w.u32(2).f32(0.).f32(0.).f32(0.).f32(0.).u32(0).u32(0); // BODY
    w.u32(3).u32(3); // WORLD, 3 things
    // Thing A, parent C inline.
    w.u32(4).u8(0xaa).u32(10);
    w.u32(5).thing_header(30, 1, 0, 0, 0x3f, 0);
    w.u32(0).u32(0).i16(-1).i16(-1).u32(0x77).u8(0).s32(0x3f).u64(bit(RENDER_MESH) | bit(POS));
    w.u32(6).res_guid(0xabc).u32(1).u32(4).res_none();
    w.f32(0.5).f32(1.0).u8(1).f32(0.).f32(0.).u32(0xff00ff00).u8(1).u8(0).u8(3).f32(1.0);
    w.u32(7).u32(0).u32(0).m44(translate(1.0, 2.0, -3.0));
    w.u32(5); // C again: back-reference
    // Thing B: SHAPE then deprecated CREATOR_ANIM.
    w.u32(8).thing_header(20, 1, 0, 0, 0x3f, bit(SHAPE) | bit(CREATOR_ANIM_BIT));
    w.u32(9).u32(4).u8(0);
    for (x, y) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
        w.f32(x).f32(y);
    }
    w.int_vector(&[4]).res_guid(0x10).res_none().f32(0.25).f32(1.).u32(0xffffffff);
    w.f32(1.).f32(0.1).m44(translate(0.5, 0.5, 0.)).u32(0).u32(0).f32(1.).raw(&[0, 0]);
    w.u32(0).u8(0).i16(3).u32(1).u32(9).u8(0); // contact cache: shared self reference
    w.u8(0).u8(0).raw(&[0; 3]).raw(&[0xff; 2]).u8(0);
    w.u32(0); // CREATOR_ANIM
    // PWorld after the array.
    w.f32(0.).f32(0.).s32(5).s32(5).u32(0).u32(31);
    w
}

#[test]
fn walks_world_things_and_reads_render_parts() {
    for flags in [0, 7] {
        let w = level(flags);
        let r = run(&w, 0x3f8);
        match &r.stop {
            Some(Stop::Unsupported { what, offset }) => {
                assert!(what.contains("PWorld tail"));
                assert_eq!(*offset, w.b.len());
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(r.cross_play_hashes, Some(1));
        assert_eq!(r.world_things_walked, Some(3));
        let uids: Vec<_> = r.things.iter().map(|t| t.uid.unwrap()).collect();
        assert_eq!(uids, [1, 10, 30, 20]);
        let (world, a, c, b) = (&r.things[0], &r.things[1], &r.things[2], &r.things[3]);
        assert!(!world.complete && a.complete && c.complete && b.complete);
        assert_eq!((a.parent, c.parent, b.parent), (Some(2), Some(0), Some(0)));
        assert_eq!(a.plan_guid, Some(0x77));
        assert_eq!(a.parts, ["RENDER_MESH", "POS"]);
        let pos = a.data.pos.as_ref().unwrap();
        assert_eq!(&pos.world[12..15], &[1.0, 2.0, -3.0]);
        let mesh = a.data.render_mesh.as_ref().unwrap();
        assert_eq!(mesh.mesh.unwrap().guid, Some(0xabc));
        assert_eq!(mesh.bone_things, [Some(1)]);
        assert_eq!((mesh.editor_color, mesh.visibility_flags), (0xff00ff00, 3));
        assert_eq!(b.parts, ["SHAPE", "CREATOR_ANIM"]);
        assert_eq!(b.read[1].state, "deprecated_zero");
        let s = b.data.shape.as_ref().unwrap();
        assert_eq!((s.vertex_count, s.requires_z, s.contacts), (4, false, 1));
        assert_eq!(s.vertices[2], [1., 1., 0.]);
        assert_eq!(s.loops, [4]);
        assert_eq!(s.material.unwrap().guid, Some(0x10));
        assert!(matches!(s.color, ShapeColor::Packed(0xffffffff)));
        assert_eq!((s.thickness, s.bevel_size, s.flags), (0.25, 0.1, 3));
        assert_eq!(s.com.unwrap()[12], 0.5);
        assert_eq!(world.data.world.as_ref().unwrap().thing_uid_counter, Some(31));
        assert_eq!(r.detached_shapes, 0);
        assert_eq!(r.max_depth, 4);
    }
}

#[test]
fn every_truncation_stops_invalid_without_panic() {
    for flags in [0, 7] {
        let w = level(flags);
        for n in 0..w.b.len() {
            let r = read_payload(&w.b[..n], rev(0x3f8, flags), Limits::default(), &none);
            assert!(matches!(r.stop, Some(Stop::Invalid { .. })), "prefix {n}: {:?}", r.stop);
            assert!(r.consumed_bytes <= n);
        }
    }
}

#[test]
fn unsupported_part_reports_name_head_and_parts_revision() {
    let mut w = W::new(7);
    w.u32(1).thing_header(1, 0, 0, 0, 0x3c, bit(POS) | bit(TRIGGER));
    w.u32(2).u32(0).u32(0).m44(translate(0., 0., 0.));
    let r = run(&w, 0x3e5);
    assert_eq!(
        r.stop,
        Some(Stop::UnsupportedPart {
            part: "TRIGGER",
            head: 0x3e5,
            parts_revision: 0x3c,
            offset: w.b.len()
        })
    );
    assert_eq!(r.cross_play_hashes, None);
    assert_eq!(r.things[0].read.len(), 1);
    assert!(r.things[0].data.pos.is_some() && !r.things[0].complete);
}

#[test]
fn null_and_shared_part_references() {
    let mut w = W::new(1);
    w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(POS) | bit(SHAPE));
    w.u32(0).u32(0); // POS null, SHAPE null
    let r = run(&w, 0x3e5);
    let states: Vec<_> = r.things[0].read.iter().map(|p| p.state).collect();
    assert_eq!(states, ["null", "null"]);
    assert!(r.things[0].complete);
    assert!(matches!(&r.stop, Some(Stop::Unsupported { what, .. }) if what.contains("RLevel tail")));
}

#[test]
fn header_violations() {
    // Bad marker.
    let mut w = W::new(1);
    w.u32(1).u8(0xab);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Deprecated part with nonzero value.
    let mut w = W::new(1);
    w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(CREATOR_ANIM_BIT)).u32(1);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Null world.
    let mut w = W::new(1);
    w.u32(0);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Out of band head and branch.
    assert!(matches!(run(&W::new(1), 0x340).stop, Some(Stop::Unsupported { .. })));
    assert!(matches!(run(&W::new(1), 0x3f9).stop, Some(Stop::Unsupported { .. })));
    let mut rv = rev(0x3e5, 1);
    rv.branch_id = 0x4431;
    let r = read_payload(&[], rv, Limits::default(), &none);
    assert!(matches!(r.stop, Some(Stop::Unsupported { .. })));
}

#[test]
fn inline_parent_chain_hits_depth_budget() {
    let mut w = W::new(1);
    for t in 1..=10u32 {
        w.u32(t).u8(0xaa).u32(t);
    }
    let limits = Limits {
        max_depth: 5,
        ..Limits::default()
    };
    let r = read_payload(&w.b, rev(0x3e5, 1), limits, &none);
    assert!(matches!(r.stop, Some(Stop::Budget { what: "reference depth", .. })));
    assert_eq!(r.max_depth, 5);
    let yes = || true;
    let r = read_payload(&w.b, rev(0x3e5, 1), Limits::default(), &yes);
    assert_eq!(r.stop, Some(Stop::Cancelled));
}

#[test]
fn thing_budget() {
    let mut w = W::new(1);
    w.u32(1).thing_header(1, 2, 0, 0, 0, 0);
    let limits = Limits {
        max_things: 1,
        ..Limits::default()
    };
    let r = read_payload(&w.b, rev(0x3e5, 1), limits, &none);
    assert!(matches!(r.stop, Some(Stop::Budget { what: "things", .. })));
}
