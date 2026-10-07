//! Presentation ordering only; catalog rows/aliases and original data are unchanged.
use super::Explorer;
use lbp_catalog::Catalog;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ResourceSort {
    #[default]
    Map,
    Path,
    Guid,
    Hash,
}
impl ResourceSort {
    pub(super) const ALL: [Self; 4] = [Self::Map, Self::Path, Self::Guid, Self::Hash];
    pub(super) fn from_value(value: &str) -> Self {
        match value {
            "path" => Self::Path,
            "guid" => Self::Guid,
            "hash" => Self::Hash,
            _ => Self::Map,
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Map => "MAP row order",
            Self::Path => "Path",
            Self::Guid => "GUID",
            Self::Hash => "SHA1",
        }
    }
    fn sort(self, catalog: &Catalog, indices: &mut [usize]) {
        // Fixed-width validated hexadecimal IDs sort numerically after ASCII folding.
        // No allocation per comparison; paths retain their exact case and byte order.
        fn identifier(a: &str, b: &str) -> Ordering {
            a.bytes()
                .map(|b| b.to_ascii_lowercase())
                .cmp(b.bytes().map(|b| b.to_ascii_lowercase()))
        }
        indices.sort_unstable_by(|&a, &b| {
            let left = &catalog.snapshot.resources[a].map_row;
            let right = &catalog.snapshot.resources[b].map_row;
            let order = match self {
                Self::Map => Ordering::Equal,
                Self::Path => left.path.cmp(&right.path),
                Self::Guid => identifier(&left.guid, &right.guid),
                Self::Hash => identifier(&left.sha1, &right.sha1),
            };
            order
                .then_with(|| left.row_index.cmp(&right.row_index))
                .then_with(|| a.cmp(&b))
        });
    }
}
impl Explorer {
    pub(super) fn refresh_visible(&mut self, catalog: &Catalog) {
        let changed_sort = self.last_sort != Some(self.resource_sort);
        let changed_folder = self.folders.cached_prefix.as_deref() != Some(&self.folders.prefix);
        if self.last_filter.as_ref() != Some(&self.filter) || changed_sort || changed_folder {
            self.visible = catalog.filtered(&self.filter);
            self.folders.rebuild(catalog, &mut self.visible);
            self.resource_sort.sort(catalog, &mut self.visible);
            self.last_filter = Some(self.filter.clone());
            self.last_sort = Some(self.resource_sort);
            if changed_sort {
                self.jump = self.selected;
            } else if changed_folder {
                self.jump = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use lbp_catalog::{demo_snapshot, Filter, LoadControl};

    #[test]
    fn ordering_retains_aliases_and_breaks_case_folded_id_ties_by_map_row() {
        let mut snapshot = demo_snapshot();
        // Duplicate identifier/path aliases with distinct MAP rows remain distinct.
        snapshot.resources[0].map_row.path = "synthetic/z".into();
        snapshot.resources[1].map_row.path = "synthetic/A".into();
        snapshot.resources[2].map_row.path = "synthetic/A".into();
        snapshot.resources[1].map_row.guid = "00000aAb".into();
        snapshot.resources[2].map_row.guid = "00000AAB".into();
        snapshot.resources[1].map_row.row_index = 90;
        snapshot.resources[2].map_row.row_index = 80;
        for link in &mut snapshot.dependency_links {
            for row in link
                .guid_map_row_indices
                .iter_mut()
                .chain(&mut link.sha1_map_row_indices)
            {
                *row = match *row {
                    1 => 90,
                    2 => 80,
                    other => other,
                };
            }
        }
        let catalog = Catalog::build(snapshot, &LoadControl::default()).unwrap();
        for mode in ResourceSort::ALL {
            let mut indices: Vec<_> = (0..6).rev().collect();
            mode.sort(&catalog, &mut indices);
            let once = indices.clone();
            indices.reverse();
            mode.sort(&catalog, &mut indices);
            assert_eq!(indices, once);
            let mut membership = indices.clone();
            membership.sort_unstable();
            assert_eq!(membership, (0..6).collect::<Vec<_>>());
            assert!(
                indices.iter().position(|i| *i == 2).unwrap()
                    < indices.iter().position(|i| *i == 1).unwrap()
            );
            if mode == ResourceSort::Path {
                assert_eq!(&indices[..2], &[2, 1]);
            }
        }
        let mut empty = vec![];
        ResourceSort::Hash.sort(&catalog, &mut empty);
    }
    #[test]
    fn filtering_sorting_navigation_and_reload_keep_catalog_indices() {
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(eframe::egui::Context::default()),
            Options {
                demo: true,
                ..Default::default()
            },
        );
        // Explorer owns options; no original data or source attachment in this test.
        let catalog = app.catalog.clone().unwrap_or_else(|| {
            std::sync::Arc::new(Catalog::build(demo_snapshot(), &LoadControl::default()).unwrap())
        });
        app.set_catalog(
            Catalog::build(demo_snapshot(), &LoadControl::default()).unwrap(),
            "Synthetic".into(),
        );
        app.select(2, true);
        app.resource_sort = ResourceSort::Path;
        app.refresh_visible(&catalog);
        assert_eq!(app.selected, Some(2));
        assert_eq!(app.jump, Some(2));
        let ordered = app.visible.clone();
        assert_eq!(&ordered[..2], &[1, 2]);
        app.filter = Filter {
            query: "checker".into(),
            ..Default::default()
        };
        app.refresh_visible(&catalog);
        assert_eq!(app.visible, vec![1, 2]);
        app.filter.query = "no match".into();
        app.refresh_visible(&catalog);
        assert!(app.visible.is_empty());
        assert_eq!(app.selected, Some(2));
        app.navigate(0); // Navigation clears hiding filters, not the chosen sort.
        app.refresh_visible(&catalog);
        assert_eq!(app.resource_sort, ResourceSort::Path);
        assert_eq!(app.visible, ordered);
        assert!(app.travel(true));
        app.refresh_visible(&catalog);
        assert_eq!(app.selected, Some(2));
        assert_eq!(app.visible, ordered);
        app.set_catalog(
            Catalog::build(demo_snapshot(), &LoadControl::default()).unwrap(),
            "Reload".into(),
        );
        app.refresh_visible(&catalog);
        assert!(app.selected.is_none());
        assert_eq!(app.resource_sort, ResourceSort::Path);
        assert_eq!(app.visible, ordered);
    }
}
