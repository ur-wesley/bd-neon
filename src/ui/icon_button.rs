use eframe::egui::{self, Ui};

use super::{button, theme};

pub struct IconButtonProps {
    pub label: &'static str,
    pub selected: bool,
}

pub fn icon_button(ui: &mut Ui, props: &IconButtonProps) -> bool {
    button::button(
        ui,
        &button::ButtonProps::new(props.label)
            .variant(button::ButtonVariant::Ghost)
            .selected(props.selected)
            .min_size(egui::vec2(theme::CONTROL_H, theme::CONTROL_H)),
    )
    .clicked()
}
