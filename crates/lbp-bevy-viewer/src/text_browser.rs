//! U3: `--text-browser`, translation tables and font faces. One tab per `.trans` table
//! (English base and patch first) and per font face. A table tab lists entries (LAMS key
//! and text) filtered by the typed search; a font tab shows the decoded glyph sheet and the
//! glyph records. Data: `lbp_preview::text`.
//! Usage: `--text-browser --snapshot SNAPSHOT --game-dir DIR [--text-browser-report]
//! [--qa-capture local-analysis/NEW.png]`.
use crate::gooey_ui::{run_gooey, GooeyImage, SharedText};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_gooey::{Align, Builder, FrameInput, ImageRef, Layout, PanelStyle, TextStyle, UiTree, Uid};
use lbp_preview::text::{count_lines, load_text, search, TextData};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const ROOT: Uid = Uid(0x5458_0000);
const PREV: Uid = Uid(0x5458_0010);
const NEXT: Uid = Uid(0x5458_0011);
const TAB_BASE: u64 = 0x5458_1000;
pub const ROWS: usize = 24;
/// Longest text shown per row, in characters.
const ROW_CHARS: usize = 110;

pub struct TextState {
    pub data: TextData,
    pub tab: usize,
    pub page: usize,
    pub query: SharedText,
    /// (tab, query) the cached matches belong to.
    cached: Option<(usize, String)>,
    matches: Vec<usize>,
}

impl TextState {
    pub fn new(data: TextData, query: SharedText) -> Self {
        Self {
            data,
            tab: 0,
            page: 0,
            query,
            cached: None,
            matches: Vec::new(),
        }
    }
    fn tabs(&self) -> usize {
        self.data.tables.len() + self.data.fonts.len()
    }
}

