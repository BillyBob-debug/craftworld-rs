//! Metadata-only browsing: no eager game reads or claims of payload compatibility.
use bevy_egui::egui;
use lbp_catalog::{Catalog, Filter, Resource};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct PreviewEvidence {
    pub instance_input: bool,
    pub unrendered_texture_groups: usize,
    pub animation_snapshot: bool,
    pub textured_groups: usize,
    pub solid_groups: usize,
    pub total_groups: usize,
    pub skipped_geometry: bool,
    pub textures_verified: bool,
    pub geometry_verified: bool,
}
impl PreviewEvidence {
    fn can_verify(&self) -> bool {
        self.total_groups > 0
            && self.textured_groups + self.solid_groups == self.total_groups
            && !self.skipped_geometry
            && !self.animation_snapshot
            && !self.instance_input
            && self.unrendered_texture_groups == 0
    }
    fn complete(&self) -> bool {
        self.can_verify() && self.textures_verified && self.geometry_verified
    }
}

#[derive(Default)]
pub struct Browser {
    pub query: String,
    pub show_unavailable: bool,
    outcomes: BTreeMap<String, Result<PreviewEvidence, String>>,
    pub displayed_guid: Option<String>,
    rows: Vec<usize>,
    dirty: bool,
    filter_key: Option<(String, bool)>,
}

fn candidate(catalog: &Catalog, resource: &Resource) -> bool {
    // Confirmed type wins over an extension hint. Unknown meshes stay unchecked.
    let mesh = resource.confirmed_type.as_deref().map_or_else(
        || {
            resource
                .type_hint
                .as_ref()
                .is_some_and(|hint| hint.name == "mesh")
        },
        |kind| kind == "mesh",
    );
    if !mesh {
        return false;
    }
    let mut inspected = resource
        .inspection_indices()
        .filter_map(|i| catalog.snapshot.resource_inspections.get(i))
        .map(|location| location.inspection.magic_hex.as_str());
    // Only ordinary MSHb has a geometry handler. Uninspected/no magic remains unchecked.
    !inspected.all(|magic| !magic.is_empty() && magic != "4d534862")
        || resource.inspection_indices().next().is_none()
}

