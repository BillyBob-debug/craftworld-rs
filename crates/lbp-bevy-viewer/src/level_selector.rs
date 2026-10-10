//! Level selector for the main viewer: lists every level in the attached game data
//! (grouped by folder, in-game names where a story slot resolves to the level), with a
//! search box, plus the loading overlay shown while a scene is being read and built.
//! The list is built off the UI thread once per attached catalog.
use bevy_egui::egui;
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_preview::level_list::{self, LevelEntry};
use std::{
    collections::BTreeMap,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
pub struct LevelSelector {
    /// Left panel shows levels instead of the mesh browser.
    pub levels_tab: bool,
    query: String,
    entries: Vec<LevelEntry>,
    /// Address of the catalog `entries` was built for.
    listed_for: Option<usize>,
    pending: Option<Mutex<mpsc::Receiver<Vec<LevelEntry>>>>,
}

impl LevelSelector {
    /// Forget the list (the snapshot or game directory changed).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.listed_for = None;
        self.pending = None;
    }

    /// The in-game or file name for a stored MAP path, if listed.
    pub fn name_of(&self, path: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.path.eq_ignore_ascii_case(path))
            .map(|e| e.name.as_str())
    }

    fn poll(&mut self, context: &(Arc<Catalog>, Arc<AttachedSource>)) {
        let key = Arc::as_ptr(&context.0) as usize;
        let received = self
            .pending
            .as_ref()
            .map(|rx| rx.lock().unwrap().try_recv());
        match received {
            Some(Ok(entries)) => {
                self.entries = entries;
                self.pending = None;
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => self.pending = None,
            Some(Err(mpsc::TryRecvError::Empty)) | None => {}
        }
        if self.listed_for == Some(key) {
            return;
        }
        self.listed_for = Some(key);
        self.entries.clear();
        let (catalog, source) = context.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let spawned = std::thread::Builder::new().spawn(move || {
            // Slot names are a nicety: a failed story read still lists every level.
            let story =
                lbp_preview::story::load_story(&source, &catalog, &LoadControl::default()).ok();
            let _ = tx.send(level_list::list_levels(&catalog, story.as_ref()));
        });
        if spawned.is_ok() {
            self.pending = Some(Mutex::new(rx));
        }
    }

    /// Draws the tab switch; returns the chosen level when one is clicked.
    pub fn panel(
        &mut self,
        ui: &mut egui::Ui,
        context: Option<&(Arc<Catalog>, Arc<AttachedSource>)>,
        current_path: Option<&str>,
    ) -> Option<LevelEntry> {
        let Some(context) = context else {
            ui.label("Attach a snapshot and game directory to list levels.");
            return None;
        };
        self.poll(context);
        if self.pending.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Listing levels…");
            });
            return None;
        }
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut self.query);
            if ui.button("Clear").clicked() {
                self.query.clear();
            }
        });
        let shown: Vec<&LevelEntry> = self
            .entries
            .iter()
            .filter(|e| e.matches(&self.query))
            .collect();
        ui.small(format!(
            "{} of {} levels (story slot names where the slot lists resolve; file names otherwise)",
            shown.len(),
            self.entries.len()
        ));
        let mut groups: BTreeMap<&str, Vec<&LevelEntry>> = BTreeMap::new();
        for e in shown {
            groups.entry(e.group.as_str()).or_default().push(e);
        }
        let searching = !self.query.trim().is_empty();
        let single = groups.len() == 1;
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (group, entries) in &groups {
                    let header =
                        egui::CollapsingHeader::new(format!("{group} ({})", entries.len()))
                            .id_salt(("level_group", *group));
                    let header = if searching || single {
                        header.open(Some(true))
                    } else {
                        header
                    };
                    header.show(ui, |ui| {
                        for e in entries {
                            let selected =
                                current_path.is_some_and(|p| p.eq_ignore_ascii_case(&e.path));
                            let label = if e.named {
                                format!("{}  [{}]", e.name, file_name(&e.path))
                            } else {
                                e.name.clone()
                            };
                            if ui
                                .selectable_label(selected, label)
                                .on_hover_text(&e.path)
                                .clicked()
                            {
                                chosen = Some((*e).clone());
                            }
                        }
                    });
                }
            });
        chosen
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Centred spinner card while a load runs.
pub fn loading_overlay(ctx: &egui::Context, text: &str, detail: &str, elapsed: Duration) {
    egui::Area::new("loading_overlay".into())
        .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.vertical(|ui| {
                        ui.strong(text);
                        ui.small(format!("{detail} ({}s)", elapsed.as_secs()));
                    });
                });
            });
        });
}