/// Sheet image key: the font's content hash (not a texture reference).
pub fn sheet_ref(sha1: &str) -> Option<ImageRef> {
    let mut h = [0u8; 20];
    for (i, b) in h.iter_mut().enumerate() {
        *b = u8::from_str_radix(sha1.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(ImageRef::Hash(h))
}

fn clip(text: &str) -> String {
    let one_line = text.replace(['\n', '\r'], " ");
    match one_line.char_indices().nth(ROW_CHARS) {
        Some((at, _)) => format!("{}...", &one_line[..at]),
        None => one_line,
    }
}

pub fn text_screen(input: FrameInput, state: &mut TextState) -> UiTree {
    let query = state.query.lock().map(|q| q.clone()).unwrap_or_default();
    let tables = state.data.tables.len();
    if state.tab < tables && state.cached.as_ref() != Some(&(state.tab, query.clone())) {
        state.matches = match &state.data.tables[state.tab].table {
            Ok(t) => search(t, &query),
            Err(_) => Vec::new(),
        };
        if state.cached.as_ref().is_some_and(|c| c.0 == state.tab) {
            state.page = 0;
        }
        state.cached = Some((state.tab, query.clone()));
    }
    let rows = if state.tab < tables {
        state.matches.len()
    } else {
        state
            .data
            .fonts
            .get(state.tab - tables)
            .map_or(0, |f| f.glyphs.len())
    };
    let pages = rows.div_ceil(ROWS).max(1);
    let (mut tab, mut page) = (state.tab, state.page);
    let data = &state.data;
    let mut b = Builder::new(ROOT, input);
    b.frame(
        Uid(0x5458_0001),
        Layout::Column,
        Align::Start,
        PanelStyle::Filled,
        16.,
        6.,
        |b| {
            b.text(
                "Strings and fonts (disc .trans tables and font faces)",
                TextStyle::Title,
            );
            for line in count_lines(&data.counts) {
                b.text(line, TextStyle::Small);
            }
            b.frame(
                Uid(0x5458_0002),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Plain,
                0.,
                6.,
                |b| {
                    for (i, t) in data.tables.iter().enumerate() {
                        let n = t.table.as_ref().map_or(0, |t| t.distinct_keys());
                        if b.button(Uid(TAB_BASE + i as u64), format!("{} ({n})", t.label()))
                            .activated
                        {
                            tab = i;
                        }
                    }
                    for (i, f) in data.fonts.iter().enumerate() {
                        let stem = f
                            .paths
                            .first()
                            .and_then(|p| p.rsplit(['/', '\\']).next())
                            .unwrap_or("?");
                        let uid = Uid(TAB_BASE + (tables + i) as u64);
                        if b.button(uid, format!("font {stem}")).activated {
                            tab = tables + i;
                        }
                    }
                },
            );
            b.frame(
                Uid(0x5458_0003),
                Layout::Row,
                Align::Start,
                PanelStyle::Plain,
                0.,
                8.,
                |b| {
                    if b.button_with(PREV, "Previous", None, state.page > 0).activated {
                        page = state.page - 1;
                    }
                    b.text(
                        format!("Page {}/{pages}, {rows} rows", state.page + 1),
                        TextStyle::Body,
                    );
                    if b.button_with(NEXT, "Next", None, state.page + 1 < pages)
                        .activated
                    {
                        page = state.page + 1;
                    }
                },
            );
            if let Some(t) = data.tables.get(state.tab) {
                b.text(
                    format!(
                        "Search: {query}_   (type text, a tag such as SLOT_NAME, or a hex key; Backspace deletes)"
                    ),
                    TextStyle::Body,
                );
                match &t.table {
                    Err(e) => b.text(format!("Not read: {e}"), TextStyle::Body),
                    Ok(table) => {
                        for &i in state.matches.iter().skip(state.page * ROWS).take(ROWS) {
                            let e = &table.entries[i];
                            b.text(
                                format!(
                                    "{:08x}  {}",
                                    e.key,
                                    clip(e.text.as_deref().unwrap_or("(no text)"))
                                ),
                                TextStyle::Small,
                            );
                        }
                    }
                }
            } else if let Some(f) = data.fonts.get(state.tab - tables) {
                if let Some(e) = &f.error {
                    b.text(format!("Stopped: {e}"), TextStyle::Body);
                }
                if let (Some(s), Some(image)) = (&f.sheet, sheet_ref(&f.sha1)) {
                    // Shown at 2x up to 1024 px wide; alpha = coverage (inferred).
                    let scale = (1024. / s.width as f32).min(2.);
                    b.text(
                        format!("Glyph sheet {}x{} (white, alpha = coverage)", s.width, s.height),
                        TextStyle::Small,
                    );
                    b.image(image, s.width as f32 * scale, s.height as f32 * scale);
                }
                for g in f.glyphs.iter().skip(state.page * ROWS).take(ROWS) {
                    let c = char::from_u32(u32::from(g.character)).unwrap_or('\u{fffd}');
                    b.text(
                        format!(
                            "U+{:04X} {c:?}  advance {}  box {}x{} at ({}, {})  data +{}",
                            g.character,
                            g.advance,
                            g.box_width,
                            g.box_height,
                            g.box_left,
                            g.box_top,
                            g.data_offset
                        ),
                        TextStyle::Small,
                    );
                }
            }
        },
    );
    let tree = b.finish();
    if tab != state.tab {
        page = 0;
        if let Ok(mut q) = state.query.lock() {
            q.clear();
        }
    }
    state.tab = tab.min(state.tabs().saturating_sub(1));
    state.page = page.min(pages - 1);
    tree
}

struct Args {
    snapshot: PathBuf,
    game: PathBuf,
    report: bool,
}

fn args() -> Result<Args, String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .map(PathBuf::from)
    };
    Ok(Args {
        snapshot: value("--snapshot").ok_or("--text-browser requires --snapshot")?,
        game: value("--game-dir").ok_or("--text-browser requires --game-dir")?,
        report: all.iter().any(|a| a == "--text-browser-report"),
    })
}