impl Browser {
    pub fn clear_source(&mut self) {
        self.outcomes.clear();
        self.displayed_guid = None;
        self.rows.clear();
        self.filter_key = None;
        self.dirty = true;
    }
    pub fn record(&mut self, guid: String, outcome: Result<PreviewEvidence, String>) {
        let guid = guid.trim().to_ascii_lowercase();
        self.displayed_guid = outcome.is_ok().then(|| guid.clone());
        self.outcomes.insert(guid, outcome);
        self.dirty = true;
    }
    pub fn counts(&self) -> (usize, usize, usize) {
        let viewable = self.outcomes.values().filter(|o| o.is_ok()).count();
        let complete = self
            .outcomes
            .values()
            .filter(|o| o.as_ref().is_ok_and(PreviewEvidence::complete))
            .count();
        let failed = self.outcomes.len() - viewable;
        (viewable, complete, failed)
    }
    pub fn review_panel(&mut self, ui: &mut egui::Ui) {
        let Some(guid) = &self.displayed_guid else {
            return;
        };
        let Some(Ok(evidence)) = self.outcomes.get_mut(guid) else {
            return;
        };
        ui.label(format!(
            "{} • {guid}",
            if evidence.complete() {
                "Complete (visually verified)"
            } else {
                "Viewable • completeness unverified"
            }
        ));
        ui.small(format!(
            "{}/{} diffuse groups supported: {} textured, {} solid color",
            evidence.textured_groups + evidence.solid_groups,
            evidence.total_groups,
            evidence.textured_groups,
            evidence.solid_groups
        ));
        if evidence.unrendered_texture_groups != 0 {
            ui.small(format!("{} groups reference additional textures that are not rendered; Complete cannot be confirmed.", evidence.unrendered_texture_groups));
        }
        if evidence.instance_input {
            ui.small("Supplied instance color: actual game inputs unverified; Complete cannot be confirmed.");
        }
        if evidence.animation_snapshot {
            ui.small("Animation base-value snapshot: game texture placement unverified; Complete cannot be confirmed.");
        }
        ui.add_enabled_ui(evidence.can_verify(), |ui| {
            ui.checkbox(
                &mut evidence.textures_verified,
                "I verified all required textures (or none required)",
            );
            ui.checkbox(
                &mut evidence.geometry_verified,
                "I verified geometry is correct",
            );
        });
        if evidence.skipped_geometry
            || evidence.textured_groups + evidence.solid_groups != evidence.total_groups
        {
            ui.small(
                "Complete cannot be confirmed while groups are neutral or geometry was skipped.",
            );
        }
        ui.small("Complete concerns textures and geometry; renderer completion is separate. Verification resets on reload/source change.");
    }
    fn visible(&self, catalog: &Catalog, resource: &Resource) -> bool {
        self.show_unavailable
            || match self
                .outcomes
                .get(&resource.map_row.guid.to_ascii_lowercase())
            {
                Some(result) => result.is_ok(),
                None => candidate(catalog, resource),
            }
    }
    fn refresh(&mut self, catalog: &Catalog) {
        let key = (self.query.clone(), self.show_unavailable);
        if !self.dirty && self.filter_key.as_ref() == Some(&key) {
            return;
        }
        self.rows = catalog
            .filtered(&Filter {
                query: self.query.clone(),
                ..Default::default()
            })
            .into_iter()
            .filter(|&i| self.visible(catalog, &catalog.snapshot.resources[i]))
            .collect();
        self.filter_key = Some(key);
        self.dirty = false;
    }
    pub fn panel(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        selected: &str,
    ) -> Option<String> {
        ui.label("Search path, filename or GUID");
        ui.text_edit_singleline(&mut self.query);
        ui.checkbox(
            &mut self.show_unavailable,
            "Show files without a current preview",
        );
        ui.small("Viewable: geometry displayed, possibly with neutral groups. Complete: all textures showing and geometry visually verified correct. Unchecked assets are not counted.");
        let (viewable, complete, failed) = self.counts();
        ui.label(format!(
            "This source session: {viewable} viewable • {complete} complete • {failed} failed"
        ));
        let ready = self
            .outcomes
            .values()
            .filter(|o| o.as_ref().is_ok_and(PreviewEvidence::can_verify))
            .count();
        ui.small(format!(
            "{ready} ready for completeness review: all diffuse groups supported"
        ));
        ui.small("Counts are distinct attempted GUIDs, include complete in viewable, and ignore search filters. Corpus totals: see the Corpus counts window (census files).");
        self.refresh(catalog);
        ui.label(format!(
            "{} shown / {} catalog rows",
            self.rows.len(),
            catalog.snapshot.resources.len()
        ));
        ui.separator();
        let mut chosen = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 42., self.rows.len(), |ui, range| {
            for row in range {
                let index = self.rows[row];
                let resource = &catalog.snapshot.resources[index];
                let guid = resource.map_row.guid.to_ascii_lowercase();
                let (label, reason) = match self.outcomes.get(&guid) {
                    Some(Ok(e)) if e.complete() => ("Complete", "All textures and geometry visually verified in this source session"),
                    Some(Ok(_)) => ("Viewable", "Geometry displayed; completeness not verified"),
                    Some(Err(reason)) => ("Last attempt failed", reason.as_str()),
                    None if candidate(catalog, resource) => ("Unchecked mesh", "Header/hint only; select to check geometry"),
                    None => ("No current handler", "No ordinary MSHb geometry handler identified; this does not mean corrupt"),
                };
                ui.push_id(index, |ui| {
                    ui.set_min_height(42.);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    let path = &resource.map_row.path;
                    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
                    if ui.selectable_label(guid.eq_ignore_ascii_case(selected), format!("{name}  [{guid}]")).on_hover_text(format!("{path}\n{reason}\nMAP row {} • {}", resource.map_row.row_index, resource.display_type())).clicked() {
                        chosen = Some(guid);
                    }
                    ui.small(format!("{label} • {}", resource.display_type()));
                });
            }
        });
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatibility_counts_require_visual_evidence_and_reset_on_reload() {
        let mut browser = Browser::default();
        let mut evidence = PreviewEvidence {
            textured_groups: 2,
            total_groups: 2,
            ..Default::default()
        };
        browser.record("ABCDEF01".into(), Ok(evidence.clone()));
        assert_eq!(browser.counts(), (1, 0, 0));
        evidence.textures_verified = true;
        assert!(!evidence.complete());
        evidence.geometry_verified = true;
        assert!(evidence.complete());
        evidence.animation_snapshot = true;
        assert!(!evidence.can_verify() && !evidence.complete());
        evidence.animation_snapshot = false;
        evidence.instance_input = true;
        assert!(!evidence.can_verify() && !evidence.complete());
        evidence.instance_input = false;
        evidence.unrendered_texture_groups = 1;
        assert!(!evidence.can_verify() && !evidence.complete());
        evidence.unrendered_texture_groups = 0;
        browser.record("abcdef01".into(), Ok(evidence.clone()));
        browser.record("abcdef02".into(), Err("Unavailable".into()));
        assert_eq!(browser.counts(), (1, 1, 1));
        browser.query = "no matches".into();
        assert_eq!(browser.counts(), (1, 1, 1));
        evidence.textured_groups = 1;
        assert!(!evidence.complete());
        evidence.textured_groups = 2;
        evidence.skipped_geometry = true;
        assert!(!evidence.complete());
        browser.record("abcdef01".into(), Ok(PreviewEvidence::default()));
        assert_eq!(browser.counts(), (1, 0, 1));
        browser.clear_source();
        assert_eq!(browser.counts(), (0, 0, 0));
        assert!(browser.displayed_guid.is_none());
        let solid = PreviewEvidence {
            solid_groups: 2,
            total_groups: 2,
            textures_verified: true,
            geometry_verified: true,
            ..Default::default()
        };
        assert!(solid.complete());
        browser.record("abcdef03".into(), Ok(solid));
        assert_eq!(browser.counts(), (1, 1, 0));
    }
    #[test]
    fn filter_preserves_unchecked_and_all_rows_and_clears_failed_source_state() {
        let mut catalog =
            Catalog::build(lbp_catalog::demo_snapshot(), &Default::default()).unwrap();
        let index = catalog
            .snapshot
            .resources
            .iter()
            .position(|r| r.confirmed_type.as_deref() == Some("mesh"))
            .unwrap();
        let guid = catalog.snapshot.resources[index]
            .map_row
            .guid
            .to_ascii_lowercase();
        let mut browser = Browser::default();
        assert!(browser.visible(&catalog, &catalog.snapshot.resources[index]));
        browser.record(guid.clone(), Err("Unsupported synthetic payload".into()));
        assert!(!browser.visible(&catalog, &catalog.snapshot.resources[index]));
        browser.show_unavailable = true;
        assert!(browser.visible(&catalog, &catalog.snapshot.resources[index]));
        browser.refresh(&catalog);
        assert_eq!(
            browser.rows,
            (0..catalog.snapshot.resources.len()).collect::<Vec<_>>()
        );
        browser.show_unavailable = false;
        browser.clear_source();
        assert!(browser.visible(&catalog, &catalog.snapshot.resources[index]));
        browser.record(guid, Ok(PreviewEvidence::default()));
        browser.query = "guid:".to_string() + &catalog.snapshot.resources[index].map_row.guid;
        browser.refresh(&catalog);
        assert_eq!(
            browser.rows,
            catalog.filtered(&Filter {
                query: browser.query.clone(),
                ..Default::default()
            })
        );
        catalog.snapshot.resources[index].confirmed_type = Some("texture".into());
        assert!(!candidate(&catalog, &catalog.snapshot.resources[index]));
    }
    #[test]
    fn confirmed_types_override_hints_and_unknown_meshes_are_not_unsupported() {
        let mut catalog =
            Catalog::build(lbp_catalog::demo_snapshot(), &Default::default()).unwrap();
        assert!(candidate(&catalog, &catalog.snapshot.resources[0]));
        catalog.snapshot.resource_inspections[0]
            .inspection
            .magic_hex = "534d4862".into();
        assert!(!candidate(&catalog, &catalog.snapshot.resources[0]));
        catalog.snapshot.resource_inspections[0]
            .inspection
            .magic_hex
            .clear();
        assert!(candidate(&catalog, &catalog.snapshot.resources[0]));
        catalog.snapshot.resources[0].confirmed_type = Some("texture".into());
        assert!(!candidate(&catalog, &catalog.snapshot.resources[0]));
        catalog.snapshot.resources[0].confirmed_type = None;
        catalog.snapshot.resources[0].archive_candidates.clear();
        assert!(candidate(&catalog, &catalog.snapshot.resources[0]));
    }
}
