//! U8: `--popit`, a Popit-style inventory browser over the disc plans. One tab per
//! inventory type (first type bit), a page of item cards (icon + translated title) and a
//! details panel for the focused item. Icons are decoded when a page shows them. Data:
//! `lbp_preview::popit`. This is an explorer view, not the game's Popit (whose layout is
//! script-driven).
//! Usage: `--popit --snapshot SNAPSHOT --game-dir DIR [--popit-report [--popit-decode-icons]]
//! [--qa-capture local-analysis/NEW.png]`.
use crate::gooey_ui::{run_gooey, GooeyImage};
use crate::slot_browser::image_ref;
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_gooey::{Align, Builder, FrameInput, ImageRef, Layout, PanelStyle, TextStyle, UiTree, Uid};
use lbp_preview::popit::{count_lines, load_popit, Popit, PopitItem};
use lbp_preview::story::{decode_icon, ResourceKey};
use std::path::PathBuf;
use std::sync::Arc;

const ROOT: Uid = Uid(0x5050_0000);
const PREV: Uid = Uid(0x5050_0010);
const NEXT: Uid = Uid(0x5050_0011);
const TAB_BASE: u64 = 0x5050_1000;
const ITEM_BASE: u64 = 0x5050_0010_0000;
pub const PAGE: usize = 24;

pub struct PopitState {
    pub popit: Popit,
    pub page_names: Vec<String>,
    pub tab: usize,
    pub page: usize,
    /// Item index of the last focused or activated card.
    pub selected: Option<usize>,
}

impl PopitState {
    pub fn new(popit: Popit) -> Self {
        let page_names = popit.pages.keys().cloned().collect();
        Self {
            popit,
            page_names,
            tab: 0,
            page: 0,
            selected: None,
        }
    }
}

fn detail_lines(item: &PopitItem) -> Vec<String> {
    let opt = |v: &Option<String>| v.clone().unwrap_or_else(|| "none".to_owned());
    vec![
        format!("{}   ({:?})", item.title, item.title_source),
        format!(
            "Types {}   subType {}   head 0x{:x}",
            item.types.join(", "),
            item.sub_type,
            item.head
        ),
        format!(
            "Location {}   category {}   creator {}",
            opt(&item.location),
            opt(&item.category),
            opt(&item.creator)
        ),
        format!(
            "Icon {} ({})   plan {}",
            item.icon.map_or("none".to_owned(), |k| k.label()),
            if item.icon_resolved {
                "resolved"
            } else {
                "not resolved"
            },
            item.paths.first().map_or("?", String::as_str)
        ),
        item.description.clone().unwrap_or_default(),
    ]
}

pub fn popit_screen(input: FrameInput, state: &mut PopitState) -> UiTree {
    let popit = &state.popit;
    let mut b = Builder::new(ROOT, input);
    let (mut tab, mut page, mut selected) = (state.tab, state.page, state.selected);
    let empty = Vec::new();
    let items = state
        .page_names
        .get(state.tab)
        .and_then(|name| popit.pages.get(name))
        .unwrap_or(&empty);
    let pages = items.len().div_ceil(PAGE).max(1);
    b.frame(
        Uid(0x5050_0001),
        Layout::Column,
        Align::Start,
        PanelStyle::Filled,
        16.,
        8.,
        |b| {
            b.text(
                "Inventory browser (disc plans by type; English tables; not the game's Popit)",
                TextStyle::Title,
            );
            for line in count_lines(popit) {
                b.text(line, TextStyle::Small);
            }
            b.frame(
                Uid(0x5050_0002),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Plain,
                0.,
                6.,
                |b| {
                    for (i, name) in state.page_names.iter().enumerate() {
                        let n = popit.pages.get(name).map_or(0, Vec::len);
                        if b.button(Uid(TAB_BASE + i as u64), format!("{name} ({n})"))
                            .activated
                        {
                            tab = i;
                            page = 0;
                        }
                    }
                },
            );
            b.frame(
                Uid(0x5050_0003),
                Layout::Row,
                Align::Start,
                PanelStyle::Plain,
                0.,
                8.,
                |b| {
                    if b.button_with(PREV, "Previous", None, state.page > 0)
                        .activated
                    {
                        page = state.page - 1;
                    }
                    b.text(
                        format!("Page {}/{pages}, {} items", state.page + 1, items.len()),
                        TextStyle::Body,
                    );
                    if b.button_with(NEXT, "Next", None, state.page + 1 < pages)
                        .activated
                    {
                        page = state.page + 1;
                    }
                },
            );
            b.frame(
                Uid(0x5050_0004),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Bubble,
                8.,
                8.,
                |b| {
                    for &i in items.iter().skip(state.page * PAGE).take(PAGE) {
                        let item = &popit.items[i];
                        let r = b.button_with(
                            Uid(ITEM_BASE + i as u64),
                            item.title.clone(),
                            item.icon.filter(|_| item.icon_resolved).map(image_ref),
                            true,
                        );
                        if r.focused || r.activated {
                            selected = Some(i);
                        }
                    }
                },
            );
            match selected.and_then(|i| popit.items.get(i)) {
                Some(item) => {
                    for line in detail_lines(item) {
                        b.text(line, TextStyle::Small);
                    }
                }
                None => b.text(
                    "Arrows or mouse focus an item to show its details.",
                    TextStyle::Small,
                ),
            }
        },
    );
    let tree = b.finish();
    if tab != state.tab {
        // A new tab starts on its first page with nothing selected.
        selected = None;
        page = 0;
    }
    state.tab = tab;
    state.page = page.min(pages - 1);
    state.selected = selected;
    tree
}

