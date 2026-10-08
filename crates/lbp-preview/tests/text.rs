//! U3 strings and fonts: authored synthetic tables and FNTb recipes only, no game bytes.
use lbp_preview::text::translation::{lams_key, parse_translation_table, TranslationLimits};
use lbp_preview::text::{count, font_sheet, search, LoadedTable};

fn table(rows: &[(u32, &str)]) -> lbp_preview::text::translation::TranslationTable {
    let mut out = (rows.len() as u32).to_be_bytes().to_vec();
    let mut blob = Vec::new();
    for (key, text) in rows {
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

/// FNTb with glyph 'A' (2x3) and an empty 'B'; `data` is the glyph data region.
fn fnt(revision: u32, flag: Option<u8>, data: &[u8]) -> Vec<u8> {
    let mut out = b"FNTb".to_vec();
    out.extend(revision.to_be_bytes());
    out.extend(0u32.to_be_bytes());
    out.extend(0u32.to_be_bytes());
    let mut index = vec![-1i16; 128];
    index[b'A' as usize] = 0;
    index[b'B' as usize] = 1;
    out.extend((index.len() as u32).to_be_bytes());
    index.iter().for_each(|v| out.extend(v.to_be_bytes()));
    out.extend(16u32.to_be_bytes());
    out.extend([0u8; 64]);
    out.extend(2u32.to_be_bytes());
    for (c, w, h, offset) in [(0x41u16, 2u8, 3u8, 0u32), (0x42, 0, 0, 0)] {
        out.extend(c.to_be_bytes());
        out.extend((-3i16).to_be_bytes());
        out.extend([0xfe, 9, w, h]);
        out.extend(offset.to_be_bytes());
        out.extend((-1i16).to_be_bytes());
    }
    out.extend(flag);
    out.extend(data);
    out
}

#[test]
fn u3_search_matches_text_tag_and_hex_key() {
    let t = table(&[
        (lams_key("SLOT_NAME"), "Gardens"),
        (0x0000_00ff, "Cave of Wonders"),
        (0x1234_5678, "garden gnome"),
    ]);
    assert_eq!(search(&t, ""), [0, 1, 2]);
    assert_eq!(search(&t, "GARDEN"), [0, 2]);
    // A tag, as typed or lower case, hashes to its LAMS key.
    assert_eq!(search(&t, "slot_name"), [0]);
    assert_eq!(search(&t, "12345678"), [2]);
    assert_eq!(search(&t, "0xff"), [1]);
    assert!(search(&t, "nothing").is_empty());
}

#[test]
fn u3_font_sheets_and_counts() {
    // Uncompressed: two raw bytes per pixel drawn as green/blue.
    let raw: Vec<u8> = (0..12).collect();
    let f = font_sheet("aa".into(), vec!["a.fnt".into()], &fnt(985, Some(0), &raw));
    assert_eq!(f.error, None);
    let s = f.sheet.as_ref().unwrap();
    // Padding 1: glyph A at (1, 1); first pixel bytes [0, 1].
    let at = ((s.width + 1) * 4) as usize;
    assert_eq!(&s.rgba[at..at + 4], &[0, 0, 1, 255]);
    // Compressed: coverage drawn as white with alpha.
    let coverage = [0u8, 64, 128, 255, 32, 16];
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut z, &coverage).unwrap();
    let stream = z.finish().unwrap();
    let mut data = ((stream.len() + 2) as u16).to_be_bytes().to_vec();
    data.extend(stream);
    let c = font_sheet("bb".into(), vec!["b.fnt".into()], &fnt(985, Some(1), &data));
    assert_eq!(c.error, None);
    let s = c.sheet.as_ref().unwrap();
    let px = |x: u32, y: u32| {
        let i = ((y * s.width + x) * 4) as usize;
        s.rgba[i..i + 4].to_vec()
    };
    assert_eq!(px(2, 1), [255, 255, 255, 64]);
    assert_eq!(px(2, 2), [255, 255, 255, 255]);
    // A broken font keeps a tab with its error.
    let bad = font_sheet("cc".into(), vec!["c.fnt".into()], b"FNTb");
    assert!(bad.error.as_deref().unwrap().starts_with("font:"));
    let tables = vec![
        LoadedTable {
            sha1: "t1".into(),
            paths: vec!["gamedata/x.trans".into()],
            guids: vec![54800],
            table: Ok(table(&[(1, "a"), (2, "b")])),
        },
        LoadedTable {
            sha1: "t2".into(),
            paths: vec!["gamedata/empty.trans".into()],
            guids: vec![1],
            table: Ok(table(&[])),
        },
        LoadedTable {
            sha1: "t3".into(),
            paths: vec!["gamedata/gone.trans".into()],
            guids: vec![],
            table: Err("Unresolved".into()),
        },
    ];
    assert_eq!(tables[0].label(), "English base (x.trans)");
    let n = count(&tables, &[f, c, bad]);
    assert_eq!(
        (n.tables, n.tables_parsed, n.empty_tables, n.keys),
        (3, 2, 1, 2)
    );
    assert_eq!((n.fonts, n.fonts_with_sheet, n.glyphs), (3, 2, 4));
    assert_eq!(
        (n.glyphs_decoded, n.glyphs_empty, n.glyphs_malformed),
        (2, 2, 0)
    );
}
