use std::time::{Duration, Instant};

use eframe::egui;
use egui_commonmark::CommonMarkCache;

use crate::model::{Action, Screen};
use crate::platform;
use crate::store::{MAX_TAB_SLOT, Store};
use crate::ui::{board, detail, project_picker, theme, window_chrome};
use crate::watch::RepoWorker;

const TICK_MS: u64 = 200;
const REPAINT_MS: u64 = 250;
const GLASS_REAPPLY_EVERY_N_FRAMES: u64 = 30;

const TAB_KEYS: [egui::Key; 9] = [
    egui::Key::Num1,
    egui::Key::Num2,
    egui::Key::Num3,
    egui::Key::Num4,
    egui::Key::Num5,
    egui::Key::Num6,
    egui::Key::Num7,
    egui::Key::Num8,
    egui::Key::Num9,
];

/// Map a 0-based `TAB_KEYS` index to a 1-based `SelectTab` slot
/// (`1..=MAX_TAB_SLOT`). `Num9` (index 8) clamps to slot `MAX_TAB_SLOT` so
/// `Ctrl+9` selects the last tab, matching Chrome behavior in
/// `Store::select_tab`.
fn tab_slot(index: usize) -> Option<u8> {
    u8::try_from(index)
        .ok()
        .map(|base| base.saturating_add(1).min(MAX_TAB_SLOT))
}

pub struct BdNeonApp {
    store: Store,
    worker: RepoWorker,
    last_tick: Instant,
    markdown: CommonMarkCache,
    last_focus: Option<bool>,
    last_maximized: Option<bool>,
    glass_applied: bool,
    glass_tick: u64,
}

impl BdNeonApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.all_styles_mut(theme::apply);
        let worker = RepoWorker::new();
        let mut store = Store::new();
        // First non-flag CLI arg is the repo path. Skip flags such as
        // `--glass-debug` / `--renderer=...` so they never become a path.
        if let Some(arg) = std::env::args().skip(1).find(|a| !a.starts_with('-')) {
            store.reduce(Action::RepoPicked(std::path::PathBuf::from(arg)));
        }
        let mut app = Self {
            store,
            worker,
            last_tick: Instant::now(),
            markdown: CommonMarkCache::default(),
            last_focus: None,
            last_maximized: None,
            glass_applied: false,
            glass_tick: 0,
        };
        if let Some(repo) = app.store.repo.clone() {
            app.worker.set_repo(&repo);
        }
        app
    }

    fn apply_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::OpenDetail(id) => {
                    if self.store.repo.is_some() {
                        self.worker.request_detail(&id);
                    }
                    let followups = self.store.reduce(Action::OpenDetail(id));
                    self.apply_actions(followups);
                }
                Action::PostComment => {
                    // Snapshot the draft before the reducer flips posting on;
                    // the worker posts on its thread and answers with a fresh
                    // DetailLoaded (or Failed), which clears the spinner.
                    if let (Some(_), Some(id)) =
                        (self.store.repo.clone(), self.store.detail_id.clone())
                    {
                        let body = self.store.comment_draft.clone();
                        if !body.trim().is_empty() && !self.store.comment_posting {
                            self.worker.post_comment(&id, &body);
                        }
                    }
                    let followups = self.store.reduce(Action::PostComment);
                    self.apply_actions(followups);
                }
                Action::Refresh => {
                    // The worker already tracks the selected repo via SetRepo;
                    // a Refresh only re-fetches the worker's current repo.
                    self.worker.schedule_refresh();
                    let followups = self.store.reduce(Action::Refresh);
                    self.apply_actions(followups);
                }
                Action::RepoPicked(path) => {
                    self.apply_repo_switch(Action::RepoPicked(path));
                }
                Action::PickRecent(path) => {
                    self.apply_repo_switch(Action::PickRecent(path));
                }
                Action::CloseTab(path) => {
                    self.apply_repo_switch(Action::CloseTab(path));
                }
                Action::NextTab | Action::PrevTab | Action::SelectTab(_) | Action::ReopenTab => {
                    self.apply_repo_switch(action);
                }
                other => {
                    let followups = self.store.reduce(other);
                    self.apply_actions(followups);
                }
            }
        }
    }

    /// Reduce a potentially repo-changing action, sync the worker to the new
    /// repo, and apply followups. `SetRepo` already triggers a fresh fetch,
    /// so `Refresh` followups are reduced store-side only (spinner state)
    /// without an extra worker round-trip.
    fn apply_repo_switch(&mut self, action: Action) {
        let before = self.store.repo.clone();
        let followups = self.store.reduce(action);
        if self.store.repo != before {
            if let Some(repo) = self.store.repo.clone() {
                self.worker.set_repo(&repo);
            } else {
                self.worker.clear();
            }
        }
        for followup in followups {
            match followup {
                Action::Refresh => {
                    let nested = self.store.reduce(Action::Refresh);
                    self.apply_actions(nested);
                }
                other => self.apply_actions(vec![other]),
            }
        }
    }

    fn handle_keyboard(&self, ctx: &egui::Context) -> Vec<Action> {
        let mut actions = Vec::new();
        // Global Ctrl chords: consume so TextEdits don't swallow them.
        // `consume_key` uses `matches_logically` (extra Shift/Alt ignored),
        // so check the most-specific Shift variants first.
        ctx.input_mut(|input| {
            if input.consume_key(
                egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
                egui::Key::Tab,
            ) {
                actions.push(Action::PrevTab);
            } else if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Tab) {
                actions.push(Action::NextTab);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::PageDown) {
                actions.push(Action::NextTab);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::PageUp) {
                actions.push(Action::PrevTab);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::P) {
                actions.push(Action::TogglePicker);
            }
            if input.consume_key(
                egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
                egui::Key::T,
            ) {
                actions.push(Action::ReopenTab);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
                && let Some(path) = self.store.repo.clone()
            {
                actions.push(Action::CloseTab(path));
            }
            // CRITICAL: TAB_KEYS is 0-based (Num1 at index 0) but SelectTab
            // slots are 1-based (1..=MAX_TAB_SLOT, Chrome clamp). Num9 also maps to 8.
            for (index, key) in TAB_KEYS.iter().enumerate() {
                if input.consume_key(egui::Modifiers::COMMAND, *key)
                    && let Some(slot) = tab_slot(index)
                {
                    actions.push(Action::SelectTab(slot));
                }
            }
        });
        // Esc priority: picker > detail > clear-search. Non-consuming so the
        // workspace popup keeps its own Esc-to-close behavior.
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Escape) {
                if self.store.picker_open {
                    actions.push(Action::ClosePicker);
                } else if self.store.screen == Screen::Detail {
                    actions.push(Action::Back);
                } else {
                    actions.push(Action::SetSearch(String::new()));
                }
            }
        });
        // While the picker modal is open, board nav is blocked (see picker issue).
        if self.store.picker_open {
            return actions;
        }
        if self.store.screen == Screen::Detail {
            return actions;
        }
        // List + Kanban share the same selection nav (Up/Down move, Enter opens).
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Slash) {
                actions.push(Action::FocusSearch);
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                actions.push(Action::MoveSelection(1));
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                actions.push(Action::MoveSelection(-1));
            }
            if input.key_pressed(egui::Key::Enter) {
                actions.push(Action::OpenSelected);
            }
        });
        actions
    }

    fn show_popup(&mut self, ctx: &egui::Context) {
        // Ctrl+P picker modal renders above everything; board keyboard nav
        // is already blocked while `picker_open` (see `handle_keyboard`).
        if self.store.picker_open {
            let props = project_picker::PickerProps {
                tabs: self.store.tab_entries(),
                recents: self.store.recent_entries(),
                active: self.store.repo.clone(),
                query: self.store.picker_query.clone(),
                selected: self.store.picker_selected,
            };
            let actions = project_picker::project_picker(ctx, &props);
            self.apply_actions(actions);
        }
        if self.store.screen != Screen::Detail {
            return;
        }
        let view = detail::DetailView {
            detail: self.store.detail.clone(),
            loading_id: self.store.detail_id.clone(),
            error: self.store.error.clone(),
        };
        let composer = detail::ComposerState {
            draft: &mut self.store.comment_draft,
            posting: self.store.comment_posting,
            error: self.store.comment_error.clone(),
        };
        // Translucent popup frame (faked blur: alpha + dim + stroke + shadow).
        // egui Modal cannot do per-widget real blur, only alpha compositing.
        let frame = theme::dialog_frame(ctx.global_style().as_ref());
        let modal = egui::Modal::new(egui::Id::new("issue_popup"))
            .frame(frame)
            .backdrop_color(theme::dialog_backdrop())
            .show(ctx, |ui| {
                detail::popup(ui, &view, &mut self.markdown, composer)
            });
        let should_close = modal.should_close();
        let mut actions = Vec::new();
        if let Some(action) = modal.inner {
            actions.push(action);
        }
        if should_close {
            actions.push(Action::Back);
        }
        self.apply_actions(actions);
    }
}

