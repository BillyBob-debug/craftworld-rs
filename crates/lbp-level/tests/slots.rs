//! U4 slot/pin/palette/pack readers: authored synthetic recipes only.
mod common;
use common::{rev, W};
use lbp_level::serial::{Limits, Stop};
use lbp_level::slots::{read_payload, UiData, UiKind};

fn none() -> bool {
    false
}

fn wstr(w: &mut W, s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    w.s32(units.len() as i32);
    for u in units {
        w.raw(&u.to_be_bytes());
    }
}

fn str8(w: &mut W, s: &str) {
    w.s32(s.len() as i32).raw(s.as_bytes());
}

/// Descriptor-form resource (no flags word).
fn desc_guid(w: &mut W, guid: u32) {
    w.u8(2).u32(guid);
}

/// One Slot at head 0x3f8 (LBP2 1.33 band: every gate up to 0x3e9 open).
fn slot_3f8(w: &mut W, number: u32, name: &str) {
    w.u32(0).u32(number); // SlotID DEVELOPER
    desc_guid(w, 0x1000 + number); // root level
    desc_guid(w, 0x2000 + number); // icon
    for f in [1.0f32, 2.0, 3.0, 4.0] {
        w.f32(f);
    }
    let mut author = [0u8; 16];
    author[..2].copy_from_slice(b"mm");
    w.raw(&author).u8(0).raw(&[0; 3]); // NetworkOnlineID (head >= 0x234)
    wstr(w, "Media Molecule"); // authorName
    str8(w, "LEVEL_NAME_TAG"); // translationTag
    wstr(w, name);
    wstr(w, "desc\0"); // NULs removed
    w.u32(4).u32(7); // primaryLinkLevel DEVELOPER_GROUP 7
    w.u32(4).u32(7); // group
    w.u8(1); // initiallyLocked
    w.u8(0).u32(0); // shareable, backgroundGUID none
    w.u8(0); // planetDecorations none
    w.u32(3); // developerLevelType TUTORIAL
    w.u32(1).u32(0xdead_beef).u32(-1i32 as u32); // labels (i32, not zigzag)
    w.u32(1);
    desc_guid(w, 0x3000); // collectabubblesRequired plan
    w.u32(5);
    w.u32(0); // collectabubblesContained
    w.u8(0); // isSubLevel
    w.u8(1).u8(4); // min/max players
    w.u8(1).u8(0).u8(1); // moveRecommended, crossCompatible, showOnPlanet
    w.u8(0xff); // livesOverride -1
}

