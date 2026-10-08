//! U7: `--slot-browser`, the story planet / slot browser screen. One tab per disc slot
//! list (distinct content hash), a page of slot badges (icon + translated name), and a
//! details panel for the focused slot. Data: `lbp_preview::story` (renderer-independent).
//! Layout is a browser, not the game's planet: badges are listed in file order, and
//! `Slot.location` is shown as numbers (planet placement is not rendered).
//! Usage: `--slot-browser --snapshot SNAPSHOT --game-dir DIR [--slot-browser-report]
//! [--qa-capture local-analysis/NEW.png]`.
use crate::gooey_ui::{run_gooey, GooeyImage};
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use lbp_gooey::{Align, Builder, FrameInput, ImageRef, Layout, PanelStyle, TextStyle, UiTree, Uid};
use lbp_preview::story::{count_lines, load_story, ResourceKey, Story, StorySlot};
use std::path::PathBuf;

const ROOT: Uid = Uid(0x534c_0000);
const PREV: Uid = Uid(0x534c_0010);
const NEXT: Uid = Uid(0x534c_0011);
const TAB_BASE: u64 = 0x534c_1000;
const SLOT_BASE: u64 = 0x534c_0010_0000;
pub const PAGE: usize = 24;

pub struct BrowserState {
    pub story: Story,
    pub list: usize,
    pub page: usize,
    /// Global slot index of the last focused or activated slot.
    pub selected: Option<usize>,
}

impl BrowserState {
    pub fn new(story: Story) -> Self {
        // Start on the first list that has slots.
        let list = story.lists.iter().position(|l| l.slots > 0).unwrap_or(0);
        Self {
            story,
            list,
            page: 0,
            selected: None,
        }
    }
}

pub(crate) fn image_ref(key: ResourceKey) -> ImageRef {
    match key {
        ResourceKey::Guid(g) => ImageRef::Guid(g),
        ResourceKey::Hash(h) => ImageRef::Hash(h),
    }
}

fn list_label(story: &Story, index: usize) -> String {
    let list = &story.lists[index];
    let stem = list
        .paths
        .first()
        .and_then(|p| p.rsplit(['/', '\\']).next())
        .unwrap_or("?");
    let extra = list.paths.len().saturating_sub(1);
    match (list.status.starts_with("complete"), extra) {
        (true, 0) => format!("{stem} ({})", list.slots),
        (true, n) => format!("{stem} +{n} ({})", list.slots),
        (false, _) => format!("{stem} (not read)"),
    }
}

fn slot_label(s: &StorySlot) -> String {
    if s.initially_locked {
        format!("{} [locked]", s.name)
    } else {
        s.name.clone()
    }
}

fn detail_lines(story: &Story, s: &StorySlot) -> Vec<String> {
    let id = |t: u32, n: u32| {
        let name = lbp_preview::story::SlotId {
            slot_type: t,
            number: n,
        }
        .type_name()
        .unwrap_or("unknown");
        format!("{name} #{n}")
    };
    let reference = |key: Option<ResourceKey>| key.map_or("none".to_owned(), |k| k.label());
    let icon_state = match s.icon {
        None => "none".to_owned(),
        Some(k) if story.icons.contains_key(&k) => format!("{} (shown)", k.label()),
        Some(k) => format!(
            "{} ({})",
            k.label(),
            story
                .icon_failures
                .get(&k)
                .map_or("not decoded", String::as_str)
        ),
    };
    let group = s.group.map_or("none".to_owned(), |g| {
        let target = story
            .find(g)
            .map_or("not in any list".to_owned(), |t| t.name.clone());
        format!("{} = {target}", id(g.slot_type, g.number))
    });
    let opt = |v: Option<bool>| v.map_or("n/a".to_owned(), |b| b.to_string());
    let l = s.location;
    vec![
        format!("{}   ({:?})", s.name, s.name_source),
        format!(
            "{}   list {}   tag {}",
            id(s.id.slot_type, s.id.number),
            story.lists[s.list]
                .paths
                .first()
                .map_or("?", String::as_str),
            s.translation_tag.as_deref().unwrap_or("none")
        ),
        format!(
            "Group {group}   locked at start {}   sub-level {}   on planet {}   level type {}",
            s.initially_locked,
            opt(s.is_sub_level),
            opt(s.show_on_planet),
            s.developer_level_type
                .map_or("n/a".to_owned(), |t| t.to_string())
        ),
        format!(
            "Root level {} ({})   icon {icon_state}",
            reference(s.root),
            if s.root_resolved {
                "resolved"
            } else {
                "not resolved"
            }
        ),
        format!(
            "Planet location [{:.3}, {:.3}, {:.3}, {:.3}]",
            l[0], l[1], l[2], l[3]
        ),
        format!(
            "{}{}",
            if s.description_translated {
                ""
            } else {
                "(stored) "
            },
            s.description
        ),
    ]
}

