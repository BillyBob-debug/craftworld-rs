//! U3: translation tables and font faces for the string/font browser. Renderer-independent:
//! tables are parsed with U1 (`lbp_resource::translation`), fonts with D79/U2
//! (`lbp_resource::font`), and each font's glyphs are composed into a CPU RGBA sheet.
//!
//! Rules: table format and LAMS keys as U1 (docs/u1-translation.md). Tables are selected by
//! a `.trans` MAP path (they have no magic) and grouped by content hash; which language a
//! table holds is UNKNOWN except g54800/g69910 (English base/patch, Toolkit
//! `RestlessPopulator`). Glyph sheet: compressed fonts draw coverage as white with alpha =
//! coverage (Toolkit draws grey; alpha meaning STRONG_INFERENCE); uncompressed fonts draw
//! the two raw bytes as green and blue like Toolkit `getGlyph` (meaning UNKNOWN).
use crate::level_scene::read_resource;
use crate::story::{BASE_TRANSLATION_GUID, PATCH_TRANSLATION_GUID};
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_resource::font::{
    atlas_layout, decode_glyphs, parse_font_face, FontLimits, FontSummary, Glyph,
    GlyphDecodeLimits, GlyphDecodeSummary, GlyphPixels,
};
/// Re-exported for hosts that build or inspect tables (tests, viewers).
pub use lbp_resource::translation;
use lbp_resource::translation::{
    lams_key, parse_translation_table, TranslationLimits, TranslationTable,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_TABLE_BYTES: usize = 32 << 20;
pub const MAX_FONT_BYTES: usize = 32 << 20;
/// Sheet width in pixels (rows wrap); sheets above 4096 x 4096 are refused.
pub const SHEET_WIDTH: u32 = 1024;
pub const MAX_SHEET_SIDE: u32 = 4096;

pub struct LoadedTable {
    pub sha1: String,
    pub paths: Vec<String>,
    pub guids: Vec<u32>,
    pub table: Result<TranslationTable, String>,
}

impl LoadedTable {
    /// "English base", "English patch" or the first path's file name.
    pub fn label(&self) -> String {
        let stem = self
            .paths
            .first()
            .and_then(|p| p.rsplit(['/', '\\']).next())
            .unwrap_or("?");
        if self.guids.contains(&BASE_TRANSLATION_GUID) {
            format!("English base ({stem})")
        } else if self.guids.contains(&PATCH_TRANSLATION_GUID) {
            format!("English patch ({stem})")
        } else {
            stem.to_owned()
        }
    }
}

/// Entry indices of `table` matching `query`, in file order: an empty query matches all;
/// otherwise a case-insensitive substring of the text, the key as 8 hex digits, or the
/// LAMS key of the query taken as a tag (as typed and upper-cased).
pub fn search(table: &TranslationTable, query: &str) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..table.entries.len()).collect();
    }
    let lower = query.to_lowercase();
    let hex = u32::from_str_radix(query.trim_start_matches("0x"), 16).ok();
    let tags = [lams_key(query), lams_key(&query.to_uppercase())];
    table
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            tags.contains(&e.key)
                || hex == Some(e.key)
                || e.tag
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&lower))
                || e.text
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&lower))
        })
        .map(|(i, _)| i)
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedFont {
    pub sha1: String,
    pub paths: Vec<String>,
    pub summary: Option<FontSummary>,
    pub glyphs: Vec<Glyph>,
    pub decode: Option<GlyphDecodeSummary>,
    pub sheet: Option<Sheet>,
    /// Why the font, its glyphs or its sheet stopped, if they did.
    pub error: Option<String>,
}

/// Decodes a whole FNTb resource into its glyph table and an RGBA sheet.
pub fn font_sheet(sha1: String, paths: Vec<String>, bytes: &[u8]) -> LoadedFont {
    let mut out = LoadedFont {
        sha1,
        paths,
        summary: None,
        glyphs: Vec::new(),
        decode: None,
        sheet: None,
        error: None,
    };
    let font = match parse_font_face(bytes, FontLimits::default()) {
        Ok(f) => f,
        Err(e) => {
            out.error = Some(format!("font: {e}"));
            return out;
        }
    };
    out.summary = Some(font.summary());
    out.glyphs = font.glyphs.clone();
    let (bitmaps, decode) = match decode_glyphs(bytes, &font, GlyphDecodeLimits::default()) {
        Ok(v) => v,
        Err(e) => {
            out.error = Some(format!("glyphs: {e}"));
            return out;
        }
    };
    out.decode = Some(decode);
    let atlas = match atlas_layout(&font.glyphs, SHEET_WIDTH, 1) {
        Ok(a) if a.width <= MAX_SHEET_SIDE && a.height <= MAX_SHEET_SIDE => a,
        Ok(a) => {
            out.error = Some(format!("sheet {}x{} over budget", a.width, a.height));
            return out;
        }
        Err(e) => {
            out.error = Some(format!("sheet: {e}"));
            return out;
        }
    };
    let (w, h) = (atlas.width.max(1) as usize, atlas.height.max(1) as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for (slot, bitmap) in atlas.slots.iter().zip(&bitmaps) {
        let Some(bitmap) = bitmap else { continue };
        let row = usize::from(bitmap.width);
        for i in 0..row * usize::from(bitmap.height) {
            let (x, y) = (slot.x as usize + i % row, slot.y as usize + i / row);
            let px = match &bitmap.pixels {
                GlyphPixels::Coverage(v) => [255, 255, 255, v[i]],
                GlyphPixels::TwoChannel(v) => [0, v[i][0], v[i][1], 255],
            };
            rgba[(y * w + x) * 4..][..4].copy_from_slice(&px);
        }
    }
    out.sheet = Some(Sheet {
        width: w as u32,
        height: h as u32,
        rgba,
    });
    out
}

/// Counts for the in-app panel. Units: distinct table content hash, distinct LAMS key per
/// table, distinct font content hash, glyph record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextCounts {
    pub tables: usize,
    pub tables_parsed: usize,
    pub empty_tables: usize,
    pub keys: usize,
    pub fonts: usize,
    pub fonts_with_sheet: usize,
    pub glyphs: usize,
    pub glyphs_decoded: usize,
    pub glyphs_empty: usize,
    pub glyphs_malformed: usize,
}

