//! Source attachment and bounded hex UI; workers own all original-file I/O.
use super::*;
use lbp_catalog::attachment::{AttachedSource, HexPage, LocationKey, HEX_PAGE_BYTES};
struct Task<T> {
    control: LoadControl,
    result: mpsc::Receiver<Result<T, String>>,
}
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
pub(super) struct IoState {
    path: String,
    attached: Option<Arc<AttachedSource>>,
    attach_job: Option<Task<AttachedSource>>,
    hex_job: Option<Task<HexPage>>,
    page: Option<HexPage>,
    key: Option<LocationKey>,
    offset: String,
    candidate: usize,
    message: String,
    hex_error: String,
    auto_attach: bool,
}
impl IoState {
    pub(super) fn source(&self) -> Option<Arc<AttachedSource>> {
        self.attached.clone()
    }
    pub(super) fn new(path: Option<PathBuf>) -> Self {
        let auto_attach = path.is_some();
        Self {
            path: path
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            attached: None,
            attach_job: None,
            hex_job: None,
            page: None,
            key: None,
            offset: "0".into(),
            candidate: 0,
            message: "Source detached; metadata browsing remains available.".into(),
            hex_error: String::new(),
            auto_attach,
        }
    }
    pub(super) fn reset_hex(&mut self) {
        self.hex_job = None;
        self.page = None;
        self.key = None;
        self.offset = "0".into();
        self.candidate = 0;
        self.hex_error.clear();
    }
    pub(super) fn reset_source(&mut self) {
        self.attach_job = None;
        self.attached = None;
        self.reset_hex();
        self.message = "Source detached.".into();
    }
    pub(super) fn busy(&self) -> bool {
        self.auto_attach || self.attach_job.is_some() || self.hex_job.is_some()
    }
    fn attach(&mut self, ctx: &egui::Context, catalog: Arc<Catalog>) {
        if self.attach_job.is_some() {
            return;
        }
        self.reset_hex();
        let path = PathBuf::from(self.path.trim());
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = AttachedSource::attach(&path, &catalog, &worker);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.attach_job = Some(Task {
            control,
            result: rx,
        });
        self.message = "Validating SFO, MAP and FARC tables…".into();
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context, catalog: Option<&Arc<Catalog>>) {
        if self.auto_attach {
            if let Some(catalog) = catalog {
                self.auto_attach = false;
                self.attach(ctx, catalog.clone());
            }
        }
        if let Some(result) = self
            .attach_job
            .as_ref()
            .and_then(|j| match j.result.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Attachment worker ended".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            })
        {
            let task = self.attach_job.take().unwrap();
            if task.control.cancelled.load(Ordering::Relaxed) {
                self.message = "Attachment cancelled; previous source retained.".into();
            } else {
                match result {
                    Ok(source) => {
                        self.message = format!(
                            "Validated {} archives / {} entries; {} metadata bytes read.",
                            source.archive_count, source.entry_count, source.validation_bytes
                        );
                        println!("{}", self.message);
                        self.attached = Some(Arc::new(source));
                    }
                    Err(e) => self.message = format!("Attachment refused: {e}"),
                }
            }
        }
        if let Some(result) = self
            .hex_job
            .as_ref()
            .and_then(|j| match j.result.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Hex worker ended".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            })
        {
            let task = self.hex_job.take().unwrap();
            if task.control.cancelled.load(Ordering::Relaxed) {
                self.hex_error = "Byte read cancelled".into();
            } else {
                match result {
                    Ok(page) => self.page = Some(page),
                    Err(e) => {
                        if e.contains("changed") || e.contains("reparse") || e.contains("outside") {
                            self.attached = None;
                            self.message =
                                "Source detached after stability/containment failure.".into();
                        }
                        self.hex_error = e;
                    }
                }
            }
        }
    }
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui, catalog: Option<&Arc<Catalog>>) {
        ui.horizontal(|ui| {
            ui.label("Game directory");
            ui.add_enabled(
                self.attach_job.is_none(),
                egui::TextEdit::singleline(&mut self.path)
                    .hint_text("Extracted disc root or PS3_GAME")
                    .desired_width((ui.available_width() - 190.0).max(80.0)),
            );
            if ui
                .add_enabled(
                    catalog.is_some() && self.attach_job.is_none() && !self.path.trim().is_empty(),
                    egui::Button::new("Attach source"),
                )
                .clicked()
            {
                self.attach(ui.ctx(), catalog.unwrap().clone());
            }
            if ui
                .add_enabled(
                    self.attached.is_some() || self.attach_job.is_some(),
                    egui::Button::new("Detach"),
                )
                .clicked()
            {
                self.reset_source();
                self.auto_attach = false;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.small(&self.message);
            if let Some(source) = &self.attached {
                ui.small("Attached").on_hover_text(source.root());
            }
            if let Some(task) = &self.attach_job {
                ui.spinner();
                ui.small(format!(
                    "{:.1} MB read",
                    task.control.bytes_read.load(Ordering::Relaxed) as f64 / 1e6
                ));
                if ui.small_button("Cancel validation").clicked() {
                    task.control.cancel();
                }
            }
        });
    }
    fn read(&mut self, ctx: &egui::Context, relative: u64) {
        self.hex_job = None;
        self.page = None;
        self.hex_error.clear();
        self.offset = relative.to_string();
        let (Some(source), Some(key)) = (self.attached.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = source.read_hex_checked(&key, relative, HEX_PAGE_BYTES, &worker);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.hex_job = Some(Task {
            control,
            result: rx,
        });
    }
    pub(super) fn hex(&mut self, ui: &mut egui::Ui, locations: &[LocationKey]) {
        if locations.is_empty() {
            ui.label("No candidate bytes in this snapshot.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("hex_candidate")
            .selected_text(format!("Candidate {}", self.candidate))
            .show_ui(ui, |ui| {
                for (i, k) in locations.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.candidate,
                        i,
                        format!("{i}: {} @ {} ({} bytes)", k.path, k.offset, k.size),
                    );
                }
            });
        let key = &locations[self.candidate];
        if self.key.as_ref() != Some(key) {
            self.hex_job = None;
            self.page = None;
            self.hex_error.clear();
            self.offset = "0".into();
            self.key = Some(key.clone());
        }
        ui.small(format!(
            "{} @ {} · {} bytes",
            key.path, key.offset, key.size
        ));
        if self.attached.is_none() {
            ui.label("Attach and validate a source to inspect bytes.");
            return;
        }
        if self.page.is_none() && self.hex_job.is_none() && self.hex_error.is_empty() {
            self.read(ui.ctx(), 0);
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Offset");
            ui.add(egui::TextEdit::singleline(&mut self.offset).desired_width(120.0));
            if ui.button("Read").clicked() {
                let text = self.offset.trim();
                let value = if let Some(hex) = text.strip_prefix("0x") {
                    u64::from_str_radix(hex, 16)
                } else {
                    text.parse()
                };
                match value {
                    Ok(n) => self.read(ui.ctx(), n),
                    Err(_) => self.hex_error = "Offset needs decimal or 0x-prefixed hex".into(),
                }
            }
            if let Some(page) = &self.page {
                let current = page.relative_offset;
                let end = current + page.bytes.len() as u64;
                if ui
                    .add_enabled(current > 0, egui::Button::new("Prev"))
                    .clicked()
                {
                    self.read(ui.ctx(), current.saturating_sub(HEX_PAGE_BYTES as u64));
                }
                if ui
                    .add_enabled(end < key.size, egui::Button::new("Next"))
                    .clicked()
                {
                    self.read(ui.ctx(), end);
                }
            }
        });
        if let Some(task) = &self.hex_job {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking stable inputs and reading one page…");
                if ui.small_button("Cancel").clicked() {
                    task.control.cancel();
                }
            });
        }
        if !self.hex_error.is_empty() {
            ui.colored_label(Color32::YELLOW, &self.hex_error);
        }
        if let Some(page) = &self.page {
            ui.small(format!(
                "{} bytes · absolute offset {} · page limit 4096",
                page.bytes.len(),
                page.absolute_offset
            ));
            ui.small("Cached page; Read rechecks the source. No export or decoding.");
            let rows = page.bytes.len().div_ceil(16);
            egui::ScrollArea::both()
                .id_salt("hex_bytes")
                .auto_shrink([false, false])
                .show_rows(ui, 18.0, rows, |ui, range| {
                    for row in range {
                        let start = row * 16;
                        let bytes = &page.bytes[start..(start + 16).min(page.bytes.len())];
                        let hex = bytes
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let ascii: String = bytes
                            .iter()
                            .map(|b| {
                                if b.is_ascii_graphic() || *b == b' ' {
                                    char::from(*b)
                                } else {
                                    '.'
                                }
                            })
                            .collect();
                        ui.add_sized(
                            [610.0, 18.0],
                            egui::Label::new(
                                RichText::new(format!(
                                    "{:016x}  {:47}  {}",
                                    page.relative_offset + start as u64,
                                    hex,
                                    ascii
                                ))
                                .monospace()
                                .size(12.0),
                            )
                            .extend(),
                        );
                    }
                });
        }
    }
}
