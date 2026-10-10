use eframe::egui::{self, Ui};

use crate::model::{Action, IssueRow};

use super::{badge, priority_pip, theme};

pub struct KanbanCardProps {
    pub row: IssueRow,
}

pub fn kanban_card(ui: &mut Ui, props: &KanbanCardProps) -> Option<Action> {
    let id = props.row.id.clone();
    let card_id = ui.make_persistent_id(&id);
    let was_hovered = ui.data_mut(|d| d.get_temp::<bool>(card_id).unwrap_or(false));

    let (fill, stroke) = if props.row.flashed {
        (
            theme::flash_bg(),
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
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    priority_pip::priority_pip(ui, props.row.priority);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(props.row.title.clone())
                                .strong()
                                .color(theme::foreground()),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(theme::SPACING);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACING;
                    badge::issue_id(ui, &props.row.id);
                    badge::kind_badge(ui, &props.row.kind);
                });
            });
        });

    let response = inner.response.interact(egui::Sense::click());
    let is_now_hovered = response.hovered();
    if is_now_hovered != was_hovered {
        ui.data_mut(|d| d.insert_temp(card_id, is_now_hovered));
        ui.ctx().request_repaint();
    }
    let response = theme::pointer(response);

    if response.clicked() {
        Some(Action::OpenDetail(id))
    } else {
        None
    }
}
