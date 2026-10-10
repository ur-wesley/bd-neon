use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, CornerRadius, Key, Modifiers, ScrollArea, Ui};

use crate::discover;
use crate::model::{Action, RecentRepo};

use super::{button, icons, theme};

pub struct RepoSwitcherProps {
    pub current: Option<String>,
    pub recents: Vec<RecentRepo>,
    pub active_path: Option<PathBuf>,
}

pub fn repo_switcher(ui: &mut Ui, props: &RepoSwitcherProps) -> Vec<Action> {
    let mut actions = Vec::new();
    let title = props.current.as_deref().unwrap_or("workspace");
    let popup_id = ui.make_persistent_id("workspace_popup");
    let query_id = ui.id().with("workspace_query");
    let mut query = ui.data_mut(|data| data.get_temp_mut_or_default::<String>(query_id).clone());

    let button = theme::glass_slot_compact(
        ui,
        egui::vec2(theme::WORKSPACE_W, theme::CONTROL_H),
        egui::Sense::click(),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                egui::RichText::new(icons::FOLDER)
                    .size(14.0)
                    .color(theme::neon_cyan()),
            );
            ui.add(
                egui::Label::new(egui::RichText::new(title).color(theme::foreground()))
                    .truncate()
                    .selectable(false),
            );
        },
    );
    if button.response.clicked() {
        toggle_popup(ui, popup_id);
    }

    let picked = workspace_popup(ui, popup_id, &button.response, |ui| {
        switcher_popup(ui, props, query_id, &mut query)
    });

    if picked.is_none() {
        ui.data_mut(|data| data.insert_temp(query_id.with("focus"), true));
    }
    if let Some(Some(path)) = picked {
        actions.push(Action::PickRecent(path));
        query.clear();
        ui.data_mut(|data| {
            data.insert_temp(query_id, String::new());
            data.insert_temp(query_id.with("selected"), 0_usize);
        });
        set_popup_open(ui, popup_id, false);
    }
    actions
}