struct Args {
    snapshot: PathBuf,
    game: PathBuf,
    report: bool,
    decode_icons: bool,
}

fn args() -> Result<Args, String> {
    let all: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        all.iter()
            .position(|a| a == flag)
            .and_then(|i| all.get(i + 1))
            .map(PathBuf::from)
    };
    let has = |flag: &str| all.iter().any(|a| a == flag);
    Ok(Args {
        snapshot: value("--snapshot").ok_or("--popit requires --snapshot")?,
        game: value("--game-dir").ok_or("--popit requires --game-dir")?,
        report: has("--popit-report"),
        decode_icons: has("--popit-decode-icons"),
    })
}

fn key_of(image: ImageRef) -> Option<ResourceKey> {
    match image {
        ImageRef::Guid(g) => Some(ResourceKey::Guid(g)),
        ImageRef::Hash(h) => Some(ResourceKey::Hash(h)),
        ImageRef::Swatch(_) => None,
    }
}

pub fn run_popit() -> Result<(), String> {
    let a = args()?;
    let control = LoadControl::default();
    let catalog = Arc::new(lbp_catalog::load(&a.snapshot, &control)?);
    let source = Arc::new(AttachedSource::attach(&a.game, &catalog, &control)?);
    let popit = load_popit(&source, &catalog, &control)?;
    for line in count_lines(&popit) {
        println!("{line}");
    }
    for t in &popit.tables {
        println!("table g{}: {:?} {}", t.guid, t.state, t.detail);
    }
    for (page, n) in &popit.counts.by_page {
        println!("page {page}: {n}");
    }
    for (reason, n) in &popit.counts.not_read {
        println!("not read {reason}: {n}");
    }
    if a.report {
        if a.decode_icons {
            // Every distinct resolved icon, decoded and dropped: the measured icon count.
            let icons: std::collections::BTreeSet<ResourceKey> = popit
                .items
                .iter()
                .filter(|i| i.icon_resolved)
                .filter_map(|i| i.icon)
                .collect();
            let mut failures = std::collections::BTreeMap::<String, usize>::new();
            let mut decoded = 0;
            for key in &icons {
                control.check()?;
                match decode_icon(&source, &catalog, *key, &control) {
                    Ok(_) => decoded += 1,
                    Err(e) => *failures.entry(e).or_default() += 1,
                }
            }
            let shown = popit
                .items
                .iter()
                .filter(|i| i.icon_resolved && i.icon.is_some())
                .count();
            println!(
                "Icons decoded {decoded}/{} distinct resolved icons ({shown} items reference them)",
                icons.len()
            );
            for (reason, n) in failures {
                println!("icon failure {reason}: {n}");
            }
        }
        return Ok(());
    }
    let loader = {
        let (source, catalog, control) = (source.clone(), catalog.clone(), control.clone());
        Box::new(move |image: ImageRef| {
            let icon = decode_icon(&source, &catalog, key_of(image)?, &control).ok()?;
            Some(GooeyImage {
                image,
                width: icon.width,
                height: icon.height,
                rgba: icon.rgba,
            })
        })
    };
    let mut state = PopitState::new(popit);
    run_gooey(
        "Inventory browser",
        Box::new(move |input| popit_screen(input, &mut state)),
        Vec::new(),
        Some(loader),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_preview::popit::TitleSource;

    fn popit(n: usize) -> Popit {
        let mut p = Popit::default();
        for i in 0..n {
            p.items.push(PopitItem {
                sha1: format!("{i:040x}"),
                paths: vec![format!("gamedata/p{i}.plan")],
                guids: Vec::new(),
                head: 0x3f8,
                title: format!("Item {i}"),
                title_source: TitleSource::Translated,
                description: None,
                types: vec![if i < 30 { "material" } else { "sticker" }],
                page: if i < 30 { "material" } else { "sticker" }.into(),
                sub_type: 0,
                location: None,
                category: None,
                creator: None,
                icon: Some(ResourceKey::Guid(100 + i as u32)),
                icon_resolved: i % 2 == 0,
            });
        }
        for (i, item) in p.items.iter().enumerate() {
            p.pages.entry(item.page.clone()).or_default().push(i);
        }
        p.counts.items = n;
        p
    }

    #[test]
    fn u8_tabs_pages_and_icons_only_for_resolved_items() {
        let mut state = PopitState::new(popit(33));
        let tree = browser(&mut state, None);
        assert!(tree.duplicate_ids.is_empty());
        // 2 tabs + Next + 24 cards on the 30-item material page.
        assert_eq!(tree.focusable().len(), 27);
        let images = tree
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.widget,
                    lbp_gooey::Widget::Button {
                        image: Some(ImageRef::Guid(_)),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(images, 12);
        browser(&mut state, Some(NEXT));
        assert_eq!(state.page, 1);
        // Switching tab resets the page and selection.
        browser(&mut state, Some(Uid(TAB_BASE + 1)));
        assert_eq!((state.tab, state.page, state.selected), (1, 0, None));
        let tree = browser(&mut state, None);
        assert!(tree.find(Uid(ITEM_BASE + 30)).is_some());
        browser(&mut state, Some(Uid(ITEM_BASE + 31)));
        assert_eq!(state.selected, Some(31));
        // Idle frames build equal trees.
        let tree = browser(&mut state, None);
        assert_eq!(browser(&mut state, None), tree);
    }

    fn browser(state: &mut PopitState, press: Option<Uid>) -> UiTree {
        popit_screen(
            FrameInput {
                focused: press,
                activated: press,
            },
            state,
        )
    }
}
