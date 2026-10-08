//! U8 inventory data: authored synthetic item details and tables only, no game bytes.
use lbp_level::item_details::{ItemDetails, Placement, Ref, UserCreated};
use lbp_level::serial::Descriptor;
use lbp_preview::popit::{build_popit, count_lines, item_text, PlanInput, TitleSource};
use lbp_preview::story::{ResourceKey, SlotId, Strings};
use lbp_resource::translation::{lams_key, parse_translation_table, TranslationLimits};

/// Binary `.trans` keyed by raw LAMS keys.
fn table(pairs: &[(u32, &str)]) -> lbp_resource::translation::TranslationTable {
    let mut rows = pairs.to_vec();
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

fn details(type_bits: u32) -> ItemDetails {
    ItemDetails {
        date_added: 0,
        level_unlock: SlotId {
            slot_type: 0,
            number: 0,
        },
        highlight_sound: None,
        colour: 0,
        type_bits,
        sub_type: 0,
        title_key: None,
        description_key: None,
        translation_tag: None,
        creation_history: Ref::Null,
        icon: None,
        user_created: Ref::Null,
        photo: Ref::Null,
        eyetoy: Ref::Null,
        location_index: 0,
        category_index: 0,
        primary_index: 0,
        creator: Ref::Null,
        tool_type: 0,
        flags: 0,
        last_used: None,
        num_uses: None,
        fluff_cost: None,
        allow_emit: None,
        shareable: None,
        copyright: None,
    }
}

fn strings() -> Strings {
    Strings {
        base: Some(table(&[
            (0x10, "Wood"),
            (0x11, "Plain wood"),
            (lams_key("OLD_NAME"), "Old item"),
            (0x20, "Materials"),
            (0x21, "Basics"),
        ])),
        patch: Some(table(&[(0x10, "Oak")])),
    }
}

#[test]
fn u8_titles_follow_update_translations_then_fallbacks() {
    let s = strings();
    let mut keyed = details(0);
    keyed.title_key = Some(0x10);
    keyed.description_key = Some(0x11);
    assert_eq!(
        item_text(&keyed, &s, "p"),
        (
            "Oak".into(),
            TitleSource::Translated,
            Some("Plain wood".into())
        )
    );
    // A tag replaces the keys (tag_NAME / tag_DESC).
    let mut tagged = keyed.clone();
    tagged.translation_tag = Some("OLD".into());
    assert_eq!(
        item_text(&tagged, &s, "p"),
        ("Old item".into(), TitleSource::Translated, None)
    );
    let mut user = details(0);
    user.user_created = Ref::Value(UserCreated {
        name: "My box".into(),
        description: "mine".into(),
    });
    assert_eq!(
        item_text(&user, &s, "p"),
        (
            "My box".into(),
            TitleSource::UserCreated,
            Some("mine".into())
        )
    );
    let mut tag_only = details(0);
    tag_only.translation_tag = Some("NOPE".into());
    assert_eq!(item_text(&tag_only, &s, "p").1, TitleSource::TagOnly);
    let mut untranslated = details(0);
    untranslated.title_key = Some(0x99);
    assert_eq!(item_text(&untranslated, &s, "p.plan").0, "p.plan");
    assert_eq!(
        item_text(&untranslated, &s, "p").1,
        TitleSource::KeyUntranslated
    );
    assert_eq!(item_text(&details(0), &s, "p").1, TitleSource::Path);
}

#[test]
fn u8_pages_group_by_type_and_count() {
    let s = strings();
    let plan = |i: u32, bits: u32, result: Result<(), &str>| {
        let mut d = details(bits);
        d.title_key = Some(0x10);
        d.icon = Some(Descriptor {
            flags: None,
            kind_byte: 2,
            guid: Some(0x500 + i),
            sha1: None,
        });
        PlanInput {
            sha1: format!("{i:040x}"),
            paths: vec![format!("gamedata/p{i}.plan")],
            guids: Vec::new(),
            head: 0x3f8,
            result: result
                .map(|_| {
                    (
                        d,
                        Some(Placement::Keys {
                            location: 0x20,
                            category: if i == 0 { 0x21 } else { 0x77 },
                        }),
                    )
                })
                .map_err(str::to_owned),
        }
    };
    let plans = vec![
        plan(0, 1 << 0, Ok(())), // primitive_material
        plan(1, 1 << 3, Ok(())), // sticker
        plan(2, 1 << 0, Ok(())),
        plan(3, 0, Ok(())),
        plan(4, 0, Err("stopped: Unsupported")),
    ];
    let resolves = |k: ResourceKey| k != ResourceKey::Guid(0x502);
    let p = build_popit(plans, &s, &resolves);
    assert_eq!(p.items.len(), 4);
    let names: Vec<_> = p.pages.keys().cloned().collect();
    assert_eq!(names, ["primitive_material", "sticker", "untyped"]);
    assert_eq!(p.pages["primitive_material"].len(), 2);
    let c = &p.counts;
    assert_eq!((c.plans, c.items), (5, 4));
    assert_eq!(c.not_read["stopped: Unsupported"], 1);
    assert_eq!((c.with_icon, c.icon_resolved), (4, 3));
    assert_eq!((c.placement_keys, c.placement_translated), (4, 1));
    assert_eq!(p.items[0].location.as_deref(), Some("Materials"));
    assert_eq!(p.items[0].category.as_deref(), Some("Basics"));
    let lines = count_lines(&p);
    assert!(lines[0].contains("Items 4/5"), "{lines:?}");
    assert!(lines[1].contains("Titles translated 4/4"), "{lines:?}");
}
