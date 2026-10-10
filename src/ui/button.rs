use eframe::egui::{self, Color32, Response, Ui};

use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Outline,
    Ghost,
    Secondary,
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub variant: ButtonVariant,
    pub selected: bool,
    pub min_size: egui::Vec2,
}

impl<'a> ButtonProps<'a> {
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            variant: ButtonVariant::Outline,
            selected: false,
            min_size: egui::Vec2::new(theme::CONTROL_H, theme::CONTROL_H),
        }
    }

    pub const fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn min_size(mut self, min_size: egui::Vec2) -> Self {
        self.min_size = min_size;
        self
    }
}

pub fn button(ui: &mut Ui, props: &ButtonProps<'_>) -> Response {
    let (fill, stroke, text) = match (props.variant, props.selected) {
        (_, true) => (
            theme::surface_active(),
            egui::Stroke::new(1.0_f32, theme::neon_cyan().gamma_multiply(0.55)),
            theme::foreground(),
        ),
        (ButtonVariant::Secondary, false) => {
            (theme::muted(), theme::glass_stroke(), theme::foreground())
        }
        (ButtonVariant::Ghost, false) => (
            Color32::TRANSPARENT,
            egui::Stroke::NONE,
            theme::foreground(),
        ),
        (ButtonVariant::Outline, false) => (
            theme::surface_elevated(),
            theme::glass_stroke(),
            theme::foreground(),
        ),
    };
    theme::pointer(
        ui.add(
            egui::Button::new(egui::RichText::new(props.label).color(text))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(theme::radius())
                .min_size(props.min_size),
        ),
    )
}
