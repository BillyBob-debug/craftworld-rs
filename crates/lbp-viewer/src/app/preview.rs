//! Single-resource CPU/GPU cache. All original reads and decoding are cancellable workers.
use super::*;
use lbp_catalog::attachment::{AttachedSource, LocationKey};
use lbp_texture::{Error, Texture};
mod export;
struct Job {
    control: LoadControl,
    result: mpsc::Receiver<Result<Texture, Error>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Default)]
pub(super) struct Preview {
    source: Option<Arc<AttachedSource>>,
    key: Option<LocationKey>,
    job: Option<Job>,
    image: Option<Texture>,
    gpu: Option<egui::TextureHandle>,
    error: String,
    candidate: usize,
    channel: usize,
    uploaded_channel: Option<usize>,
    zoom: f32,
    fit: bool,
    export_path: String,
    export_message: String,
}
fn channel_pixels(rgba: &[u8], channel: usize) -> Vec<u8> {
    if channel == 0 {
        return rgba.to_vec();
    }
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let c = p[(channel - 1).min(3)];
            [c, c, c, 255]
        })
        .collect()
}
impl Preview {
    pub(super) fn new(channel: usize) -> Self {
        Self {
            channel,
            fit: true,
            zoom: 1.0,
            ..Default::default()
        }
    }
    pub(super) fn reset(&mut self) {
        let channel = self.channel;
        *self = Self {
            channel,
            zoom: 1.0,
            fit: true,
            ..Default::default()
        };
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let result = export::save(self, path);
        self.export_message = match &result {
            Ok(()) => "Saved cached texture diagnostic JSON. Game data was not reread.".into(),
            Err(error) => format!("Export failed: {error}"),
        };
        result
    }
    fn start(&mut self, ctx: &egui::Context) {
        self.job = None;
        self.image = None;
        self.gpu = None;
        self.error.clear();
        self.export_message.clear();
        self.uploaded_channel = None;
        let (Some(source), Some(key)) = (self.source.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let decoded = lbp_texture::decode_reader(
                key.size,
                |offset, count| {
                    source
                        .read_hex_checked(&key, offset, count, &worker)
                        .map(|p| p.bytes)
                        .map_err(Error::Input)
                },
                || worker.cancelled.load(Ordering::Relaxed),
            );
            let _ = tx.send(decoded);
            ctx.request_repaint();
        });
        self.job = Some(Job { control, result });
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
        let result = self.job.as_ref().and_then(|j| match j.result.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(Error::Input("Preview worker ended".into())))
            }
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            let task = self.job.take().unwrap();
            if task.control.cancelled.load(Ordering::Relaxed) {
                self.error = "Texture decode cancelled".into();
            } else {
                match result {
                    Ok(t) => {
                        println!(
                            "Texture preview: {}x{} {} top mip",
                            t.header.width, t.header.height, t.header.pixel_format
                        );
                        self.image = Some(t);
                    }
                    Err(e) => self.error = e.to_string(),
                }
            }
        }
        if let Some(t) = &self.image {
            if self.uploaded_channel != Some(self.channel) {
                let pixels = channel_pixels(&t.rgba, self.channel);
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [t.header.width as usize, t.header.height as usize],
                    &pixels,
                );
                self.gpu =
                    Some(ctx.load_texture("Texture top mip", image, egui::TextureOptions::NEAREST));
                self.uploaded_channel = Some(self.channel);
            }
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
            ui.label("Attach and validate a source to preview textures.");
            return;
        }
        if locations.is_empty() {
            ui.label("No available resource candidate.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("texture_candidate")
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
            self.job = None;
            self.image = None;
            self.gpu = None;
            self.error.clear();
            self.key = Some(key.clone());
            self.uploaded_channel = None;
            self.start(ui.ctx());
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Reload preview").clicked() {
                self.start(ui.ctx());
            }
            if let Some(job) = &self.job {
                ui.spinner();
                ui.label("Reading and decoding…");
                if ui.small_button("Cancel").clicked() {
                    job.control.cancel();
                }
            }
        });
        if !self.error.is_empty() {
            ui.colored_label(Color32::YELLOW, &self.error);
            ui.small("Supported: plain 2D GTF/TEX BC1/BC3, TEX/DDS BGRA8; top mip only.");
        }
        egui::CollapsingHeader::new("Export cached texture diagnostic JSON").show(ui, |ui| {
            ui.small("Choose a new .json file under local-analysis. Metadata only; no pixels or extra game reads.");
            ui.add(egui::TextEdit::singleline(&mut self.export_path)
                .hint_text("local-analysis/texture-diagnostic.json").desired_width(f32::INFINITY));
            if ui.add_enabled(self.job.is_none() && (self.image.is_some() || !self.error.is_empty()),
                              egui::Button::new("Save diagnostic JSON")).clicked() {
                let path = PathBuf::from(&self.export_path);
                let _ = self.export_to(&path);
            }
        });
        if !self.export_message.is_empty() {
            ui.small(&self.export_message);
        }
        if let Some(t) = &self.image {
            ui.label(format!(
                "{} × {} · {} {} · top mip only",
                t.header.width, t.header.height, t.header.wrapper, t.header.pixel_format
            ));
            if t.header.mips == 0 {
                ui.small("DDS mip count unspecified (raw 0); showing top mip.");
            }
            ui.small(format!(
                "{} · {} bytes after top mip remain unparsed",
                if t.header.mips == 0 {
                    "Mip count unspecified".into()
                } else {
                    format!("{} declared mips", t.header.mips)
                },
                t.unparsed_after_top_mip
            ));
            if t.header.no_srgb {
                ui.colored_label(ACCENT, "Non-color / bump data: raw channel preview");
            }
            ui.small("Cached preview. Reload rechecks source. No asset export.");
            if let Some(dds) = &t.header.dds {
                ui.collapsing("DDS header", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.monospace(serde_json::to_string_pretty(dds).unwrap_or_default());
                        });
                });
            }
            ui.horizontal_wrapped(|ui| {
                for (i, label) in ["RGBA", "R", "G", "B", "A"].iter().enumerate() {
                    ui.selectable_value(&mut self.channel, i, *label);
                }
                ui.checkbox(&mut self.fit, "Fit");
                if !self.fit {
                    ui.add(egui::Slider::new(&mut self.zoom, 0.125..=8.0).text("Zoom"));
                }
            });
            let natural = egui::vec2(t.header.width as f32, t.header.height as f32);
            let scale = if self.fit {
                (ui.available_width() / natural.x)
                    .min((ui.available_height().max(80.0) - 5.0) / natural.y)
                    .max(0.001)
            } else {
                self.zoom.max(0.125)
            };
            if let Some(gpu) = &self.gpu {
                egui::ScrollArea::both()
                    .id_salt("texture_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(natural * scale, egui::Sense::hover());
                        let visible = rect.intersect(ui.clip_rect());
                        let step = 16.0;
                        let x0 = ((visible.min.x - rect.min.x) / step).floor().max(0.0) as usize;
                        let y0 = ((visible.min.y - rect.min.y) / step).floor().max(0.0) as usize;
                        let x1 = ((visible.max.x - rect.min.x) / step).ceil().max(0.0) as usize;
                        let y1 = ((visible.max.y - rect.min.y) / step).ceil().max(0.0) as usize;
                        for y in y0..y1 {
                            for x in x0..x1 {
                                let cell = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(x as f32 * step, y as f32 * step),
                                    egui::vec2(step, step),
                                )
                                .intersect(rect);
                                ui.painter().rect_filled(
                                    cell,
                                    0.0,
                                    Color32::from_gray(if (x + y) % 2 == 0 { 65 } else { 100 }),
                                );
                            }
                        }
                        ui.painter().image(
                            gpu.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    });
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cached() -> Preview {
        Preview {
            key: Some(LocationKey {
                path: "USRDIR/authored.farc".into(),
                offset: 64,
                size: 80,
            }),
            image: Some(Texture {
                header: lbp_texture::Header {
                    pixel_format: lbp_texture::PixelFormat::BC1,
                    width: 4,
                    height: 4,
                    mips: 1,
                    wrapper: "GTF",
                    flags: Some(0),
                    remap: Some(0),
                    format_raw: Some(0x86),
                    no_srgb: false,
                    dds: None,
                },
                expanded_bytes: 80,
                top_mip_block_bytes: 8,
                unparsed_after_top_mip: 0,
                rgba: vec![123; 64],
            }),
            ..Default::default()
        }
    }
    #[test]
    fn cached_texture_export_whitelists_metadata_and_ignores_channels_zoom_and_pixels() {
        let mut preview = cached();
        let report = export::report(&preview).unwrap();
        let metadata = &report["stages"]["texture"]["metadata"];
        assert_eq!(metadata.as_object().unwrap().len(), 4);
        assert!(metadata.get("rgba").is_none());
        assert_eq!(metadata["header"]["width"], 4);
        assert_eq!(report["stages"]["texture"]["status"], "available");
        preview.channel = 4;
        preview.zoom = 8.;
        preview.fit = false;
        preview.image.as_mut().unwrap().rgba.fill(0);
        assert_eq!(export::report(&preview).unwrap(), report);
        let texture = preview.image.as_mut().unwrap();
        texture.header.wrapper = "TEX/DDS";
        texture.header.mips = 0;
        texture.header.dds = Some(lbp_texture::DdsHeader {
            flags: u32::MAX,
            pitch_or_linear_size: 8,
            depth: 0,
            pixel_flags: 4,
            fourcc: "DXT1".into(),
            rgb_bits: 0,
            masks: [0; 4],
            caps: [u32::MAX; 4],
        });
        let dds = export::report(&preview).unwrap();
        assert_eq!(dds["stages"]["texture"]["metadata"]["header"]["mips"], 0);
        assert_eq!(
            dds["stages"]["texture"]["metadata"]["header"]["dds"]["caps"][0],
            u32::MAX
        );
    }
    #[test]
    fn texture_export_keeps_failure_categories_and_refuses_busy_uncompleted_and_reset() {
        let mut preview = cached();
        preview.image = None;
        assert!(export::report(&preview).is_err());
        for (error, status) in [
            (Error::Blocked, "blocked"),
            (Error::Unsupported("authored layout"), "unsupported"),
            (Error::Cancelled, "cancelled"),
            (Error::Budget, "failed"),
            (Error::Input("C:\\private\\game\\file".into()), "failed"),
        ] {
            preview.error = error.to_string();
            let report = export::report(&preview).unwrap();
            assert_eq!(report["stages"]["texture"]["status"], status);
            assert!(report["stages"]["texture"]["metadata"].is_null());
            assert!(!report.to_string().contains("private"));
        }
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        preview.job = Some(Job {
            control: control.clone(),
            result,
        });
        assert!(export::report(&preview).is_err());
        preview.export_message = "old".into();
        preview.reset();
        assert!(control.cancelled.load(Ordering::Relaxed));
        assert!(preview.export_message.is_empty() && export::report(&preview).is_err());
    }
    #[test]
    fn channels_preserve_raw_alpha_and_do_not_mutate_cpu_image() {
        let original = [10, 20, 30, 0, 40, 50, 60, 255];
        assert_eq!(channel_pixels(&original, 0), original);
        assert_eq!(
            channel_pixels(&original, 4),
            [0, 0, 0, 255, 255, 255, 255, 255]
        );
        assert_eq!(
            channel_pixels(&original, 2),
            [20, 20, 20, 255, 50, 50, 50, 255]
        );
    }
    #[test]
    fn reset_cancels_job_and_discards_candidate_and_cache() {
        let control = LoadControl::default();
        let (tx, result) = mpsc::sync_channel(1);
        let mut preview = Preview {
            job: Some(Job {
                control: control.clone(),
                result,
            }),
            key: Some(LocationKey {
                path: "synthetic".into(),
                offset: 0,
                size: 8,
            }),
            error: "old".into(),
            ..Default::default()
        };
        preview.reset();
        assert!(control.cancelled.load(Ordering::Relaxed));
        assert!(preview.key.is_none());
        assert!(preview.error.is_empty());
        drop(tx);
    }
}
