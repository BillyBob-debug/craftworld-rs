//! Bounded native prefix/graph/tail diagnostics, no execution or byte export.
use super::*;
use lbp_catalog::attachment::{AttachedSource, LocationKey};
use lbp_resource::material::{self, GraphMetadata, MaterialInspection, TailMetadata};
mod diagram;
mod export;
struct LoadedMaterial {
    prefix: MaterialInspection,
    graph: Option<Result<GraphMetadata, String>>,
    tail: Option<Result<TailMetadata, String>>,
}
fn inspect_metadata(
    data: &[u8],
    control: &LoadControl,
    graph: bool,
    tail_requested: bool,
) -> Result<LoadedMaterial, String> {
    let cancelled = || control.cancelled.load(Ordering::Relaxed);
    // Successful requests expand once. Failure falls back to earlier stages;
    // fallback expansions are sequential/bounded on the same single input read.
    let tail = if tail_requested {
        match material::inspect_tail(data, cancelled) {
            Ok(v) => {
                return Ok(LoadedMaterial {
                    prefix: v.inspection.material,
                    graph: Some(Ok(v.inspection.graph)),
                    tail: Some(Ok(v.tail)),
                })
            }
            Err(e) => Some(Err(e.to_string())),
        }
    } else {
        None
    };
    let graph = if graph || tail_requested {
        match material::inspect_graph(data, cancelled) {
            Ok(v) => {
                return Ok(LoadedMaterial {
                    prefix: v.material,
                    graph: Some(Ok(v.graph)),
                    tail,
                })
            }
            Err(e) => Some(Err(e.to_string())),
        }
    } else {
        None
    };
    let prefix = material::inspect(data, cancelled).map_err(|e| e.to_string())?;
    Ok(LoadedMaterial {
        prefix,
        graph,
        tail,
    })
}
struct MaterialJob {
    control: LoadControl,
    result: mpsc::Receiver<Result<LoadedMaterial, String>>,
}
impl Drop for MaterialJob {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Default)]
pub(super) struct MaterialView {
    source: Option<Arc<AttachedSource>>,
    key: Option<LocationKey>,
    job: Option<MaterialJob>,
    metadata: Option<MaterialInspection>,
    graph: Option<GraphMetadata>,
    graph_error: String,
    graph_requested: bool,
    tail_requested: bool,
    tail: Option<TailMetadata>,
    tail_error: String,
    selected_animation: usize,
    selected_box: usize,
    selected_wire: usize,
    error: String,
    candidate: usize,
    pending_jump: Option<usize>,
    diagram: diagram::DiagramView,
    export_path: String,
    export_message: String,
}
impl MaterialView {
    pub(super) fn new(graph_requested: bool, tail_requested: bool) -> Self {
        Self {
            graph_requested,
            tail_requested,
            ..Default::default()
        }
    }
    pub(super) fn reset(&mut self) {
        *self = Self::new(self.graph_requested, self.tail_requested);
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn take_jump(&mut self) -> Option<usize> {
        self.pending_jump.take()
    }
    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let result = export::save(self, path);
        self.export_message = match &result {
            Ok(()) => "Saved cached diagnostic JSON. Game data was not reread.".into(),
            Err(error) => format!("Export failed: {error}"),
        };
        result
    }
    fn start(&mut self, ctx: &egui::Context, preserve_prefix: bool) {
        self.job = None;
        if !preserve_prefix {
            self.metadata = None;
            self.graph = None;
            self.diagram = diagram::DiagramView::default();
        }
        self.tail = None;
        self.tail_error.clear();
        self.selected_animation = 0;
        self.graph_error.clear();
        self.selected_box = 0;
        self.selected_wire = 0;
        self.error.clear();
        self.export_message.clear();
        self.pending_jump = None;
        let (Some(source), Some(key)) = (self.source.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::sync_channel(1);
        let graph_requested = self.graph_requested;
        let tail_requested = self.tail_requested;
        std::thread::spawn(move || {
            let run = || -> Result<LoadedMaterial, String> {
                let mut data = source
                    .read_hex_checked(&key, 0, key.size.min(32) as usize, &worker)?
                    .bytes;
                if lbp_resource::is_protected_prefix(&data) {
                    return Err("Blocked protected material".into());
                }
                if data.get(..4) != Some(b"GMTb") {
                    return Err("Unsupported: only ordinary clear GMTb material prefixes".into());
                }
                if key.size > material::MAX_INPUT as u64 {
                    return Err("Material stored input budget exceeded".into());
                }
                data.reserve(key.size as usize - data.len());
                while data.len() < key.size as usize {
                    if worker.cancelled.load(Ordering::Relaxed) {
                        return Err("Material read cancelled".into());
                    }
                    let n = (key.size as usize - data.len()).min(4096);
                    data.extend(
                        source
                            .read_hex_checked(&key, data.len() as u64, n, &worker)?
                            .bytes,
                    );
                }
                inspect_metadata(&data, &worker, graph_requested, tail_requested)
            };
            let _ = tx.send(run());
            ctx.request_repaint();
        });
        self.job = Some(MaterialJob { control, result });
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
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Material worker ended".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            let job = self.job.take().unwrap();
            if job.control.cancelled.load(Ordering::Relaxed) {
                self.error = "Material read cancelled".into();
            } else {
                match result {
                    Ok(m) => {
                        println!(
                            "Material metadata: revision {}, {} texture slots",
                            m.prefix.prefix.revision,
                            m.prefix.prefix.textures.len()
                        );
                        self.metadata = Some(m.prefix);
                        if let Some(graph) = m.graph {
                            match graph {
                                Ok(graph) => {
                                    println!(
                                        "Material graph metadata: {} boxes, {} wires",
                                        graph.box_count, graph.wire_count
                                    );
                                    self.graph = Some(graph);
                                }
                                Err(e) => self.graph_error = e,
                            }
                        }
                        if let Some(tail) = m.tail {
                            match tail {
                                Ok(tail) => {
                                    println!("Material tail metadata: sound raw {}, {} parameter animations, {} residual bytes", tail.sound_enum_raw, tail.parameter_animation_count, tail.unparsed_bytes);
                                    self.tail = Some(tail);
                                }
                                Err(e) => self.tail_error = e,
                            }
                        }
                    }
                    Err(e) => self.error = e,
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
        catalog: &Catalog,
    ) {
        self.poll(ui.ctx(), source);
        if self.source.is_none() {
            ui.label("Attach and validate a source to inspect material prefixes.");
            return;
        }
        if locations.is_empty() {
            ui.label("No available resource candidate.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("material_candidate")
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
            self.start(ui.ctx(), false);
        }
        ui.horizontal(|ui| {
            if ui.button("Reload material").clicked() {
                self.start(ui.ctx(), false);
            }
            if let Some(job) = &self.job {
                ui.spinner();
                if ui.button("Cancel").clicked() {
                    job.control.cancel();
                }
            }
        });
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.graph_requested && !self.tail_requested, "Prefix")
                .clicked()
            {
                self.graph_requested = false;
                self.tail_requested = false;
            }
            if ui
                .selectable_label(
                    self.graph_requested && !self.tail_requested,
                    "Inspect graph metadata",
                )
                .clicked()
            {
                self.graph_requested = true;
                self.tail_requested = false;
                if self.graph.is_none() && self.job.is_none() {
                    self.start(ui.ctx(), true);
                }
            }
            if ui
                .selectable_label(self.tail_requested, "Inspect tail metadata")
                .clicked()
            {
                self.graph_requested = true;
                self.tail_requested = true;
                if self.tail.is_none() && self.job.is_none() {
                    self.start(ui.ctx(), true);
                }
            }
        });
        // A deeper-stage request made during an earlier load is queued once.
        // Cancellation and parse failures require an explicit retry.
        if self.graph_requested
            && self.metadata.is_some()
            && (self.graph.is_none() || (self.tail_requested && self.tail.is_none()))
            && self.job.is_none()
            && self.graph_error.is_empty()
            && self.tail_error.is_empty()
            && self.error.is_empty()
        {
            self.start(ui.ctx(), true);
        }
        if self.tail_requested && !self.tail_error.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                format!(
                    "Tail unavailable: {}. Earlier metadata remains available.",
                    self.tail_error
                ),
            );
        }
        if self.graph_requested && !self.graph_error.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                format!(
                    "Graph unavailable: {}. Prefix remains available.",
                    self.graph_error
                ),
            );
        }
        if !self.error.is_empty() {
            ui.colored_label(Color32::YELLOW, &self.error);
            ui.small("Supported: bounded PS3 GMTb prefix, heads 920–1016, ordinary branch zero.");
        }
        egui::CollapsingHeader::new("Export cached diagnostic JSON").show(ui, |ui| {
            ui.small("Choose a new .json file under local-analysis. Metadata only; no additional game reads.");
            ui.add(egui::TextEdit::singleline(&mut self.export_path)
                .hint_text("local-analysis/material-diagnostic.json").desired_width(f32::INFINITY));
            if ui.add_enabled(self.job.is_none() && (self.metadata.is_some() || !self.error.is_empty()),
                              egui::Button::new("Save diagnostic JSON")).clicked() {
                let path=PathBuf::from(&self.export_path);
                let _=self.export_to(&path);
            }
        });
        if !self.export_message.is_empty() {
            ui.small(&self.export_message);
        }
        if let Some(metadata) = &self.metadata {
            if self.graph_requested && !self.tail_requested {
                if let Some(graph) = &self.graph {
                    self.diagram
                        .ui(ui, graph, &mut self.selected_box, &mut self.selected_wire);
                }
            }
            egui::ScrollArea::vertical()
                .id_salt("material_details")
                .show(ui, |ui| {
                    if self.tail_requested {
                        if let Some(tail) = &self.tail {
                            tail_details(ui, metadata, tail, &mut self.selected_animation);
                            return;
                        }
                    }
                    if self.graph_requested {
                        if let Some(graph) = &self.graph {
                            graph_details(
                                ui,
                                metadata,
                                graph,
                                catalog,
                                &mut self.selected_box,
                                &mut self.selected_wire,
                                &mut self.pending_jump,
                            );
                        } else {
                            details(ui, metadata, catalog, &mut self.pending_jump);
                        }
                    } else {
                        details(ui, metadata, catalog, &mut self.pending_jump);
                    }
                });
        }
    }
}
fn tail_details(
    ui: &mut egui::Ui,
    material: &MaterialInspection,
    tail: &TailMetadata,
    selected: &mut usize,
) {
    ui.label(format!(
        "Revision {} · parameter animation tail",
        material.prefix.revision
    ));
    ui.label(format!(
        "Sound value (raw): {} · 0x{:08x}",
        tail.sound_enum_raw, tail.sound_enum_raw
    ));
    ui.small(format!(
        "Tail @ {} · {} parsed bytes · {} residual bytes",
        tail.tail_start, tail.parsed_tail_bytes, tail.unparsed_bytes
    ));
    ui.small("Sound meaning and animation binding/timing are unknown. Metadata only.");
    ui.separator();
    ui.strong(format!(
        "Parameter animations: {}",
        tail.parameter_animation_count
    ));
    if tail.parameter_animations.is_empty() {
        ui.label("No serialized animation records. Original null versus empty is unknown.");
    }
    egui::ScrollArea::vertical()
        .id_salt("tail_animations")
        .max_height(130.)
        .show_rows(ui, 24., tail.parameter_animations.len(), |ui, range| {
            for i in range {
                let a = &tail.parameter_animations[i];
                if ui
                    .selectable_label(
                        *selected == i,
                        format!(
                            "{i}: {} values · {} derived keys · mask 0x{:02x}",
                            a.component_value_count, a.derived_key_count, a.components_raw
                        ),
                    )
                    .clicked()
                {
                    *selected = i;
                }
            }
        });
    if let Some(a) = tail.parameter_animations.get(*selected) {
        ui.label(format!(
            "Animation {} · raw name bytes (hex): {}",
            *selected, a.name_hex
        ));
        ui.label("Base vector — IEEE bits");
        ui.horizontal_wrapped(|ui| {
            for (i, bits) in a.base_value_bits.iter().enumerate() {
                ui.monospace(format!("{i}: 0x{bits:08x}"));
            }
        });
        ui.add(egui::Label::new(format!("Component SHA256: {}", a.component_bytes_sha256)).wrap());
    }
    egui::CollapsingHeader::new("Tail diagnostic metadata JSON").show(ui, |ui| {
        ui.monospace(serde_json::to_string_pretty(tail).unwrap_or_default());
    });
}
fn details(
    ui: &mut egui::Ui,
    metadata: &MaterialInspection,
    catalog: &Catalog,
    jump: &mut Option<usize>,
) {
    let p = &metadata.prefix;
    ui.label(format!(
        "PS3 material prefix · revision {} · flags {}",
        p.revision, metadata.compression_flags
    ));
    ui.small(format!(
        "{} expanded bytes · {} prefix bytes · {} unparsed tail bytes",
        metadata.expanded_bytes, p.parsed_prefix_bytes, p.unparsed_bytes
    ));
    ui.small("Texture roles and graph/shader meaning are not established.");
    egui::CollapsingHeader::new("Scalar fields and IEEE bits").show(ui, |ui| {
        let s = &p.scalars;
        field(ui, "Material flags", &format!("0x{:08x}", s.flags));
        field(
            ui,
            "Alpha layer / mode / shadow (raw)",
            &format!(
                "{} / {} / {}",
                s.alpha_layer_raw, s.alpha_mode_raw, s.shadow_cast_raw
            ),
        );
        for (name, bits) in [
            ("Alpha test", s.alpha_test_bits),
            ("Bump level", s.bump_level_bits),
            ("Cosine power", s.cosine_power_bits),
            ("Reflection blur", s.reflection_blur_bits),
            ("Refractive index", s.refractive_index_bits),
        ] {
            field(
                ui,
                name,
                &format!("{} · IEEE 0x{bits:08x}", f32::from_bits(bits)),
            );
        }
    });
    egui::CollapsingHeader::new("Opaque shader interval metadata").show(ui, |ui| {
        ui.label(format!(
            "{} offsets · {} shader intervals · {} opaque blob bytes at {}",
            p.shader_offset_count, p.shader_count, p.blob_bytes, p.blob_start
        ));
        ui.label(format!(
            "{} uninterpreted bytes after final shader end",
            p.uninterpreted_after_shader_bytes
        ));
        let mut start = 0;
        for (i, &end) in p.shader_end_offsets.iter().enumerate() {
            ui.monospace(format!("{i}: {start}..{end} ({} bytes)", end - start));
            start = end;
        }
    });
    ui.separator();
    ui.label("Texture slots — raw descriptor order");
    for (slot, descriptor) in p.textures.iter().enumerate() {
        ui.push_id(slot, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Slot {slot}"));
                ui.small(format!(
                    "kind {} · flags 0x{:08x}",
                    descriptor.kind, descriptor.flags
                ));
                let s = p
                    .wrap_s_raw
                    .get(slot)
                    .map(u8::to_string)
                    .unwrap_or_else(|| "not declared".into());
                let t = p
                    .wrap_t_raw
                    .get(slot)
                    .map(u8::to_string)
                    .unwrap_or_else(|| "not declared".into());
                ui.small(format!("wrap S/T {s}/{t}"));
            });
            slot_links(
                ui,
                descriptor.guid,
                descriptor.sha1.as_deref(),
                catalog,
                jump,
            );
            ui.separator();
        });
    }
    egui::CollapsingHeader::new("Diagnostic metadata JSON").show(ui, |ui| {
        ui.monospace(serde_json::to_string_pretty(metadata).unwrap_or_default());
    });
}
fn slot_links(
    ui: &mut egui::Ui,
    guid: Option<u32>,
    hash: Option<&str>,
    catalog: &Catalog,
    jump: &mut Option<usize>,
) {
    let groups = reference_groups(&catalog.by_guid, &catalog.by_hash, guid, hash);
    if groups.is_empty() {
        ui.small("Null descriptor (no identifier)");
    }
    for (label, targets) in groups {
        ui.push_id(&label, |ui| {
            ui.menu_button(format!("{label} · {} candidates", targets.len()), |ui| {
                if targets.is_empty() {
                    ui.label("Unresolved or raw null reference");
                }
                egui::ScrollArea::vertical().max_height(200.).show_rows(
                    ui,
                    24.,
                    targets.len(),
                    |ui, range| {
                        for i in range {
                            let target = targets[i];
                            let row = &catalog.snapshot.resources[target].map_row;
                            if ui.button(format!("{} · {}", row.guid, row.path)).clicked() {
                                *jump = Some(target);
                                ui.close();
                            }
                        }
                    },
                );
            });
        });
    }
}
// Labels are limited community annotations, not inferred graph semantics.
// Toolkit pinned d6a2ef80b5a09999f08f23a803b516096055e5c9,
// BoxType::TEXTURE_SAMPLE/OUTPUT; docs/material-graph.md (heads 920–951).
// https://github.com/ennuo/toolkit/blob/d6a2ef80b5a09999f08f23a803b516096055e5c9/lib/cwlib/src/main/java/cwlib/enums/BoxType.java
fn box_annotation(kind: u32) -> &'static str {
    match kind {
        0 => "output (community)",
        1 => "texture sample (community)",
        _ => "unlabelled",
    }
}
fn graph_details(
    ui: &mut egui::Ui,
    material: &MaterialInspection,
    graph: &GraphMetadata,
    catalog: &Catalog,
    selected_box: &mut usize,
    selected_wire: &mut usize,
    jump: &mut Option<usize>,
) {
    ui.label(format!(
        "Revision {} · {} boxes · {} wires",
        material.prefix.revision, graph.box_count, graph.wire_count
    ));
    ui.small(format!(
        "Graph @ {} · {} parsed bytes · {} unparsed tail bytes",
        graph.graph_start, graph.parsed_graph_bytes, graph.unparsed_bytes
    ));
    ui.small(
        "Diagnostic metadata only. Raw kinds/ports remain authoritative; texture roles unknown.",
    );
    ui.strong("Boxes — select a row");
    egui::ScrollArea::vertical()
        .id_salt("graph_boxes")
        .max_height(130.)
        .show_rows(ui, 24., graph.boxes.len(), |ui, range| {
            for i in range {
                let b = &graph.boxes[i];
                if ui
                    .selectable_label(
                        *selected_box == i,
                        format!(
                            "{i}: type {} · {} · subtype {}",
                            b.type_raw,
                            box_annotation(b.type_raw),
                            b.subtype_raw
                        ),
                    )
                    .clicked()
                {
                    *selected_box = i;
                }
            }
        });
    if let Some(b) = graph.boxes.get(*selected_box) {
        ui.push_id("selected_graph_box", |ui| {
            ui.label(format!(
                "Box {} · {} declared parameters",
                *selected_box, b.declared_parameter_count
            ));
            ui.horizontal_wrapped(|ui| {
                for (i, v) in b.parameters_raw.iter().enumerate() {
                    ui.monospace(format!("p{i}=0x{v:08x}"));
                }
            });
            if let Some(slot) = b.texture_slot_raw {
                ui.label(format!(
                    "Texture slot {slot} · in range: {:?}",
                    b.texture_slot_in_range
                ));
                if let Some(d) = material.prefix.textures.get(slot as usize) {
                    slot_links(ui, d.guid, d.sha1.as_deref(), catalog, jump);
                } else {
                    ui.colored_label(Color32::YELLOW, "Slot outside prefix descriptor array");
                }
            }
            egui::CollapsingHeader::new("Position bits and animation summaries").show(ui, |ui| {
                ui.monospace(format!("Position IEEE bits: {:08x?}", b.position_bits));
                for (i, a) in b.animations.iter().enumerate() {
                    ui.label(format!(
                        "Animation {i}: {} values · {} derived keys · mask 0x{:02x}",
                        a.component_value_count, a.derived_key_count, a.components_raw
                    ));
                    ui.monospace(format!("Base IEEE bits: {:08x?}", a.base_value_bits));
                    ui.label(format!("Name bytes (hex): {}", a.name_hex));
                    ui.horizontal_wrapped(|ui| {
                        ui.small(format!("Component SHA256: {}", a.component_bytes_sha256));
                    });
                }
            });
        });
    }
    ui.separator();
    ui.strong("Wires — raw indices and ports");
    egui::ScrollArea::vertical()
        .id_salt("graph_wires")
        .max_height(110.)
        .show_rows(ui, 24., graph.wires.len(), |ui, range| {
            for i in range {
                let w = &graph.wires[i];
                if ui
                    .selectable_label(
                        *selected_wire == i,
                        format!(
                            "{i}: {}:{} → {}:{} · range {}/{}",
                            w.box_from,
                            w.port_from_raw,
                            w.box_to,
                            w.port_to_raw,
                            w.from_in_range,
                            w.to_in_range
                        ),
                    )
                    .clicked()
                {
                    *selected_wire = i;
                }
            }
        });
    if let Some(w) = graph.wires.get(*selected_wire) {
        ui.monospace(format!(
            "Wire {} swizzle bytes: {:02x?}",
            *selected_wire, w.swizzle_raw
        ));
    }
    egui::CollapsingHeader::new("Graph diagnostic metadata JSON").show(ui, |ui| {
        ui.monospace(serde_json::to_string_pretty(graph).unwrap_or_default());
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_export_preserves_unrequested_failed_and_available_stages() {
        let mut view = MaterialView {
            key: Some(LocationKey {
                path: "USRDIR/authored.farc".into(),
                offset: 64,
                size: 500,
            }),
            metadata: Some(synthetic()),
            ..Default::default()
        };
        let first = export::report(&view).unwrap();
        assert_eq!(first["stages"]["prefix"]["status"], "available");
        assert_eq!(first["stages"]["graph"]["status"], "not_requested");
        assert_eq!(first["stages"]["tail"]["metadata"], serde_json::Value::Null);
        view.graph_requested = true;
        view.tail_requested = true;
        let loaded = inspect_metadata(
            &synthetic_graph_bytes(false),
            &LoadControl::default(),
            true,
            true,
        )
        .unwrap();
        view.metadata = Some(loaded.prefix);
        view.graph = Some(loaded.graph.unwrap().unwrap());
        view.tail_error = loaded.tail.unwrap().unwrap_err();
        let partial = export::report(&view).unwrap();
        assert_eq!(partial["last_request"]["outcome"], "partial");
        assert_eq!(partial["stages"]["graph"]["status"], "available");
        assert_eq!(partial["stages"]["tail"]["status"], "failed");
        assert_eq!(
            partial["stages"]["prefix"]["metadata"],
            serde_json::to_value(view.metadata.as_ref().unwrap()).unwrap()
        );
        view.error = "Material read cancelled".into();
        let cancelled = export::report(&view).unwrap();
        assert_eq!(cancelled["last_request"]["error_kind"], "cancelled");
        assert_eq!(cancelled["stages"]["graph"]["status"], "available");
        assert_eq!(export::report(&view).unwrap(), cancelled);
        let (_tx, result) = mpsc::sync_channel(1);
        view.job = Some(MaterialJob {
            control: LoadControl::default(),
            result,
        });
        assert!(export::report(&view).is_err());
        view.reset();
        assert!(export::report(&view).is_err());
    }
    #[test]
    fn cached_full_tail_export_keeps_fields_and_is_independent_of_ui_selection() {
        let loaded = inspect_metadata(
            &synthetic_graph_bytes(true),
            &LoadControl::default(),
            true,
            true,
        )
        .unwrap();
        let mut view = MaterialView {
            key: Some(LocationKey {
                path: "USRDIR/authored.farc".into(),
                offset: 64,
                size: 500,
            }),
            metadata: Some(loaded.prefix),
            graph: Some(loaded.graph.unwrap().unwrap()),
            tail: Some(loaded.tail.unwrap().unwrap()),
            graph_requested: true,
            tail_requested: true,
            ..Default::default()
        };
        let report = export::report(&view).unwrap();
        assert_eq!(
            report["stages"]["tail"]["metadata"],
            serde_json::to_value(view.tail.as_ref().unwrap()).unwrap()
        );
        assert_eq!(report["stages"]["tail"]["status"], "available");
        view.selected_box = usize::MAX;
        view.selected_wire = 99;
        view.selected_animation = 1;
        assert_eq!(export::report(&view).unwrap(), report);
        view.graph_requested = false;
        view.tail_requested = false;
        let prefix_mode = export::report(&view).unwrap();
        assert_eq!(prefix_mode["stages"]["tail"]["status"], "available");
        assert_eq!(
            prefix_mode["stages"]["tail"]["requested_in_current_mode"],
            false
        );
    }
    fn synthetic_bytes() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&[0; 4]);
        p.extend_from_slice(&0x7fc00001u32.to_be_bytes());
        p.extend_from_slice(&[0, 0, 255]);
        p.extend_from_slice(&[0; 16]);
        for _ in 0..11 {
            p.extend_from_slice(&[0; 4]);
        } // Ten empty intervals plus blob count.
        for _ in 0..8 {
            p.extend_from_slice(&[0; 5]);
        }
        for _ in 0..2 {
            p.extend_from_slice(&8u32.to_be_bytes());
            p.extend_from_slice(&[255; 8]);
        }
        let mut wire = b"GMTb".to_vec();
        wire.extend_from_slice(&950u32.to_be_bytes());
        wire.extend_from_slice(&(18u32 + p.len() as u32).to_be_bytes());
        wire.extend_from_slice(&[0; 6]);
        wire.extend(p);
        wire.extend_from_slice(&[0; 4]);
        wire
    }
    fn synthetic() -> MaterialInspection {
        material::inspect(&synthetic_bytes(), || false).unwrap()
    }
    #[test]
    fn route_transition_cancels_material_worker_and_discards_old_cache() {
        let mut app = Explorer::new(
            &eframe::CreationContext::_new_kittest(egui::Context::default()),
            Options {
                demo: true,
                ..Default::default()
            },
        );
        app.select(0, false);
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        app.material.job = Some(MaterialJob { control, result });
        app.material.metadata = Some(synthetic());
        app.material.pending_jump = Some(2);
        assert!(app.show_overview(false));
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(!app.material.busy());
        assert!(app.material.metadata.is_none());
        assert!(app.material.take_jump().is_none());
        assert!(app.raw.is_empty());
        assert!(app.travel(true));
        assert!(app.material.metadata.is_none());
    }
    #[test]
    fn reset_cancels_discards_metadata_and_pending_navigation() {
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        let mut view = MaterialView {
            job: Some(MaterialJob { control, result }),
            metadata: Some(synthetic()),
            pending_jump: Some(2),
            ..Default::default()
        };
        view.reset();
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(view.metadata.is_none() && view.key.is_none() && view.take_jump().is_none());
        drop(tx);
    }
    #[test]
    fn cancelled_or_failed_worker_cannot_publish_metadata() {
        let ctx = egui::Context::default();
        for cancelled in [true, false] {
            let (tx, result) = mpsc::sync_channel(1);
            let control = LoadControl::default();
            if cancelled {
                control.cancel();
            }
            let mut view = MaterialView {
                job: Some(MaterialJob { control, result }),
                ..Default::default()
            };
            tx.send(if cancelled {
                Ok(LoadedMaterial {
                    prefix: synthetic(),
                    graph: None,
                    tail: None,
                })
            } else {
                Err("unsupported authored input".into())
            })
            .unwrap();
            view.poll(&ctx, None);
            assert!(view.metadata.is_none());
            assert!(!view.error.is_empty());
            assert!(!view.busy());
        }
    }
    #[test]
    fn metadata_layout_handles_unknown_bits_null_alias_conflict_and_unresolved_slots() {
        let ctx = egui::Context::default();
        let mut catalog =
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap();
        catalog.by_guid.insert("00000011".into(), vec![0, 2]);
        catalog.by_hash.insert("a".repeat(40), vec![1, 2]);
        let mut metadata = synthetic();
        metadata.prefix.textures[0].kind = 3;
        metadata.prefix.textures[0].guid = Some(17);
        metadata.prefix.textures[0].sha1 = Some("a".repeat(40));
        metadata.prefix.textures[1].kind = 2;
        metadata.prefix.textures[1].guid = Some(0);
        metadata.prefix.textures[2].kind = 2;
        metadata.prefix.textures[2].guid = Some(99);
        let mut jump = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            details(ui, &metadata, &catalog, &mut jump);
        });
        assert!(!out.shapes.is_empty());
        out.textures_delta.clear(); // Headless test has no GPU consumer.
        assert!(jump.is_none());
        let groups = reference_groups(
            &catalog.by_guid,
            &catalog.by_hash,
            Some(17),
            Some(&"a".repeat(40)),
        );
        assert_eq!(groups[0].1, [0, 2]);
        assert_eq!(groups[1].1, [1, 2]);
    }
    #[test]
    fn graph_failure_preserves_prefix_and_cancel_preserves_cached_prefix() {
        let data = synthetic_bytes(); // Valid prefix, missing graph count fields.
        let loaded = inspect_metadata(&data, &LoadControl::default(), true, false).unwrap();
        assert!(loaded.graph.as_ref().unwrap().is_err());
        assert_eq!(loaded.prefix.prefix.scalars.alpha_test_bits, 0x7fc00001);
        let ctx = egui::Context::default();
        let (tx, result) = mpsc::sync_channel(1);
        let mut view = MaterialView {
            job: Some(MaterialJob {
                control: LoadControl::default(),
                result,
            }),
            graph_requested: true,
            ..Default::default()
        };
        tx.send(Ok(loaded)).unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_some() && view.graph.is_none());
        assert!(!view.graph_error.is_empty() && view.error.is_empty());
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        control.cancel();
        view.job = Some(MaterialJob { control, result });
        tx.send(Err("cancelled".into())).unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_some());
        view.reset();
        assert!(view.graph_requested && view.graph_error.is_empty() && view.metadata.is_none());
    }
    fn synthetic_graph_bytes(tail: bool) -> Vec<u8> {
        let mut bytes = synthetic_bytes();
        let mut extra = vec![0; 8]; // Empty fixed-width graph counts.
        if tail {
            extra.extend_from_slice(&u32::MAX.to_be_bytes());
            extra.extend_from_slice(&0u32.to_be_bytes());
        }
        bytes.splice(bytes.len() - 4..bytes.len() - 4, extra);
        let end = bytes.len() as u32 - 4;
        bytes[8..12].copy_from_slice(&end.to_be_bytes());
        bytes
    }
    #[test]
    fn tail_failure_preserves_graph_prefix_cancel_retains_cache_and_reset_clears() {
        let data = synthetic_graph_bytes(false);
        let loaded = inspect_metadata(&data, &LoadControl::default(), true, true).unwrap();
        assert!(loaded.graph.as_ref().unwrap().is_ok() && loaded.tail.as_ref().unwrap().is_err());
        let ctx = egui::Context::default();
        let (tx, result) = mpsc::sync_channel(1);
        let mut view = MaterialView {
            job: Some(MaterialJob {
                control: LoadControl::default(),
                result,
            }),
            graph_requested: true,
            tail_requested: true,
            ..Default::default()
        };
        tx.send(Ok(loaded)).unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_some() && view.graph.is_some() && view.tail.is_none());
        assert!(
            !view.tail_error.is_empty() && view.graph_error.is_empty() && view.error.is_empty()
        );
        let valid = inspect_metadata(
            &synthetic_graph_bytes(true),
            &LoadControl::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            valid
                .tail
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .sound_enum_raw,
            u32::MAX
        );
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        control.cancel();
        view.job = Some(MaterialJob { control, result });
        tx.send(Ok(valid)).unwrap();
        view.poll(&ctx, None);
        assert!(view.metadata.is_some() && view.graph.is_some() && view.tail.is_none());
        assert!(token.cancelled.load(Ordering::Relaxed));
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        view.job = Some(MaterialJob { control, result });
        view.tail = Some(
            material::inspect_tail(&synthetic_graph_bytes(true), || false)
                .unwrap()
                .tail,
        );
        view.selected_animation = 99;
        view.reset();
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(view.tail_requested && view.graph_requested);
        assert!(
            view.metadata.is_none()
                && view.graph.is_none()
                && view.tail.is_none()
                && view.tail_error.is_empty()
        );
        assert_eq!(view.selected_animation, 0);
        drop(tx);
        let failed_graph =
            inspect_metadata(&synthetic_bytes(), &LoadControl::default(), true, true).unwrap();
        assert!(failed_graph.graph.unwrap().is_err() && failed_graph.tail.unwrap().is_err());
    }
    #[test]
    fn tail_layout_handles_empty_maximum_array_unknown_bits_and_invalid_selection() {
        let ctx = egui::Context::default();
        let metadata = synthetic();
        let mut tail = material::inspect_tail(&synthetic_graph_bytes(true), || false)
            .unwrap()
            .tail;
        for count in [0, 1, material::MAX_PARAMETER_ANIMATIONS] {
            tail.parameter_animation_count = count;
            tail.parameter_animations = (0..count)
                .map(|_| material::AnimationMetadata {
                    base_value_bits: [0x7fc00001, 0x80000000, 0, 0],
                    component_value_count: 4,
                    component_bytes_sha256: "a".repeat(64),
                    name_hex: "00ff".repeat(32),
                    components_raw: 0xf5,
                    derived_key_count: 2,
                })
                .collect();
            for mut selected in [0, count.saturating_sub(1), usize::MAX] {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(480., 640.),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        tail_details(ui, &metadata, &tail, &mut selected);
                    },
                );
                assert!(!out.shapes.is_empty());
                out.textures_delta.clear();
            }
        }
    }
    #[test]
    fn graph_layout_preserves_unknowns_bad_links_and_bounded_animation_summaries() {
        use material::{AnimationMetadata, BoxMetadata, WireMetadata};
        let animation = || AnimationMetadata {
            base_value_bits: [0x7fc00001, 0x80000000, 0, 0],
            component_value_count: 0,
            component_bytes_sha256: "a".repeat(64),
            name_hex: "0000ff".into(),
            components_raw: 255,
            derived_key_count: 0,
        };
        let mut graph = GraphMetadata {
            box_count: 2,
            wire_count: 1,
            graph_start: 100,
            parsed_graph_bytes: 80,
            unparsed_bytes: 2,
            boxes: vec![
                BoxMetadata {
                    type_raw: u32::MAX,
                    declared_parameter_count: 8,
                    parameters_raw: [u32::MAX; 8],
                    position_bits: [0x7fc00001; 4],
                    subtype_raw: u32::MAX,
                    animations: [animation(), animation()],
                    texture_slot_raw: None,
                    texture_slot_in_range: None,
                },
                BoxMetadata {
                    type_raw: 1,
                    declared_parameter_count: 8,
                    parameters_raw: [99; 8],
                    position_bits: [0; 4],
                    subtype_raw: 1,
                    animations: [animation(), animation()],
                    texture_slot_raw: Some(99),
                    texture_slot_in_range: Some(false),
                },
            ],
            wires: vec![WireMetadata {
                box_from: i32::MIN,
                box_to: i32::MAX,
                port_from_raw: 255,
                port_to_raw: 255,
                swizzle_raw: [255; 5],
                from_in_range: false,
                to_in_range: false,
            }],
        };
        let ctx = egui::Context::default();
        let catalog =
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap();
        let metadata = synthetic();
        let mut jump = None;
        let mut wire = 0;
        for mut selected in [0, 1, usize::MAX] {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                graph_details(
                    ui,
                    &metadata,
                    &graph,
                    &catalog,
                    &mut selected,
                    &mut wire,
                    &mut jump,
                );
            });
            assert!(!out.shapes.is_empty());
            out.textures_delta.clear();
            assert!(jump.is_none());
        }
        graph.boxes.clear();
        graph.wires.clear();
        let mut selected = 0;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            graph_details(
                ui,
                &metadata,
                &graph,
                &catalog,
                &mut selected,
                &mut wire,
                &mut jump,
            );
        });
        out.textures_delta.clear();
        assert_eq!(box_annotation(u32::MAX), "unlabelled");
    }
}
