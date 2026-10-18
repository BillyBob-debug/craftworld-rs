//! D378: the Pod's launch warning popup on the screen (docs/level-scripts.md part 125).
//!
//! The game draws it with the script `gamedata/scripts/gamemessagingmanager.ff`, which
//! only the patch's encrypted archive holds, so this is the viewer's built-in box:
//! INFERRED, the look. What it says and what its buttons do come from
//! `lbp_runtime::LevelWorld::launch_warning_popup`.
use bevy_egui::egui;
use lbp_runtime::LaunchWarningPopup;

/// The box's text width in screen pixels.
const TEXT_WIDTH: f32 = 420.;

/// Draws the popup in the middle of the screen, the text over the two buttons' prompts.
pub fn draw(ctx: &egui::Context, popup: &LaunchWarningPopup) {
    egui::Area::new(egui::Id::new("launch_warning_popup"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_max_width(TEXT_WIDTH);
                    let text = popup
                        .text
                        .clone()
                        .unwrap_or_else(|| format!("{:?}", popup.warning));
                    ui.label(egui::RichText::new(text).size(16.));
                    ui.add_space(12.);
                    ui.horizontal(|ui| {
                        let [carry_on, cancel] = &popup.buttons;
                        ui.label(format!("(X / Enter) {carry_on}"));
                        ui.add_space(24.);
                        ui.label(format!("(O / C) {cancel}"));
                    });
                });
        });
}
