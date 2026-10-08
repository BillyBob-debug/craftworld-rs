//! U8: Popit-style inventory data. Reads the details of every magic-confirmed plan (U5),
//! names each item through the English tables (U1) and groups items into pages by
//! inventory type. Renderer-independent; icons are decoded on demand by the caller
//! (`story::decode_icon`), since 10k+ icons do not fit an eager budget.
//!
//! Rules (independent notes, ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! lib/cwlib/src/main/java/cwlib):
//! - Title/description: `InventoryItemDetails.updateTranslations`: a non-empty translation
//!   tag replaces the keys with `lams(tag + "_NAME")` / `lams(tag + "_DESC")`; non-zero
//!   keys are then looked up. CONFIRMED_FROM_COMMUNITY_CODE.
//! - When no translation exists the browser shows, in order, the user-created name, the
//!   tag, then the first MAP path's file stem (browser fallback, not a game rule).
//! - Page = first name of the item's type bits (`InventoryObjectType` with the game-version
//!   filter, U5 `type_names`). The game's own Popit page layout is script-driven and not
//!   reproduced; this grouping is an explorer choice.
//! - RPlan location/category keys are looked up in the same tables. That they are LAMS
//!   keys of page/category names is STRONG_INFERENCE (Toolkit stores them as u32 keys and,
//!   for profiles, resolves indices through the profile string table); counted.
use crate::level_scene::read_resource;
use crate::story::{load_strings, resolve_key, ResourceKey, Strings, TableStatus};
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_level::item_details::{self, type_names, ItemDetails, Placement};
use lbp_level::Limits;
use lbp_resource::translation::lams_key;
use std::collections::{BTreeMap, BTreeSet};