pub fn browser_screen(input: FrameInput, state: &mut BrowserState) -> UiTree {
    let story = &state.story;
    let mut b = Builder::new(ROOT, input);
    let mut list = state.list;
    let mut page = state.page;
    let mut selected = state.selected;
    let in_list: Vec<usize> = story
        .slots
        .iter()
        .enumerate()
        .filter(|(_, s)| s.list == state.list)
        .map(|(i, _)| i)
        .collect();
    let pages = in_list.len().div_ceil(PAGE).max(1);
    b.frame(
        Uid(0x534c_0001),
        Layout::Column,
        Align::Start,
        PanelStyle::Filled,
        16.,
        8.,
        |b| {
            b.text(
                "Story slot browser (disc slot lists; English tables; not the game's planet view)",
                TextStyle::Title,
            );
            for line in count_lines(story) {
                b.text(line, TextStyle::Small);
            }
            b.frame(
                Uid(0x534c_0002),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Plain,
                0.,
                6.,
                |b| {
                    for i in 0..story.lists.len() {
                        if b.button(Uid(TAB_BASE + i as u64), list_label(story, i))
                            .activated
                        {
                            list = i;
                            page = 0;
                        }
                    }
                },
            );
            b.frame(
                Uid(0x534c_0003),
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
                        format!("Page {}/{pages}, {} slots", state.page + 1, in_list.len()),
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
                Uid(0x534c_0004),
                Layout::Wrap,
                Align::Start,
                PanelStyle::Bubble,
                8.,
                8.,
                |b| {
                    for &i in in_list.iter().skip(state.page * PAGE).take(PAGE) {
                        let s = &story.slots[i];
                        let r = b.button_with(
                            Uid(SLOT_BASE + i as u64),
                            slot_label(s),
                            s.icon.map(image_ref),
                            true,
                        );
                        if r.focused || r.activated {
                            selected = Some(i);
                        }
                    }
                },
            );
            match selected.and_then(|i| story.slots.get(i)) {
                Some(s) => {
                    for line in detail_lines(story, s) {
                        b.text(line, TextStyle::Small);
                    }
                }
                None => b.text(
                    "Arrows or mouse focus a slot to show its details.",
                    TextStyle::Small,
                ),
            }
        },
    );
    let tree = b.finish();
    state.list = list;
    state.page = page.min(pages - 1);
    state.selected = selected;
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
        snapshot: value("--snapshot").ok_or("--slot-browser requires --snapshot")?,
        game: value("--game-dir").ok_or("--slot-browser requires --game-dir")?,
        report: all.iter().any(|a| a == "--slot-browser-report"),
    })
}

pub fn run_slot_browser() -> Result<(), String> {
    let a = args()?;
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(&a.snapshot, &control)?;
    let source = AttachedSource::attach(&a.game, &catalog, &control)?;
    let mut story = load_story(&source, &catalog, &control)?;
    for line in count_lines(&story) {
        println!("{line}");
    }
    for t in &story.tables {
        println!("table g{}: {:?} {}", t.guid, t.state, t.detail);
    }
    for l in &story.lists {
        println!(
            "list {} {}: {} slots, {}",
            l.sha1,
            l.paths.join(" | "),
            l.slots,
            l.status
        );
    }
    let mut failures = std::collections::BTreeMap::<&str, usize>::new();
    for reason in story.icon_failures.values() {
        *failures
            .entry(reason.split(':').next().unwrap_or(""))
            .or_default() += 1;
    }
    for (reason, n) in failures {
        println!("icon failures {reason}: {n}");
    }
    if a.report {
        return Ok(());
    }
    // The GPU takes the pixels; the story keeps each icon's size for the details panel.
    let images = story
        .icons
        .iter_mut()
        .map(|(key, icon)| GooeyImage {
            image: image_ref(*key),
            width: icon.width,
            height: icon.height,
            rgba: std::mem::take(&mut icon.rgba),
        })
        .collect::<Vec<_>>();
    let mut state = BrowserState::new(story);
    run_gooey(
        "Story slot browser",
        Box::new(move |input| browser_screen(input, &mut state)),
        images,
        None,
        None,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_preview::story::SlotId;
    use lbp_preview::story::{NameSource, StoryList};

    fn story(slots: usize) -> Story {
        let mut s = Story::default();
        s.lists.push(StoryList {
            paths: vec!["gamedata/x/story.slt".into()],
            sha1: "aa".into(),
            status: "complete".into(),
            slots,
        });
        for i in 0..slots {
            s.slots.push(StorySlot {
                list: 0,
                index: i,
                id: SlotId {
                    slot_type: 0,
                    number: i as u32 + 1,
                },
                name: format!("Level {}", i + 1),
                name_source: NameSource::Stored,
                description: String::new(),
                description_translated: false,
                translation_tag: None,
                group: None,
                initially_locked: i % 2 == 1,
                is_sub_level: None,
                show_on_planet: None,
                developer_level_type: None,
                location: [0.; 4],
                root: None,
                root_resolved: false,
                icon: Some(ResourceKey::Guid(7)),
            });
        }
        s.counts = lbp_preview::story::count(&s);
        s
    }

    #[test]
    fn u7_browser_pages_and_focus_details() {
        let mut state = BrowserState::new(story(30));
        let tree = browser_screen(FrameInput::default(), &mut state);
        assert!(tree.duplicate_ids.is_empty());
        // 1 tab + Next + 24 slots (Previous disabled on page 1).
        assert_eq!(tree.focusable().len(), 26);
        let slot = Uid(SLOT_BASE + 1);
        let tree = browser_screen(
            FrameInput {
                focused: Some(slot),
                activated: None,
            },
            &mut state,
        );
        assert_eq!(state.selected, Some(1));
        let texts: Vec<_> = tree
            .nodes
            .iter()
            .filter_map(|n| match &n.widget {
                lbp_gooey::Widget::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("DEVELOPER #2")),
            "{texts:?}"
        );
        browser_screen(
            FrameInput {
                focused: Some(NEXT),
                activated: Some(NEXT),
            },
            &mut state,
        );
        assert_eq!(state.page, 1);
        let tree = browser_screen(FrameInput::default(), &mut state);
        // Page 2: 1 tab + Previous + 6 slots (Next disabled).
        assert_eq!(tree.focusable().len(), 8);
        assert!(tree.find(Uid(SLOT_BASE + 29)).is_some());
        // Idle frames build equal trees (no per-frame churn).
        assert_eq!(browser_screen(FrameInput::default(), &mut state), tree);
    }
}
