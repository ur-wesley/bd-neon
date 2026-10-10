use eframe::egui::{self, Color32, Ui};

use super::theme;

pub fn banner(ui: &mut Ui, message: &str) {
    theme::card_frame()
        .fill(Color32::from_rgb(52, 20, 28))
        .stroke(egui::Stroke::new(1.0_f32, theme::neon_magenta()))
        .inner_margin(egui::Margin::symmetric(theme::PAD_I8, theme::SPACING_I8))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(message).color(theme::foreground()));
        });
}
