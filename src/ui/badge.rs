use eframe::egui::{self, Color32, Ui};

use crate::model::split_issue_id;

use super::theme;

pub fn badge(ui: &mut Ui, text: &str) {
    badge_color(ui, text, theme::foreground());
}

pub fn badge_color(ui: &mut Ui, text: &str, color: Color32) {
    theme::muted_frame()
        .inner_margin(egui::Margin::symmetric(6, 2))
        .corner_radius(theme::radius_sm())
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}

pub fn kind_badge(ui: &mut Ui, kind: &str) {
    badge_color(ui, kind, kind_color(kind));
}

fn kind_color(kind: &str) -> Color32 {
    match kind.trim().to_ascii_lowercase().as_str() {
        "bug" | "defect" => theme::neon_magenta(),
        "feature" => theme::neon_cyan(),
        "task" => theme::neon_blue(),
        "epic" => theme::neon_amber(),
        "chore" => theme::dim_text(),
        "spike" => theme::neon_violet(),
        "docs" | "documentation" => theme::neon_green(),
        _ => theme::foreground(),
    }
}

pub fn status_color(status: &str) -> Color32 {
    match status {
        "open" | "ready" => theme::neon_cyan(),
        "in_progress" => theme::neon_amber(),
        "blocked" => theme::neon_magenta(),
        "closed" => theme::dim_text(),
        _ => theme::foreground(),
    }
}

pub fn status_badge(ui: &mut Ui, status: &str) {
    badge_color(ui, status, status_color(status));
}

pub const fn priority_color(priority: u8) -> Color32 {
    match priority {
        0 => theme::neon_magenta(),
        1 => theme::neon_amber(),
        2 => theme::neon_cyan(),
        _ => theme::dim_text(),
    }
}

pub fn priority_badge(ui: &mut Ui, priority: u8) {
    badge_color(ui, &format!("P{priority}"), priority_color(priority));
}

pub fn issue_id(ui: &mut Ui, id: &str) {
    let parts = split_issue_id(id);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add(
            egui::Label::new(
                egui::RichText::new(&parts.prefix)
                    .small()
                    .strong()
                    .color(theme::neon_cyan()),
            )
            .selectable(false),
        );
        if !parts.number.is_empty() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("-{}", parts.number))
                        .small()
                        .strong()
                        .color(theme::neon_amber()),
                )
                .selectable(false),
            );
        }
    });
}
