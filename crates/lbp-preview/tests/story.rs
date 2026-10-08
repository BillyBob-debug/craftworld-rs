//! U7 story browser data: authored synthetic slots and tables only, no game bytes.
use lbp_level::serial::Descriptor;
use lbp_level::slots::{Slot, SlotId, SlotList};
use lbp_preview::story::{
    build_story, count, count_lines, display_name, Icon, ListInput, NameSource, ResourceKey,
    Strings,
};
use lbp_resource::translation::{lams_key, parse_translation_table, TranslationLimits};

/// Binary `.trans` with rows sorted by key and FEFF-marked UTF-16BE strings.
fn table(pairs: &[(&str, &str)]) -> lbp_resource::translation::TranslationTable {
    let mut rows: Vec<(u32, &str)> = pairs.iter().map(|(t, s)| (lams_key(t), *s)).collect();
    rows.sort_by_key(|r| r.0);
    let mut blob = Vec::new();
    let mut out = (rows.len() as u32).to_be_bytes().to_vec();
    for (key, text) in &rows {
        out.extend_from_slice(&key.to_be_bytes());
        out.extend_from_slice(&(blob.len() as u32).to_be_bytes());
        blob.extend_from_slice(&0xFEFFu16.to_be_bytes());
        for u in text.encode_utf16() {
            blob.extend_from_slice(&u.to_be_bytes());
        }
    }
    out.extend(blob);
    parse_translation_table(&out, TranslationLimits::default()).unwrap()
}

fn guid(g: u32) -> Descriptor {
    Descriptor {
        flags: None,
        kind_byte: 2,
        guid: Some(g),
        sha1: None,
    }
}

fn slot(number: u32, tag: Option<&str>, name: &str) -> Slot {
    Slot {
        id: SlotId {
            slot_type: 0,
            number,
        },
        root: Some(guid(0x1000 + number)),
        icon: Some(guid(0x2000 + number % 2)),
        location: [0., 1., 0., 0.],
        author_id: String::new(),
        author_name: None,
        translation_tag: tag.map(str::to_owned),
        name: name.to_owned(),
        description: "stored".to_owned(),
        primary_link_level: SlotId {
            slot_type: 0,
            number: 0,
        },
        group: None,
        initially_locked: number > 1,
        shareable: None,
        background_guid: None,
        planet_decorations: None,
        developer_level_type: Some(0),
        game_progression_state: None,
        labels: Vec::new(),
        collectabubbles_required: Vec::new(),
        collectabubbles_contained: Vec::new(),
        is_sub_level: Some(false),
        min_players: None,
        max_players: None,
        move_recommended: None,
        cross_compatible: None,
        show_on_planet: Some(true),
        lives_override: None,
    }
}

#[test]
fn u7_names_follow_toolkit_slot_to_string_with_patch_over_base() {
    let strings = Strings {
        base: Some(table(&[
            ("GARDEN_NAME", "Old Garden"),
            ("CAVE_NAME", "Cave"),
        ])),
        patch: Some(table(&[("GARDEN_NAME", "The Garden")])),
    };
    let cases = [
        (
            slot(1, Some("GARDEN"), "x"),
            "The Garden",
            NameSource::Translated,
        ),
        (slot(2, Some("CAVE"), ""), "Cave", NameSource::Translated),
        (slot(3, Some("NOPE"), "stored"), "NOPE", NameSource::TagOnly),
        (slot(4, Some(""), "Mine"), "Mine", NameSource::Stored),
        (slot(5, None, ""), "Unnamed Level", NameSource::Unnamed),
    ];
    for (s, name, source) in cases {
        assert_eq!(display_name(&s, &strings), (name.to_owned(), source));
    }
    // No tables: the tag is shown.
    let (name, source) = display_name(&slot(1, Some("GARDEN"), ""), &Strings::default());
    assert_eq!((name.as_str(), source), ("GARDEN", NameSource::TagOnly));
}

#[test]
fn u7_story_counts_lists_slots_icons_and_groups() {
    let strings = Strings {
        base: Some(table(&[("A_NAME", "Alpha"), ("A_DESC", "First")])),
        patch: None,
    };
    let mut grouped = slot(2, Some("B"), "");
    grouped.group = Some(SlotId {
        slot_type: 4,
        number: 9,
    });
    let mut ungrouped = slot(3, None, "Three");
    // Toolkit default SlotID: no group.
    ungrouped.group = Some(SlotId {
        slot_type: 0,
        number: 0,
    });
    ungrouped.icon = None;
    let lists = vec![
        ListInput {
            paths: vec!["gamedata/a.slt".into()],
            sha1: "aa".into(),
            result: Ok((
                SlotList {
                    slots: vec![slot(1, Some("A"), ""), grouped, ungrouped],
                    from_production_build: None,
                },
                0,
            )),
        },
        ListInput {
            paths: vec!["gamedata/b.slt".into()],
            sha1: "bb".into(),
            result: Err("stopped: Invalid".into()),
        },
    ];
    let resolves = |k: ResourceKey| k != ResourceKey::Guid(0x1002);
    let mut story = build_story(lists, &strings, &resolves);
    assert_eq!(story.slots.len(), 3);
    assert_eq!(story.slots[0].name, "Alpha");
    assert_eq!(story.slots[0].description, "First");
    assert!(story.slots[0].description_translated);
    assert_eq!(story.slots[2].group, None);
    let c = &story.counts;
    assert_eq!(
        (c.lists, c.lists_read, c.slots, c.distinct_ids),
        (2, 1, 3, 3)
    );
    assert_eq!((c.with_root, c.root_resolved), (3, 2));
    assert_eq!((c.grouped, c.group_missing), (1, 1));
    assert_eq!(c.initially_locked, 2);
    assert_eq!((c.with_icon, c.icon_decoded, c.distinct_icons), (2, 0, 2));
    assert_eq!(story.lists[1].status, "stopped: Invalid");
    // Icons decoded later are counted per slot and per distinct reference.
    story.icons.insert(
        ResourceKey::Guid(0x2001),
        Icon {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        },
    );
    story
        .icon_failures
        .insert(ResourceKey::Guid(0x2000), "Unresolved: GUID".into());
    story.counts = count(&story);
    let c = &story.counts;
    assert_eq!(
        (c.icon_decoded, c.icon_resolved, c.distinct_icons_decoded),
        (1, 1, 1)
    );
    let lines = count_lines(&story);
    assert!(lines[0].contains("Slot lists read 1/2"), "{lines:?}");
    assert!(lines[1].contains("Names translated 1/3"), "{lines:?}");
    assert!(lines[2].contains("Icons shown 1/2"), "{lines:?}");
}
