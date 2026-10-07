//! Literal MAP metadata prefixes only; never filesystem paths or source reads.
use super::Explorer;
use eframe::egui;
use lbp_catalog::Catalog;
use std::collections::BTreeMap;
const MAX_PREFIX_BYTES: usize = 4096;
const MAX_DEPTH: usize = 32;

fn ordinary(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
pub(super) fn prefix(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let body = value.strip_suffix('/').unwrap_or(value);
    if !ordinary(body) || body.len() >= MAX_PREFIX_BYTES || body.split('/').count() > MAX_DEPTH {
        return Err(
            "Folder needs a literal relative MAP prefix (at most 4096 bytes / 32 components)"
                .into(),
        );
    }
    Ok(format!("{body}/"))
}
#[derive(Default)]
pub(super) struct Folders {
    pub(super) prefix: String,
    pub(super) cached_prefix: Option<String>,
    pub(super) children: Vec<(String, usize)>,
    pub(super) direct: usize,
    pub(super) ungrouped: usize,
}
impl Folders {
    pub(super) fn rebuild(&mut self, catalog: &Catalog, indices: &mut Vec<usize>) {
        let mut children = BTreeMap::new();
        self.direct = 0;
        self.ungrouped = 0;
        indices.retain(|&index| {
            let path = &catalog.snapshot.resources[index].map_row.path;
            let valid = ordinary(path);
            if !self.prefix.is_empty() && (!valid || !path.starts_with(&self.prefix)) {
                return false;
            }
            if !valid {
                self.ungrouped += 1;
                return true;
            }
            let tail = &path[self.prefix.len()..];
            if let Some((child, _)) = tail.split_once('/') {
                if self
                    .prefix
                    .len()
                    .checked_add(child.len())
                    .and_then(|n| n.checked_add(1))
                    .is_none_or(|n| n > MAX_PREFIX_BYTES)
                {
                    self.ungrouped += 1;
                    return true;
                }
                let candidate = format!("{}{child}/", self.prefix);
                if prefix(&candidate).is_ok() {
                    *children.entry(candidate).or_insert(0) += 1;
                } else {
                    self.ungrouped += 1;
                }
            } else {
                self.direct += 1;
            }
            true
        });
        self.children = children.into_iter().collect();
        self.cached_prefix = Some(self.prefix.clone());
    }
    fn parent(&self) -> String {
        self.prefix
            .trim_end_matches('/')
            .rsplit_once('/')
            .map_or_else(String::new, |(parent, _)| format!("{parent}/"))
    }
}
impl Explorer {
    pub(super) fn folder_ui(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.folders.prefix.is_empty(), egui::Button::new("Root"))
                .clicked()
            {
                self.folders.prefix.clear();
            }
            if ui
                .add_enabled(!self.folders.prefix.is_empty(), egui::Button::new("Up"))
                .clicked()
            {
                self.folders.prefix = self.folders.parent();
            }
            let label = if self.folders.prefix.is_empty() {
                "MAP folders"
            } else {
                &self.folders.prefix
            };
            ui.add(egui::Label::new(label).truncate())
                .on_hover_text(label);
        });
        self.refresh_visible(catalog);
        egui::CollapsingHeader::new("Child folders")
            .default_open(true)
            .show(ui, |ui| {
                ui.small("Counts follow filters; the resource list includes subfolders.");
                if self.folders.children.is_empty() {
                    ui.small("No child folders match.");
                }
                let mut chosen = None;
                egui::ScrollArea::vertical()
                    .id_salt("map_folders")
                    .max_height(110.0)
                    .show_rows(ui, 22.0, self.folders.children.len(), |ui, range| {
                        for index in range {
                            let (path, count) = &self.folders.children[index];
                            let name = path.strip_prefix(&self.folders.prefix).unwrap_or(path);
                            if ui
                                .add(egui::Button::new(format!("{name} ({count} rows)")).truncate())
                                .clicked()
                            {
                                chosen = Some(path.clone());
                            }
                        }
                    });
                ui.small(format!(
                    "{} direct rows · {} ungrouped rows",
                    self.folders.direct, self.folders.ungrouped
                ));
                if let Some(path) = chosen {
                    self.folders.prefix = path;
                }
            });
        self.refresh_visible(catalog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_catalog::{demo_snapshot, LoadControl};
    fn catalog() -> Catalog {
        let mut snapshot = demo_snapshot();
        for (row, path) in snapshot.resources.iter_mut().zip([
            "synthetic/a/item",
            "synthetic/a/same",
            "synthetic/a/same",
            "synthetic/ab/other",
            "../unusual",
            "root_file",
        ]) {
            row.map_row.path = path.into();
        }
        Catalog::build(snapshot, &LoadControl::default()).unwrap()
    }
    #[test]
    fn prefix_boundaries_alias_counts_and_unusual_root_rows_are_lossless() {
        let catalog = catalog();
        let mut state = Folders::default();
        let mut rows = (0..6).collect();
        state.rebuild(&catalog, &mut rows);
        assert_eq!(rows, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(state.children, vec![("synthetic/".into(), 4)]);
        assert_eq!((state.direct, state.ungrouped), (1, 1));
        state.prefix = prefix("synthetic").unwrap();
        state.rebuild(&catalog, &mut rows);
        assert_eq!(
            state.children,
            vec![("synthetic/a/".into(), 3), ("synthetic/ab/".into(), 1)]
        );
        state.prefix = prefix("synthetic/a/").unwrap();
        state.rebuild(&catalog, &mut rows);
        assert_eq!(rows, vec![0, 1, 2]);
        assert_eq!(state.direct, 3);
        assert!(state.children.is_empty());
        assert_eq!(state.parent(), "synthetic/");
        assert!(catalog.by_hash.values().any(|aliases| aliases.len() == 2));
    }
    #[test]
    fn prefix_limits_case_and_invalid_components_are_explicit() {
        for value in [
            "/absolute",
            "a//b",
            "a/../b",
            "a/./b",
            "C:/data",
            "a\\b",
            "a\0b",
            "a\nb",
        ] {
            assert!(prefix(value).is_err(), "{value:?}");
        }
        assert_eq!(prefix("Case/子").unwrap(), "Case/子/");
        assert_eq!(prefix("").unwrap(), "");
        assert!(prefix(&"a".repeat(4095)).is_ok());
        assert!(prefix(&"a".repeat(4096)).is_err());
        assert!(prefix(&"x/".repeat(32)).is_ok());
        assert!(prefix(&"x/".repeat(33)).is_err());
        let mut snapshot = demo_snapshot();
        snapshot.resources[0].map_row.path = "a".repeat(5000) + "/file";
        snapshot.resources[1].map_row.path = "x/".repeat(33) + "file";
        let catalog = Catalog::build(snapshot, &LoadControl::default()).unwrap();
        let mut state = Folders::default();
        let mut rows = (0..6).collect();
        state.rebuild(&catalog, &mut rows);
        assert_eq!(rows.len(), 6);
        assert_eq!(state.ungrouped, 1); // Oversized next prefix is kept as a resource.
        state.prefix = "x/".repeat(32);
        state.rebuild(&catalog, &mut rows);
        assert_eq!(rows, vec![1]);
        assert!(state.children.is_empty());
        assert_eq!(state.ungrouped, 1); // Depth limit does not drop the resource.
    }
    #[test]
    fn combined_filter_sort_selection_navigation_and_snapshot_reset() {
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(egui::Context::default()),
            crate::Options::default(),
        );
        app.set_catalog(catalog(), "Synthetic".into());
        let catalog = app.catalog.clone().unwrap();
        app.select(2, true);
        app.folders.prefix = prefix("synthetic/a").unwrap();
        app.filter.query = "same".into();
        app.filter.kind = Some("texture".into());
        app.filter.archive = Some("USRDIR/SYNTHETIC.FARC".into());
        app.filter.status = Some("identified_unparsed".into());
        app.resource_sort = super::super::ResourceSort::Hash;
        app.refresh_visible(&catalog);
        assert_eq!(app.visible, vec![1, 2]);
        assert_eq!(app.selected, Some(2));
        assert_eq!(app.folders.direct, 2);
        app.folders.prefix = prefix("synthetic/ab").unwrap();
        app.refresh_visible(&catalog);
        assert!(app.visible.is_empty());
        assert_eq!(app.selected, Some(2));
        app.navigate(0);
        app.refresh_visible(&catalog);
        assert!(app.folders.prefix.is_empty());
        assert_eq!(app.visible.len(), 6);
        app.folders.prefix = prefix("synthetic").unwrap();
        app.set_catalog(self::catalog(), "Reload".into());
        assert!(app.folders.prefix.is_empty() && app.selected.is_none());
    }
}