/// Plans read per load and their total stored bytes.
pub const MAX_PLANS: usize = 50_000;
pub const MAX_TOTAL_BYTES: u64 = 2 << 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TitleSource {
    Translated,
    UserCreated,
    TagOnly,
    /// Title key present, no string for it.
    KeyUntranslated,
    /// Nothing to show: the MAP path stem.
    Path,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopitItem {
    pub sha1: String,
    /// Sorted MAP paths and GUIDs sharing this content hash.
    pub paths: Vec<String>,
    pub guids: Vec<String>,
    pub head: u32,
    pub title: String,
    pub title_source: TitleSource,
    pub description: Option<String>,
    pub types: Vec<&'static str>,
    pub page: String,
    pub sub_type: u32,
    pub location: Option<String>,
    pub category: Option<String>,
    pub creator: Option<String>,
    pub icon: Option<ResourceKey>,
    pub icon_resolved: bool,
}

/// Plans that did not give an item, by reason.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopitCounts {
    pub plans: usize,
    pub items: usize,
    pub not_read: BTreeMap<String, usize>,
    pub by_page: BTreeMap<String, usize>,
    pub by_title: BTreeMap<String, usize>,
    pub descriptions: usize,
    pub with_icon: usize,
    pub icon_resolved: usize,
    pub placement_keys: usize,
    pub placement_translated: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Popit {
    pub items: Vec<PopitItem>,
    /// Page name -> item indices, items sorted by category then title.
    pub pages: BTreeMap<String, Vec<usize>>,
    pub tables: Vec<TableStatus>,
    pub counts: PopitCounts,
}

/// Title and description per `updateTranslations`, with the browser fallbacks.
pub fn item_text(
    d: &ItemDetails,
    strings: &Strings,
    path_stem: &str,
) -> (String, TitleSource, Option<String>) {
    let tag = d.translation_tag.as_deref().filter(|t| !t.is_empty());
    let (title_key, description_key) = match tag {
        Some(tag) => (
            Some(lams_key(&format!("{tag}_NAME"))),
            Some(lams_key(&format!("{tag}_DESC"))),
        ),
        None => (d.title_key, d.description_key),
    };
    let look = |k: Option<u32>| k.filter(|k| *k != 0).and_then(|k| strings.get(k));
    let description = look(description_key).map(str::to_owned);
    if let Some(t) = look(title_key) {
        return (t.to_owned(), TitleSource::Translated, description);
    }
    if let Some(u) = d.user_created.value().filter(|u| !u.name.is_empty()) {
        let description = description.or(Some(u.description.clone()).filter(|s| !s.is_empty()));
        return (u.name.clone(), TitleSource::UserCreated, description);
    }
    if let Some(tag) = tag {
        return (tag.to_owned(), TitleSource::TagOnly, description);
    }
    let source = if title_key.is_some_and(|k| k != 0) {
        TitleSource::KeyUntranslated
    } else {
        TitleSource::Path
    };
    (path_stem.to_owned(), source, description)
}

/// One plan as read: content hash, MAP paths/GUIDs, and the details or why not.
pub struct PlanInput {
    pub sha1: String,
    pub paths: Vec<String>,
    pub guids: Vec<String>,
    pub head: u32,
    pub result: Result<(ItemDetails, Option<Placement>), String>,
}

pub fn build_popit(
    plans: Vec<PlanInput>,
    strings: &Strings,
    resolves: &dyn Fn(ResourceKey) -> bool,
) -> Popit {
    let mut popit = Popit::default();
    let mut c = PopitCounts {
        plans: plans.len(),
        ..Default::default()
    };
    for plan in plans {
        let (d, placement) = match plan.result {
            Ok(v) => v,
            Err(reason) => {
                *c.not_read.entry(reason).or_default() += 1;
                continue;
            }
        };
        let stem = plan
            .paths
            .first()
            .and_then(|p| p.rsplit(['/', '\\']).next())
            .unwrap_or("?")
            .to_owned();
        let (title, title_source, description) = item_text(&d, strings, &stem);
        let (location, category) = match &placement {
            Some(Placement::Keys { location, category }) => {
                c.placement_keys += 1;
                let l = strings.get(*location).map(str::to_owned);
                let k = strings.get(*category).map(str::to_owned);
                c.placement_translated += usize::from(l.is_some() && k.is_some());
                (l, k)
            }
            Some(Placement::Tags { location, category }) => (
                Some(location.clone()).filter(|s| !s.is_empty()),
                Some(category.clone()).filter(|s| !s.is_empty()),
            ),
            None => (None, None),
        };
        let types = type_names(d.type_bits, plan.head & 0xffff);
        let page = types.first().copied().unwrap_or("untyped").to_owned();
        let icon = d.icon.as_ref().and_then(ResourceKey::of);
        let item = PopitItem {
            sha1: plan.sha1,
            paths: plan.paths,
            guids: plan.guids,
            head: plan.head,
            title,
            title_source,
            description,
            types,
            page,
            sub_type: d.sub_type,
            location,
            category,
            creator: d.creator.value().filter(|c| !c.is_empty()).cloned(),
            icon,
            icon_resolved: icon.is_some_and(resolves),
        };
        *c.by_page.entry(item.page.clone()).or_default() += 1;
        *c.by_title
            .entry(format!("{:?}", item.title_source))
            .or_default() += 1;
        c.descriptions += usize::from(item.description.is_some());
        c.with_icon += usize::from(item.icon.is_some());
        c.icon_resolved += usize::from(item.icon_resolved);
        popit.items.push(item);
    }
    c.items = popit.items.len();
    for (i, item) in popit.items.iter().enumerate() {
        popit.pages.entry(item.page.clone()).or_default().push(i);
    }
    for list in popit.pages.values_mut() {
        list.sort_by(|&a, &b| {
            let (a, b) = (&popit.items[a], &popit.items[b]);
            (&a.category, &a.title, &a.sha1).cmp(&(&b.category, &b.title, &b.sha1))
        });
    }
    popit.counts = c;
    popit
}

pub fn count_lines(popit: &Popit) -> Vec<String> {
    let c = &popit.counts;
    let not_read: usize = c.not_read.values().sum();
    let tables = popit
        .tables
        .iter()
        .map(|t| format!("g{} {:?}", t.guid, t.state))
        .collect::<Vec<_>>()
        .join(", ");
    let title = |k: &str| c.by_title.get(k).copied().unwrap_or(0);
    vec![
        format!(
            "Items {}/{} plans (unit: distinct content hash; {not_read} not read); {} pages",
            c.items,
            c.plans,
            popit.pages.len()
        ),
        format!(
            "Titles translated {}/{} (user-created {}, tag only {}, key untranslated {}, path {}); descriptions {}",
            title("Translated"),
            c.items,
            title("UserCreated"),
            title("TagOnly"),
            title("KeyUntranslated"),
            title("Path"),
            c.descriptions
        ),
        format!(
            "Icons resolved {}/{} items with an icon (decoded per page on view); placement keys translated {}/{} (inferred)",
            c.icon_resolved, c.with_icon, c.placement_translated, c.placement_keys
        ),
        format!(
            "Tables: {}",
            if tables.is_empty() { "none".into() } else { tables }
        ),
    ]
}

/// Reads every magic-confirmed plan's details and the English tables.
pub fn load_popit(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<Popit, String> {
    let (strings, tables) = load_strings(source, catalog, control);
    let mut by_hash: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "plan" {
            let e = by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default();
            e.0.insert(r.map_row.path.clone());
            e.1.insert(r.map_row.guid.to_ascii_lowercase());
        }
    }
    if by_hash.len() > MAX_PLANS {
        return Err(format!("Budget: {} plans", by_hash.len()));
    }
    let mut plans = Vec::with_capacity(by_hash.len());
    let mut total = 0u64;
    for (sha1, (paths, guids)) in by_hash {
        control.check()?;
        let mut head = 0;
        let result = crate::resolve(catalog, None, Some(&sha1))
            .and_then(|key| {
                total += key.size;
                if total > MAX_TOTAL_BYTES {
                    return Err("Budget: total plan bytes".to_owned());
                }
                read_resource(source, &key, control, item_details::MAX_INPUT)
            })
            .and_then(|bytes| {
                let report = item_details::read_plan_details(&bytes, Limits::default(), &|| {
                    control.check().is_err()
                })
                .map_err(|e| format!("wrapper: {e}"))?;
                head = report.revision.head;
                let p = report
                    .result
                    .map_err(|stop| format!("stopped: {}", stop.label()))?;
                match p.details {
                    Some(d) => Ok((d, p.placement)),
                    None => Err("no details (head below 0x197)".to_owned()),
                }
            });
        plans.push(PlanInput {
            sha1,
            paths: paths.into_iter().collect(),
            guids: guids.into_iter().collect(),
            head,
            result,
        });
    }
    let resolves = |key| resolve_key(catalog, key).is_ok();
    let mut popit = build_popit(plans, &strings, &resolves);
    popit.tables = tables;
    Ok(popit)
}
