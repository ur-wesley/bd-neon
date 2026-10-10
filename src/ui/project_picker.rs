use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Key, Modifiers, ScrollArea, Ui};

use crate::discover;
use crate::model::{recent_from_path, Action, RecentRepo};

use super::{icons, theme};

const PICKER_WIDTH: f32 = 600.0;
const PICKER_LIST_H: f32 = 320.0;

pub struct PickerProps {
    pub tabs: Vec<RecentRepo>,
    pub recents: Vec<RecentRepo>,
    pub active: Option<PathBuf>,
    pub query: String,
    pub selected: usize,
}

struct PickerItem {
    path: PathBuf,
    name: String,
    parent: String,
    is_tab: bool,
    is_typed: bool,
    is_new: bool,
}

struct BodyOut {
    query: Option<String>,
    delta: Option<i32>,
    chosen: Option<PathBuf>,
    close: bool,
}

/// VSCode-style `Ctrl+P` project picker: centered modal, autofocus filter
/// input, typed-path escape hatch pinned at row 0, then open tabs, then
/// recents + neighbor dirs ranked by [`discover::suggest_fuzzy`].
#[allow(clippy::too_many_lines)]
pub fn project_picker(ctx: &egui::Context, props: &PickerProps) -> Vec<Action> {
    let picker_id = egui::Id::new("project_picker");
    let focus_id = picker_id.with("autofocus");
    let mut query = props.query.clone();

    let seeds: Vec<PathBuf> = props
        .recents
        .iter()
        .map(|recent| recent.path.clone())
        .chain(props.active.clone())
        .collect();
    let neighbors = discover::beads_neighbors(&seeds);
    let items = picker_items(props, &neighbors);
    let selected = clamp_selection(props.selected, items.len());

    let frame = theme::dialog_frame(ctx.global_style().as_ref());
    let modal = egui::Modal::new(picker_id)
        .frame(frame)
        .backdrop_color(theme::dialog_backdrop())
        .show(ctx, |ui| {
            ui.set_min_width(PICKER_WIDTH);
            ui.set_max_width(PICKER_WIDTH);
            picker_body(
                ui,
                picker_id,
                &mut query,
                &items,
                selected,
                props.active.as_deref(),
            )
        });
    let should_close = modal.should_close();
    let out = modal.inner;
    let mut actions = Vec::new();
    if let Some(next) = out.query {
        if next != props.query {
            actions.push(Action::SetPickerQuery(next));
        }
    }
    if let Some(delta) = out.delta {
        actions.push(Action::MovePicker(delta));
    }
    if let Some(path) = out.chosen {
        // `set_repo` closes the picker; the explicit close keeps the
        // autofocus flag reset below in sync even if routing changes.
        actions.push(Action::PickRecent(path));
        actions.push(Action::ClosePicker);
        ctx.data_mut(|data| data.insert_temp(focus_id, true));
    } else if out.close || should_close {
        actions.push(Action::ClosePicker);
        ctx.data_mut(|data| data.insert_temp(focus_id, true));
    }
    actions
}

fn picker_items(props: &PickerProps, neighbors: &[PathBuf]) -> Vec<PickerItem> {
    let typed = discover::typed_beads(&props.query);
    let filtered_tabs = discover::suggest_fuzzy(&props.query, &props.tabs);
    let mut pool: Vec<RecentRepo> = Vec::new();
    for recent in &props.recents {
        if props.tabs.iter().any(|tab| tab.path == recent.path) {
            continue;
        }
        pool.push(recent.clone());
    }
    for neighbor in neighbors {
        if props.tabs.iter().any(|tab| tab.path == *neighbor) {
            continue;
        }
        if pool.iter().any(|recent| recent.path == *neighbor) {
            continue;
        }
        pool.push(recent_from_path(neighbor.clone()));
    }
    let filtered_recents = discover::suggest_fuzzy(&props.query, &pool);
    let mut items = Vec::new();
    if let Some(ref typed_path) = typed {
        if let Some(tab) = filtered_tabs.iter().find(|tab| tab.path == *typed_path) {
            items.push(typed_item(tab, true));
        } else if let Some(recent) = filtered_recents
            .iter()
            .find(|item| item.path == *typed_path)
        {
            items.push(typed_item(recent, false));
        } else {
            let fresh = recent_from_path(typed_path.clone());
            items.push(PickerItem {
                path: fresh.path,
                name: fresh.name,
                parent: fresh.parent,
                is_tab: false,
                is_typed: true,
                is_new: true,
            });
        }
    }
    for tab in &filtered_tabs {
        if typed.as_ref().is_some_and(|path| path == &tab.path) {
            continue;
        }
        items.push(PickerItem {
            path: tab.path.clone(),
            name: tab.name.clone(),
            parent: tab.parent.clone(),
            is_tab: true,
            is_typed: false,
            is_new: false,
        });
    }
    for recent in &filtered_recents {
        if typed.as_ref().is_some_and(|path| path == &recent.path) {
            continue;
        }
        items.push(PickerItem {
            path: recent.path.clone(),
            name: recent.name.clone(),
            parent: recent.parent.clone(),
            is_tab: false,
            is_typed: false,
            is_new: false,
        });
    }
    items
}

