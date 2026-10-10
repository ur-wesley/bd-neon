use eframe::egui::{self, Ui};

use crate::model::{Action, Filter};

use super::{button, icons, theme};

pub struct ChipProps {
    pub filter: Filter,
    pub active: bool,
    pub label: &'static str,
}

pub fn chip(ui: &mut Ui, props: &ChipProps) -> Option<Action> {
    let label = icons::with_icon(icons::lane_icon(props.filter), props.label);
    let clicked = button::button(
        ui,
        &button::ButtonProps::new(&label)
            .variant(button::ButtonVariant::Secondary)
            .selected(props.active)
            .min_size(egui::vec2(72.0, theme::CONTROL_H)),
    )
    .clicked();
    if clicked {
        Some(Action::SetFilter(props.filter))
    } else {
        None
    }
}
