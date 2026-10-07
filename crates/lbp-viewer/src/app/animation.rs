//! Bounded native prefix diagnostics. Layout/coverage: docs/animation-parser.md.
//! No animation playback, keyframe interpretation or original writes.
use super::*;
use animation::AnimationInspection;
use lbp_catalog::attachment::{AttachedSource, LocationKey};
use lbp_resource::animation;
mod export;
struct AnimationJob {
    control: LoadControl,
    result: mpsc::Receiver<Result<AnimationInspection, String>>,
}
impl Drop for AnimationJob {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Default)]
pub(super) struct AnimationView {
    source: Option<Arc<AttachedSource>>,
    key: Option<LocationKey>,
    job: Option<AnimationJob>,
    metadata: Option<AnimationInspection>,
    candidate: usize,
    error: String,
    qa_metadata: bool,
    export_path: String,
    export_message: String,
}
fn read_metadata(
    size: u64,
    mut read: impl FnMut(u64, usize) -> Result<Vec<u8>, String>,
    cancel: impl Fn() -> bool,
) -> Result<AnimationInspection, String> {
    let cancelled = || -> Result<(), String> {
        if cancel() {
            Err("Animation read cancelled".into())
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let count = size.min(32) as usize;
    let mut bytes = read(0, count)?;
    if bytes.len() != count {
        return Err("Invalid: short animation header read".into());
    }
    if lbp_resource::is_protected_prefix(&bytes[..bytes.len().min(4)]) {
        return Err("Blocked protected animation".into());
    }
    if bytes.get(..4) != Some(b"ANMb") {
        return Err("Unsupported: only ordinary clear ANMb animation prefixes".into());
    }
    if bytes.len() < 18 {
        return Err("Invalid: truncated animation header".into());
    }
    let revision = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    if !(888..=951).contains(&revision)
        || bytes[12..16] != [0; 4]
        || !matches!(bytes[16], 0 | 7)
        || bytes[17] > 1
    {
        return Err("Unsupported: animation revision/branch/flags".into());
    }
    if size > animation::MAX_INPUT as u64 {
        return Err("Animation stored input budget exceeded".into());
    }
    let length = usize::try_from(size).map_err(|_| "Animation stored input budget exceeded")?;
    bytes.reserve(length - bytes.len());
    while bytes.len() < length {
        cancelled()?;
        let count = (length - bytes.len()).min(4096);
        let part = read(bytes.len() as u64, count)?;
        if part.len() != count {
            return Err("Invalid: short animation page".into());
        }
        bytes.extend(part);
    }
    animation::inspect(&bytes, cancel).map_err(|e| e.to_string())
}
impl AnimationView {
    pub(super) fn new(qa_metadata: bool) -> Self {
        Self {
            qa_metadata,
            ..Default::default()
        }
    }
    pub(super) fn reset(&mut self) {
        *self = Self::new(self.qa_metadata);
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let result = export::save(self, path);
        self.export_message = match &result {
            Ok(()) => "Saved cached animation diagnostic JSON. Game data was not reread.".into(),
            Err(error) => format!("Export failed: {error}"),
        };
        result
    }
    fn start(&mut self, ctx: &egui::Context) {
        self.export_message.clear();
        self.job = None;
        self.metadata = None;
        self.error.clear();
        let (Some(source), Some(key)) = (self.source.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = read_metadata(
                key.size,
                |offset, count| {
                    source
                        .read_hex_checked(&key, offset, count, &worker)
                        .map(|page| page.bytes)
                },
                || worker.cancelled.load(Ordering::Relaxed),
            );
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.job = Some(AnimationJob { control, result });
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
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Animation worker ended".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = result {
            let job = self.job.take().unwrap();
            if job.control.cancelled.load(Ordering::Relaxed) {
                self.error = "Animation read cancelled".into();
            } else {
                match result {
                    Ok(metadata) => {
                        println!(
                            "Animation prefix: revision {}, raw frames {}, raw fps {}",
                            metadata.prefix.revision,
                            metadata.prefix.frames_raw_u16,
                            metadata.prefix.fps_raw_u16
                        );
                        if self.qa_metadata {
                            if let Ok(value) = serde_json::to_string(&metadata) {
                                println!("QA cached animation metadata: {value}");
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
            ui.label("Attach and validate a source to inspect animation prefixes.");
            return;
        }
        if locations.is_empty() {
            ui.label("No available resource candidate.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("animation_candidate")
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
        ui.horizontal(|ui| {
            if ui.button("Reload animation metadata").clicked() {
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
        egui::CollapsingHeader::new("Export cached animation diagnostic JSON").show(ui, |ui| {
            ui.small("Choose a new .json file under local-analysis. Metadata only; no animation bytes or extra game reads.");
            ui.add(egui::TextEdit::singleline(&mut self.export_path)
                .hint_text("local-analysis/animation-diagnostic.json").desired_width(f32::INFINITY));
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
                .id_salt("animation_metadata")
                .show(ui, |ui| details(ui, metadata));
        }
    }
}
fn details(ui: &mut egui::Ui, metadata: &AnimationInspection) {
    let p = &metadata.prefix;
    ui.label(format!(
        "Animation prefix · revision {} · flags {}",
        p.revision, metadata.compression_flags
    ));
    ui.small("Raw metadata only. Keyframes, locator records and playback remain unparsed.");
    field(ui, "Frame count (raw16)", &p.frames_raw_u16.to_string());
    field(ui, "FPS (raw16)", &p.fps_raw_u16.to_string());
    field(ui, "Loop start (raw16)", &p.loop_start_raw_u16.to_string());
    field(ui, "Morph count (raw8)", &p.morph_count_raw_u8.to_string());
    ui.separator();
    for (label, value) in [
        "Bones",
        "Rotation tracks",
        "Position tracks",
        "Scale tracks",
        "Morph tracks",
        "Locator keys",
    ]
    .iter()
    .zip(p.counts)
    {
        field(ui, label, &value.to_string());
    }
    ui.small("Counts do not establish valid tracks or an empty animation.");
    egui::CollapsingHeader::new("Optional position fields — raw IEEE bits").show(ui, |ui| {
        for (label, bits) in [
            ("Offset", p.position_offset_bits),
            ("Scale", p.position_scale_bits),
        ] {
            let text = bits
                .map(|bits| bits.map(|v| format!("{v:08x}")).join(" "))
                .unwrap_or_else(|| "Absent in this revision".into());
            field(ui, label, &text);
        }
    });
    field(
        ui,
        "Fat byte (meaning unknown)",
        &p.fat_raw_u8
            .map(|v| v.to_string())
            .unwrap_or_else(|| "Absent in this revision".into()),
    );
    ui.separator();
    field(
        ui,
        "Expanded payload bytes",
        &metadata.expanded_bytes.to_string(),
    );
    field(
        ui,
        "Opaque animation bytes",
        &p.animation_data_bytes.to_string(),
    );
    field(
        ui,
        "Data starts at expanded offset",
        &p.animation_data_start.to_string(),
    );
    field(
        ui,
        "Parsed prefix bytes",
        &p.parsed_prefix_bytes.to_string(),
    );
    field(
        ui,
        "Unparsed bytes including blob",
        &p.unparsed_bytes.to_string(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    fn synthetic() -> Vec<u8> {
        let mut p = vec![];
        for word in [17u16, 30, 3] {
            p.extend_from_slice(&word.to_be_bytes());
        }
        p.extend_from_slice(&[255, 7, 5, 3, 1, 0]);
        p.extend_from_slice(&2u16.to_be_bytes());
        for bits in [
            0x7fc00001u32,
            0x80000000,
            0,
            0xffffffff,
            0x3f800000,
            1,
            0,
            0xdeadbeef,
        ] {
            p.extend_from_slice(&bits.to_be_bytes());
        }
        p.extend_from_slice(&[255, 0]); // Unknown fat and zero opaque-array length.
        let mut data = b"ANMb".to_vec();
        data.extend_from_slice(&946u32.to_be_bytes());
        data.extend_from_slice(&(18 + p.len() as u32).to_be_bytes());
        data.extend_from_slice(&[0, 0, 0, 0, 7, 0]);
        data.extend(p);
        data.extend_from_slice(&[0; 4]);
        data
    }
    fn metadata() -> AnimationInspection {
        animation::inspect(&synthetic(), || false).unwrap()
    }
    fn cached() -> AnimationView {
        AnimationView {
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
    fn export_projects_raw_fields_nulls_without_source_or_payload() {
        let mut view = cached();
        for optional in [true, false] {
            if !optional {
                let p = &mut view.metadata.as_mut().unwrap().prefix;
                p.position_offset_bits = None;
                p.position_scale_bits = None;
                p.fat_raw_u8 = None;
            }
            let report = export::report(&view).unwrap();
            assert_eq!(
                report["stages"]["animation"]["metadata"],
                serde_json::to_value(view.metadata.as_ref().unwrap()).unwrap()
            );
            assert_eq!(report["stages"]["animation"]["status"], "available");
            assert_eq!(report["location"]["source_path"], "USRDIR/authored.farc");
            assert_eq!(
                report["stages"]["animation"]["metadata"]["prefix"]["fat_raw_u8"].is_null(),
                !optional
            );
            let keys = report["stages"]["animation"]["metadata"]["prefix"]
                .as_object()
                .unwrap();
            assert_eq!(keys.len(), 13);
            assert!(!keys.contains_key("keyframes") && !keys.contains_key("animation_data"));
        }
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
                "Unsupported: animation revision",
                "unsupported",
                "unsupported_layout",
            ),
            (
                "Blocked protected animation",
                "blocked",
                "blocked_protected",
            ),
            ("Animation read cancelled", "cancelled", "cancelled"),
            (
                "Animation stored input budget exceeded",
                "failed",
                "budget_exceeded",
            ),
            (
                "Source changed C:\\private\\game",
                "failed",
                "source_changed",
            ),
            (
                "Invalid: short animation page C:\\private\\game",
                "failed",
                "inspection_failed",
            ),
        ] {
            view.error = error.into();
            let report = export::report(&view).unwrap();
            assert_eq!(report["stages"]["animation"]["status"], status);
            assert_eq!(report["last_request"]["error_kind"], kind);
            assert!(report["stages"]["animation"]["metadata"].is_null());
            assert!(!report.to_string().contains("private"));
        }
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        view.job = Some(AnimationJob {
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
                animation::MAX_INPUT as u64 + 1
            } else {
                bytes.len() as u64
            };
            match mode {
                0 => bytes[..4].copy_from_slice(b"ANMe"),
                1 => bytes[4..8].copy_from_slice(&844u32.to_be_bytes()),
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
        assert_eq!(parsed.prefix.unparsed_bytes, 8192);
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
        assert!(error.contains("short animation page"));
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
            let mut view = AnimationView {
                job: Some(AnimationJob { control, result }),
                ..Default::default()
            };
            if mode == 2 {
                drop(tx);
            } else {
                tx.send(if mode == 0 {
                    Ok(metadata())
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
        app.tab = Tab::Animation;
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        app.animation.job = Some(AnimationJob { control, result });
        app.animation.metadata = Some(metadata());
        app.animation.candidate = 2;
        app.select_entry(0, 0);
        assert_eq!(app.tab, Tab::Animation);
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(app.animation.metadata.is_none());
        assert_eq!(app.animation.candidate, 0);
        app.animation.metadata = Some(metadata());
        app.set_catalog(
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap(),
            "Authored replacement".into(),
        );
        assert!(app.animation.metadata.is_none());
        let mut view = AnimationView::new(true);
        view.metadata = Some(metadata());
        view.reset();
        assert!(view.metadata.is_none() && view.qa_metadata);
    }
    #[test]
    fn raw_unknown_optional_metadata_and_detached_ui_render_without_evaluation() {
        let ctx = egui::Context::default();
        for optional in [false, true] {
            let mut data = metadata();
            if !optional {
                data.prefix.position_offset_bits = None;
                data.prefix.position_scale_bits = None;
                data.prefix.fat_raw_u8 = None;
            }
            let mut view = AnimationView::default();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(510.0, 700.0),
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
            assert_eq!(data.prefix.morph_count_raw_u8, 255);
            assert_eq!(data.prefix.fps_raw_u16, 30);
        }
    }
}