fn typed_item(recent: &RecentRepo, is_tab: bool) -> PickerItem {
    PickerItem {
        path: recent.path.clone(),
        name: recent.name.clone(),
        parent: recent.parent.clone(),
        is_tab,
        is_typed: true,
        is_new: false,
    }
}

const fn clamp_selection(selected: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        let max = len.saturating_sub(1);
        if selected > max {
            max
        } else {
            selected
        }
    }
}

fn step_selection(selected: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let clamped = clamp_selection(selected, len);
    let current = i32::try_from(clamped).unwrap_or(0);
    let max = i32::try_from(len.saturating_sub(1)).unwrap_or(0);
    let next = current.saturating_add(delta).clamp(0, max);
    usize::try_from(next).unwrap_or(0)
}

fn picker_body(
    ui: &mut Ui,
    picker_id: egui::Id,
    query: &mut String,
    items: &[PickerItem],
    selected: usize,
    active: Option<&Path>,
) -> BodyOut {
    let query_id = picker_id.with("query");
    let focus_id = picker_id.with("autofocus");
    let mut out = BodyOut {
        query: None,
        delta: None,
        chosen: None,
        close: false,
    };
    theme::paint_dialog_top_highlight(ui);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(icons::MAGNIFYING_GLASS)
                .size(14.0)
                .color(theme::dim_text()),
        );
        let response = ui.add(
            egui::TextEdit::singleline(query)
                .id(query_id)
                .hint_text("type to filter projects, Enter to open")
                .text_color(theme::foreground())
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
        if response.changed() {
            out.query = Some(query.clone());
        }
        let steal = ui.data_mut(|data| {
            let slot = data.get_temp_mut_or_insert_with(focus_id, || true);
            let first = *slot;
            *slot = false;
            first
        });
        if steal {
            response.request_focus();
        }
    });
    ui.separator();
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top() + 1.0,
        theme::dialog_highlight_stroke(),
    );

    let down = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
    let up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
    if down {
        out.delta = Some(1);
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    if up {
        out.delta = Some(-1);
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    let esc = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    if esc {
        out.close = true;
    }

    ScrollArea::vertical()
        .id_salt("project_picker_list")
        .max_height(PICKER_LIST_H)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            theme::fill_width(ui);
            if items.is_empty() {
                ui.label(
                    egui::RichText::new("no matching projects")
                        .small()
                        .color(theme::dim_text()),
                );
                return;
            }
            let preview = step_selection(selected, items.len(), out.delta.unwrap_or(0));
            let mut emitted_tabs = false;
            let mut emitted_recents = false;
            for (index, item) in items.iter().enumerate() {
                if item.is_typed && index == 0 {
                    // Escape hatch stays above the sections, no header.
                } else if item.is_tab {
                    if !emitted_tabs {
                        section_label(ui, "Open tabs");
                        emitted_tabs = true;
                    }
                } else if !emitted_recents {
                    section_label(ui, "Recents & neighbors");
                    emitted_recents = true;
                }
                let is_active = active.is_some_and(|path| path == item.path.as_path());
                if picker_row(ui, item, index == preview, is_active) {
                    out.chosen = Some(item.path.clone());
                }
            }
        });

    let enter = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    if enter {
        if out.chosen.is_none() {
            out.chosen = items.get(selected).map(|item| item.path.clone());
        }
        ui.memory_mut(|mem| mem.request_focus(query_id));
    }
    out
}

fn section_label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).small().color(theme::dim_text()));
}