#[allow(clippy::too_many_lines)]
fn switcher_popup(
    ui: &mut Ui,
    props: &RepoSwitcherProps,
    query_id: egui::Id,
    query: &mut String,
) -> Option<PathBuf> {
    ui.set_min_width(theme::WORKSPACE_POPUP_W);
    ui.set_max_width(theme::WORKSPACE_POPUP_W);
    ui.visuals_mut().override_text_color = Some(theme::foreground());
    theme::paint_dialog_top_highlight(ui);
    let selected_key = query_id.with("selected");
    let mut selected = ui.data(|data| data.get_temp::<usize>(selected_key).unwrap_or(0));
    let mut add_clicked = false;
    ui.horizontal(|ui| {
        ui.set_height(theme::CONTROL_H);
        ui.label(
            egui::RichText::new(icons::MAGNIFYING_GLASS)
                .size(14.0)
                .color(theme::dim_text()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_add = discover::typed_beads(query).is_some();
            add_clicked = button::button(
                ui,
                &button::ButtonProps::new("Add")
                    .selected(can_add)
                    .min_size(egui::vec2(52.0, theme::CONTROL_H)),
            )
            .clicked();
            let response = ui.add(
                egui::TextEdit::singleline(query)
                    .id(query_id)
                    .hint_text(egui::RichText::new("path to add").color(theme::dim_text()))
                    .text_color(theme::foreground())
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                persist_query(ui, query_id, query);
                selected = 0;
                ui.data_mut(|data| data.insert_temp(selected_key, 0_usize));
            }
            let steal = ui.data_mut(|data| {
                let slot = data.get_temp_mut_or_insert_with(query_id.with("focus"), || true);
                let first = *slot;
                *slot = false;
                first
            });
            if steal {
                response.request_focus();
            }
        });
    });

    let is_path = discover::looks_like_path(query);
    let suggestion_count = if is_path {
        discover::path_completions(query).len()
    } else {
        suggestion_vec(props, query).len()
    };
    if suggestion_count > 0 {
        selected = selected.min(suggestion_count.saturating_sub(1));
    } else {
        selected = 0;
    }

    let tab = ui.input(|input| input.key_pressed(Key::Tab));
    if tab && let Some(next) = discover::complete_path(query) {
        *query = next;
        persist_query(ui, query_id, query);
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    let down = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
    let up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
    if down && suggestion_count > 0 {
        selected = (selected.saturating_add(1)).min(suggestion_count.saturating_sub(1));
        ui.data_mut(|data| data.insert_temp(selected_key, selected));
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    if up && suggestion_count > 0 {
        selected = selected.saturating_sub(1);
        ui.data_mut(|data| data.insert_temp(selected_key, selected));
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    let esc = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    if esc {
        ui.memory_mut(|mem| mem.surrender_focus(query_id));
    }

    ui.separator();
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top() + 1.0,
        theme::dialog_highlight_stroke(),
    );
    let list_h = ui.available_height();
    let mut chosen = ScrollArea::vertical()
        .max_height(list_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            theme::fill_width(ui);
            if is_path {
                path_list(ui, query_id, query, selected);
                None
            } else {
                recent_list(ui, props, query, selected)
            }
        })
        .inner;

    let enter = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    if add_clicked {
        chosen = discover::typed_beads(query);
    }
    if enter {
        if is_path {
            if let Some(path) = discover::typed_beads(query) {
                chosen = Some(path);
            } else {
                let completions = discover::path_completions(query);
                if let Some(pick) = completions.get(selected) {
                    *query = discover::dir_query(pick);
                    persist_query(ui, query_id, query);
                    ui.memory_mut(|mem| mem.request_focus(query_id));
                } else if let Some(next) = discover::complete_path(query) {
                    *query = next;
                    persist_query(ui, query_id, query);
                    ui.memory_mut(|mem| mem.request_focus(query_id));
                }
            }
        } else {
            let suggestions = suggestion_vec(props, query);
            if let Some(pick) = suggestions.get(selected) {
                chosen = Some(pick.path.clone());
            } else if let Some(path) = discover::typed_beads(query) {
                chosen = Some(path);
            } else if chosen.is_none() {
                chosen = first_recent(props, query);
            }
        }
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    chosen
}

fn is_popup_open(ui: &Ui, popup_id: egui::Id) -> bool {
    ui.data(|data| data.get_temp::<bool>(popup_id).unwrap_or(false))
}

fn set_popup_open(ui: &Ui, popup_id: egui::Id, open: bool) {
    ui.data_mut(|data| data.insert_temp(popup_id, open));
}

fn toggle_popup(ui: &Ui, popup_id: egui::Id) {
    let open = is_popup_open(ui, popup_id);
    set_popup_open(ui, popup_id, !open);
}

fn workspace_popup<R>(
    ui: &Ui,
    popup_id: egui::Id,
    widget: &egui::Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if !is_popup_open(ui, popup_id) {
        return None;
    }

    let screen_rect = ui.ctx().viewport_rect();
    let backdrop_id = popup_id.with("backdrop");
    let backdrop_response = egui::Area::new(backdrop_id)
        .order(egui::Order::Foreground)
        .fixed_pos(screen_rect.min)
        .interactable(true)
        .show(ui.ctx(), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(screen_rect.size(), egui::Sense::click_and_drag());
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, theme::dialog_backdrop());
            response
        });

    let widget_rect = widget.rect;
    let screen = screen_rect.shrink(theme::PAD);
    let below = space_below(screen, widget_rect);
    let above = space_above(screen, widget_rect);
    let (pos, pivot, cap) = if below >= above {
        (widget_rect.left_bottom(), egui::Align2::LEFT_TOP, below)
    } else {
        (widget_rect.left_top(), egui::Align2::LEFT_BOTTOM, above)
    };

    let frame = theme::dialog_frame(ui.style());
    let response = egui::Area::new(popup_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .pivot(pivot)
        .constrain(true)
        .constrain_to(screen)
        .default_width(theme::WORKSPACE_POPUP_W)
        .default_height(cap)
        .show(ui.ctx(), |ui| {
            frame
                .show(ui, |ui| {
                    ui.set_min_width(theme::WORKSPACE_POPUP_W);
                    ui.set_max_width(theme::WORKSPACE_POPUP_W);
                    ui.set_max_height(cap);
                    add_contents(ui)
                })
                .inner
        });

    let backdrop_clicked = !widget.clicked() && backdrop_response.inner.clicked();
    let should_close = backdrop_clicked
        || (widget.clicked_elsewhere() && response.response.clicked_elsewhere())
        || ui.input(|input| input.key_pressed(Key::Escape));
    if should_close {
        set_popup_open(ui, popup_id, false);
    }
    Some(response.inner)
}

fn space_below(screen: egui::Rect, widget: egui::Rect) -> f32 {
    let mut area = screen;
    area.min.y = widget.max.y;
    area.intersect(screen).height().max(0.0)
}

fn space_above(screen: egui::Rect, widget: egui::Rect) -> f32 {
    let mut area = screen;
    area.max.y = widget.min.y;
    area.intersect(screen).height().max(0.0)
}

fn persist_query(ui: &Ui, query_id: egui::Id, query: &str) {
    ui.data_mut(|data| data.insert_temp(query_id, query.to_owned()));
}

fn suggestion_vec(props: &RepoSwitcherProps, query: &str) -> Vec<RecentRepo> {
    let seeds: Vec<PathBuf> = props
        .recents
        .iter()
        .map(|recent| recent.path.clone())
        .chain(props.active_path.clone())
        .collect();
    let neighbors = discover::beads_neighbors(&seeds);
    discover::suggest(&props.recents, &neighbors, query)
}

fn path_list(ui: &mut Ui, query_id: egui::Id, query: &mut String, selected: usize) {
    let completions = discover::path_completions(query);
    if completions.is_empty() {
        ui.label(
            egui::RichText::new("no matching folder")
                .small()
                .color(theme::dim_text()),
        );
        return;
    }
    for (index, path) in completions.iter().enumerate() {
        if path_item(ui, path, index == selected) {
            *query = discover::dir_query(path);
            persist_query(ui, query_id, query);
            ui.memory_mut(|mem| mem.request_focus(query_id));
        }
    }
}

fn recent_list(
    ui: &mut Ui,
    props: &RepoSwitcherProps,
    query: &str,
    selected: usize,
) -> Option<PathBuf> {
    let suggestions = suggestion_vec(props, query);
    if suggestions.is_empty() {
        ui.label(
            egui::RichText::new("no matching workspace")
                .small()
                .color(theme::dim_text()),
        );
        return None;
    }
    let mut chosen = None;
    for (index, recent) in suggestions.iter().enumerate() {
        if recent_item(ui, props, recent, index == selected) {
            chosen = Some(recent.path.clone());
        }
    }
    chosen
}

fn first_recent(props: &RepoSwitcherProps, query: &str) -> Option<PathBuf> {
    let seeds: Vec<PathBuf> = props
        .recents
        .iter()
        .map(|recent| recent.path.clone())
        .chain(props.active_path.clone())
        .collect();
    let neighbors = discover::beads_neighbors(&seeds);
    discover::suggest(&props.recents, &neighbors, query)
        .into_iter()
        .next()
        .map(|recent| recent.path)
}

fn path_item(ui: &mut Ui, path: &Path, is_selected: bool) -> bool {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |part| part.to_string_lossy().into_owned(),
    );
    let parent = path
        .parent()
        .map(|part| part.display().to_string())
        .unwrap_or_default();
    let beads = path.join(".beads").is_dir();
    let title = if beads {
        icons::with_icon(icons::PLUS, &format!("add {name}"))
    } else {
        icons::with_icon(icons::FOLDER, &name)
    };
    let (fill, stroke) = if is_selected {
        (
            theme::surface_hover(),
            egui::Stroke::new(1.0_f32, theme::border_hover()),
        )
    } else {
        (Color32::TRANSPARENT, theme::glass_stroke())
    };
    let response = theme::card_frame()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(theme::PAD_I8, theme::SPACING_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(theme::foreground()),
                );
                ui.label(egui::RichText::new(parent).small().color(theme::dim_text()));
            });
        })
        .response
        .interact(egui::Sense::click());
    if is_selected {
        response.scroll_to_me(None);
    }
    theme::pointer(response).clicked()
}

