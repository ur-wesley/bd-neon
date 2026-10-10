use eframe::egui::{self, ScrollArea, Ui};

use crate::model::{Action, Filter, IssueRow, ViewMode};
use crate::store::Store;

use super::{
    banner, chip, footer, issue_row, kanban, project_tabs, repo_switcher, search_field, segment,
    theme,
};

#[allow(clippy::struct_excessive_bools)]
pub struct BoardView {
    pub current_repo: Option<String>,
    pub active_path: Option<std::path::PathBuf>,
    pub recents: Vec<crate::model::RecentRepo>,
    pub tabs: Vec<crate::model::RecentRepo>,
    pub last_ok_label: String,
    pub live: bool,
    pub view_mode: ViewMode,
    pub filter: Filter,
    pub search: String,
    pub search_focused: bool,
    pub selected: usize,
    pub rows: Vec<IssueRow>,
    pub lanes: Vec<crate::model::KanbanLane>,
    pub collapsed: Vec<Filter>,
    pub error: Option<String>,
    pub repo_path: String,
    pub issue_count: usize,
    pub loading: bool,
    pub stale: bool,
}

impl BoardView {
    pub fn from_store(store: &Store) -> Self {
        // `lanes()` already scans once per lane; derive the visible rows
        // from the matching lane instead of rescanning a 6th time via
        // `visible_rows()`. Runs every frame, so the saved scan matters.
        let lanes = store.lanes();
        let rows = lanes
            .iter()
            .find(|lane| lane.filter == store.filter)
            .map_or_else(Vec::new, |lane| lane.cards.clone());
        Self {
            current_repo: Some(store.repo_label()),
            active_path: store.repo.clone(),
            recents: store.recent_entries(),
            tabs: store.tab_entries(),
            last_ok_label: store.last_ok_label(),
            live: store.last_ok.is_some(),
            view_mode: store.view_mode,
            filter: store.filter,
            search: store.search.clone(),
            search_focused: store.search_focused,
            selected: store.selected,
            rows,
            lanes,
            collapsed: store.collapsed_vec(),
            error: store.error.clone(),
            repo_path: store.repo_path_label(),
            issue_count: store.issue_count(),
            loading: store.loading,
            stale: store.stale,
        }
    }
}

pub fn board(ui: &mut Ui, view: &mut BoardView) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        footer::footer(
            ui,
            &footer::FooterProps {
                live: view.live,
                loading: view.loading,
                stale: view.stale,
                updated: view.last_ok_label.clone(),
                project: view.repo_path.clone(),
                issues: view.issue_count,
            },
        );
        egui::Frame::new()
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(egui::Margin::ZERO)
            .show(ui, |ui| {
                // Content stacks top-down (tabs -> header -> body). The outer
                // `bottom_up` layout only pins the footer; inheriting it here
                // would stack the content in reverse and overlap the sections.
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    actions.extend(project_tabs::project_tabs(
                        ui,
                        &project_tabs::ProjectTabsProps {
                            tabs: view.tabs.clone(),
                            active: view.active_path.clone(),
                            loading: view.loading,
                        },
                    ));
                    egui::Frame::new()
                        .fill(egui::Color32::TRANSPARENT)
                        .inner_margin(egui::Margin {
                            left: theme::PAD_I8,
                            right: theme::PAD_I8,
                            top: theme::SPACING_I8,
                            bottom: theme::PAD_I8,
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            actions.extend(header(ui, view));
                            if let Some(error) = &view.error {
                                ui.add_space(theme::SPACING);
                                banner::banner(ui, error);
                            }
                            ui.add_space(6.0);
                            theme::remaining_body(ui, |ui| {
                                // Disk cache serves instantly: only show the
                                // full spinner when there is nothing cached.
                                // No "cached — updating" text: the footer
                                // live-dot color carries staleness with zero
                                // layout shift.
                                if view.loading && view.issue_count == 0 {
                                    loading_body(ui, view);
                                } else if view.view_mode == ViewMode::List {
                                    actions.extend(list_body(ui, view));
                                } else {
                                    actions.extend(kanban::kanban(
                                        ui,
                                        &kanban::KanbanView {
                                            lanes: view.lanes.clone(),
                                            collapsed: view.collapsed.clone(),
                                        },
                                    ));
                                }
                            });
                        });
                });
            });
    });
    actions
}

fn loading_body(ui: &mut Ui, view: &BoardView) {
    ui.allocate_ui_with_layout(
        ui.available_size(),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(120.0);
            ui.add(egui::Spinner::new().size(24.0));
            ui.add_space(theme::SPACING);
            let name = view
                .current_repo
                .as_deref()
                .filter(|label| *label != "pick a beads repo")
                .unwrap_or(view.repo_path.as_str());
            ui.label(egui::RichText::new(format!("loading {name}…")).color(theme::dim_text()));
        },
    );
}

fn header(ui: &mut Ui, view: &mut BoardView) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), theme::HEADER_H),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.spacing_mut().interact_size.y = theme::CONTROL_H;
            if let Some(action) = segment::segment(
                ui,
                &segment::SegmentProps {
                    view: view.view_mode,
                },
            ) {
                actions.push(action);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.spacing_mut().interact_size.y = theme::CONTROL_H;
                actions.extend(repo_switcher::repo_switcher(
                    ui,
                    &repo_switcher::RepoSwitcherProps {
                        current: view.current_repo.clone(),
                        recents: view.recents.clone(),
                        active_path: view.active_path.clone(),
                    },
                ));
                let mut search_props = search_field::SearchFieldProps {
                    value: view.search.clone(),
                    focused: view.search_focused,
                    hint: "search title or id",
                };
                let (_, changed) = search_field::search_field(ui, &mut search_props);
                view.search = search_props.value;
                view.search_focused = search_props.focused;
                if changed {
                    actions.push(Action::SetSearch(view.search.clone()));
                }
            });
        },
    );
    actions
}

fn list_body(ui: &mut Ui, view: &BoardView) -> Vec<Action> {
    let mut actions = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for filter in crate::model::ALL_LANES {
            if let Some(action) = chip::chip(
                ui,
                &chip::ChipProps {
                    filter,
                    active: view.filter == filter,
                    label: filter.label(),
                },
            ) {
                actions.push(action);
            }
        }
    });
    ui.add_space(theme::SPACING);
    let body_h = ui.available_height();
    ScrollArea::vertical()
        .id_salt("issue_list")
        .auto_shrink([false, false])
        .max_height(body_h)
        .show(ui, |ui| {
            theme::fill_width(ui);
            ui.vertical(|ui| {
                theme::fill_width(ui);
                for (index, row) in view.rows.iter().enumerate() {
                    if let Some(action) = issue_row::issue_row(
                        ui,
                        &issue_row::IssueRowProps {
                            row: row.clone(),
                            selected: index == view.selected,
                        },
                    ) {
                        actions.push(action);
                    }
                    ui.add_space(theme::SPACING);
                }
                if view.rows.is_empty() {
                    ui.label(egui::RichText::new("no issues match").color(theme::dim_text()));
                }
            });
        });
    actions
}