pub struct TextData {
    pub tables: Vec<LoadedTable>,
    pub fonts: Vec<LoadedFont>,
    pub counts: TextCounts,
}

pub fn count(tables: &[LoadedTable], fonts: &[LoadedFont]) -> TextCounts {
    let mut c = TextCounts {
        tables: tables.len(),
        fonts: fonts.len(),
        ..Default::default()
    };
    for t in tables {
        if let Ok(table) = &t.table {
            c.tables_parsed += 1;
            c.empty_tables += usize::from(table.entries.is_empty());
            c.keys += table.distinct_keys();
        }
    }
    for f in fonts {
        c.fonts_with_sheet += usize::from(f.sheet.is_some());
        c.glyphs += f.glyphs.len();
        if let Some(d) = f.decode {
            c.glyphs_decoded += d.decoded;
            c.glyphs_empty += d.empty;
            c.glyphs_malformed += d.malformed;
        }
    }
    c
}

pub fn count_lines(c: &TextCounts) -> Vec<String> {
    vec![
        format!(
            "Tables parsed {}/{} (unit: distinct .trans content hash; {} empty); keys {} (distinct per table, summed)",
            c.tables_parsed, c.tables, c.empty_tables, c.keys
        ),
        format!(
            "Fonts with a glyph sheet {}/{} present; glyphs decoded {} + empty {} of {} ({} malformed)",
            c.fonts_with_sheet,
            c.fonts,
            c.glyphs_decoded,
            c.glyphs_empty,
            c.glyphs,
            c.glyphs_malformed
        ),
    ]
}

/// Groups catalog rows by content hash, keeping sorted paths and GUIDs.
fn rows(
    catalog: &Catalog,
    keep: impl Fn(&lbp_catalog::Resource) -> bool,
) -> BTreeMap<String, (BTreeSet<String>, BTreeSet<u32>)> {
    let mut out: BTreeMap<String, (BTreeSet<String>, BTreeSet<u32>)> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if keep(r) {
            let e = out.entry(r.map_row.sha1.to_ascii_lowercase()).or_default();
            e.0.insert(r.map_row.path.clone());
            if let Ok(g) = u32::from_str_radix(r.map_row.guid.trim(), 16) {
                e.1.insert(g);
            }
        }
    }
    out
}

/// Reads every `.trans` table and every magic-confirmed font face in the catalog.
/// Rows whose bytes are not present (for example inside an encrypted archive) are listed
/// with the resolve/read error.
pub fn load_text(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<TextData, String> {
    let mut tables = Vec::new();
    for (sha1, (paths, guids)) in rows(catalog, |r| {
        r.map_row.path.to_ascii_lowercase().ends_with(".trans")
    }) {
        control.check()?;
        let table = crate::resolve(catalog, None, Some(&sha1))
            .and_then(|key| read_resource(source, &key, control, MAX_TABLE_BYTES))
            .and_then(|bytes| {
                parse_translation_table(&bytes, TranslationLimits::default())
                    .map_err(|e| e.to_string())
            });
        tables.push(LoadedTable {
            sha1,
            paths: paths.into_iter().collect(),
            guids: guids.into_iter().collect(),
            table,
        });
    }
    // English first, then the others by label.
    tables.sort_by_key(|t| {
        let rank = if t.guids.contains(&BASE_TRANSLATION_GUID) {
            0
        } else if t.guids.contains(&PATCH_TRANSLATION_GUID) {
            1
        } else {
            2
        };
        (rank, t.label())
    });
    let mut fonts = Vec::new();
    for (sha1, (paths, _)) in rows(catalog, |r| r.display_type() == "fontface") {
        control.check()?;
        let paths: Vec<String> = paths.into_iter().collect();
        let bytes = crate::resolve(catalog, None, Some(&sha1))
            .and_then(|key| read_resource(source, &key, control, MAX_FONT_BYTES));
        fonts.push(match bytes {
            Ok(bytes) => font_sheet(sha1, paths, &bytes),
            Err(e) => LoadedFont {
                sha1,
                paths,
                summary: None,
                glyphs: Vec::new(),
                decode: None,
                sheet: None,
                error: Some(e),
            },
        });
    }
    let counts = count(&tables, &fonts);
    Ok(TextData {
        tables,
        fonts,
        counts,
    })
}