impl eframe::App for BdNeonApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let focused = ctx.input(|i| i.viewport().focused) != Some(false);
        let maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
        // DWM resets backdrop attributes after resize/maximize, and hwnd
        // resolve can fail on early frames: reapply on focus/maximize change
        // and periodically (every GLASS_REAPPLY_EVERY_N_FRAMES frames ≈ 7.5s
        // at REPAINT_MS repaint cadence).
        let focus_changed = self.last_focus != Some(focused);
        let maximized_changed = self.last_maximized != Some(maximized);
        let periodic = self.glass_tick.is_multiple_of(GLASS_REAPPLY_EVERY_N_FRAMES);
        if (!self.glass_applied || focus_changed || maximized_changed || periodic)
            && platform::refresh_glass(Some(frame), focused)
        {
            self.glass_applied = true;
            self.last_focus = Some(focused);
            self.last_maximized = Some(maximized);
        }
        self.glass_tick = self.glass_tick.wrapping_add(1);

        if self.last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            self.last_tick = Instant::now();
            self.apply_actions(vec![Action::Tick]);
        }

        let worker_actions = self.worker.poll(&ctx);
        self.apply_actions(worker_actions);

        let keyboard_actions = self.handle_keyboard(&ctx);
        self.apply_actions(keyboard_actions);

        let mut actions = Vec::new();

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                window_chrome::shell(ui, |ui| {
                    let mut board_view = board::BoardView::from_store(&self.store);
                    actions = board::board(ui, &mut board_view);
                });
            });

        self.show_popup(&ctx);
        self.apply_actions(actions);

        ctx.request_repaint_after(Duration::from_millis(REPAINT_MS));
    }
}

#[cfg(test)]
mod tests {
    use super::tab_slot;

    #[test]
    fn tab_slot_is_one_based_with_num9_clamped_to_8() {
        assert_eq!(tab_slot(0), Some(1));
        assert_eq!(tab_slot(1), Some(2));
        assert_eq!(tab_slot(6), Some(7));
        assert_eq!(tab_slot(7), Some(8));
        // Ctrl+1 must NOT become slot 0 (no-op); Num9 clamps to last slot.
        assert_eq!(tab_slot(8), Some(8));
    }
}
