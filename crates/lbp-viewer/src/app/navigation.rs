//! Authored bounded UI route history. No resource reads or format interpretation.
use super::*;
const MAX_VISITS: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Overview(bool),
    Map(usize),
    Entry(usize, usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn demo() -> Explorer {
        Explorer::new(
            &eframe::CreationContext::_new_kittest(egui::Context::default()),
            Options {
                demo: true,
                ..Default::default()
            },
        )
    }
    #[test]
    fn cross_mode_roundtrip_restores_tabs_and_scroll_targets() {
        let mut app = demo();
        app.select(0, false);
        app.tab = Tab::Mesh;
        app.select_entry(0, 1);
        app.show_overview(true);
        app.navigate(1);
        app.filter.query = "hides everything".into();
        assert!(app.travel(true));
        assert_eq!(
            app.current_visit(),
            Visit {
                route: Route::Overview(true),
                tab: Tab::Mesh
            }
        );
        assert!(app.raw.is_empty());
        assert_eq!(app.filter, Filter::default());
        assert!(app.travel(true));
        assert_eq!(app.entry_selection, Some((0, 1)));
        assert_eq!(app.entry_jump, Some((0, 1)));
        assert_eq!(app.tab, Tab::Mesh);
        assert!(app.travel(true));
        assert_eq!(app.selected, Some(0));
        assert_eq!(app.jump, Some(0));
        assert_eq!(app.inspection, Some(0));
        assert_eq!(app.entry_jump, None);
        assert!(!app.archive_mode);
        assert_eq!(app.tab, Tab::Mesh);
        for _ in 0..3 {
            assert!(app.travel(false));
        }
        assert_eq!(app.selected, Some(1));
        assert_eq!(app.tab, Tab::Metadata);
        assert!(app.history.forward.is_empty());
    }
    #[test]
    fn invalid_duplicate_and_branch_routes_preserve_expected_history() {
        let mut app = demo();
        app.select(0, true);
        app.navigate(1);
        assert!(app.travel(true));
        let before = app.current_visit();
        let lengths = (app.history.back.len(), app.history.forward.len());
        app.filter.query = "keep".into();
        for route in [
            Route::Map(usize::MAX),
            Route::Entry(usize::MAX, 0),
            Route::Entry(0, usize::MAX),
        ] {
            assert!(!app.transition(
                Visit {
                    route,
                    tab: Tab::Raw
                },
                true,
                true
            ));
            assert_eq!(app.current_visit(), before);
            assert_eq!(app.filter.query, "keep");
        }
        app.select(0, true);
        assert_eq!((app.history.back.len(), app.history.forward.len()), lengths);
        app.select(2, true);
        assert!(app.history.forward.is_empty());
        app.history.back.push(Visit {
            route: Route::Map(usize::MAX),
            tab: Tab::Raw,
        });
        assert!(app.travel(true));
        assert_eq!(app.selected, Some(0));
        app.select_entry(0, 0);
        assert_eq!(app.tab, Tab::Hex);
        app.tab = Tab::Raw;
        app.select_entry(0, 0);
        assert_eq!(app.tab, Tab::Hex);
    }
    #[test]
    fn bounded_visits_and_catalog_replacement_clear_both_directions() {
        let mut app = demo();
        for i in 0..400 {
            app.select(i % 2, true);
        }
        assert_eq!(app.history.back.len(), MAX_VISITS);
        for _ in 0..MAX_VISITS {
            assert!(app.travel(true));
        }
        assert!(!app.travel(true));
        assert_eq!(app.history.forward.len(), MAX_VISITS);
        assert!(app.travel(false));
        assert!(app.history.can_back() && app.history.can_forward());
        app.set_catalog(
            Catalog::build(lbp_catalog::demo_snapshot(), &LoadControl::default()).unwrap(),
            "replacement".into(),
        );
        assert!(!app.history.can_back() && !app.history.can_forward());
        assert_eq!(app.selected, None);
        assert_eq!(app.entry_selection, None);
        assert_eq!(app.jump, None);
        assert_eq!(app.entry_jump, None);
    }
    fn frame(
        app: &mut Explorer,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let catalog = app.catalog.clone().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(980.0, 640.0),
                )),
                events,
                ..Default::default()
            },
            |ui| app.detail(ui, &catalog),
        );
        output.textures_delta.clear();
        output
    }
    fn button(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    (text.galley.text() == label)
                        .then(|| text.pos + text.galley.rect.center().to_vec2())
                } else {
                    None
                }
            })
            .expect("rendered toolbar label")
    }
    #[test]
    fn pointer_buttons_navigate_before_old_detail_is_rendered() {
        let mut app = demo();
        let ctx = egui::Context::default();
        app.select(0, false);
        app.navigate(1);
        for (label, expected) in [
            ("Back", Route::Map(0)),
            ("Forward", Route::Map(1)),
            ("Overview", Route::Overview(false)),
        ] {
            let output = frame(&mut app, &ctx, vec![]);
            let pos = button(&output, label);
            frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
            );
            let output = frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                }],
            );
            assert_eq!(app.current_visit().route, expected);
            // Detail returns immediately on a route change; only toolbar text appears.
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape {
                        Some(text.galley.text())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(
                texts,
                [
                    "Back",
                    "Forward",
                    "Overview",
                    "Alt+Left: Back · Alt+Right: Forward"
                ]
            );
        }
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        }
    }
    fn shortcuts(
        app: &mut Explorer,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> (bool, Vec<egui::Event>) {
        let mut changed = false;
        let mut remaining = vec![];
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                changed = app.navigation_shortcuts(ui.ctx());
                remaining = ui.input(|input| input.events.clone());
            },
        );
        output.textures_delta.clear();
        (changed, remaining)
    }
    #[test]
    fn shortcuts_consume_once_ignore_hold_repeat_and_restore_both_directions() {
        let mut app = demo();
        let ctx = egui::Context::default();
        for i in 0..3 {
            app.select(i, true);
        }
        let press = || key(egui::Key::ArrowLeft, egui::Modifiers::ALT, true);
        let (changed, events) = shortcuts(&mut app, &ctx, vec![press(), press()]);
        assert!(changed && events.is_empty());
        assert_eq!(app.selected, Some(1));
        assert!(!shortcuts(&mut app, &ctx, vec![press()]).0);
        assert_eq!(app.selected, Some(1));
        assert!(!shortcuts(&mut app, &ctx, vec![]).0);
        shortcuts(
            &mut app,
            &ctx,
            vec![key(egui::Key::ArrowLeft, egui::Modifiers::ALT, false)],
        );
        assert!(shortcuts(&mut app, &ctx, vec![press()]).0);
        assert_eq!(app.selected, Some(0));
        assert!(
            shortcuts(
                &mut app,
                &ctx,
                vec![key(egui::Key::ArrowRight, egui::Modifiers::ALT, true)]
            )
            .0
        );
        assert_eq!(app.selected, Some(1));
    }
    #[test]
    fn disabled_shortcuts_and_extra_modifiers_do_not_navigate() {
        let mut app = demo();
        let ctx = egui::Context::default();
        let (changed, events) = shortcuts(
            &mut app,
            &ctx,
            vec![key(egui::Key::ArrowRight, egui::Modifiers::ALT, true)],
        );
        assert!(!changed && events.is_empty());
        app.select(0, true);
        app.select(1, true);
        for modifiers in [
            egui::Modifiers::NONE,
            egui::Modifiers::SHIFT,
            egui::Modifiers::CTRL,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::ALT
            },
            egui::Modifiers {
                ctrl: true,
                ..egui::Modifiers::ALT
            },
        ] {
            let event = key(egui::Key::ArrowLeft, modifiers, true);
            let (changed, remaining) = shortcuts(&mut app, &ctx, vec![event.clone()]);
            assert!(!changed);
            assert_eq!(remaining, vec![event]);
            assert_eq!(app.selected, Some(1));
            shortcuts(
                &mut app,
                &ctx,
                vec![key(egui::Key::ArrowLeft, modifiers, false)],
            );
        }
    }
    #[test]
    fn plain_arrows_and_text_still_edit_a_focused_field() {
        let mut app = demo();
        app.select(0, true);
        app.select(1, true);
        let ctx = egui::Context::default();
        let mut text = String::new();
        let field_id = egui::Id::new("authored_navigation_typing");
        ctx.memory_mut(|memory| memory.request_focus(field_id));
        let mut render = |events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    assert!(!app.navigation_shortcuts(ui.ctx()));
                    ui.add(egui::TextEdit::singleline(&mut text).id(field_id));
                },
            );
            output.textures_delta.clear();
        };
        render(vec![]);
        render(vec![egui::Event::Text("ab".into())]);
        render(vec![key(egui::Key::ArrowLeft, egui::Modifiers::NONE, true)]);
        render(vec![egui::Event::Text("X".into())]);
        assert_eq!(text, "aXb");
        assert_eq!(app.selected, Some(1));
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Visit {
    pub route: Route,
    pub tab: Tab,
}
#[derive(Default)]
pub(super) struct History {
    back: Vec<Visit>,
    forward: Vec<Visit>,
}
fn push(stack: &mut Vec<Visit>, visit: Visit) {
    if stack.len() == MAX_VISITS {
        stack.remove(0);
    }
    stack.push(visit);
}
impl History {
    pub(super) fn clear(&mut self) {
        self.back.clear();
        self.forward.clear();
    }
    pub(super) fn can_back(&self) -> bool {
        !self.back.is_empty()
    }
    pub(super) fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
    fn record(&mut self, old: Visit, new: Visit) {
        if old.route != new.route {
            push(&mut self.back, old);
            self.forward.clear();
        }
    }
    fn travel(
        &mut self,
        current: Visit,
        back: bool,
        valid: impl Fn(Route) -> bool,
    ) -> Option<Visit> {
        let (from, to) = if back {
            (&mut self.back, &mut self.forward)
        } else {
            (&mut self.forward, &mut self.back)
        };
        while let Some(visit) = from.pop() {
            if valid(visit.route) {
                push(to, current);
                return Some(visit);
            }
        }
        None
    }
}
fn valid(catalog: &Catalog, route: Route) -> bool {
    match route {
        Route::Overview(_) => true,
        Route::Map(i) => catalog.snapshot.resources.get(i).is_some(),
        Route::Entry(a, e) => catalog
            .snapshot
            .archives
            .get(a)
            .and_then(|a| a.entries.get(e))
            .is_some(),
    }
}
impl Explorer {
    // Exact Alt-only modifiers reserve these two navigation keys while leaving
    // plain/Shift/Ctrl arrows to widgets. Drain repeats too, but act once per frame.
    pub(super) fn navigation_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let direction = ctx.input_mut(|input| {
            let mut direction = None;
            input.events.retain(|event| match event {
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    repeat,
                    ..
                } if *modifiers == egui::Modifiers::ALT
                    && matches!(key, egui::Key::ArrowLeft | egui::Key::ArrowRight) =>
                {
                    if !repeat && direction.is_none() {
                        direction = Some(*key == egui::Key::ArrowLeft);
                    }
                    false
                }
                _ => true,
            });
            direction
        });
        let changed = direction.is_some_and(|back| self.travel(back));
        if changed {
            ctx.request_repaint();
        }
        changed
    }
    // Fixed synthetic native QA recipe, only enabled by --demo + --qa-capture.
    // Uses production transitions, never opens game data or dispatches input events.
    pub(super) fn qa_navigation_roundtrip(&mut self) -> Result<(), String> {
        self.history.clear();
        self.transition(
            Visit {
                route: Route::Map(0),
                tab: Tab::Raw,
            },
            false,
            true,
        );
        self.select_entry(0, 1);
        self.show_overview(true);
        self.navigate(1);
        for (back, route, tab) in [
            (true, Route::Overview(true), Tab::Hex),
            (true, Route::Entry(0, 1), Tab::Hex),
            (true, Route::Map(0), Tab::Raw),
            (false, Route::Entry(0, 1), Tab::Hex),
        ] {
            if !self.travel(back) || self.current_visit() != (Visit { route, tab }) {
                return Err("Synthetic route/tab roundtrip mismatch".into());
            }
            println!(
                "Navigation QA: {:?}, back={}, forward={}",
                self.current_visit(),
                self.history.can_back(),
                self.history.can_forward()
            );
        }
        Ok(())
    }
    pub(super) fn current_visit(&self) -> Visit {
        Visit {
            route: if let Some((a, e)) = self.entry_selection {
                Route::Entry(a, e)
            } else if let Some(i) = self.selected {
                Route::Map(i)
            } else {
                Route::Overview(self.archive_mode)
            },
            tab: self.tab,
        }
    }
    pub(super) fn transition(
        &mut self,
        mut visit: Visit,
        remember: bool,
        clear_filters: bool,
    ) -> bool {
        if !self.catalog.as_ref().is_some_and(|c| valid(c, visit.route)) {
            return false;
        }
        if matches!(visit.route, Route::Entry(_, _))
            && !matches!(
                visit.tab,
                Tab::Hex | Tab::Texture | Tab::Mesh | Tab::Material | Tab::Animation | Tab::Plan
            )
        {
            visit.tab = Tab::Hex;
        }
        let previous = self.current_visit();
        if clear_filters {
            self.filter = Filter::default();
            self.folders.prefix.clear();
            self.last_filter = None;
        }
        if previous.route == visit.route {
            let changed = self.tab != visit.tab;
            self.tab = visit.tab;
            return changed;
        }
        if remember {
            self.history.record(previous, visit);
        }
        self.preview.reset();
        self.mesh.reset();
        self.material.reset();
        self.animation.reset();
        self.plan.reset();
        self.io.reset_hex();
        self.raw.clear();
        self.jump = None;
        self.entry_jump = None;
        self.selected = None;
        self.inspection = None;
        self.entry_selection = None;
        self.tab = visit.tab;
        match visit.route {
            Route::Overview(entries) => self.archive_mode = entries,
            Route::Map(i) => {
                self.archive_mode = false;
                self.selected = Some(i);
                self.jump = Some(i);
                self.inspection = self
                    .catalog
                    .as_ref()
                    .and_then(|c| c.snapshot.resources[i].inspection_indices().next());
                self.cache_raw();
            }
            Route::Entry(a, e) => {
                self.archive_mode = true;
                self.entry_selection = Some((a, e));
                self.entry_jump = Some((a, e));
            }
        }
        self.last_filter = None;
        self.ready_frames = 0;
        true
    }
    pub(super) fn travel(&mut self, back: bool) -> bool {
        let Some(catalog) = self.catalog.clone() else {
            return false;
        };
        let current = self.current_visit();
        if let Some(visit) = self
            .history
            .travel(current, back, |route| valid(&catalog, route))
        {
            self.transition(visit, false, true)
        } else {
            false
        }
    }
    pub(super) fn show_overview(&mut self, entries: bool) -> bool {
        self.transition(
            Visit {
                route: Route::Overview(entries),
                tab: self.tab,
            },
            true,
            false,
        )
    }
    // All route buttons run before binding/rendering the selected detail body.
    pub(super) fn navigation_bar(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.history.can_back(), egui::Button::new("Back"))
                .clicked()
            {
                changed |= self.travel(true);
            }
            if ui
                .add_enabled(self.history.can_forward(), egui::Button::new("Forward"))
                .clicked()
            {
                changed |= self.travel(false);
            }
            if ui
                .add_enabled(
                    self.selected.is_some() || self.entry_selection.is_some(),
                    egui::Button::new("Overview"),
                )
                .clicked()
            {
                changed |= self.show_overview(self.archive_mode);
            }
        });
        ui.small("Alt+Left: Back · Alt+Right: Forward");
        if changed {
            ui.ctx().request_repaint();
        }
        changed
    }
}
