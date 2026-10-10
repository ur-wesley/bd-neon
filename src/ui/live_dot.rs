use eframe::egui::{self, Color32, Ui};

use super::theme;

pub struct LiveDotProps {
    pub live: bool,
    pub label: String,
}

pub fn live_dot(ui: &mut Ui, props: &LiveDotProps) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SPACING;
        let color = if props.live {
            theme::neon_cyan()
        } else {
            Color32::from_rgb(140, 152, 170)
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0_f32, color);
        ui.add(
            egui::Label::new(
                egui::RichText::new(&props.label)
                    .small()
                    .color(theme::dim_text()),
            )
            .truncate()
            .selectable(false),
        );
    });
}
