use std::path::PathBuf;

use eframe::egui::{self, Color32, CornerRadius, PointerButton, ScrollArea, Ui};

use crate::model::{Action, RecentRepo};

use super::{icons, theme};

pub struct ProjectTabsProps {
    pub tabs: Vec<RecentRepo>,
    pub active: Option<PathBuf>,
    pub loading: bool,
}

pub fn project_tabs(ui: &mut Ui, props: &ProjectTabsProps) -> Vec<Action> {
    let mut actions = Vec::new();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, theme::TAB_H), egui::Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        theme::muted(),
        theme::glass_stroke(),
        egui::StrokeKind::Inside,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_clip_rect(rect);
    child.set_height(theme::TAB_H);
    child.spacing_mut().item_spacing.x = 0.0;
    ScrollArea::horizontal()
        .id_salt("project_tabs")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.set_height(theme::TAB_H);
            ui.spacing_mut().item_spacing.x = 0.0;
            if props.tabs.is_empty() {
                ui.allocate_ui_with_layout(
                    egui::vec2(theme::TAB_MIN_W + theme::PAD, theme::TAB_H),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add_space(theme::PAD);
                        ui.label(
                            egui::RichText::new("no open projects")
                                .small()
                                .color(theme::dim_text()),
                        );
                    },
                );
            }
            for tab in &props.tabs {
                if let Some(action) = tab_chip(ui, tab, props) {
                    actions.push(action);
                }
            }
        });
    actions
}

fn tab_chip(ui: &mut Ui, tab: &RecentRepo, props: &ProjectTabsProps) -> Option<Action> {
    let active = props.active.as_ref().is_some_and(|path| path == &tab.path);
    let fill = if active {
        theme::surface_active()
    } else {
        Color32::TRANSPARENT
    };
    let label = tab_label(props.tabs.as_slice(), tab);
    let tab_w = tab_width(ui, &label, active && props.loading);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(tab_w, theme::TAB_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
        let divider = egui::Rect::from_min_max(egui::pos2(rect.max.x - 1.0, rect.min.y), rect.max);
        ui.painter()
            .rect_filled(divider, CornerRadius::ZERO, theme::border());
        if active {
            let bar = egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max);
            ui.painter()
                .rect_filled(bar, CornerRadius::ZERO, theme::neon_cyan());
        }
    }
    let inner = rect.shrink2(egui::vec2(theme::TAB_PAD_X, 0.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.set_clip_rect(inner);
    child.set_height(theme::TAB_H);
    child.spacing_mut().item_spacing.x = theme::SPACING;
    let mut close = false;
    child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        close = theme::pointer(
            ui.add(
                egui::Button::new(
                    egui::RichText::new(icons::X)
                        .small()
                        .color(theme::dim_text()),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(theme::TAB_CLOSE_W, theme::TAB_CLOSE_W)),
            ),
        )
        .on_hover_text("close")
        .clicked();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme::SPACING;
            if active && props.loading {
                ui.add(egui::Spinner::new().size(12.0));
            }
            ui.add(
                egui::Label::new(egui::RichText::new(label).small().color(if active {
                    theme::foreground()
                } else {
                    theme::dim_text()
                }))
                .truncate()
                .selectable(false),
            );
        });
    });
    let response = theme::pointer(response);
    if close || response.clicked_by(PointerButton::Middle) {
        Some(Action::CloseTab(tab.path.clone()))
    } else if response.clicked() {
        Some(Action::PickRecent(tab.path.clone()))
    } else {
        None
    }
}

fn tab_width(ui: &Ui, label: &str, spinning: bool) -> f32 {
    let font_id = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font_id, theme::foreground());
    let spinner = if spinning { 18.0 } else { 0.0 };
    let text_w = galley.size().x;
    let raw = theme::TAB_PAD_X + spinner + text_w + theme::TAB_PAD_X + theme::TAB_CLOSE_W;
    raw.clamp(theme::TAB_MIN_W, theme::TAB_MAX_W)
}

fn tab_label(tabs: &[RecentRepo], tab: &RecentRepo) -> String {
    let duplicate = tabs.iter().filter(|item| item.name == tab.name).count() > 1;
    if duplicate {
        format!("{} — {}", tab.name, tab.parent)
    } else {
        tab.name.clone()
    }
}
