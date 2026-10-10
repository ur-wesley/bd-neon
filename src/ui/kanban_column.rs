use eframe::egui::{self, ScrollArea, Ui};

use crate::model::{Action, Filter, KanbanLane};

use super::{badge, button, icons, kanban_card, theme};

pub struct KanbanColumnProps {
    pub lane: KanbanLane,
    pub collapsed: bool,
    pub height: f32,
    pub width: f32,
}

pub fn kanban_column(ui: &mut Ui, props: &KanbanColumnProps) -> Vec<Action> {
    let size = egui::vec2(props.width, props.height);
    if props.collapsed {
        collapsed_lane(ui, props, size)
    } else {
        expanded_lane(ui, props, size)
    }
}

fn collapsed_lane(ui: &mut Ui, props: &KanbanColumnProps, size: egui::Vec2) -> Vec<Action> {
    let mut actions = Vec::new();
    let count = props.lane.cards.len();
    let label = props.lane.label;
    let response = theme::exact_card(ui, size, theme::muted(), theme::LANE_COLLAPSED_PAD, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(icons::lane_icon(props.lane.filter))
                    .size(16.0)
                    .color(theme::neon_cyan()),
            );
            ui.label(
                egui::RichText::new(format!("{count}"))
                    .strong()
                    .color(theme::neon_cyan()),
            );
            ui.add_space(theme::SPACING);
            rotated_label(ui, label);
        });
    })
    .response
    .on_hover_text(label);
    let response = theme::pointer(response);
    if response.clicked() {
        actions.push(Action::ToggleLane(props.lane.filter));
    }
    actions
}

fn expanded_lane(ui: &mut Ui, props: &KanbanColumnProps, size: egui::Vec2) -> Vec<Action> {
    let mut actions = Vec::new();
    let filter = props.lane.filter;
    let count = props.lane.cards.len();
    let _ = theme::exact_card(ui, size, theme::muted(), theme::LANE_PAD, |ui| {
        lane_header(ui, props.lane.label, count, filter, &mut actions);
        ui.add_space(theme::SPACING);
        let body_h = ui.available_height();
        ScrollArea::vertical()
            .id_salt(filter.key())
            .auto_shrink([false, false])
            .max_height(body_h)
            .show(ui, |ui| {
                theme::fill_width(ui);
                ui.vertical(|ui| {
                    theme::fill_width(ui);
                    actions.extend(lane_cards(ui, &props.lane));
                });
            });
    });
    actions
}

fn rotated_label(ui: &mut Ui, text: &str) {
    let font_id = egui::TextStyle::Small.resolve(ui.style());
    let color = theme::dim_text();
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font_id, color);
    let size = egui::vec2(galley.size().y, galley.size().x);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let pos = egui::pos2(rect.left(), rect.bottom());
        ui.painter().add(
            egui::epaint::TextShape::new(pos, galley, color)
                .with_angle(-std::f32::consts::FRAC_PI_2),
        );
    }
}

fn lane_header(ui: &mut Ui, label: &str, count: usize, filter: Filter, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(icons::with_icon(icons::lane_icon(filter), label)).strong());
        badge::badge(ui, &format!("{count}"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button::button(
                ui,
                &button::ButtonProps::new(icons::CARET_LEFT)
                    .variant(button::ButtonVariant::Ghost)
                    .min_size(egui::vec2(22.0, 22.0)),
            )
            .clicked()
            {
                actions.push(Action::ToggleLane(filter));
            }
        });
    });
}

fn lane_cards(ui: &mut Ui, lane: &KanbanLane) -> Vec<Action> {
    let mut actions = Vec::new();
    for card in &lane.cards {
        if let Some(action) =
            kanban_card::kanban_card(ui, &kanban_card::KanbanCardProps { row: card.clone() })
        {
            actions.push(action);
        }
        ui.add_space(theme::SPACING);
    }
    if lane.cards.is_empty() {
        theme::card_frame()
            .fill(theme::surface_elevated())
            .inner_margin(egui::Margin::same(theme::PAD_I8))
            .show(ui, |ui| {
                theme::fill_width(ui);
                ui.label(
                    egui::RichText::new("no issues")
                        .small()
                        .color(theme::dim_text()),
                );
            });
    }
    actions
}
