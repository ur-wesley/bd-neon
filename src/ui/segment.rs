use eframe::egui::{self, Ui};

use crate::model::{Action, ViewMode};

use super::{button, icons};

pub struct SegmentProps {
    pub view: ViewMode,
}

pub fn segment(ui: &mut Ui, props: &SegmentProps) -> Option<Action> {
    let mut action = None;
    let list_label = icons::with_icon(icons::LIST, "List");
    let board_label = icons::with_icon(icons::COLUMNS, "Board");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if button::button(
            ui,
            &button::ButtonProps::new(&list_label)
                .variant(button::ButtonVariant::Ghost)
                .selected(props.view == ViewMode::List)
                .min_size(egui::vec2(64.0, 26.0)),
        )
        .clicked()
        {
            action = Some(Action::SetView(ViewMode::List));
        }
        if button::button(
            ui,
            &button::ButtonProps::new(&board_label)
                .variant(button::ButtonVariant::Ghost)
                .selected(props.view == ViewMode::Kanban)
                .min_size(egui::vec2(76.0, 26.0)),
        )
        .clicked()
        {
            action = Some(Action::SetView(ViewMode::Kanban));
        }
    });
    action
}
