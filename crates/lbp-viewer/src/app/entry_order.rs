//! UI-only global archive-entry ordering; preserves original (archive, entry) indices.
use super::Explorer;
use lbp_catalog::Catalog;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum EntrySort {
    #[default]
    Table,
    Offset,
    Size,
    Hash,
}
impl EntrySort {
    pub(super) const ALL: [Self; 4] = [Self::Table, Self::Offset, Self::Size, Self::Hash];
    pub(super) fn from_value(value: &str) -> Self {
        match value {
            "offset" => Self::Offset,
            "size" => Self::Size,
            "hash" => Self::Hash,
            _ => Self::Table,
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Table => "Table order",
            Self::Offset => "Offset",
            Self::Size => "Size",
            Self::Hash => "SHA1",
        }
    }
    fn sort(self, catalog: &Catalog, indices: &mut [(usize, usize)]) {
        indices.sort_unstable_by(|&(a, e), &(b, f)| {
            let left = &catalog.snapshot.archives[a].entries[e];
            let right = &catalog.snapshot.archives[b].entries[f];
            let primary = match self {
                Self::Table => Ordering::Equal,
                Self::Offset => left.offset.cmp(&right.offset),
                Self::Size => left.size.cmp(&right.size),
                Self::Hash => left
                    .sha1
                    .bytes()
                    .map(|v| v.to_ascii_lowercase())
                    .cmp(right.sha1.bytes().map(|v| v.to_ascii_lowercase())),
            };
            primary
                .then_with(|| a.cmp(&b))
                .then_with(|| left.entry_index.cmp(&right.entry_index))
                .then_with(|| e.cmp(&f))
        });
    }
}
impl Explorer {
    pub(super) fn refresh_entries(&mut self, catalog: &Catalog) {
        let changed_sort = self.last_entry_sort != Some(self.entry_sort);
        if self.last_filter.as_ref() != Some(&self.filter) || changed_sort {
            let query = self
                .filter
                .query
                .trim()
                .trim_start_matches("hash:")
                .to_ascii_lowercase();
            self.entry_visible = catalog
                .snapshot
                .archives
                .iter()
                .enumerate()
                .flat_map(|(a, archive)| {
                    archive.entries.iter().enumerate().filter_map({
                        let query = &query;
                        let selected_archive = &self.filter.archive;
                        move |(e, entry)| {
                            if selected_archive
                                .as_ref()
                                .is_some_and(|p| p != &archive.path)
                            {
                                return None;
                            }
                            let text =
                                format!("{} {} {}", archive.path, entry.sha1, entry.entry_index)
                                    .to_ascii_lowercase();
                            text.contains(query).then_some((a, e))
                        }
                    })
                })
                .collect();
            self.entry_sort.sort(catalog, &mut self.entry_visible);
            self.last_filter = Some(self.filter.clone());
            self.last_entry_sort = Some(self.entry_sort);
            if changed_sort {
                self.entry_jump = self.entry_selection;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_catalog::{demo_snapshot, LoadControl};
    use serde_json::json;

    fn catalog() -> Catalog {
        let mut snapshot = demo_snapshot();
        let mut archive = serde_json::to_value(&snapshot.archives[0]).unwrap();
        archive["path"] = json!("USRDIR/SECOND.FARC");
        archive["entry_count"] = json!(3);
        archive["entries"] = json!([
            {"entry_index":9,"sha1":"A".repeat(40),"offset":20,"size":10},
            {"entry_index":7,"sha1":"a".repeat(40),"offset":20,"size":10},
            {"entry_index":8,"sha1":"2".repeat(40),"offset":50,"size":5}
        ]);
        snapshot
            .archives
            .push(serde_json::from_value(archive).unwrap());
        snapshot.counts.insert("archives".into(), 2);
        snapshot.counts.insert("archive_entries".into(), 7);
        Catalog::build(snapshot, &LoadControl::default()).unwrap()
    }
    #[test]
    fn global_order_ties_case_folded_hashes_orphans_and_aliases_are_preserved() {
        let catalog = catalog();
        let expected = [
            vec![(0, 0), (0, 1), (0, 2), (0, 3), (1, 1), (1, 2), (1, 0)],
            vec![(0, 0), (1, 1), (1, 0), (1, 2), (0, 1), (0, 2), (0, 3)],
            vec![(1, 2), (1, 1), (1, 0), (0, 0), (0, 1), (0, 2), (0, 3)],
            vec![(0, 0), (0, 1), (1, 2), (0, 2), (0, 3), (1, 1), (1, 0)],
        ];
        for (mode, expected) in EntrySort::ALL.into_iter().zip(expected) {
            let mut indices = expected.clone();
            indices.reverse();
            mode.sort(&catalog, &mut indices);
            assert_eq!(indices, expected);
            mode.sort(&catalog, &mut indices);
            assert_eq!(indices, expected);
            assert_eq!(indices.len(), 7);
        }
        assert!(!catalog.by_hash.contains_key(&"a".repeat(40)));
        assert_eq!(catalog.by_hash[&"2".repeat(40)].len(), 2);
        let mut empty = vec![];
        EntrySort::Offset.sort(&catalog, &mut empty);
    }
    #[test]
    fn sorting_filtering_navigation_and_reload_keep_original_entry_selection() {
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(eframe::egui::Context::default()),
            crate::Options::default(),
        );
        app.set_catalog(catalog(), "Synthetic".into());
        let source = app.catalog.clone().unwrap();
        app.select_entry(1, 0);
        app.entry_sort = EntrySort::Size;
        app.refresh_entries(&source);
        assert_eq!(app.entry_selection, Some((1, 0)));
        assert_eq!(app.entry_jump, Some((1, 0)));
        app.filter.query = "hash:".to_owned() + &"a".repeat(40);
        app.refresh_entries(&source);
        assert_eq!(app.entry_visible, vec![(1, 1), (1, 0)]);
        app.filter.archive = Some("USRDIR/SYNTHETIC.FARC".into());
        app.refresh_entries(&source);
        assert!(app.entry_visible.is_empty());
        assert_eq!(app.entry_selection, Some((1, 0)));
        app.navigate(1);
        assert!(app.travel(true));
        app.refresh_entries(&source);
        assert_eq!(app.entry_selection, Some((1, 0)));
        assert_eq!(app.entry_sort, EntrySort::Size);
        assert_eq!(app.entry_visible.len(), 7);
        app.set_catalog(catalog(), "Reload".into());
        app.refresh_entries(&source);
        assert!(app.entry_selection.is_none());
        assert_eq!(app.entry_sort, EntrySort::Size);
        assert_eq!(app.entry_visible.len(), 7);
    }
}
