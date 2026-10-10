//! Level selector data: every magic-confirmed level MAP row, grouped by the folder it
//! lives in, named from the story slot lists where a slot's root level resolves to it.
//! Renderer-independent; the viewer only displays these entries.
use crate::story::{ResourceKey, Story};
use lbp_catalog::Catalog;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelEntry {
    /// MAP path as stored (the selector loads by this path).
    pub path: String,
    pub guid: Option<u32>,
    /// Parent folder of the path (lower-case, `/` separators); "(root)" when none.
    pub group: String,
    /// Display name: the in-game slot name when a slot resolves to this level, else the
    /// file stem.
    pub name: String,
    /// True when `name` came from a story slot.
    pub named: bool,
}

impl LevelEntry {
    /// Case-insensitive match of `query` against the name, path and hex GUID.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        q.is_empty()
            || self.name.to_ascii_lowercase().contains(&q)
            || self.path.to_ascii_lowercase().contains(&q)
            || self.guid.is_some_and(|g| format!("{g:08x}").contains(&q))
    }
}

fn split_path(path: &str) -> (String, String) {
    let norm = path.replace('\\', "/");
    match norm.rsplit_once('/') {
        Some((dir, file)) => (dir.to_ascii_lowercase(), file.to_string()),
        None => ("(root)".into(), norm),
    }
}

fn stem(file: &str) -> String {
    file.rsplit_once('.')
        .map_or(file, |(stem, _)| stem)
        .to_string()
}

/// Pure core: `rows` are `(path, guid hex)` of the level MAP rows; `names` maps a level
/// GUID to its in-game name. Duplicate paths collapse; order is group, then name, then path.
pub fn entries_from_rows<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str)>,
    names: &BTreeMap<u32, String>,
) -> Vec<LevelEntry> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (path, guid) in rows {
        if !seen.insert(path.replace('\\', "/").to_ascii_lowercase()) {
            continue;
        }
        let guid = u32::from_str_radix(guid.trim(), 16).ok();
        let (group, file) = split_path(path);
        let slot_name = guid
            .and_then(|g| names.get(&g))
            .filter(|n| !n.trim().is_empty());
        out.push(LevelEntry {
            path: path.to_string(),
            guid,
            group,
            name: slot_name.cloned().unwrap_or_else(|| stem(&file)),
            named: slot_name.is_some(),
        });
    }
    out.sort_by(|a, b| {
        (&a.group, a.name.to_ascii_lowercase(), &a.path).cmp(&(
            &b.group,
            b.name.to_ascii_lowercase(),
            &b.path,
        ))
    });
    out
}

/// GUID -> first slot name (list order) for slots whose root is a GUID.
pub fn slot_names(story: &Story) -> BTreeMap<u32, String> {
    let mut names = BTreeMap::new();
    for slot in &story.slots {
        if let Some(ResourceKey::Guid(g)) = slot.root {
            names.entry(g).or_insert_with(|| slot.name.clone());
        }
    }
    names
}

/// Every level in the catalog. `story` adds in-game names when it is available.
pub fn list_levels(catalog: &Catalog, story: Option<&Story>) -> Vec<LevelEntry> {
    let names = story.map(slot_names).unwrap_or_default();
    entries_from_rows(
        catalog
            .snapshot
            .resources
            .iter()
            .filter(|r| r.display_type() == "level")
            .map(|r| (r.map_row.path.as_str(), r.map_row.guid.as_str())),
        &names,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_names_and_dedups() {
        let mut names = BTreeMap::new();
        names.insert(0x1db4f, "Basic Training".to_string());
        let rows = [
            ("lbp2/th_tutorials/levels/tutorial_23.bin", "0001db4f"),
            ("lbp2/th_tutorials/levels/Tutorial_23.bin", "0001db4f"),
            ("lbp2/th_tutorials/levels/tutorial_1.bin", "00000010"),
            ("lonely.bin", "zz"),
        ];
        let e = entries_from_rows(rows, &names);
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].group, "(root)");
        assert_eq!(e[0].guid, None);
        assert_eq!(e[0].name, "lonely");
        let named = e.iter().find(|x| x.guid == Some(0x1db4f)).unwrap();
        assert!(named.named);
        assert_eq!(named.name, "Basic Training");
        assert_eq!(named.group, "lbp2/th_tutorials/levels");
        let plain = e.iter().find(|x| x.guid == Some(0x10)).unwrap();
        assert!(!plain.named);
        assert_eq!(plain.name, "tutorial_1");
    }

    #[test]
    fn search_hits_name_path_and_guid() {
        let e = entries_from_rows(
            [("a/b/tutorial_23.bin", "0001db4f")],
            &BTreeMap::from([(0x1db4f, "Basic Training".to_string())]),
        );
        assert!(e[0].matches("basic"));
        assert!(e[0].matches("TUTORIAL_23"));
        assert!(e[0].matches("1DB4"));
        assert!(!e[0].matches("zzz"));
        assert!(e[0].matches("  "));
    }
}
