//! D79 FNTb font face. Entirely authored synthetic recipes, no game bytes.
use lbp_resource::font::{parse_font_face, FontLimits, Glyph, GLYPH_RECORD_BYTES};
use lbp_resource::{inspect, InspectionLimits};
use std::io::Cursor;

struct Recipe {
    revision: u32,
    index: Vec<i16>,
    pages: Vec<u32>,
    glyphs: Vec<[u8; GLYPH_RECORD_BYTES]>,
    flag: Option<u8>,
    data: Vec<u8>,
}
fn glyph(character: u16, width: u8, height: u8, offset: u32) -> [u8; GLYPH_RECORD_BYTES] {
    let mut g = [0; GLYPH_RECORD_BYTES];
    g[..2].copy_from_slice(&character.to_be_bytes());
    g[2..4].copy_from_slice(&(-3_i16).to_be_bytes());
    g[4] = 0xfe; // box_left -2
    g[5] = 9;
    g[6] = width;
    g[7] = height;
    g[8..12].copy_from_slice(&offset.to_be_bytes());
    g[12..].copy_from_slice(&(-1_i16).to_be_bytes());
    g
}
impl Recipe {
    fn new(revision: u32) -> Self {
        let mut index = vec![-1_i16; 128];
        index[b'A' as usize] = 0;
        index[b'B' as usize] = 1;
        Self {
            revision,
            index,
            pages: vec![0; 16],
            glyphs: vec![glyph(0x41, 2, 3, 0), glyph(0x42, 0, 0, 12)],
            flag: (revision > 0x272).then_some(0),
            data: vec![0x5a; 12],
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut out = b"FNTb".to_vec();
        out.extend(self.revision.to_be_bytes());
        out.extend(0xdead_beef_u32.to_be_bytes());
        out.extend(0_u32.to_be_bytes());
        out.extend((self.index.len() as u32).to_be_bytes());
        self.index.iter().for_each(|v| out.extend(v.to_be_bytes()));
        out.extend((self.pages.len() as u32).to_be_bytes());
        self.pages.iter().for_each(|v| out.extend(v.to_be_bytes()));
        out.extend((self.glyphs.len() as u32).to_be_bytes());
        self.glyphs.iter().for_each(|g| out.extend(g));
        out.extend(self.flag);
        out.extend(&self.data);
        out
    }
}
fn table_end(r: &Recipe) -> u64 {
    (16 + 4 + r.index.len() * 2 + 4 + r.pages.len() * 4 + 4 + r.glyphs.len() * 14) as u64
}

#[test]
fn d79_uncompressed_layout_fields_and_spans() {
    let recipe = Recipe::new(985);
    let font = parse_font_face(&recipe.bytes(), FontLimits::default()).unwrap();
    assert_eq!(font.revision, 985);
    assert_eq!(font.declared_data_offset, 0xdead_beef);
    assert_eq!(font.glyph_index.len(), 128);
    assert_eq!(font.glyph_index[b'B' as usize], 1);
    assert_eq!(font.glyph_page_used, vec![0; 16]);
    assert_eq!(
        font.glyphs[0],
        Glyph {
            character: 0x41,
            advance: -3,
            box_left: -2,
            box_top: 9,
            box_width: 2,
            box_height: 3,
            data_offset: 0,
            next_glyph: -1,
        }
    );
    assert_eq!(font.compressed, Some(false));
    assert_eq!(font.data_start, table_end(&recipe) + 1);
    assert_eq!(font.data_length, 12);
    // 2x3 box at two bytes per pixel = 12 bytes; empty box at offset 12 has a zero span.
    let check = font.glyph_data;
    assert_eq!((check.in_bounds, check.out_of_bounds), (2, 0));
    assert_eq!((check.empty_boxes, check.index_out_of_range), (1, 0));
    let summary = font.summary();
    assert_eq!(summary.glyph_count, 2);
    assert!(!summary.declared_data_offset_matches_data_start);
}

#[test]
fn d79_compression_bool_only_after_0x272() {
    for revision in [0x271, 0x272] {
        let recipe = Recipe::new(revision);
        assert_eq!(recipe.flag, None);
        let font = parse_font_face(&recipe.bytes(), FontLimits::default()).unwrap();
        assert_eq!(font.compressed, None);
        assert_eq!(font.data_start, table_end(&recipe));
    }
    let mut recipe = Recipe::new(0x273);
    recipe.flag = Some(2);
    let err = parse_font_face(&recipe.bytes(), FontLimits::default()).unwrap_err();
    assert_eq!(err.kind, lbp_core::ErrorKind::Unsupported);
}

#[test]
fn d79_compressed_spans_use_inclusive_u16_prefix_and_are_not_inflated() {
    let mut recipe = Recipe::new(985);
    recipe.flag = Some(1);
    recipe.glyphs = vec![
        glyph(0x41, 4, 4, 0),  // prefix 6: 4 bytes of (fake) stream, in bounds
        glyph(0x42, 4, 4, 6),  // prefix 1: below its own size
        glyph(0x43, 4, 4, 8),  // prefix 0x40: runs past the end
        glyph(0x44, 4, 4, 11), // prefix itself truncated
        glyph(0x45, 4, 4, u32::MAX),
    ];
    recipe.data = vec![0, 6, 1, 2, 3, 4, 0, 1, 0, 0x40, 0, 0];
    let font = parse_font_face(&recipe.bytes(), FontLimits::default()).unwrap();
    assert_eq!(font.compressed, Some(true));
    let check = font.glyph_data;
    assert_eq!((check.in_bounds, check.out_of_bounds), (1, 4));
}

#[test]
fn d79_index_entries_past_glyph_count_are_counted_not_fatal() {
    let mut recipe = Recipe::new(985);
    recipe.index[0] = 2;
    recipe.index[1] = i16::MAX;
    let font = parse_font_face(&recipe.bytes(), FontLimits::default()).unwrap();
    assert_eq!(font.glyph_data.index_out_of_range, 2);
}

#[test]
fn d79_truncation_counts_and_magic() {
    let bytes = Recipe::new(985).bytes();
    for cut in 0..bytes.len() - 12 {
        // Every cut inside the header or tables (up to and including the flag) is malformed.
        let err = parse_font_face(&bytes[..cut], FontLimits::default()).unwrap_err();
        assert_eq!(err.kind, lbp_core::ErrorKind::Malformed, "cut {cut}");
    }
    let limits = FontLimits {
        max_glyphs: 1,
        ..FontLimits::default()
    };
    let err = parse_font_face(&bytes, limits).unwrap_err();
    assert_eq!(err.kind, lbp_core::ErrorKind::LimitExceeded);
    let mut huge = bytes.clone();
    huge[16..20].copy_from_slice(&0x7fff_ffff_u32.to_be_bytes());
    let err = parse_font_face(&huge, FontLimits::default()).unwrap_err();
    assert_eq!(err.kind, lbp_core::ErrorKind::LimitExceeded);
    let mut wrong = bytes;
    wrong[3] = b't';
    let err = parse_font_face(&wrong, FontLimits::default()).unwrap_err();
    assert_eq!(err.kind, lbp_core::ErrorKind::Unsupported);
}

#[test]
fn d79_inspect_reports_font_summary_without_dependency_table() {
    let recipe = Recipe::new(985);
    let bytes = recipe.bytes();
    let read = |data: &[u8], limits: InspectionLimits| {
        inspect(&mut Cursor::new(data), 0, data.len() as u64, limits)
    };
    let result = read(&bytes, InspectionLimits::default());
    assert_eq!(result.status, "metadata_parsed", "{:?}", result.reason);
    assert_eq!(result.confirmed_type.unwrap().name, "fontface");
    assert_eq!(result.dependency_status, "not_applicable_to_font_face");
    assert_eq!(result.dependency_count_raw, None);
    let header = result.header.unwrap();
    assert_eq!(header.scope, "font_face_header");
    assert_eq!(header.compressed, Some(false));
    assert_eq!(header.payload_offset, Some(table_end(&recipe) as u32 + 1));
    let font = result.font.unwrap();
    assert_eq!((font.glyph_index_count, font.glyph_count), (128, 2));
    // Tables and the flag are read; glyph bitmap bytes are not.
    // Inspect reads the magic and revision words once before the 16-byte font header.
    assert_eq!(
        result.metadata_bytes_read as u64,
        8 + table_end(&recipe) + 1
    );

    let tight = InspectionLimits {
        max_metadata_bytes: 64,
        ..InspectionLimits::default()
    };
    let result = read(&bytes, tight);
    assert_eq!(result.status, "limit_exceeded");
    assert!(result.font.is_none());
    assert_eq!(result.header.unwrap().scope, "font_face_header");

    let result = read(&bytes[..40], InspectionLimits::default());
    assert_eq!(result.status, "malformed");
    // Non-font inspections never carry the font field in JSON.
    let mut other = bytes;
    other[..4].copy_from_slice(b"FNTt");
    let result = read(&other, InspectionLimits::default());
    assert_eq!(result.status, "identified_unparsed");
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("font").is_none());
}