#[test]
fn u4_slot_list_reads_lbp2_slot_fields() {
    for flags in [0u8, 7] {
        let mut w = W::new(flags);
        w.u32(2);
        slot_3f8(&mut w, 1, "Introduction");
        slot_3f8(&mut w, 2, "Rookie Test");
        w.u8(1); // fromProductionBuild
        let report = read_payload(
            UiKind::SlotList,
            &w.b,
            rev(0x3f8, flags),
            Limits::default(),
            &none,
        );
        let Ok(UiData::SlotList(list)) = report.result else {
            panic!("{:?}", report.result)
        };
        assert_eq!(report.checks.trailing_bytes, 0);
        assert_eq!(list.from_production_build, Some(true));
        assert_eq!(list.slots.len(), 2);
        let s = &list.slots[1];
        assert_eq!(s.id.number, 2);
        assert_eq!(s.id.type_name(), Some("DEVELOPER"));
        assert_eq!(s.root.unwrap().guid, Some(0x1002));
        assert_eq!(s.icon.unwrap().guid, Some(0x2002));
        assert_eq!(s.location, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(s.author_id, "mm");
        assert_eq!(s.author_name.as_deref(), Some("Media Molecule"));
        assert_eq!(s.translation_tag.as_deref(), Some("LEVEL_NAME_TAG"));
        assert_eq!(s.name, "Rookie Test");
        assert_eq!(s.description, "desc");
        assert_eq!(s.group.unwrap().slot_type, 4);
        assert!(s.initially_locked);
        assert_eq!(s.background_guid, None);
        assert_eq!(s.developer_level_type, Some(3));
        assert_eq!(s.game_progression_state, None);
        assert_eq!(s.labels[0].key, 0xdead_beef);
        assert_eq!(s.labels[0].order, -1);
        assert_eq!(s.collectabubbles_required[0].count, 5);
        assert_eq!((s.min_players, s.max_players), (Some(1), Some(4)));
        assert_eq!(s.cross_compatible, Some(false));
        assert_eq!(s.lives_override, Some(-1));
    }
}

#[test]
fn u4_slot_gates_stop_early_at_older_heads() {
    // Head 0x2c3: ends after developerLevelType/gameProgressionState; no labels, no list
    // production flag. Descriptors still flag-less, NetworkOnlineID unprefixed (>= 0x234).
    let mut w = W::new(0);
    w.u32(1);
    w.u32(5).u32(9); // POD
    w.u8(0).u8(0); // root, icon none
    for _ in 0..4 {
        w.f32(0.);
    }
    w.raw(&[0; 16]).u8(0).raw(&[0; 3]);
    wstr(&mut w, "");
    str8(&mut w, "");
    wstr(&mut w, "Pod");
    wstr(&mut w, "");
    w.u32(0).u32(0).u32(0).u32(0);
    w.u8(0);
    w.u8(1).u32(0x77); // shareable, background
    w.u32(2); // developerLevelType
    w.u32(1); // gameProgressionState
    let report = read_payload(
        UiKind::SlotList,
        &w.b,
        rev(0x2c3, 0),
        Limits::default(),
        &none,
    );
    let Ok(UiData::SlotList(list)) = report.result else {
        panic!("{:?}", report.result)
    };
    let s = &list.slots[0];
    assert_eq!(s.id.type_name(), Some("POD"));
    assert_eq!(s.background_guid, Some(0x77));
    assert_eq!(s.game_progression_state, Some(1));
    assert!(s.labels.is_empty() && s.is_sub_level.is_none() && s.min_players.is_none());
    assert_eq!(list.from_production_build, None);
    assert_eq!(report.checks.trailing_bytes, 0);
}

#[test]
fn u4_pins_palette_and_packs() {
    let mut w = W::new(7);
    w.u32(1);
    w.u32(42).u32(1).u32(2).u32(0x1111).u32(0x2222);
    desc_guid(&mut w, 0x4444);
    w.u32(0).u32(10).u8(3).i16(0x0102).u8(0xfe);
    let report = read_payload(UiKind::Pins, &w.b, rev(0x3f8, 7), Limits::default(), &none);
    let Ok(UiData::Pins { pins }) = report.result else {
        panic!("{:?}", report.result)
    };
    assert_eq!(pins[0].title_key, 0x1111);
    assert_eq!(pins[0].icon.unwrap().guid, Some(0x4444));
    assert_eq!(pins[0].behaviour_flags, 0x0102);
    assert_eq!(pins[0].trophy_to_unlock_lbp1, Some(-2));

    // Palette: plans carry the flags word (not descriptor calls), converted plans do not.
    let mut w = W::new(0);
    w.u32(2).res_guid(0x10).res_none();
    w.u32(5).u32(6);
    w.u32(1);
    desc_guid(&mut w, 0x20);
    let report = read_payload(
        UiKind::Palette,
        &w.b,
        rev(0x3f8, 0),
        Limits::default(),
        &none,
    );
    let Ok(UiData::Palette(p)) = report.result else {
        panic!("{:?}", report.result)
    };
    assert_eq!(p.plans.len(), 2);
    assert_eq!(p.plans[0].unwrap().guid, Some(0x10));
    assert!(p.plans[1].is_none());
    assert_eq!((p.location, p.description), (5, 6));
    assert_eq!(p.converted_plans.unwrap()[0].unwrap().guid, Some(0x20));
    assert_eq!(report.checks.trailing_bytes, 0);

    let mut w = W::new(7);
    w.u32(1).u32(3); // COSTUME
    w.u8(0); // mesh none
    slot_3f8(&mut w, 9, "Pack");
    str8(&mut w, "UP9000-BCES00850_00-COSTUME0000001");
    w.uleb(3); // zigzag -2
    let report = read_payload(UiKind::Packs, &w.b, rev(0x3f8, 7), Limits::default(), &none);
    let Ok(UiData::Packs { packs }) = report.result else {
        panic!("{:?}", report.result)
    };
    assert_eq!(packs[0].contents_type, 3);
    assert_eq!(packs[0].slot.name, "Pack");
    assert_eq!(packs[0].timestamp, -2);
    assert_eq!(report.checks.trailing_bytes, 0);
}

#[test]
fn u4_untrusted_input_stops_explicitly() {
    let lim = Limits::default();
    // Huge count against a short payload.
    let mut w = W::new(0);
    w.u32(1000);
    let r = read_payload(UiKind::Pins, &w.b, rev(0x3f8, 0), lim, &none);
    assert!(matches!(r.result, Err(Stop::Invalid { .. })));
    // Count above the array budget.
    let mut w = W::new(0);
    w.u32(0x00ff_ffff);
    let r = read_payload(UiKind::Pins, &w.b, rev(0x3f8, 0), lim, &none);
    assert!(matches!(r.result, Err(Stop::Budget { .. })));
    // Negative string length.
    let mut w = W::new(0);
    w.u32(1).u32(0).u32(0).u8(0).u8(0);
    for _ in 0..4 {
        w.f32(0.);
    }
    w.raw(&[0; 20]).s32(-1);
    let r = read_payload(UiKind::SlotList, &w.b, rev(0x3f8, 0), lim, &none);
    assert!(
        matches!(r.result, Err(Stop::Invalid { .. })),
        "{:?}",
        r.result
    );
    // Truncated pin.
    let mut w = W::new(0);
    w.u32(1).u32(1);
    let r = read_payload(UiKind::Pins, &w.b, rev(0x3f8, 0), lim, &none);
    assert!(matches!(r.result, Err(Stop::Invalid { .. })));
    // Vita branch and LBP3 heads are refused, not misread.
    let mut vita = rev(0x3e2, 0);
    vita.branch_id = 0x4431;
    for rv in [vita, rev(0x0021_0272, 0), rev(0x3f9, 0), rev(0x190, 0)] {
        let r = read_payload(UiKind::Pins, &[0, 0, 0, 0], rv, lim, &none);
        assert!(matches!(r.result, Err(Stop::Unsupported { .. })), "{rv:?}");
    }
    // Unknown enum values and trailing bytes are counted, not fatal.
    let mut w = W::new(0);
    w.u32(1).u32(99);
    w.u8(0);
    slot_3f8(&mut w, 1, "x");
    str8(&mut w, "");
    w.raw(&0i64.to_be_bytes()).u8(0xaa);
    let r = read_payload(UiKind::Packs, &w.b, rev(0x3f8, 0), lim, &none);
    assert!(r.result.is_ok(), "{:?}", r.result);
    assert_eq!(r.checks.unknown_contents_types, 1);
    assert_eq!(r.checks.trailing_bytes, 1);
}

#[test]
fn u4_text_form_is_an_explicit_exclusion() {
    use lbp_resource::binary::Error;
    let lim = Limits::default();
    let text = lbp_level::slots::read(b"SLTt\0\0\0\0", lim, &none);
    assert_eq!(
        text.unwrap_err(),
        Error::Unsupported(lbp_level::slots::NON_BINARY_TEXT)
    );
    assert!(matches!(
        lbp_level::slots::read(b"SLTx", lim, &none),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        lbp_level::slots::read(b"MSHb", lim, &none),
        Err(Error::Unsupported("not a U4 resource type"))
    ));
}
