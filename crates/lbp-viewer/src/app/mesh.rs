//! Native stored-geometry wireframe/neutral surfaces; bounded worker/cache, no asset export.
use super::*;
use lbp_catalog::attachment::{AttachedSource, LocationKey};
use lbp_mesh::Geometry;
mod export;
use lbp_preview as linked;
mod surface;
use lbp_preview::LoadedMesh;
struct MeshJob {
    control: LoadControl,
    result: mpsc::Receiver<Result<LoadedMesh, String>>,
}
impl Drop for MeshJob {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Default)]
pub(super) struct MeshView {
    source: Option<Arc<AttachedSource>>,
    key: Option<LocationKey>,
    job: Option<MeshJob>,
    geometry: Option<Geometry>,
    error: String,
    candidate: usize,
    yaw: f64,
    pitch: f64,
    zoom: f64,
    pending_jump: Option<usize>,
    export_path: String,
    export_message: String,
    solid: bool,
    qa_surface: bool,
    surface_logged: bool,
    textured: bool,
    linked: Option<linked::MaterialAsset>,
    texture_handle: Option<egui::TextureHandle>,
    texture_error: String,
}
fn project(p: [f32; 3], bounds: [[f32; 3]; 2], yaw: f64, pitch: f64) -> [f64; 2] {
    let p = project3(p, bounds, yaw, pitch);
    [p[0], p[1]]
}
fn project3(p: [f32; 3], bounds: [[f32; 3]; 2], yaw: f64, pitch: f64) -> [f64; 3] {
    let span = (0..3)
        .map(|i| f64::from(bounds[1][i]) - f64::from(bounds[0][i]))
        .fold(0f64, f64::max)
        .max(1e-30);
    let v: [f64; 3] = std::array::from_fn(|i| {
        (f64::from(p[i]) - (f64::from(bounds[0][i]) + f64::from(bounds[1][i])) / 2.) / span
    });
    let x = v[0] * yaw.cos() + v[2] * yaw.sin();
    let z = -v[0] * yaw.sin() + v[2] * yaw.cos();
    [
        x,
        v[1] * pitch.cos() - z * pitch.sin(),
        v[1] * pitch.sin() + z * pitch.cos(),
    ]
}
impl MeshView {
    pub(super) fn new(solid: bool, qa_surface: bool) -> Self {
        Self {
            yaw: 0.5,
            pitch: 0.3,
            zoom: 1.,
            solid,
            qa_surface,
            ..Default::default()
        }
    }
    pub(super) fn take_jump(&mut self) -> Option<usize> {
        self.pending_jump.take()
    }
    pub(super) fn with_textured(mut self, textured: bool) -> Self {
        self.textured = textured;
        self
    }
    pub(super) fn reset(&mut self) {
        *self = Self::new(self.solid, self.qa_surface).with_textured(self.textured);
    }
    pub(super) fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let result = export::save(self, path);
        self.export_message = match &result {
            Ok(()) => "Saved cached mesh diagnostic JSON. Game data was not reread.".into(),
            Err(error) => format!("Export failed: {error}"),
        };
        result
    }
    fn start(&mut self, ctx: &egui::Context, catalog: Arc<Catalog>) {
        self.job = None;
        self.geometry = None;
        self.linked = None;
        self.texture_handle = None;
        self.texture_error.clear();
        self.error.clear();
        self.export_message.clear();
        self.surface_logged = false;
        let (Some(source), Some(key)) = (self.source.clone(), self.key.clone()) else {
            return;
        };
        let control = LoadControl::default();
        let worker = control.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::sync_channel(1);
        let textured = self.textured;
        std::thread::spawn(move || {
            let _ = tx.send(lbp_preview::load_mesh(
                &source, &catalog, &key, textured, &worker,
            ));
            ctx.request_repaint();
        });
        self.job = Some(MeshJob { control, result });
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
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Mesh worker ended".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(r) = result {
            let job = self.job.take().unwrap();
            if job.control.cancelled.load(Ordering::Relaxed) {
                self.error = "Mesh decode cancelled".into();
            } else {
                match r {
                    Ok(loaded) => {
                        let g = loaded.geometry;
                        println!(
                            "Mesh preview: {} vertices, {} triangles",
                            g.positions.len(),
                            g.triangles.len()
                        );
                        self.geometry = Some(g);
                        match loaded.linked {
                            Some(Ok(asset)) => {
                                let material = asset;
                                if material.texture().is_some_and(|a| a.fragment.is_some()) {
                                    // D189: no per-fragment graph path here (docs/d189-mixed-uv-graphs.md).
                                    self.texture_error="Unsupported in legacy viewer: per-fragment UV0/UV1 graph; use Bevy viewer".into();
                                    println!("{}", self.texture_error);
                                    return;
                                }
                                if material.texture().is_some_and(|a| a.dual.is_some()) {
                                    self.texture_error="Unsupported in legacy viewer: independent two-texture sampling; use Bevy viewer".into();
                                    println!("{}", self.texture_error);
                                    return;
                                }
                                if material
                                    .texture()
                                    .is_some_and(|a| a.wrap.uniform_axis().is_none())
                                {
                                    self.texture_error="Unsupported in legacy viewer: independent U/V wrapping; use Bevy viewer".into();
                                    println!("{}", self.texture_error);
                                    return;
                                }
                                if let Some(asset) = material.texture() {
                                    let image = egui::ColorImage::from_rgba_unmultiplied(
                                        asset.display_extent().map(|v| v as usize),
                                        asset.display_pixels(),
                                    );
                                    let mut options = match asset.wrap.uniform_axis() {
                                        Some(lbp_preview::TextureAddress::Repeat) => {
                                            egui::TextureOptions::LINEAR_REPEAT
                                        }
                                        Some(lbp_preview::TextureAddress::MirroredRepeat) => {
                                            egui::TextureOptions::LINEAR_MIRRORED_REPEAT
                                        }
                                        Some(
                                            lbp_preview::TextureAddress::ClampToEdge
                                            | lbp_preview::TextureAddress::Clamp,
                                        ) => egui::TextureOptions::LINEAR,
                                        None => unreachable!(
                                            "mixed axes rejected before image installation"
                                        ),
                                    };
                                    if asset.nearest_sampling() {
                                        options.magnification = egui::TextureFilter::Nearest;
                                        options.minification = egui::TextureFilter::Nearest;
                                    }
                                    self.texture_handle = Some(ctx.load_texture(
                                        "mesh direct diffuse",
                                        image,
                                        options,
                                    ));
                                    println!(
                                        "Linked diffuse preview: {}",
                                        serde_json::json!({
                                    "material":{"path":asset.material_key.path,"offset":asset.material_key.offset,"size":asset.material_key.size},
                                    "texture":{"path":asset.texture_key.path,"offset":asset.texture_key.offset,"size":asset.texture_key.size},
                                    "width":asset.texture.header.width,"height":asset.texture.header.height,
                                    "display_extent":asset.display_extent(),"product":asset.product.as_ref().map(|p|serde_json::json!({"second_texture":{"path":p.texture_key.path,"offset":p.texture_key.offset,"size":p.texture_key.size},"filter":"nearest"})),
                                    "uv":asset.uv_metadata,"selected_uv_channel":asset.selection.uv_channel,"sampling_transform_subtype":asset.selection.uv_transform.subtype(),"sampling_transform_bits":asset.selection.uv_transform.bits(),"diffuse_tint_bits":asset.selection.tint.bits,"multiply_index":asset.selection.multiply_index,"wrap_uv_raw":asset.source_wrap_raw(),"display_wrap_uv_raw":asset.wrap.raw()})
                                    );
                                }
                                println!("Diffuse material: {}", material.label());
                                self.linked = Some(material);
                            }
                            Some(Err(error)) => {
                                println!("Linked diffuse unavailable: {error}");
                                self.texture_error = error;
                            }
                            None => {}
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
        catalog: Arc<Catalog>,
    ) {
        self.poll(ui.ctx(), source);
        if self.source.is_none() {
            ui.label("Attach and validate a source to preview meshes.");
            return;
        }
        if locations.is_empty() {
            ui.label("No available resource candidate.");
            return;
        }
        self.candidate = self.candidate.min(locations.len() - 1);
        egui::ComboBox::from_id_salt("mesh_candidate")
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
            self.start(ui.ctx(), catalog.clone());
        }
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.textured && !self.solid, "Wireframe")
                .clicked()
            {
                self.solid = false;
                self.textured = false;
                self.surface_logged = false;
            }
            if ui
                .selectable_label(!self.textured && self.solid, "Neutral solid")
                .clicked()
            {
                self.solid = true;
                self.textured = false;
                self.surface_logged = false;
            }
            if ui
                .selectable_label(self.textured, "Direct diffuse")
                .clicked()
            {
                self.textured = true;
                if self.linked.is_none() && self.job.is_none() {
                    self.start(ui.ctx(), catalog.clone());
                }
            }
            if ui.button("Reload mesh").clicked() {
                self.start(ui.ctx(), catalog.clone());
            }
            if let Some(j) = &self.job {
                ui.spinner();
                if ui.button("Cancel").clicked() {
                    j.control.cancel();
                }
            }
            if ui.button("Reset view").clicked() {
                self.yaw = 0.5;
                self.pitch = 0.3;
                self.zoom = 1.;
            }
        });
        if !self.error.is_empty() {
            ui.colored_label(Color32::YELLOW, &self.error);
            ui.small(
                "Supported: ordinary clear MSH, triangle lists/strips, bounded stored geometry.",
            );
        }
        egui::CollapsingHeader::new("Export cached mesh diagnostic JSON").show(ui, |ui| {
            ui.small("Choose a new .json file under local-analysis. Metadata only; no geometry arrays or extra game reads.");
            ui.add(egui::TextEdit::singleline(&mut self.export_path)
                .hint_text("local-analysis/mesh-diagnostic.json").desired_width(f32::INFINITY));
            if ui.add_enabled(self.job.is_none() && (self.geometry.is_some() || !self.error.is_empty()),
                              egui::Button::new("Save diagnostic JSON")).clicked() {
                let path = PathBuf::from(&self.export_path);
                let _ = self.export_to(&path);
            }
        });
        if !self.export_message.is_empty() {
            ui.small(&self.export_message);
        }
        let Some(g) = &self.geometry else {
            return;
        };
        ui.label(format!(
            "{} vertices · {} triangles · {} primitives · {} bones",
            g.positions.len(),
            g.triangles.len(),
            g.metadata.primitives.len(),
            g.metadata.bone_count
        ));
        ui.small(
            "Stored geometry. Bones and morphs are not applied. Drag to orbit; scroll to zoom.",
        );
        if self.textured {
            ui.small("Direct diffuse diagnostic: unlit top mip, static UV0/UV1. Average-depth ordering; no game shader reproduction.");
            if let Some(asset) = &self.linked {
                ui.small(asset.label());
                if let Some(asset) = asset.texture() {
                    ui.small(format!("Diffuse-only projection: ignores {} nodes / {} wires; other material outputs are not reproduced.",asset.selection.ignored_nodes,asset.selection.ignored_wires));
                    ui.small(asset.selection.tint.label());
                    ui.small(format!(
                        "UV{} · {} · {}×{} · {} · {}",
                        asset.selection.uv_channel,
                        asset.selection.uv_transform.label(),
                        asset.texture.header.width,
                        asset.texture.header.height,
                        asset.texture.header.pixel_format,
                        asset.wrap.label()
                    ));
                }
            }
            if !self.texture_error.is_empty() {
                ui.colored_label(Color32::YELLOW, &self.texture_error);
            }
        } else if self.solid {
            ui.small("Double-sided flat shading; average-depth ordering can misorder intersecting faces.");
        }
        ui.small(format!(
            "{} bytes after topology remain unparsed",
            g.metadata.unparsed_after_geometry
        ));
        egui::CollapsingHeader::new("Material references")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(120.).show_rows(
                    ui,
                    24.,
                    g.metadata.primitives.len(),
                    |ui, range| {
                        for i in range {
                            let p = &g.metadata.primitives[i];
                            ui.horizontal(|ui| {
                                ui.small(format!("Primitive {i}"));
                                let groups = reference_groups(
                                    &catalog.by_guid,
                                    &catalog.by_hash,
                                    p.material_guid,
                                    p.material_hash.as_deref(),
                                );
                                if groups.is_empty() {
                                    ui.small("No material reference");
                                }
                                for (label, targets) in groups {
                                    let unresolved = targets.is_empty();
                                    ui.push_id((i, &label), |ui| {
                                        ui.menu_button(
                                            format!("{} · {} candidates", label, targets.len()),
                                            |ui| {
                                                ui.monospace(&label);
                                                if unresolved {
                                                    ui.label("Unresolved or raw null reference");
                                                }
                                                egui::ScrollArea::vertical()
                                                    .max_height(160.)
                                                    .show_rows(
                                                        ui,
                                                        24.,
                                                        targets.len(),
                                                        |ui, range| {
                                                            for j in range {
                                                                let target = targets[j];
                                                                let row = &catalog
                                                                    .snapshot
                                                                    .resources[target]
                                                                    .map_row;
                                                                if ui
                                                                    .button(format!(
                                                                        "{} · {}",
                                                                        row.guid, row.path
                                                                    ))
                                                                    .clicked()
                                                                {
                                                                    self.pending_jump =
                                                                        Some(target);
                                                                    ui.close();
                                                                }
                                                            }
                                                        },
                                                    );
                                            },
                                        );
                                    });
                                }
                            });
                        }
                    },
                );
            });
        egui::CollapsingHeader::new("Geometry metadata").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(160.)
                .show(ui, |ui| {
                    ui.monospace(serde_json::to_string_pretty(&g.metadata).unwrap_or_default());
                });
        });
        let size = egui::vec2(ui.available_width().max(1.), ui.available_height().max(80.));
        let (response, painter) = ui.allocate_painter(size, egui::Sense::drag());
        if response.dragged() {
            let d = ui.input(|i| i.pointer.delta());
            self.yaw += f64::from(d.x) * 0.01;
            self.pitch = (self.pitch + f64::from(d.y) * 0.01).clamp(-1.55, 1.55);
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.zoom = (self.zoom * (f64::from(scroll) * 0.003).exp()).clamp(0.1, 10.);
        }
        painter.rect_filled(response.rect, 0., Color32::from_rgb(18, 23, 25));
        let Some(bounds) = g.metadata.prefix.position_bounds else {
            return;
        };
        let center = response.rect.center();
        let scale = f64::from(size.x.min(size.y)) * 0.8 * self.zoom;
        if self.textured {
            let Some(asset) = &self.linked else {
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    "Direct diffuse unavailable; see diagnostic above",
                    egui::FontId::proportional(12.),
                    Color32::YELLOW,
                );
                return;
            };
            match surface::Scene::build(g, bounds, self.yaw, self.pitch).and_then(|s| match asset {
                linked::MaterialAsset::Texture(a) => s.textured_mesh(
                    center,
                    scale,
                    &a.sampling_uv,
                    self.texture_handle
                        .as_ref()
                        .ok_or("Missing diffuse texture handle")?
                        .id(),
                ),
                linked::MaterialAsset::Solid(a) => {
                    let rgba = a
                        .selection
                        .color
                        .pixels(&[255; 4], || false)?
                        .unwrap_or_else(|| vec![255; 4]);
                    s.color_mesh(center, scale, Color32::from_rgb(rgba[0], rgba[1], rgba[2]))
                }
            }) {
                Ok(mesh) => {
                    painter.add(egui::Shape::mesh(mesh));
                }
                Err(error) => {
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        error,
                        egui::FontId::proportional(12.),
                        Color32::YELLOW,
                    );
                }
            }
            return;
        }
        if self.solid {
            let result = surface::Scene::build(g, bounds, self.yaw, self.pitch).and_then(|scene| {
                let mesh = scene.mesh(center, scale)?;
                Ok((scene, mesh))
            });
            match result {
                Ok((scene, mesh)) => {
                    painter.add(egui::Shape::mesh(mesh));
                    if self.qa_surface && !self.surface_logged {
                        println!(
                            "QA solid mesh preview: {}",
                            serde_json::json!({
                            "triangles":g.triangles.len(),"surface_faces":scene.face_count(),
                            "skipped_degenerate_faces":scene.skipped,"vertices":g.positions.len(),
                            "ordering":"average_camera_depth","shading":"generated_two_sided_flat"})
                        );
                        self.surface_logged = true;
                    }
                }
                Err(error) => {
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        error,
                        egui::FontId::proportional(12.),
                        Color32::YELLOW,
                    );
                }
            }
            return;
        }
        let projected = g
            .positions
            .iter()
            .map(|&p| {
                let p = project(p, bounds, self.yaw, self.pitch);
                egui::pos2(
                    center.x + (p[0] * scale) as f32,
                    center.y - (p[1] * scale) as f32,
                )
            })
            .collect::<Vec<_>>();
        let stroke = egui::Stroke::new(1., ACCENT);
        for t in &g.triangles {
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                continue;
            }
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                painter.line_segment([projected[a as usize], projected[b as usize]], stroke);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn cached() -> MeshView {
        let prefix = lbp_mesh::MeshPrefix {
            revision: 950,
            compression_flags: 7,
            counts: [3, 3, 0, 0, 1, 0, 0],
            morph_names_hex: vec!["00ff".into()],
            min_uv_bits: vec![0x80000000],
            max_uv_bits: vec![0x7fc00001],
            area_scale_bits: Some(u32::MAX),
            stream_offsets: vec![0, 48],
            position_bounds: Some([[-f32::MAX; 3], [f32::MAX; 3]]),
            position_bounds_bits: Some([[(-f32::MAX).to_bits(); 3], [f32::MAX.to_bits(); 3]]),
            position_bytes_sha256: "a".repeat(64),
            attribute_bytes: 0,
            attribute_sha256: "b".repeat(64),
            index_bytes: 6,
            index_sha256: "c".repeat(64),
            expanded_bytes: 400,
            parsed_prefix_bytes: 300,
            unparsed_bytes: 100,
        };
        let primitive = lbp_mesh::Primitive {
            material_flags: u32::MAX,
            material_kind: 3,
            material_guid: Some(0),
            material_hash: Some("0".repeat(40)),
            alternatives_flags: 3,
            alternatives_kind: 3,
            alternatives_guid: Some(u32::MAX),
            alternatives_hash: Some("f".repeat(40)),
            min_vertex_raw: 0,
            max_vertex_raw: 2,
            first_index: 0,
            index_count: 3,
            region: 9,
        };
        MeshView {
            key: Some(LocationKey {
                path: "USRDIR/authored.farc".into(),
                offset: 64,
                size: 100,
            }),
            geometry: Some(Geometry {
                metadata: lbp_mesh::GeometryMetadata {
                    prefix,
                    primitives: vec![primitive],
                    bone_count: 1,
                    primitive_type: 5,
                    triangle_count: 1,
                    triangle_indices_sha256: "d".repeat(64),
                    parsed_geometry_bytes: 350,
                    unparsed_after_geometry: 50,
                },
                positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                triangles: vec![[0, 1, 2]],
                primitive_triangles: std::iter::once(0..1).collect(),
                skeleton: Default::default(),
            }),
            ..Default::default()
        }
    }
    #[test]
    fn mesh_export_whitelists_metadata_preserves_ieee_bits_and_both_reference_candidates() {
        let mut view = cached();
        let report = export::report(&view).unwrap();
        let metadata = &report["stages"]["mesh"]["metadata"];
        assert_eq!(metadata.as_object().unwrap().len(), 9);
        assert_eq!(metadata["vertex_count"], 3);
        for name in ["positions", "triangles", "indices", "bones", "pixels"] {
            assert!(metadata.get(name).is_none());
        }
        assert_eq!(
            metadata["prefix"],
            serde_json::to_value(&view.geometry.as_ref().unwrap().metadata.prefix).unwrap()
        );
        assert_eq!(
            metadata["primitives"],
            serde_json::to_value(&view.geometry.as_ref().unwrap().metadata.primitives).unwrap()
        );
        assert_eq!(metadata["primitives"][0]["material_guid"], 0);
        assert_eq!(metadata["primitives"][0]["material_hash"], "0".repeat(40));
        view.yaw = 5.;
        view.pitch = -1.;
        view.zoom = 8.;
        view.solid = true;
        view.pending_jump = Some(12);
        let geometry = view.geometry.as_mut().unwrap();
        geometry.positions[0] = [99.; 3];
        geometry.triangles[0] = [u32::MAX; 3];
        assert_eq!(export::report(&view).unwrap(), report);
    }
    #[test]
    fn mesh_export_refuses_busy_uncompleted_and_reset_and_preserves_failure_categories() {
        let mut view = cached();
        view.geometry = None;
        assert!(export::report(&view).is_err());
        for (error, status) in [
            ("Unsupported: static mesh", "unsupported"),
            ("Blocked protected mesh", "blocked"),
            ("Mesh read cancelled", "cancelled"),
            ("Wireframe preview budget exceeded", "failed"),
            ("Source read failed: C:\\private\\game\\file", "failed"),
        ] {
            view.error = error.into();
            let report = export::report(&view).unwrap();
            assert_eq!(report["stages"]["mesh"]["status"], status);
            assert!(report["stages"]["mesh"]["metadata"].is_null());
            assert!(!report.to_string().contains("private"));
        }
        let (_tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        view.job = Some(MeshJob {
            control: control.clone(),
            result,
        });
        assert!(export::report(&view).is_err());
        view.export_message = "old".into();
        view.reset();
        assert!(control.cancelled.load(Ordering::Relaxed));
        assert!(view.export_message.is_empty() && export::report(&view).is_err());
    }
    #[test]
    fn material_candidates_preserve_aliases_conflicts_and_raw_nulls() {
        let guid = std::collections::HashMap::from([
            ("0000002a".into(), vec![0, 2]),
            ("00000000".into(), vec![9]),
        ]);
        let hash = std::collections::HashMap::from([
            ("a".repeat(40), vec![1, 2]),
            ("0".repeat(40), vec![8]),
        ]);
        let groups = reference_groups(&guid, &hash, Some(42), Some(&"a".repeat(40)));
        assert_eq!(groups[0].1, [0, 2]);
        assert_eq!(groups[1].1, [1, 2]);
        let nulls = reference_groups(&guid, &hash, Some(0), Some(&"0".repeat(40)));
        assert!(nulls.iter().all(|(_, v)| v.is_empty()));
        assert!(reference_groups(&guid, &hash, Some(7), None)[0]
            .1
            .is_empty());
    }
    #[test]
    fn extreme_finite_bounds_project_without_overflow() {
        let p = project([f32::MAX, 0., 0.], [[-f32::MAX; 3], [f32::MAX; 3]], 0., 0.);
        assert_eq!(p, [0.5, 0.]);
    }
    #[test]
    fn reset_cancels_and_drops_geometry() {
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        let token = control.clone();
        let mut view = MeshView {
            job: Some(MeshJob { control, result }),
            solid: true,
            textured: true,
            texture_error: "old linked failure".into(),
            qa_surface: true,
            ..Default::default()
        };
        view.reset();
        assert!(token.cancelled.load(Ordering::Relaxed));
        assert!(view.geometry.is_none());
        assert!(view.solid && view.qa_surface && !view.surface_logged);
        assert!(
            view.textured
                && view.linked.is_none()
                && view.texture_handle.is_none()
                && view.texture_error.is_empty()
        );
        drop(tx);
    }
    #[test]
    fn linked_failure_keeps_geometry_and_cancelled_worker_cannot_publish() {
        let mut original = cached();
        let geometry = original.geometry.take().unwrap();
        let (tx, result) = mpsc::sync_channel(1);
        let mut view = MeshView {
            job: Some(MeshJob {
                control: LoadControl::default(),
                result,
            }),
            ..Default::default()
        };
        tx.send(Ok(LoadedMesh {
            geometry,
            linked: Some(Err("Unsupported: complex material".into())),
        }))
        .unwrap();
        view.poll(&egui::Context::default(), None);
        assert!(view.geometry.is_some() && view.error.is_empty());
        assert!(view.texture_error.contains("Unsupported") && view.linked.is_none());
        let geometry = view.geometry.take().unwrap();
        let (tx, result) = mpsc::sync_channel(1);
        let control = LoadControl::default();
        control.cancel();
        view.job = Some(MeshJob { control, result });
        tx.send(Ok(LoadedMesh {
            geometry,
            linked: None,
        }))
        .unwrap();
        view.poll(&egui::Context::default(), None);
        assert!(view.geometry.is_none() && view.error.contains("cancelled"));
    }
}