fn picker_row(ui: &mut Ui, item: &PickerItem, is_selected: bool, is_active: bool) -> bool {
    let title = if item.is_new {
        icons::with_icon(icons::PLUS, &format!("add {}", item.name))
    } else if item.is_typed {
        icons::with_icon(icons::PLUS, &format!("open {}", item.name))
    } else {
        icons::with_icon(icons::FOLDER, &item.name)
    };
    let (fill, stroke) = if is_active {
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
                    egui::RichText::new(item.parent.clone())
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::model::recent_from_path;

    use super::{clamp_selection, picker_items, step_selection, PickerProps};

    fn props_with(tabs: &[PathBuf], recents: &[PathBuf], query: &str) -> PickerProps {
        PickerProps {
            tabs: tabs.iter().cloned().map(recent_from_path).collect(),
            recents: recents.iter().cloned().map(recent_from_path).collect(),
            active: tabs.first().cloned(),
            query: query.to_owned(),
            selected: 0,
        }
    }

    fn beads_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bd-neon-picker-{tag}"));
        let _ = std::fs::create_dir_all(dir.join(".beads"));
        dir
    }

    #[test]
    fn typed_valid_path_pins_row_zero() {
        let typed = beads_dir("typed-top");
        let props = props_with(
            &[PathBuf::from("/work/alpha"), PathBuf::from("/work/beta")],
            &[PathBuf::from("/work/gamma")],
            typed.to_str().unwrap_or("/work"),
        );
        let items = picker_items(&props, &[]);
        assert!(!items.is_empty());
        assert!(items
            .first()
            .is_some_and(|first| first.path == typed && first.is_typed));
        let _ = std::fs::remove_dir_all(&typed);
    }

    #[test]
    fn typed_matching_tab_moves_to_top_without_dup() {
        let tab_a = beads_dir("tab-a");
        let tab_b = beads_dir("tab-b");
        let props = props_with(
            &[tab_a.clone(), tab_b.clone()],
            &[],
            tab_b.to_str().unwrap_or("/work"),
        );
        let items = picker_items(&props, &[]);
        assert!(items
            .first()
            .is_some_and(|first| first.path == tab_b && first.is_typed));
        assert_eq!(items.iter().filter(|item| item.path == tab_b).count(), 1);
        let _ = std::fs::remove_dir_all(&tab_a);
        let _ = std::fs::remove_dir_all(&tab_b);
    }

    #[test]
    fn fuzzy_query_filters_with_tabs_first() {
        let props = props_with(
            &[PathBuf::from("/work/neon-tabs")],
            &[
                PathBuf::from("/work/neon-helper"),
                PathBuf::from("/work/unrelated-zzz"),
            ],
            "neon",
        );
        let items = picker_items(&props, &[]);
        assert!(items.iter().all(|item| item.is_typed || {
            let text = item.path.to_string_lossy().to_ascii_lowercase();
            text.contains("neon") || item.name.to_ascii_lowercase().contains("neon")
        }));
        assert!(items
            .iter()
            .all(|item| item.path.as_path() != Path::new("/work/unrelated-zzz")));
        let first_tab = items.iter().position(|item| item.is_tab);
        let first_recent = items.iter().position(|item| !item.is_tab && !item.is_typed);
        if let (Some(tab), Some(recent)) = (first_tab, first_recent) {
            assert!(tab < recent);
        }
    }

    #[test]
    fn no_match_no_typed_yields_empty() {
        let props = props_with(
            &[PathBuf::from("/work/alpha")],
            &[PathBuf::from("/work/beta")],
            "zzz-no-such-project",
        );
        let items = picker_items(&props, &[]);
        assert!(items.is_empty());
    }

    #[test]
    fn empty_query_keeps_tabs_then_recents_order() {
        let props = props_with(
            &[PathBuf::from("/work/t1"), PathBuf::from("/work/t2")],
            &[PathBuf::from("/work/r1")],
            "",
        );
        let items = picker_items(&props, &[]);
        let paths: Vec<PathBuf> = items.iter().map(|item| item.path.clone()).collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/work/t1"),
                PathBuf::from("/work/t2"),
                PathBuf::from("/work/r1"),
            ]
        );
    }

    #[test]
    fn clamp_and_step_stay_in_range() {
        assert_eq!(clamp_selection(5, 3), 2);
        assert_eq!(clamp_selection(0, 0), 0);
        assert_eq!(clamp_selection(1, 2), 1);
        assert_eq!(step_selection(0, 3, -1), 0);
        assert_eq!(step_selection(2, 3, 1), 2);
        assert_eq!(step_selection(0, 3, 1), 1);
        assert_eq!(step_selection(2, 3, -1), 1);
        assert_eq!(step_selection(9, 3, 0), 2);
        assert_eq!(step_selection(0, 0, 1), 0);
    }
}
