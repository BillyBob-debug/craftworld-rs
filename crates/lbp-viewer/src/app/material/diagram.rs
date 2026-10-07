//! Authored diagnostic UI layout, not original shader/editor layout or evaluation.
//! Endpoint provenance: existing independent GraphMetadata; see
//! docs/material-graph.md and docs/material-census.md, Toolkit pinned
//! d6a2ef80b5a09999f08f23a803b516096055e5c9 MaterialWire::serialize.
//! No new format rule. Signed indices/ports/swizzle remain raw and authoritative.
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use lbp_resource::material::{GraphMetadata, MAX_BOXES, MAX_WIRES};

const NODE: Vec2 = Vec2::new(128., 48.);
const STEP: Vec2 = Vec2::new(196., 104.);

#[derive(Debug)]
pub(super) struct DiagramView {
    zoom: f32,
    pan: Vec2,
    visible: bool,
}

impl Default for DiagramView {
    fn default() -> Self {
        Self {
            zoom: 1.,
            pan: Vec2::ZERO,
            visible: true,
        }
    }
}

// Index grid never traverses edges, so cycles/self-links cannot cause iteration.
fn layout(count: usize) -> Option<(Vec<Rect>, Rect)> {
    if count > MAX_BOXES {
        return None;
    }
    let columns = ((count.max(1) as f32).sqrt().ceil() as usize).max(1);
    let nodes: Vec<_> = (0..count)
        .map(|i| {
            Rect::from_min_size(
                Pos2::new((i % columns) as f32 * STEP.x, (i / columns) as f32 * STEP.y),
                NODE,
            )
        })
        .collect();
    let bounds = nodes
        .iter()
        .copied()
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::from_min_size(Pos2::ZERO, NODE));
    Some((nodes, bounds.expand(40.)))
}
fn endpoint(index: i32, count: usize) -> Option<usize> {
    usize::try_from(index).ok().filter(|&n| n < count)
}
fn project(point: Pos2, world: Rect, canvas: Rect, zoom: f32, pan: Vec2) -> Pos2 {
    canvas.center() + (point - world.center()) * zoom + pan
}
fn fit(world: Rect, canvas: Rect) -> f32 {
    (canvas.width() / world.width())
        .min(canvas.height() / world.height())
        .max(0.0001)
}
fn route(from: Rect, to: Rect, same: bool) -> [Pos2; 17] {
    let a = from.right_center();
    let d = to.left_center();
    let (b, c) = if same {
        (a + Vec2::new(50., -90.), d + Vec2::new(-50., -90.))
    } else {
        let reach = (d.x - a.x).abs().max(64.) * 0.5;
        (a + Vec2::new(reach, 0.), d - Vec2::new(reach, 0.))
    };
    std::array::from_fn(|i| {
        let t = i as f32 / 16.;
        let u = 1. - t;
        (a.to_vec2() * (u * u * u)
            + b.to_vec2() * (3. * u * u * t)
            + c.to_vec2() * (3. * u * t * t)
            + d.to_vec2() * (t * t * t))
            .to_pos2()
    })
}
fn distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let v = b - a;
    let t = if v.length_sq() > 0. {
        ((point - a).dot(v) / v.length_sq()).clamp(0., 1.)
    } else {
        0.
    };
    point.distance(a + v * t)
}
impl DiagramView {
    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        graph: &GraphMetadata,
        selected_box: &mut usize,
        selected_wire: &mut usize,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.visible, "Graph diagram");
            if self.visible {
                if ui.button("Fit").clicked() {
                    self.zoom = 1.;
                    self.pan = Vec2::ZERO;
                }
                if ui.button("−").clicked() {
                    self.zoom = (self.zoom / 1.25).max(0.25);
                }
                if ui.button("+").clicked() {
                    self.zoom = (self.zoom * 1.25).min(16.);
                }
                ui.small("Drag to pan · scroll to zoom · select box or wire");
            }
        });
        if !self.visible {
            return;
        }
        let Some((nodes, mut world)) = layout(graph.boxes.len()) else {
            ui.colored_label(
                Color32::YELLOW,
                "Diagram box budget exceeded; metadata remains below.",
            );
            return;
        };
        if graph.wires.len() > MAX_WIRES {
            ui.colored_label(
                Color32::YELLOW,
                "Diagram wire budget exceeded; metadata remains below.",
            );
            return;
        }
        // Include routed curves in Fit, including backwards and self links.
        for wire in &graph.wires {
            if let (Some(a), Some(b)) = (
                endpoint(wire.box_from, nodes.len()),
                endpoint(wire.box_to, nodes.len()),
            ) {
                for point in route(nodes[a], nodes[b], a == b) {
                    world = world.union(Rect::from_center_size(point, Vec2::splat(20.)));
                }
            }
        }
        ui.small(format!(
            "{} boxes · {} wires · diagnostic index layout (original positions not applied)",
            graph.boxes.len(),
            graph.wires.len()
        ));
        let height = (ui.available_height() * 0.48).clamp(180., 360.);
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width().max(1.), height),
            egui::Sense::click_and_drag(),
        );
        let canvas = response.rect;
        if response.dragged() {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                self.zoom =
                    (self.zoom * (scroll.clamp(-100., 100.) * 0.005).exp()).clamp(0.25, 16.);
            }
        }
        self.pan.x = self.pan.x.clamp(-10000., 10000.);
        self.pan.y = self.pan.y.clamp(-10000., 10000.);
        let scale = fit(world, canvas) * self.zoom;
        let transform = |p| project(p, world, canvas, scale, self.pan);
        painter.rect_filled(canvas, 4., Color32::from_rgb(22, 27, 34));
        let pointer = response.hover_pos();
        let mut hovered_wire = None;
        let mut nearest = 7.;
        let mut invalid = Vec::new();
        // Selected wire drawn last so it stays legible over crossing edges.
        let order = (0..graph.wires.len())
            .filter(|i| *i != *selected_wire)
            .chain((*selected_wire < graph.wires.len()).then_some(*selected_wire));
        for i in order {
            let w = &graph.wires[i];
            let (Some(a), Some(b)) = (
                endpoint(w.box_from, nodes.len()),
                endpoint(w.box_to, nodes.len()),
            ) else {
                invalid.push(i);
                continue;
            };
            let points = route(nodes[a], nodes[b], a == b).map(transform);
            let color = if i == *selected_wire {
                Color32::from_rgb(255, 205, 97)
            } else {
                Color32::from_rgb(95, 135, 160)
            };
            for segment in points.windows(2) {
                painter.line_segment(
                    [segment[0], segment[1]],
                    Stroke::new(if i == *selected_wire { 2.5 } else { 1. }, color),
                );
                if let Some(p) = pointer {
                    let d = distance(p, segment[0], segment[1]);
                    if d < nearest {
                        nearest = d;
                        hovered_wire = Some(i);
                    }
                }
            }
            let end = points[16];
            let direction = (end - points[15]).normalized();
            let normal = Vec2::new(-direction.y, direction.x);
            painter.line_segment(
                [end, end - direction * 8. + normal * 4.],
                Stroke::new(1.5, color),
            );
            painter.line_segment(
                [end, end - direction * 8. - normal * 4.],
                Stroke::new(1.5, color),
            );
        }
        let mut hovered_box = None;
        for (i, node) in nodes.iter().enumerate() {
            let rect = Rect::from_min_max(transform(node.min), transform(node.max));
            if !rect.intersects(canvas) {
                continue;
            }
            let selected = i == *selected_box;
            painter.rect_filled(
                rect,
                4.,
                if selected {
                    Color32::from_rgb(47, 90, 81)
                } else {
                    Color32::from_rgb(40, 48, 59)
                },
            );
            painter.rect_stroke(
                rect,
                4.,
                Stroke::new(
                    if selected { 2. } else { 1. },
                    if selected {
                        super::ACCENT
                    } else {
                        Color32::GRAY
                    },
                ),
                egui::StrokeKind::Inside,
            );
            if rect.width() >= 45. && rect.height() >= 20. {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("#{i}\ntype {}", graph.boxes[i].type_raw),
                    egui::FontId::monospace((12. * scale).clamp(9., 16.)),
                    Color32::WHITE,
                );
            }
            if pointer.is_some_and(|p| rect.contains(p)) {
                hovered_box = Some(i);
            }
        }
        if graph.boxes.is_empty() {
            painter.text(
                canvas.center(),
                egui::Align2::CENTER_CENTER,
                "No serialized boxes",
                egui::FontId::proportional(14.),
                Color32::GRAY,
            );
        }
        if response.clicked() {
            if let Some(i) = hovered_box {
                *selected_box = i;
            } else if let Some(i) = hovered_wire {
                *selected_wire = i;
            }
        }
        if let Some(i) = hovered_box {
            response.on_hover_text(format!(
                "Box {i} · raw type {} · click for metadata",
                graph.boxes[i].type_raw
            ));
        } else if let Some(i) = hovered_wire {
            let w = &graph.wires[i];
            response.on_hover_text(format!(
                "Wire {i}: {}:{} → {}:{} · swizzle {:02x?}",
                w.box_from, w.port_from_raw, w.box_to, w.port_to_raw, w.swizzle_raw
            ));
        }
        if let Some(w) = graph.wires.get(*selected_wire) {
            ui.small(format!(
                "Selected wire {}: {}:{} → {}:{} · swizzle {:02x?}",
                *selected_wire, w.box_from, w.port_from_raw, w.box_to, w.port_to_raw, w.swizzle_raw
            ));
        }
        invalid.sort_unstable();
        if !invalid.is_empty() {
            ui.colored_label(
                Color32::YELLOW,
                format!(
                    "{} wires have invalid endpoints; no invented connection drawn",
                    invalid.len()
                ),
            );
            egui::ScrollArea::vertical()
                .id_salt("diagram_invalid_wires")
                .max_height(65.)
                .show_rows(ui, 22., invalid.len(), |ui, range| {
                    for j in range {
                        let i = invalid[j];
                        let w = &graph.wires[i];
                        if ui
                            .selectable_label(
                                *selected_wire == i,
                                format!(
                                    "Wire {i}: raw {}:{} → {}:{}",
                                    w.box_from, w.port_from_raw, w.box_to, w.port_to_raw
                                ),
                            )
                            .clicked()
                        {
                            *selected_wire = i;
                        }
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lbp_resource::material::{AnimationMetadata, BoxMetadata, WireMetadata};
    fn graph() -> GraphMetadata {
        let animation = || AnimationMetadata {
            base_value_bits: [u32::MAX; 4],
            component_value_count: 0,
            component_bytes_sha256: "a".repeat(64),
            name_hex: String::new(),
            components_raw: 0,
            derived_key_count: 0,
        };
        let wire = |a, b| WireMetadata {
            box_from: a,
            box_to: b,
            port_from_raw: 255,
            port_to_raw: 254,
            swizzle_raw: [255; 5],
            from_in_range: false,
            to_in_range: false,
        };
        GraphMetadata {
            box_count: 3,
            wire_count: 3,
            graph_start: 0,
            parsed_graph_bytes: 0,
            unparsed_bytes: 0,
            boxes: (0..3)
                .map(|_| BoxMetadata {
                    type_raw: u32::MAX,
                    declared_parameter_count: 8,
                    parameters_raw: [u32::MAX; 8],
                    position_bits: [0x7fc00001; 4],
                    subtype_raw: u32::MAX,
                    animations: [animation(), animation()],
                    texture_slot_raw: None,
                    texture_slot_in_range: None,
                })
                .collect(),
            wires: vec![wire(0, 1), wire(1, 1), wire(i32::MIN, i32::MAX)],
        }
    }
    #[test]
    fn grid_fit_is_deterministic_bounded_and_nonoverlapping() {
        for count in [0, 1, 19, MAX_BOXES] {
            let (nodes, world) = layout(count).unwrap();
            assert_eq!(layout(count).unwrap(), (nodes.clone(), world));
            assert_eq!(nodes.len(), count);
            let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(480., 180.));
            for (i, node) in nodes.iter().enumerate() {
                assert!(world.contains_rect(*node));
                assert!(canvas.contains(project(
                    node.min,
                    world,
                    canvas,
                    fit(world, canvas),
                    Vec2::ZERO
                )));
                assert!(canvas.contains(project(
                    node.max,
                    world,
                    canvas,
                    fit(world, canvas),
                    Vec2::ZERO
                )));
                assert!(!nodes
                    .iter()
                    .skip(i + 1)
                    .any(|other| node.intersects(*other)));
            }
        }
        assert!(layout(MAX_BOXES + 1).is_none());
    }
    #[test]
    fn routes_support_cycles_self_links_and_signed_extremes_without_traversal() {
        let (nodes, _) = layout(3).unwrap();
        for (a, b) in [(0, 1), (1, 0), (1, 1)] {
            let points = route(nodes[a], nodes[b], a == b);
            assert_eq!(points[0], nodes[a].right_center());
            assert_eq!(points[16], nodes[b].left_center());
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
            assert_eq!(distance(points[5], points[5], points[5]), 0.);
        }
        assert_eq!(endpoint(-1, 3), None);
        assert_eq!(endpoint(i32::MIN, 3), None);
        assert_eq!(endpoint(i32::MAX, 3), None);
        assert_eq!(endpoint(3, 3), None);
        assert_eq!(endpoint(2, 3), Some(2));
    }
    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(700., 700.))),
            events,
            ..Default::default()
        }
    }
    fn click(p: Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    #[test]
    fn actual_pointer_events_select_boxes_and_wires_without_mutating_metadata() {
        let ctx = egui::Context::default();
        let mut view = DiagramView::default();
        let graph = graph();
        let before = serde_json::to_value(&graph).unwrap();
        let mut selected_box = 0;
        let mut selected_wire = 1;
        let mut out = ctx.run_ui(input(vec![]), |ui| {
            view.ui(ui, &graph, &mut selected_box, &mut selected_wire)
        });
        out.textures_delta.clear();
        let canvas = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.fill == Color32::from_rgb(22, 27, 34) => {
                    Some(r.rect)
                }
                _ => None,
            })
            .unwrap();
        let (nodes, mut world) = layout(3).unwrap();
        for w in &graph.wires {
            if let (Some(a), Some(b)) = (endpoint(w.box_from, 3), endpoint(w.box_to, 3)) {
                for point in route(nodes[a], nodes[b], a == b) {
                    world = world.union(Rect::from_center_size(point, Vec2::splat(20.)));
                }
            }
        }
        let node = project(
            nodes[1].center(),
            world,
            canvas,
            fit(world, canvas),
            Vec2::ZERO,
        );
        let mut out = ctx.run_ui(input(click(node)), |ui| {
            view.ui(ui, &graph, &mut selected_box, &mut selected_wire)
        });
        out.textures_delta.clear();
        assert_eq!(selected_box, 1);
        let wire = project(
            route(nodes[0], nodes[1], false)[8],
            world,
            canvas,
            fit(world, canvas),
            Vec2::ZERO,
        );
        let mut out = ctx.run_ui(input(click(wire)), |ui| {
            view.ui(ui, &graph, &mut selected_box, &mut selected_wire)
        });
        out.textures_delta.clear();
        assert_eq!(selected_wire, 0);
        assert_eq!(serde_json::to_value(&graph).unwrap(), before);
    }
    #[test]
    fn empty_invalid_and_maximum_graphs_render_with_clamped_pan() {
        let ctx = egui::Context::default();
        let mut view = DiagramView {
            pan: Vec2::new(1e30, -1e30),
            ..Default::default()
        };
        let mut box_index = usize::MAX;
        let mut wire_index = usize::MAX;
        let mut data = graph();
        for count in [3, 0, MAX_WIRES, MAX_WIRES + 1] {
            data.wires = (0..count)
                .map(|_| WireMetadata {
                    box_from: i32::MIN,
                    box_to: i32::MAX,
                    port_from_raw: 0,
                    port_to_raw: 0,
                    swizzle_raw: [0; 5],
                    from_in_range: false,
                    to_in_range: false,
                })
                .collect();
            if count == 0 {
                data.boxes.clear();
            }
            let mut out = ctx.run_ui(input(vec![]), |ui| {
                view.ui(ui, &data, &mut box_index, &mut wire_index)
            });
            out.textures_delta.clear();
            assert!(!out.shapes.is_empty());
            assert!(view.pan.x.abs() <= 10000. && view.pan.y.abs() <= 10000.);
        }
    }
}
