//! Bounded native prefix diagnostics. Layout/coverage: docs/plan-parser.md.
//! No Thing traversal, inventory interpretation or original writes.
use super::*;
use lbp_catalog::attachment::{AttachedSource, LocationKey};
use lbp_resource::plan;
use plan::{InventoryPrefix, PlanInspection};
mod export;
struct PlanLoad {
    metadata: PlanInspection,
    inventory: Option<Result<InventoryPrefix, String>>,
}
impl From<PlanInspection> for PlanLoad {
    fn from(metadata: PlanInspection) -> Self {
        Self {
            metadata,
            inventory: None,
        }
    }
}
struct PlanJob {
    control: LoadControl,
    result: mpsc::Receiver<Result<PlanLoad, String>>,
}
impl Drop for PlanJob {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Default)]
pub(super) struct PlanView {
    source: Option<Arc<AttachedSource>>,
    key: Option<LocationKey>,
    job: Option<PlanJob>,
    metadata: Option<PlanInspection>,
    inventory: Option<InventoryPrefix>,
    inventory_error: String,
    inventory_mode: bool,
    candidate: usize,
    error: String,
    qa_metadata: bool,
    export_path: String,
    export_message: String,
}
fn read_bytes(
    size: u64,
    mut read: impl FnMut(u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
) -> Result<Vec<u8>, String> {
    let cancelled = || -> Result<(), String> {
        if cancel() {
            Err("Plan read cancelled".into())
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let count = size.min(32) as usize;
    let mut bytes = read(0, count)?;
    if bytes.len() != count {
        return Err("Invalid: short plan header read".into());
    }
    if lbp_resource::is_protected_prefix(&bytes[..bytes.len().min(4)]) {
        return Err("Blocked protected plan".into());
    }
    if bytes.get(..4) != Some(b"PLNb") {
        return Err("Unsupported: only ordinary clear PLNb plan prefixes".into());
    }
    if bytes.len() < 18 {
        return Err("Invalid: truncated plan header".into());
    }
    let revision = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    if !(893..=lbp_resource::LBP2_MAX_REVISION).contains(&revision)
        || bytes[12..16] != [0; 4]
        || !matches!(bytes[16], 0 | 7)
        || bytes[17] > 1
    {
        return Err("Unsupported: plan revision/branch/flags".into());
    }
    if size > plan::MAX_INPUT as u64 {
        return Err("Plan stored input budget exceeded".into());
    }
    let length = usize::try_from(size).map_err(|_| "Plan stored input budget exceeded")?;
    bytes.reserve(length - bytes.len());
    while bytes.len() < length {
        cancelled()?;
        let count = (length - bytes.len()).min(4096);
        let part = read(bytes.len() as u64, count)?;
        if part.len() != count {
            return Err("Invalid: short plan page".into());
        }
        bytes.extend(part);
    }
    Ok(bytes)
}
fn read_result(
    size: u64,
    read: impl FnMut(u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
    inventory: bool,
) -> Result<PlanLoad, String> {
    let bytes = read_bytes(size, read, &cancel)?;
    if inventory {
        plan::inspect_inventory(&bytes, cancel)
            .map(|r| PlanLoad {
                metadata: r.plan,
                inventory: Some(r.inventory.map_err(|e| e.to_string())),
            })
            .map_err(|e| e.to_string())
    } else {
        plan::inspect(&bytes, cancel)
            .map(Into::into)
            .map_err(|e| e.to_string())
    }
}
#[cfg(test)]
fn read_metadata(
    size: u64,
    read: impl FnMut(u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
) -> Result<PlanInspection, String> {
    read_result(size, read, cancel, false).map(|r| r.metadata)
}
impl PlanView {
    pub(super) fn new(qa_metadata: bool) -> Self {
        Self {
            qa_metadata,
            ..Default::default()
        }
    }
    pub(super) fn reset(&mut self) {
        *self = Self::new(self.qa_metadata).with_inventory(self.inventory_mode);
    }
    pub(super) fn with_inventory(mut self, enabled: bool) -> Self {
        self.inventory_mode = enabled;
        self
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let result = export::save(self, path);
        self.export_message = match &result {
            Ok(()) => "Saved cached plan diagnostic JSON. Game data was not reread.".into(),
            Err(error) => format!("Export failed: {error}"),
        };
        result
    }
    fn start(&mut self, ctx: &egui::Context) {
        self.export_message.clear();
        self.job = None;
        self.metadata = None;
        self.inventory = None;
        self.inventory_error.clear();
        self.error.clear();
        let (Some(source), Some(key)) = (self.source.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::sync_channel(1);
        let inventory_mode = self.inventory_mode;
        std::thread::spawn(move || {
            let result = read_result(
                key.size,
                |offset, count| {
                    source
                        .read_hex_checked(&key, offset, count, &worker)
                        .map(|page| page.bytes)
                },
                || worker.cancelled.load(Ordering::Relaxed),
                inventory_mode,
            );
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.job = Some(PlanJob { control, result });
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context, source: Option<Arc<AttachedSource>>) {
        let same = match (&self.source, &source) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.reset();
            self.source = source;
        }
        let result = self
            .job
            .as_ref()
            .and_then(|job| match job.result.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Plan worker ended".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = result {
            let job = self.job.take().unwrap();
            if job.control.cancelled.load(Ordering::Relaxed) {
                self.error = "Plan read cancelled".into();
            } else {
                match result {
                    Ok(result) => {
                        if let Some(inventory) = result.inventory {
                            match inventory {
                                Ok(metadata) => {
                                    if self.qa_metadata {
                                        if let Ok(value) = serde_json::to_string(&metadata) {
                                            println!("QA cached plan inventory metadata: {value}");
                                        }
                                    }
                                    self.inventory = Some(metadata);
                                }
                                Err(error) => self.inventory_error = error,
                            }
                        }
                        let metadata = result.metadata;
                        println!("Plan prefix: revision {}, raw inner revision {}, opaque Thing bytes {}",
                            metadata.prefix.revision, metadata.prefix.inner_revision_raw_u32,
                            metadata.prefix.thing_data_bytes);
                        if self.qa_metadata {
                            if let Ok(value) = serde_json::to_string(&metadata) {
                                println!("QA cached plan metadata: {value}");
                            }
                        }
                        self.metadata = Some(metadata);
                    }
                    Err(error) => self.error = error,
                }
            }
            ctx.request_repaint();
        }
    }
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        source: Option<Arc<AttachedSource>>,
        locations: &[LocationKey],
    ) {
        self.poll(ui.ctx(), source);
        if self.source.is_none() {
            ui.label("Attach and validate a source to inspect plan prefixes.");
            return;
        }
        if locations.is_empty() {
            ui.label("No available resource candidate.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("plan_candidate")
            .selected_text(format!("Candidate {}", self.candidate))
            .show_ui(ui, |ui| {
                for (i, key) in locations.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.candidate,
                        i,
                        format!("{i}: {} @ {}", key.path, key.offset),
                    );
                }
            });
        let key = &locations[self.candidate];
        if self.key.as_ref() != Some(key) {
            self.key = Some(key.clone());
            self.start(ui.ctx());
        }
        if ui
            .checkbox(&mut self.inventory_mode, "Inspect inventory scalar prefix")
            .changed()
        {
            self.start(ui.ctx());
        }
        ui.horizontal(|ui| {
            if ui.button("Reload plan metadata").clicked() {
                self.start(ui.ctx());
            }
            if let Some(job) = &self.job {
                ui.spinner();
                if ui.button("Cancel").clicked() {
                    job.control.cancel();
                }
            }
        });
        if !self.error.is_empty() {
            ui.colored_label(Color32::YELLOW, &self.error);
        }
        egui::CollapsingHeader::new("Export cached plan diagnostic JSON").show(ui, |ui| {
            ui.small("Version 2 exports cached base and requested inventory scalar stages. Choose a new .json under local-analysis; no payload bytes or extra reads.");
            ui.add(egui::TextEdit::singleline(&mut self.export_path)
                .hint_text("local-analysis/plan-diagnostic.json").desired_width(f32::INFINITY));
            if ui.add_enabled(self.job.is_none() && (self.metadata.is_some() || !self.error.is_empty()),
                              egui::Button::new("Save diagnostic JSON")).clicked() {
                let path = PathBuf::from(&self.export_path);
                let _ = self.export_to(&path);
            }
        });
        if !self.export_message.is_empty() {
            ui.small(&self.export_message);
        }
        if let Some(metadata) = &self.metadata {
            egui::ScrollArea::vertical()
                .id_salt("plan_metadata")
                .show(ui, |ui| {
                    if self.inventory_mode {
                        inventory_details(ui, self.inventory.as_ref(), &self.inventory_error);
                        egui::CollapsingHeader::new("Base plan prefix")
                            .show(ui, |ui| details(ui, metadata));
                    } else {
                        ui.small("Inventory scalar prefix not requested.");
                        details(ui, metadata);
                    }
                });
        }
    }
}
fn inventory_details(ui: &mut egui::Ui, metadata: Option<&InventoryPrefix>, error: &str) {
    ui.heading("Inventory scalar prefix");
    ui.small(
        "Raw fields only. No timestamp, enum normalization, title lookup or recursive references.",
    );
    if !error.is_empty() {
        ui.colored_label(Color32::YELLOW, format!("Inventory prefix failed: {error}"));
        ui.small("The validated base plan prefix remains available below.");
    }
    let Some(m) = metadata else {
        ui.label("Inventory metadata unavailable.");
        return;
    };
    field(
        ui,
        "Date encoded bits (hex64)",
        &m.date_added_encoded_u64_hex,
    );
    for (label, value) in [
        ("Slot type (raw32)", m.slot_type_raw_u32),
        ("Slot number (raw32)", m.slot_number_raw_u32),
        (
            "Highlight sound GUID (raw32)",
            m.highlight_sound_guid_raw_u32,
        ),
        ("Colour (raw32)", m.colour_raw_u32),
        ("Type flags (raw32)", m.type_flags_raw_u32),
        ("Subtype (raw32)", m.sub_type_raw_u32),
        ("Title key (raw32)", m.title_key_raw_u32),
        ("Description key (raw32)", m.description_key_raw_u32),
    ] {
        field(ui, label, &format!("{value} / {value:08x}"));
    }
    for (label, value) in [
        (
            "Inventory prefix starts at expanded offset",
            m.inventory_prefix_start,
        ),
        (
            "Parsed inventory prefix bytes",
            m.parsed_inventory_prefix_bytes,
        ),
        (
            "Inventory prefix ends at expanded offset",
            m.inventory_prefix_end,
        ),
        (
            "Unparsed bytes after inventory prefix",
            m.unparsed_after_inventory_prefix,
        ),
    ] {
        field(ui, label, &value.to_string());
    }
    ui.small(
        "Parsing stops before CreationHistory. Remaining bytes do not establish valid inventory.",
    );
}
fn details(ui: &mut egui::Ui, metadata: &PlanInspection) {
    let p = &metadata.prefix;
    ui.label(format!(
        "Plan prefix · revision {} · flags {}",
        p.revision, metadata.compression_flags
    ));
    ui.small("Raw metadata only. Things and inventory contents remain unparsed.");
    field(
        ui,
        "Inner revision (raw32)",
        &p.inner_revision_raw_u32.to_string(),
    );
    ui.small("Inner/outer revision mismatch does not establish corruption.");
    for (label, value) in [
        ("Expanded payload bytes", metadata.expanded_bytes),
        ("Opaque Thing bytes", p.thing_data_bytes),
        ("Thing data starts at expanded offset", p.thing_data_start),
        ("Parsed prefix bytes", p.parsed_prefix_bytes),
        ("Unparsed bytes including Thing data", p.unparsed_bytes),
        (
            "Inventory tail starts at expanded offset",
            p.inventory_tail_start,
        ),
        ("Unparsed inventory tail bytes", p.inventory_tail_bytes),
    ] {
        field(ui, label, &value.to_string());
    }
    ui.separator();
    ui.label("Opaque Thing data SHA256");
    ui.add(egui::Label::new(egui::RichText::new(&p.thing_data_sha256).monospace()).wrap());
    ui.small("Byte extents and digest do not establish valid Things or inventory.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn synthetic() -> Vec<u8> {
        let mut payload = vec![255, 255, 255, 255, 15, 0]; // raw u32max inner revision, empty Thing data
        payload.extend_from_slice(&[0; 20]); // uninterpreted tail; adequate header preflight length
        let mut data = b"PLNb".to_vec();
        data.extend_from_slice(&919u32.to_be_bytes());
        data.extend_from_slice(&(18 + payload.len() as u32).to_be_bytes());
        data.extend_from_slice(&[0, 0, 0, 0, 7, 0]);
        data.extend(payload);
        data.extend_from_slice(&[0; 4]);
        data
    }
    fn metadata() -> PlanInspection {
        plan::inspect(&synthetic(), || false).unwrap()
    }
    fn inventory() -> InventoryPrefix {
        plan::inspect_inventory(&synthetic(), || false)
            .unwrap()
            .inventory
            .unwrap()
    }
    #[test]
    fn combined_worker_reads_once_and_preserves_prefix_on_deeper_failure() {
        let input = synthetic();
        let bytes_read = Cell::new(0);
        let load = read_result(
            input.len() as u64,
            |offset, count| {
                bytes_read.set(bytes_read.get() + count);
                Ok(input[offset as usize..offset as usize + count].to_vec())
            },
            || false,
            true,
        )
        .unwrap();
        assert_eq!(bytes_read.get(), input.len());
        assert!(load.inventory.unwrap().is_ok());
        let plain = read_result(
            input.len() as u64,
            |offset, count| Ok(input[offset as usize..offset as usize + count].to_vec()),
            || false,
            false,
        )
        .unwrap();
        assert!(plain.inventory.is_none());
        let mut short = input[..28].to_vec();
        short[8..12].copy_from_slice(&24u32.to_be_bytes());
        let load = read_result(
            short.len() as u64,
            |offset, count| Ok(short[offset as usize..offset as usize + count].to_vec()),
            || false,
            true,
        )
        .unwrap();
        assert_eq!(load.metadata.prefix.inner_revision_raw_u32, u32::MAX);
        assert_eq!(load.metadata.prefix.inventory_tail_bytes, 0);
        assert!(load.inventory.unwrap().unwrap_err().contains("date bounds"));
    }
    #[test]
    fn inventory_mode_result_failure_cancel_reset_and_export_stages_stay_separate() {
        let ctx = egui::Context::default();
        let mut view = cached().with_inventory(true);
        let (tx, result) = mpsc::sync_channel(1);
        view.job = Some(PlanJob {
            control: LoadControl::default(),
            result,
        });
        tx.send(Ok(PlanLoad {
            metadata: metadata(),
            inventory: Some(Err("Invalid inventory prefix".into())),
        }))
        .unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_some() && view.inventory.is_none());
        assert!(!view.inventory_error.is_empty() && view.error.is_empty());
        let report = export::report(&view).unwrap();
        assert_eq!(report["schema_version"], 2);
        assert_eq!(report["last_request"]["outcome"], "partial");
        assert_eq!(report["stages"]["inventory"]["status"], "failed");
        assert_eq!(report["stages"]["plan"]["status"], "available");
        view.inventory = Some(inventory());
        view.start(&ctx); // No source: clears both caches before refusing a read.
        assert!(
            view.metadata.is_none() && view.inventory.is_none() && view.inventory_error.is_empty()
        );
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        control.cancel();
        view.job = Some(PlanJob { control, result });
        tx.send(Ok(PlanLoad {
            metadata: metadata(),
            inventory: Some(Ok(inventory())),
        }))
        .unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_none() && view.inventory.is_none());
        assert!(view.error.contains("cancelled"));
        view.inventory = Some(inventory());
        view.inventory_error = "old".into();
        view.reset();
        assert!(view.inventory_mode && view.inventory.is_none() && view.inventory_error.is_empty());
    }
    #[test]
    fn inventory_layout_retains_extreme_bits_and_explicit_failure() {
        let ctx = egui::Context::default();
        let mut data = inventory();
        data.date_added_encoded_u64_hex = "ffffffffffffffff".into();
        data.type_flags_raw_u32 = u32::MAX;
        for failed in [false, true] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(510., 700.),
                    )),
                    ..Default::default()
                },
                |ui| {
                    inventory_details(
                        ui,
                        if failed { None } else { Some(&data) },
                        if failed { "Invalid date bounds" } else { "" },
                    )
                },
            );
            assert!(!output.shapes.is_empty());
            output.textures_delta.clear();
        }
        assert_eq!(data.date_added_encoded_u64_hex, "ffffffffffffffff");
        assert_eq!(data.type_flags_raw_u32, u32::MAX);
    }
    fn cached() -> PlanView {
        PlanView {
            key: Some(LocationKey {
                path: "USRDIR/authored.farc".into(),
                offset: 64,
                size: 100,
            }),
            metadata: Some(metadata()),
            ..Default::default()
        }
    }
    #[test]
    fn export_inventory_preserves_raw_fields_and_distinguishes_requested_outcomes() {
        let mut view = cached();
        let base = export::report(&view).unwrap();
        assert_eq!(base["inspection_mode"], "plan_prefix");
        assert_eq!(base["stages"]["inventory"]["status"], "not_requested");
        assert_eq!(
            base["stages"]["inventory"]["requested_in_current_mode"],
            false
        );
        view.inventory_mode = true;
        assert!(export::report(&view).is_err()); // Requested stage is not completed.
        let mut data = inventory();
        data.date_added_encoded_u64_hex = "ffffffffffffffff".into();
        data.slot_type_raw_u32 = u32::MAX;
        data.type_flags_raw_u32 = u32::MAX;
        let expected = serde_json::to_value(&data).unwrap();
        view.inventory = Some(data);
        let complete = export::report(&view).unwrap();
        assert_eq!(complete["last_request"]["outcome"], "complete");
        assert_eq!(complete["inspection_mode"], "plan_inventory_prefix");
        assert_eq!(complete["stages"]["inventory"]["status"], "available");
        let projected = &complete["stages"]["inventory"]["metadata"];
        assert_eq!(projected, &expected);
        assert_eq!(projected.as_object().unwrap().len(), 13);
        assert!(view.source.is_none());
        let encoded = super::super::diagnostics::encode(&complete).unwrap();
        assert_eq!(
            encoded,
            super::super::diagnostics::encode(&export::report(&view).unwrap()).unwrap()
        );
        view.inventory = None;
        for (error, status, kind) in [
            (
                "Invalid inventory bounds C:\\private\\game",
                "failed",
                "inspection_failed",
            ),
            ("Inventory cancelled", "cancelled", "cancelled"),
            ("Unsupported inventory", "unsupported", "unsupported_layout"),
        ] {
            view.inventory_error = error.into();
            let partial = export::report(&view).unwrap();
            assert_eq!(partial["last_request"]["outcome"], "partial");
            assert_eq!(partial["last_request"]["error_kind"], kind);
            assert_eq!(partial["stages"]["plan"], complete["stages"]["plan"]);
            assert_eq!(partial["stages"]["inventory"]["status"], status);
            assert!(partial["stages"]["inventory"]["metadata"].is_null());
            assert!(!partial.to_string().contains("private"));
        }
        view.inventory_error.clear();
        view.metadata = None;
        view.error = "Unsupported plan revision".into();
        let failed = export::report(&view).unwrap();
        assert_eq!(failed["last_request"]["outcome"], "failed");
        assert_eq!(failed["stages"]["inventory"]["status"], "not_available");
        assert_eq!(
            failed["stages"]["inventory"]["requested_in_current_mode"],
            true
        );
        assert!(failed["stages"]["inventory"]["error_kind"].is_null());
    }
    #[test]
    fn export_rejects_stale_or_contradictory_stage_caches() {
        let mut view = cached();
        view.inventory = Some(inventory());
        assert!(export::report(&view).is_err()); // Mode does not request this cache.
        view.inventory_mode = true;
        view.inventory_error = "old error".into();
        assert!(export::report(&view).is_err());
        view.inventory_error.clear();
        view.error = "old base error".into();
        assert!(export::report(&view).is_err());
        view.metadata = None;
        assert!(export::report(&view).is_err()); // Deeper cache without base.
    }
    #[test]
    fn cached_export_projects_raw_extents_hash_without_source_or_payload() {
        let mut view = cached();
        let report = export::report(&view).unwrap();
        assert_eq!(
            report["stages"]["plan"]["metadata"],
            serde_json::to_value(view.metadata.as_ref().unwrap()).unwrap()
        );
        let p = &report["stages"]["plan"]["metadata"]["prefix"];
        assert_eq!(p["inner_revision_raw_u32"], u32::MAX);
        assert_eq!(p["thing_data_bytes"], 0);
        assert_eq!(p["inventory_tail_bytes"], 20);
        assert_eq!(
            p["thing_data_sha256"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(p.as_object().unwrap().len(), 9);
        assert!(p.get("thing_data").is_none() && p.get("inventory_data").is_none());
        assert!(view.source.is_none());
        view.candidate = 10;
        view.qa_metadata = true;
        assert_eq!(export::report(&view).unwrap(), report);
        view.key.as_mut().unwrap().path = "../private/game.farc".into();
        assert!(export::report(&view).is_err());
    }
    #[test]
    fn export_refuses_busy_uncompleted_reset_and_sanitizes_failed_outcomes() {
        let mut view = cached();
        view.metadata = None;
        assert!(export::report(&view).is_err());
        for (error, status, kind) in [
            (
                "Unsupported: plan revision",
                "unsupported",
                "unsupported_layout",
            ),
            ("Blocked protected plan", "blocked", "blocked_protected"),
            ("Plan read cancelled", "cancelled", "cancelled"),
            (
                "Plan stored input budget exceeded",
                "failed",
                "budget_exceeded",
            ),
            (
                "Source changed C:\\private\\game",
                "failed",
                "source_changed",
            ),
            (
                "Invalid: short plan page C:\\private\\game",
                "failed",
                "inspection_failed",
            ),
        ] {
            view.error = error.into();
            let r = export::report(&view).unwrap();
            assert_eq!(r["stages"]["plan"]["status"], status);
            assert_eq!(r["last_request"]["error_kind"], kind);
            assert!(r["stages"]["plan"]["metadata"].is_null());
            assert!(!r.to_string().contains("private"));
        }
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        view.job = Some(PlanJob {
            control: control.clone(),
            result,
        });
        view.metadata = Some(metadata());
        assert!(export::report(&view).is_err());
        view.export_message = "old".into();
        view.reset();
        assert!(control.cancelled.load(Ordering::Relaxed));
        assert!(view.export_message.is_empty() && export::report(&view).is_err());
    }
    #[test]
    fn worker_preflight_stops_protected_old_wrong_magic_and_budget_before_body() {
        for mode in 0..4 {
            let mut bytes = synthetic();
            let size = if mode == 3 {
                plan::MAX_INPUT as u64 + 1
            } else {
                bytes.len() as u64
            };
            match mode {
                0 => bytes[..4].copy_from_slice(b"PLNe"),
                1 => bytes[4..8].copy_from_slice(&622u32.to_be_bytes()),
                2 => bytes[..4].copy_from_slice(b"MSHb"),
                _ => {}
            }
            let calls = Cell::new(0);
            let error = read_metadata(
                size,
                |offset, count| {
                    assert_eq!(offset, 0);
                    assert_eq!(count, 32);
                    calls.set(calls.get() + 1);
                    Ok(bytes[..count].to_vec())
                },
                || false,
            )
            .unwrap_err();
            assert_eq!(calls.get(), 1);
            assert!(if mode == 0 {
                error.contains("Blocked")
            } else if mode == 3 {
                error.contains("budget")
            } else {
                error.contains("Unsupported")
            });
        }
        assert!(read_metadata(100, |_, _| Ok(vec![]), || false)
            .unwrap_err()
            .contains("short"));
    }
    #[test]
    fn paged_worker_read_cancel_and_short_page_do_not_publish_metadata() {
        let mut bytes = synthetic();
        // Opaque tail requires multiple approved reads without being interpreted.
        let at = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        bytes.splice(at..at, std::iter::repeat_n(0, 8192));
        bytes[8..12].copy_from_slice(&((at + 8192) as u32).to_be_bytes());
        let calls = Cell::new(0);
        let parsed = read_metadata(
            bytes.len() as u64,
            |offset, count| {
                assert!(count <= 4096);
                calls.set(calls.get() + 1);
                Ok(bytes[offset as usize..offset as usize + count].to_vec())
            },
            || false,
        )
        .unwrap();
        assert!(calls.get() > 2);
        assert_eq!(parsed.prefix.unparsed_bytes, 8212);
        let error = read_metadata(
            bytes.len() as u64,
            |offset, count| {
                Ok(
                    bytes[offset as usize..offset as usize + count - usize::from(offset != 0)]
                        .to_vec(),
                )
            },
            || false,
        )
        .unwrap_err();
        assert!(error.contains("short plan page"));
        let calls = Cell::new(0);
        assert!(read_metadata(
            bytes.len() as u64,
            |offset, count| {
                calls.set(calls.get() + 1);
                Ok(bytes[offset as usize..offset as usize + count].to_vec())
            },
            || calls.get() == 1
        )
        .unwrap_err()
        .contains("cancelled"));
        assert_eq!(calls.get(), 1);
    }
    #[test]
    fn cancelled_failed_and_disconnected_jobs_cannot_publish_and_routes_reset_cache() {
        let ctx = egui::Context::default();
        for mode in 0..3 {
            let (tx, result) = mpsc::sync_channel(1);
            let control = LoadControl::default();
            if mode == 0 {
                control.cancel();
            }
            let mut view = PlanView {
                job: Some(PlanJob { control, result }),
                ..Default::default()
            };
            if mode == 2 {
                drop(tx);
            } else {
                tx.send(if mode == 0 {
                    Ok(metadata().into())
                } else {
                    Err("Authored malformed prefix".into())
                })
                .unwrap();
            }
            view.poll(&ctx, None);
            assert!(!view.busy());
            assert!(view.metadata.is_none());
            assert!(!view.error.is_empty());
        }
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(ctx),
            Options {
                demo: true,
                ..Default::default()
            },
        );
        app.select(0, false);
        app.tab = Tab::Plan;
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        app.plan.job = Some(PlanJob { control, result });
        app.plan.metadata = Some(metadata());
        app.plan.candidate = 2;
        app.plan.inventory_mode = true;
        app.plan.inventory = Some(inventory());
        app.plan.inventory_error = "old".into();
        app.select_entry(0, 0);
        assert_eq!(app.tab, Tab::Plan);
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(app.plan.metadata.is_none());
        assert_eq!(app.plan.candidate, 0);
        assert!(
            app.plan.inventory.is_none()
                && app.plan.inventory_error.is_empty()
                && app.plan.inventory_mode
        );
        app.plan.metadata = Some(metadata());
        app.plan.inventory = Some(inventory());
        app.set_catalog(
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap(),
            "Authored replacement".into(),
        );
        assert!(app.plan.metadata.is_none());
        assert!(app.plan.inventory.is_none() && app.plan.inventory_mode);
        let mut view = PlanView::new(true);
        view.metadata = Some(metadata());
        view.reset();
        assert!(view.metadata.is_none() && view.qa_metadata);
    }
    #[test]
    fn raw_inner_zero_extents_and_detached_ui_render_without_inventory_evaluation() {
        let ctx = egui::Context::default();
        let mut view = PlanView::default();
        let data = metadata();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(510., 700.),
                )),
                ..Default::default()
            },
            |ui| {
                details(ui, &data);
                view.ui(ui, None, &[]);
            },
        );
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
        assert_eq!(data.prefix.inner_revision_raw_u32, u32::MAX);
        assert_eq!(data.prefix.thing_data_bytes, 0);
        assert_eq!(data.prefix.inventory_tail_bytes, 20);
    }
}
