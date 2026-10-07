use super::*;
use lbp_catalog::attachment::LocationKey;
impl Explorer {
    pub(super) fn select_entry(&mut self, a: usize, e: usize) {
        self.transition(
            Visit {
                route: Route::Entry(a, e),
                tab: self.tab,
            },
            true,
            false,
        );
    }
    pub(super) fn entry_list(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.add(
            egui::TextEdit::singleline(&mut self.filter.query)
                .hint_text("Search archive, hash or entry number…")
                .desired_width(ui.available_width()),
        );
        ui.small("Every table entry, including hashes without MAP aliases. Type/status filters apply to MAP rows only.");
        egui::ComboBox::from_id_salt("entry_sort")
            .selected_text(format!("Sort: {}", self.entry_sort.label()))
            .show_ui(ui, |ui| {
                for order in EntrySort::ALL {
                    ui.selectable_value(&mut self.entry_sort, order, order.label());
                }
            }).response.on_hover_text("Sort all listed archives together. Equal values use archive order, then table entry number.");
        self.refresh_entries(catalog);
        ui.label(format!("{} table entries", self.entry_visible.len()));
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("archive_entries")
            .auto_shrink([false, false]);
        if let Some(target) = self.entry_jump.take() {
            if let Some(position) = self.entry_visible.iter().position(|&entry| entry == target) {
                scroll = scroll
                    .vertical_scroll_offset(position as f32 * (ROW + ui.spacing().item_spacing.y));
            }
        }
        scroll.show_rows(ui, ROW, self.entry_visible.len(), |ui, range| {
            for row in range {
                let (a, e) = self.entry_visible[row];
                let archive = &catalog.snapshot.archives[a];
                let entry = &archive.entries[e];
                let aliases = catalog
                    .by_hash
                    .get(&entry.sha1.to_ascii_lowercase())
                    .map_or(0, Vec::len);
                if ui
                    .add_sized(
                        [ui.available_width(), ROW],
                        egui::Button::selectable(
                            self.entry_selection == Some((a, e)),
                            RichText::new(format!(
                                "#{}  {}…  {} bytes  {} aliases",
                                entry.entry_index,
                                &entry.sha1[..12],
                                entry.size,
                                aliases
                            ))
                            .monospace()
                            .size(12.0),
                        )
                        .truncate(),
                    )
                    .on_hover_text(format!(
                        "{}\n{} @ {}",
                        entry.sha1, archive.path, entry.offset
                    ))
                    .clicked()
                {
                    self.select_entry(a, e);
                }
            }
        });
    }
    pub(super) fn entry_detail(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        a: usize,
        e: usize,
    ) {
        let archive = &catalog.snapshot.archives[a];
        let entry = &archive.entries[e];
        ui.heading(format!("Archive entry {}", entry.entry_index));
        ui.label(&archive.path);
        ui.monospace(&entry.sha1);
        field(ui, "Offset", &entry.offset.to_string());
        field(ui, "Bytes", &entry.size.to_string());
        if ui.small_button("Copy SHA1").clicked() {
            ui.ctx().copy_text(entry.sha1.clone());
        }
        let aliases = catalog
            .by_hash
            .get(&entry.sha1.to_ascii_lowercase())
            .map(Vec::as_slice)
            .unwrap_or_default();
        ui.label(format!("{} MAP aliases for this hash", aliases.len()));
        let mut alias_jump = None;
        egui::ScrollArea::horizontal()
            .id_salt("entry_aliases")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for &i in aliases {
                        let row = &catalog.snapshot.resources[i];
                        if ui
                            .small_button(&row.map_row.guid)
                            .on_hover_text(&row.map_row.path)
                            .clicked()
                        {
                            alias_jump = Some(i);
                        }
                    }
                });
            });
        if let Some(target) = alias_jump {
            self.navigate(target);
            return;
        }
        if aliases.is_empty() {
            ui.label("No MAP alias; entry remains browsable.");
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Hex, "Hex bytes");
            ui.selectable_value(&mut self.tab, Tab::Texture, "Texture");
            ui.selectable_value(&mut self.tab, Tab::Mesh, "Mesh");
            ui.selectable_value(&mut self.tab, Tab::Material, "Material");
            ui.selectable_value(&mut self.tab, Tab::Animation, "Animation");
            ui.selectable_value(&mut self.tab, Tab::Plan, "Plan");
        });
        let locations = [LocationKey {
            path: archive.path.clone(),
            offset: entry.offset,
            size: u64::from(entry.size),
        }];
        if self.tab == Tab::Texture {
            self.preview.ui(ui, self.io.source(), &locations);
        } else if self.tab == Tab::Mesh {
            self.mesh.ui(
                ui,
                self.io.source(),
                &locations,
                self.catalog.as_ref().unwrap().clone(),
            );
            if let Some(target) = self.mesh.take_jump() {
                self.navigate(target);
            }
        } else if self.tab == Tab::Material {
            self.material.ui(ui, self.io.source(), &locations, catalog);
            if let Some(target) = self.material.take_jump() {
                self.navigate(target);
            }
        } else if self.tab == Tab::Animation {
            self.animation.ui(ui, self.io.source(), &locations);
        } else if self.tab == Tab::Plan {
            self.plan.ui(ui, self.io.source(), &locations);
        } else {
            self.io.hex(ui, &locations);
        }
    }
}