pub fn run_text_browser() -> Result<(), String> {
    let a = args()?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(&a.snapshot, &control)?;
    let source = AttachedSource::attach(&a.game, &catalog, &control)?;
    let mut data = load_text(&source, &catalog, &control)?;
    for line in count_lines(&data.counts) {
        println!("{line}");
    }
    for t in &data.tables {
        match &t.table {
            Ok(table) => println!(
                "table {} {}: {} keys, {:?}",
                t.sha1,
                t.label(),
                table.distinct_keys(),
                table.form
            ),
            Err(e) => println!("table {} {}: not read: {e}", t.sha1, t.label()),
        }
    }
    for f in &data.fonts {
        println!(
            "font {} {}: {} glyphs, decode {:?}, sheet {:?}, error {:?}",
            f.sha1,
            f.paths.join(" | "),
            f.glyphs.len(),
            f.decode,
            f.sheet.as_ref().map(|s| (s.width, s.height)),
            f.error
        );
    }
    if a.report {
        return Ok(());
    }
    let images = data
        .fonts
        .iter_mut()
        .filter_map(|f| {
            let image = sheet_ref(&f.sha1)?;
            let sheet = f.sheet.as_mut()?;
            Some(GooeyImage {
                image,
                width: sheet.width,
                height: sheet.height,
                rgba: std::mem::take(&mut sheet.rgba),
            })
        })
        .collect();
    let query: SharedText = Arc::new(Mutex::new(String::new()));
    let mut state = TextState::new(data, query.clone());
    run_gooey(
        "Strings and fonts",
        Box::new(move |input| text_screen(input, &mut state)),
        images,
        None,
        Some(query),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_preview::text::translation::{
        parse_translation_table, TranslationLimits, TranslationTable,
    };
    use lbp_preview::text::{count, font_sheet, LoadedTable};

    fn table() -> TranslationTable {
        // 30 rows: keys 1..=30, "Text n".
        let mut out = 30u32.to_be_bytes().to_vec();
        let mut blob = Vec::new();
        for k in 1..=30u32 {
            out.extend_from_slice(&k.to_be_bytes());
            out.extend_from_slice(&(blob.len() as u32).to_be_bytes());
            blob.extend_from_slice(&0xFEFFu16.to_be_bytes());
            for u in format!("Text {k}").encode_utf16() {
                blob.extend_from_slice(&u.to_be_bytes());
            }
        }
        out.extend(blob);
        parse_translation_table(&out, TranslationLimits::default()).unwrap()
    }

    fn texts(tree: &UiTree) -> Vec<String> {
        tree.nodes
            .iter()
            .filter_map(|n| match &n.widget {
                lbp_gooey::Widget::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn u3_search_pages_and_tab_switch_clears_query() {
        let tables = vec![LoadedTable {
            sha1: "aa".into(),
            paths: vec!["gamedata/a.trans".into()],
            guids: vec![54800],
            table: Ok(table()),
        }];
        // A font that does not parse still gets a tab and an explicit error.
        let fonts = vec![font_sheet(
            "bb".repeat(20),
            vec!["gamedata/f.fnt".into()],
            b"nope",
        )];
        let counts = count(&tables, &fonts);
        let data = TextData {
            tables,
            fonts,
            counts,
        };
        let query: SharedText = Arc::new(Mutex::new(String::new()));
        let mut state = TextState::new(data, query.clone());
        let tree = text_screen(FrameInput::default(), &mut state);
        assert!(tree.duplicate_ids.is_empty());
        let t = texts(&tree);
        assert!(t.iter().any(|t| t.contains("30 rows")), "{t:?}");
        assert!(t.iter().any(|t| t.starts_with("00000001  Text 1")), "{t:?}");
        // Typed text filters: "Text 2" matches 2 and 20..=29.
        query.lock().unwrap().push_str("text 2");
        let t = texts(&text_screen(FrameInput::default(), &mut state));
        assert!(t.iter().any(|t| t.contains("11 rows")), "{t:?}");
        // A hex key matches exactly that key (and texts containing it).
        *query.lock().unwrap() = "0000001e".into();
        let t = texts(&text_screen(FrameInput::default(), &mut state));
        assert!(
            t.iter().any(|t| t.starts_with("0000001e  Text 30")),
            "{t:?}"
        );
        // Switching to the font tab clears the query and shows the error.
        let font_tab = Uid(TAB_BASE + 1);
        text_screen(
            FrameInput {
                focused: Some(font_tab),
                activated: Some(font_tab),
            },
            &mut state,
        );
        assert_eq!((state.tab, state.query.lock().unwrap().as_str()), (1, ""));
        let tree = text_screen(FrameInput::default(), &mut state);
        assert!(texts(&tree).iter().any(|t| t.starts_with("Stopped: font")));
        // Idle frames build equal trees.
        assert_eq!(text_screen(FrameInput::default(), &mut state), tree);
    }
}
