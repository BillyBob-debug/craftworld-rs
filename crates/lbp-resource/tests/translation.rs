//! U1 translation tables. Entirely authored synthetic recipes, no game bytes.
//! Expected LAMS keys were computed by running Toolkit `RTranslationTable.makeLamsKeyID`
//! at d6a2ef80b5a09999f08f23a803b516096055e5c9 on these authored tags.
use lbp_core::ErrorKind;
use lbp_resource::translation::{
    lams_key, parse_translation_table, TranslationForm, TranslationLimits,
};

fn utf16_marked(text: &str) -> Vec<u8> {
    let mut out = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

/// Binary table: rows (key, offset into the strings blob) then the blob.
fn binary(rows: &[(u32, u32)], blob: &[u8]) -> Vec<u8> {
    let mut out = (rows.len() as u32).to_be_bytes().to_vec();
    for (key, offset) in rows {
        out.extend_from_slice(&key.to_be_bytes());
        out.extend_from_slice(&offset.to_be_bytes());
    }
    out.extend_from_slice(blob);
    out
}

#[test]
fn u1_lams_key_matches_toolkit_vectors() {
    for (tag, key) in [
        ("", 0x545e_8000),
        ("A", 0x545e_8021),
        ("\u{e9}", 0x545e_80c9),
        ("COSTUME_TORSO", 0xea31_e460),
        ("SLOT_NAME_TEST", 0xf96f_211f),
        ("01234567890123456789012345678901", 0x6f63_f800),
        ("012345678901234567890123456789012", 0x179b_64ce),
        (
            "0123456789012345678901234567890123456789012345678901234567890123456789",
            0x42bf_9308,
        ),
    ] {
        assert_eq!(lams_key(tag), key, "{tag:?}");
    }
    // Units past 64 do not contribute.
    let base = "0123456789012345678901234567890123456789012345678901234567890123";
    assert_eq!(lams_key(base), lams_key(&format!("{base}tail")));
}

#[test]
fn u1_binary_strings_run_to_next_mark_or_end() {
    let first = utf16_marked("Hello");
    let second = utf16_marked("Pod \u{2764}");
    let blob = [first.clone(), second].concat();
    let key_a = lams_key("A_NAME");
    let key_b = lams_key("B_NAME");
    let bytes = binary(&[(key_a, 0), (key_b, first.len() as u32)], &blob);
    let table = parse_translation_table(&bytes, TranslationLimits::default()).unwrap();
    assert_eq!(table.form, TranslationForm::Binary);
    assert_eq!(table.entries.len(), 2);
    assert_eq!(table.strings_start, 4 + 16);
    assert_eq!(table.translate("A_NAME"), Some("Hello"));
    assert_eq!(table.get(key_b), Some("Pod \u{2764}"));
    assert_eq!(table.get(0x1234_5678), None);
    let checks = table.checks;
    assert_eq!(checks.strings_without_mark, 0);
    assert_eq!(checks.out_of_order_keys, usize::from(key_b <= key_a));
}

#[test]
fn u1_empty_string_and_end_of_data() {
    // A lone mark as the last unit decodes to an empty string.
    let bytes = binary(&[(7, 0)], &[0xFE, 0xFF]);
    let table = parse_translation_table(&bytes, TranslationLimits::default()).unwrap();
    assert_eq!(table.get(7), Some(""));
    // An empty table is valid.
    let empty = parse_translation_table(&binary(&[], &[]), TranslationLimits::default()).unwrap();
    assert_eq!(empty.summary().entry_count, 0);
}

#[test]
fn u1_repeated_keys_keep_all_rows_and_last_wins() {
    let a = utf16_marked("old");
    let b = utf16_marked("new");
    let blob = [a.clone(), b].concat();
    let bytes = binary(&[(5, 0), (5, a.len() as u32), (9, 0)], &blob);
    let table = parse_translation_table(&bytes, TranslationLimits::default()).unwrap();
    assert_eq!(table.entries.len(), 3);
    assert_eq!(table.get(5), Some("new"));
    let summary = table.summary();
    assert_eq!(summary.distinct_keys, 2);
    assert_eq!(summary.checks.repeated_keys, 1);
    assert_eq!(summary.checks.out_of_order_keys, 1);
    assert_eq!(summary.checks.shared_offsets, 1);
}

#[test]
fn u1_unmarked_little_endian_and_invalid_strings_are_counted() {
    // No mark: decoded big-endian, kept. Little-endian mark: decoded little-endian.
    let mut blob = vec![0x00, 0x41, 0x00, 0x42];
    let le_at = blob.len() as u32;
    blob.extend_from_slice(&[0xFF, 0xFE, 0x43, 0x00]);
    let bad_at = blob.len() as u32;
    blob.extend_from_slice(&[0xFE, 0xFF, 0xD8, 0x00, 0x00, 0x44]);
    // The first string runs into the second (no mark between), as in Toolkit.
    let bytes = binary(&[(1, 0), (2, le_at), (3, bad_at)], &blob);
    let table = parse_translation_table(&bytes, TranslationLimits::default()).unwrap();
    assert_eq!(table.get(2), Some("C"));
    assert_eq!(table.get(3), Some("\u{fffd}D"));
    assert!(table.get(1).unwrap().starts_with("AB"));
    let checks = table.checks;
    assert_eq!(checks.strings_without_mark, 1);
    assert_eq!(checks.little_endian_strings, 1);
    assert!(checks.invalid_utf16_strings >= 1);
}

#[test]
fn u1_malformed_offsets_counts_and_limits() {
    let limits = TranslationLimits::default();
    let malformed = |bytes: &[u8]| {
        let error = parse_translation_table(bytes, limits).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Malformed, "{}", error.message);
    };
    // Truncated count and rows.
    malformed(&[0, 0, 0]);
    malformed(&binary(&[(1, 0)], &[])[..8]);
    // Negative count and offsets.
    malformed(&[0x80, 0, 0, 0]);
    malformed(&binary(&[(1, 0x8000_0000)], &utf16_marked("x")));
    // String start past the data, and an odd tail with no mark.
    malformed(&binary(&[(1, 4)], &utf16_marked("x")));
    malformed(&binary(&[(1, 0)], &[0xFE, 0xFF, 0x00]));
    // Row table larger than the data.
    let mut huge = 1000u32.to_be_bytes().to_vec();
    huge.extend_from_slice(&[0; 16]);
    malformed(&huge);

    let tight = TranslationLimits {
        max_entries: 1,
        max_total_code_units: 3,
    };
    let two = binary(&[(1, 0), (2, 0)], &utf16_marked("x"));
    assert_eq!(
        parse_translation_table(&two, tight).unwrap_err().kind,
        ErrorKind::LimitExceeded
    );
    let long = binary(&[(1, 0)], &utf16_marked("long"));
    assert_eq!(
        parse_translation_table(&long, tight).unwrap_err().kind,
        ErrorKind::LimitExceeded
    );
}

#[test]
fn u1_legacy_text_form_alternates_tag_and_value() {
    let text = "\r\nPOD_TITLE\r\nThe Pod\nEMPTY_VALUE\n\nMOON\rMy Moon\nDANGLING";
    let bytes = utf16_marked(text);
    let table = parse_translation_table(&bytes, TranslationLimits::default()).unwrap();
    assert_eq!(table.form, TranslationForm::LegacyText);
    assert_eq!(table.translate("POD_TITLE"), Some("The Pod"));
    assert_eq!(table.translate("EMPTY_VALUE"), Some(""));
    assert_eq!(table.translate("MOON"), Some("My Moon"));
    assert_eq!(table.translate("DANGLING"), None);
    assert_eq!(table.entries.len(), 4);
    assert_eq!(table.entries[3].tag.as_deref(), Some("DANGLING"));
    assert_eq!(table.entries[3].text, None);
    assert_eq!(table.entries[0].key, lams_key("POD_TITLE"));

    let mut odd = utf16_marked("A\nB");
    odd.push(0);
    assert_eq!(
        parse_translation_table(&odd, TranslationLimits::default())
            .unwrap_err()
            .kind,
        ErrorKind::Malformed
    );
}
