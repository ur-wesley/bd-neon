use eframe::egui::{self, Ui};

use super::theme;

pub fn priority_pip(ui: &mut Ui, priority: u8) {
    let color = match priority {
        0 => theme::neon_magenta(),
        1 => theme::neon_amber(),
        2 => theme::neon_cyan(),
        _ => theme::dim_text(),
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 14.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}
