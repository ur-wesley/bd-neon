use eframe::egui::{self, Ui};

use crate::model::{Action, IssueRow};

use super::{badge, priority_pip, theme};

pub struct IssueRowProps {
    pub row: IssueRow,
    pub selected: bool,
}

pub fn issue_row(ui: &mut Ui, props: &IssueRowProps) -> Option<Action> {
    let id = props.row.id.clone();
    let row_id = ui.make_persistent_id(&id);
    let was_hovered = ui.data_mut(|d| d.get_temp::<bool>(row_id).unwrap_or(false));

    let (fill, stroke) = if props.row.flashed {
        (
            theme::flash_bg(),
            egui::Stroke::new(1.0_f32, theme::neon_cyan()),
        )
    } else if props.selected {
        (
            theme::surface_active(),
            egui::Stroke::new(1.0_f32, theme::neon_cyan()),
        )
    } else if was_hovered {
        (
            theme::surface_hover(),
            egui::Stroke::new(1.0_f32, theme::border_hover()),
        )
    } else {
        (theme::surface_elevated(), theme::hairline())
    };

    let inner = theme::card_frame()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(egui::Margin::same(theme::PAD_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            ui.set_min_height(theme::ROW_HEIGHT);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SPACING;
                priority_pip::priority_pip(ui, props.row.priority);
                badge::issue_id(ui, &props.row.id);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACING;
                    badge::status_badge(ui, &props.row.status);
                    badge::kind_badge(ui, &props.row.kind);

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&props.row.title).color(theme::foreground()),
                            )
                            .truncate()
                            .selectable(false),
                        );
                    });
                });
            });
        });

    let response = inner.response.interact(egui::Sense::click());
    let is_now_hovered = response.hovered();
    if is_now_hovered != was_hovered {
        ui.data_mut(|d| d.insert_temp(row_id, is_now_hovered));
        ui.ctx().request_repaint();
    }
    let response = theme::pointer(response);

    if response.clicked() {
        Some(Action::OpenDetail(id))
    } else {
        None
    }
}