fn recent_item(
    ui: &mut Ui,
    props: &RepoSwitcherProps,
    recent: &RecentRepo,
    is_selected: bool,
) -> bool {
    let active = props
        .active_path
        .as_ref()
        .is_some_and(|path| path == &recent.path);
    let (fill, stroke) = if active {
        (
            theme::surface_active(),
            egui::Stroke::new(1.0_f32, theme::neon_cyan()),
        )
    } else if is_selected {
        (
            theme::surface_hover(),
            egui::Stroke::new(1.0_f32, theme::border_hover()),
        )
    } else {
        (Color32::TRANSPARENT, theme::glass_stroke())
    };
    let known = props.recents.iter().any(|item| item.path == recent.path);
    let title = if known {
        icons::with_icon(icons::FOLDER, &recent.name)
    } else {
        icons::with_icon(icons::PLUS, &format!("add {}", recent.name))
    };
    let response = theme::card_frame()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(theme::PAD_I8, theme::SPACING_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(theme::foreground()),
                );
                ui.label(
                    egui::RichText::new(recent.parent.clone())
                        .small()
                        .color(theme::dim_text()),
                );
            });
        })
        .response
        .interact(egui::Sense::click());
    if is_selected {
        response.scroll_to_me(None);
    }
    theme::pointer(response).clicked()
}
