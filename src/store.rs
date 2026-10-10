use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use crate::model::{
    format_clock, matches_filter, matches_search, recent_from_path, Action, ClosedTab, Filter,
    Issue, IssueDetail, IssueRow, KanbanLane, RecentRepo, Screen, Snapshot, ViewMode, ALL_LANES,
};

const FLASH_MS: u64 = 1200;
const MAX_RECENTS: usize = 12;
const MAX_TABS: usize = 8;
/// 1-based tab slot max for `SelectTab` (Ctrl+1..Ctrl+8, Chrome clamp).
pub const MAX_TAB_SLOT: u8 = 8;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PersistedState {
    pub recents: Vec<PathBuf>,
    pub open_tabs: Vec<PathBuf>,
    #[serde(default)]
    pub selected_tab: Option<PathBuf>,
    #[serde(default)]
    pub collapsed_lanes: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)]
pub struct Store {
    pub repo: Option<PathBuf>,
    pub issues: Vec<Issue>,
    pub ready_ids: HashSet<String>,
    pub screen: Screen,
    pub view_mode: ViewMode,
    pub filter: Filter,
    pub search: String,
    pub search_focused: bool,
    pub selected: usize,
    pub detail_id: Option<String>,
    pub detail: Option<IssueDetail>,
    pub detail_cache: HashMap<String, IssueDetail>,
    pub comment_draft: String,
    pub comment_posting: bool,
    pub comment_error: Option<String>,
    pub collapsed_lanes: HashSet<Filter>,
    pub recents: Vec<PathBuf>,
    pub open_tabs: Vec<PathBuf>,
    pub closed_tab_stack: Vec<ClosedTab>,
    pub picker_open: bool,
    pub picker_query: String,
    pub picker_selected: usize,
    pub last_ok: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub loading: bool,
    pub stale: bool,
    pub flash_until: HashMap<String, Instant>,
    pub revision: u64,
    previous_updated: HashMap<String, DateTime<Utc>>,
}

impl Store {
    pub fn new() -> Self {
        let persisted = load_state();
        let recents = sanitize_paths(&persisted.recents);
        let mut open_tabs = sanitize_paths(&persisted.open_tabs);
        // Restore the previously selected tab when it is still open;
        // otherwise fall back to the first valid recent.
        let selected_restored = persisted
            .selected_tab
            .as_ref()
            .filter(|p| open_tabs.iter().any(|t| t == *p))
            .cloned();
        let repo = selected_restored.or_else(|| {
            recents
                .iter()
                .find(|p| p.join(".beads").is_dir())
                .or_else(|| recents.first())
                .cloned()
        });
        if let Some(ref r) = repo {
            if !open_tabs.iter().any(|p| p == r) {
                open_tabs.insert(0, r.clone());
            }
        }
        let open_tabs: Vec<PathBuf> = open_tabs
            .into_iter()
            .filter(|p| p.join(".beads").is_dir())
            .collect();
        let collapsed_lanes: HashSet<Filter> = persisted
            .collapsed_lanes
            .iter()
            .filter_map(|key| Filter::from_key(key))
            .collect();

        Self {
            repo,
            issues: Vec::new(),
            ready_ids: HashSet::new(),
            screen: Screen::Board,
            view_mode: ViewMode::Kanban,
            filter: Filter::Ready,
            search: String::new(),
            search_focused: false,
            selected: 0,
            detail_id: None,
            detail: None,
            detail_cache: HashMap::new(),
            comment_draft: String::new(),
            comment_posting: false,
            comment_error: None,
            collapsed_lanes,
            recents,
            open_tabs,
            closed_tab_stack: Vec::new(),
            picker_open: false,
            picker_query: String::new(),
            picker_selected: 0,
            last_ok: None,
            error: None,
            loading: false,
            stale: false,
            flash_until: HashMap::new(),
            revision: 0,
            previous_updated: HashMap::new(),
        }
    }

    // Large action enum: one arm per variant. Split would scatter the
    // reducer; the allow matches `repo_switcher::switcher_popup`.
    #[allow(clippy::too_many_lines)]
    pub fn reduce(&mut self, action: Action) -> Vec<Action> {
        match action {
            Action::RepoPicked(path) => {
                self.set_repo_guarded(path, "Not a beads repository (.beads/ is missing)")
            }
            Action::PickRecent(path) => {
                self.set_repo_guarded(path, "Selected repository is no longer valid")
            }
            Action::SetFilter(filter) => {
                self.filter = filter;
                self.selected = 0;
                self.touch();
                Vec::new()
            }
            Action::SetView(mode) => {
                self.view_mode = mode;
                self.touch();
                Vec::new()
            }
            Action::ToggleLane(filter) => {
                if self.collapsed_lanes.contains(&filter) {
                    self.collapsed_lanes.remove(&filter);
                } else {
                    self.collapsed_lanes.insert(filter);
                }
                self.touch();
                self.persist();
                Vec::new()
            }
            Action::SetSearch(query) => {
                self.search = query;
                self.selected = 0;
                self.touch();
                Vec::new()
            }
            Action::FocusSearch => {
                self.search_focused = true;
                self.touch();
                Vec::new()
            }
            Action::MoveSelection(delta) => {
                self.move_selection(delta);
                Vec::new()
            }
            Action::OpenSelected => {
                let id = self
                    .visible_rows()
                    .get(self.selected)
                    .map(|row| row.id.clone());
                id.map_or_else(Vec::new, |id| self.open_detail(&id))
            }
            Action::OpenDetail(id) => self.open_detail(&id),
            Action::Back => {
                self.close_detail();
                Vec::new()
            }
            Action::Refresh => {
                self.loading = true;
                self.touch();
                Vec::new()
            }
            Action::Snapshot(snapshot) => {
                self.apply_snapshot(snapshot);
                Vec::new()
            }
            Action::DetailLoaded(repo, detail) => {
                self.apply_detail_loaded(&repo, detail);
                Vec::new()
            }
            Action::Failed(repo, message) => {
                self.apply_failed(repo.as_deref(), message);
                Vec::new()
            }
            Action::Tick => {
                self.apply_tick();
                Vec::new()
            }
            Action::CloseTab(path) => self.close_tab(&path),
            Action::NextTab => self.step_tab(1),
            Action::PrevTab => self.step_tab(-1),
            Action::SelectTab(slot) => self.select_tab(slot),
            Action::ReopenTab => self.reopen_tab(),
            Action::ClosePicker => self.set_picker(false),
            Action::TogglePicker => {
                let open = !self.picker_open;
                self.set_picker(open)
            }
            Action::SetPickerQuery(query) => self.set_picker_query(query),
            Action::MovePicker(delta) => self.move_picker(delta),
            Action::PostComment => {
                if self.comment_draft.trim().is_empty() {
                    self.comment_error = Some("Comment is empty".to_owned());
                    self.touch();
                    return Vec::new();
                }
                self.comment_posting = true;
                self.comment_error = None;
                self.touch();
                Vec::new()
            }
        }
    }

    pub fn visible_rows(&self) -> Vec<IssueRow> {
        self.rows_for_filter(self.filter)
    }

    pub fn lanes(&self) -> Vec<KanbanLane> {
        ALL_LANES
            .iter()
            .map(|filter| KanbanLane {
                filter: *filter,
                label: filter.label(),
                cards: self.rows_for_filter(*filter),
            })
            .collect()
    }

    pub fn tab_entries(&self) -> Vec<RecentRepo> {
        self.open_tabs
            .iter()
            .map(|path| recent_from_path(path.clone()))
            .collect()
    }

    pub fn recent_entries(&self) -> Vec<RecentRepo> {
        self.recents
            .iter()
            .map(|path| recent_from_path(path.clone()))
            .collect()
    }

    pub fn repo_label(&self) -> String {
        self.repo.as_ref().map_or_else(
            || "pick a beads repo".to_owned(),
            |p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("repo")
                    .to_owned()
            },
        )
    }

    pub fn repo_path_label(&self) -> String {
        self.repo
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_owned()
    }

    pub fn last_ok_label(&self) -> String {
        self.last_ok
            .map_or_else(|| "waiting".to_owned(), format_clock)
    }

    pub const fn issue_count(&self) -> usize {
        self.issues.len()
    }

    pub fn collapsed_vec(&self) -> Vec<Filter> {
        self.collapsed_lanes.iter().copied().collect()
    }

    fn preview_detail(&self, id: &str) -> Option<IssueDetail> {
        self.issues
            .iter()
            .find(|issue| issue.id == id)
            .map(|issue| IssueDetail {
                id: issue.id.clone(),
                title: issue.title.clone(),
                description: issue.description.clone(),
                acceptance_criteria: issue.acceptance_criteria.clone(),
                status: issue.status.clone(),
                priority: issue.priority,
                kind: issue.kind.clone(),
                assignee: issue.assignee.clone(),
                owner: None,
                parent: None,
                created_at: None,
                updated_at: issue.updated_at,
                started_at: None,
                closed_at: None,
                dependencies: Vec::new(),
                dependents: Vec::new(),
                dependency_count: 0,
                dependent_count: 0,
                comment_count: 0,
                comments: Vec::new(),
            })
    }

    fn rows_for_filter(&self, filter: Filter) -> Vec<IssueRow> {
        // Sequential scan: typical repos are hundreds of issues, and this
        // runs up to 6x per frame via `lanes()` + `visible_rows()`. Rayon
        // thread-pool hops cost more than the scan itself on the UI thread.
        let now = Instant::now();
        self.issues
            .iter()
            .filter(|issue| matches_filter(issue, filter, &self.ready_ids))
            .filter(|issue| matches_search(issue, &self.search))
            .map(|issue| IssueRow {
                id: issue.id.clone(),
                title: issue.title.clone(),
                status: issue.status.clone(),
                priority: issue.priority,
                kind: issue.kind.clone(),
                flashed: self
                    .flash_until
                    .get(&issue.id)
                    .is_some_and(|until| *until > now),
            })
            .collect()
    }

    fn open_detail(&mut self, id: &str) -> Vec<Action> {
        self.detail_id = Some(id.to_string());
        self.comment_draft.clear();
        self.comment_posting = false;
        self.comment_error = None;
        if let Some(cached) = self.detail_cache.get(id) {
            self.detail = Some(cached.clone());
        } else if let Some(repo) = self.repo.clone() {
            // Disk preview first (instant), then the worker fetches fresh.
            if let Some(disk) = crate::cache::load_detail(&repo, id) {
                self.detail_cache.insert(id.to_owned(), disk.clone());
                self.detail = Some(disk);
            } else if let Some(preview) = self.preview_detail(id) {
                self.detail = Some(preview);
            } else {
                self.detail = None;
            }
        } else if let Some(preview) = self.preview_detail(id) {
            self.detail = Some(preview);
        } else {
            self.detail = None;
        }
        self.screen = Screen::Detail;
        self.touch();
        Vec::new()
    }

    fn close_detail(&mut self) {
        self.screen = Screen::Board;
        self.detail_id = None;
        self.detail = None;
        self.comment_draft.clear();
        self.comment_posting = false;
        self.comment_error = None;
        self.touch();
    }

    fn set_repo_guarded(&mut self, path: PathBuf, missing_msg: &str) -> Vec<Action> {
        if !path.join(".beads").is_dir() {
            self.error = Some(missing_msg.to_owned());
            self.loading = false;
            self.touch();
            return Vec::new();
        }
        self.set_repo(path);
        vec![Action::Refresh]
    }

    fn set_repo(&mut self, path: PathBuf) {
        self.repo = Some(path.clone());
        self.push_recent(path.clone());
        self.push_tab(path);
        self.picker_open = false;
        self.issues.clear();
        self.ready_ids.clear();
        self.detail = None;
        self.detail_id = None;
        self.detail_cache.clear();
        self.selected = 0;
        self.error = None;
        self.comment_draft.clear();
        self.comment_posting = false;
        self.comment_error = None;
        self.flash_until.clear();
        self.previous_updated.clear();
        self.screen = Screen::Board;
        // Instant first paint: a single `snapshot.json` read + parse with no
        // `.beads` mtime walk, no size rescan, and no blocking index I/O
        // (those run on background threads inside the cache). Details stay
        // lazy (`load_detail` only on popup open). The worker refresh queued
        // by the caller revalidates right after, so `stale` starts true.
        // No flash on cache load; seed `previous_updated` from the cache.
        if let Some(hit) = crate::cache::load_cached_fast(
            self.repo
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("")),
        ) {
            let mut seeded = HashMap::new();
            for issue in &hit.issues {
                if let Some(updated) = issue.updated_at {
                    seeded.insert(issue.id.clone(), updated);
                }
            }
            self.previous_updated = seeded;
            self.issues = hit.issues;
            self.ready_ids = hit.ready_ids;
            self.last_ok = Some(hit.fetched_at);
            self.loading = true;
            // Fast path always seeds `stale: true` so the worker revalidates;
            // the flag stays internal only (no layout-shifting label).
            self.stale = hit.stale;
        } else {
            self.last_ok = None;
            self.loading = true;
            self.stale = false;
        }
        self.touch();
        self.persist();
    }

    fn push_recent(&mut self, path: PathBuf) {
        self.recents.retain(|p| p != &path);
        self.recents.insert(0, path);
        if self.recents.len() > MAX_RECENTS {
            self.recents.truncate(MAX_RECENTS);
        }
    }

    fn push_tab(&mut self, path: PathBuf) {
        if !self.open_tabs.contains(&path) {
            self.open_tabs.push(path);
            if self.open_tabs.len() > MAX_TABS {
                self.open_tabs.remove(0);
            }
        }
    }

    fn close_tab(&mut self, path: &Path) -> Vec<Action> {
        let Some(index) = self.open_tabs.iter().position(|p| p == path) else {
            return Vec::new();
        };
        let removed = self.open_tabs.remove(index);
        self.closed_tab_stack.push(ClosedTab {
            path: removed,
            index,
        });
        if self.open_tabs.is_empty() {
            self.clear_repo();
            return Vec::new();
        }
        let was_active = self.repo.as_deref() == Some(path);
        if was_active {
            if let Some(next) = self.open_tabs.last().cloned() {
                self.set_repo(next);
                return vec![Action::Refresh];
            }
        }
        self.touch();
        self.persist();
        Vec::new()
    }

    #[must_use]
    pub fn active_tab_index(&self) -> Option<usize> {
        self.repo
            .as_ref()
            .and_then(|repo| self.open_tabs.iter().position(|p| p == repo))
    }

    fn step_tab(&mut self, delta: i32) -> Vec<Action> {
        let len = self.open_tabs.len();
        if len < 2 {
            return Vec::new();
        }
        let Some(current) = self.active_tab_index() else {
            return Vec::new();
        };
        let next = if delta >= 0 {
            current
                .saturating_add(1)
                .checked_rem(len)
                .unwrap_or(current)
        } else {
            current
                .checked_sub(1)
                .unwrap_or_else(|| len.saturating_sub(1))
        };
        if next == current {
            return Vec::new();
        }
        let Some(path) = self.open_tabs.get(next).cloned() else {
            return Vec::new();
        };
        self.set_repo(path);
        vec![Action::Refresh]
    }

    fn select_tab(&mut self, slot: u8) -> Vec<Action> {
        debug_assert!(MAX_TABS >= usize::from(MAX_TAB_SLOT));
        if !(1..=MAX_TAB_SLOT).contains(&slot) || self.open_tabs.is_empty() {
            return Vec::new();
        }
        let len = self.open_tabs.len();
        // Chrome behavior: slot 8 selects the last tab when more than 8 exist.
        let target = if slot == MAX_TAB_SLOT && len > MAX_TABS {
            len.saturating_sub(1)
        } else {
            usize::from(slot).saturating_sub(1)
        };
        if target >= len {
            return Vec::new();
        }
        if self.active_tab_index() == Some(target) {
            return Vec::new();
        }
        let Some(path) = self.open_tabs.get(target).cloned() else {
            return Vec::new();
        };
        self.set_repo(path);
        vec![Action::Refresh]
    }

    fn reopen_tab(&mut self) -> Vec<Action> {
        let Some(closed) = self.closed_tab_stack.pop() else {
            return Vec::new();
        };
        if self.open_tabs.contains(&closed.path) {
            self.touch();
            return Vec::new();
        }
        let mut index = closed.index.min(self.open_tabs.len());
        if self.open_tabs.len() >= MAX_TABS {
            self.open_tabs.remove(0);
            index = index.saturating_sub(1).min(self.open_tabs.len());
        }
        self.open_tabs.insert(index, closed.path.clone());
        self.set_repo(closed.path);
        vec![Action::Refresh]
    }

    fn set_picker(&mut self, open: bool) -> Vec<Action> {
        if self.picker_open != open {
            self.picker_open = open;
            self.touch();
        }
        if open {
            // Fresh autofocused query on every open (VSCode Ctrl+P behavior).
            self.picker_query.clear();
            self.picker_selected = 0;
            self.touch();
        }
        Vec::new()
    }

    fn set_picker_query(&mut self, query: String) -> Vec<Action> {
        self.picker_query = query;
        self.picker_selected = 0;
        self.touch();
        Vec::new()
    }

    fn move_picker(&mut self, delta: i32) -> Vec<Action> {
        // Upper-bound clamping happens in the picker view (it knows the
        // filtered length); the store only saturates at zero here.
        let current = i32::try_from(self.picker_selected).unwrap_or(0);
        let next = current.saturating_add(delta).max(0);
        let next_selected = usize::try_from(next).unwrap_or(0);
        if next_selected != self.picker_selected {
            self.picker_selected = next_selected;
            self.touch();
        }
        Vec::new()
    }

    fn clear_repo(&mut self) {
        self.repo = None;
        self.issues.clear();
        self.ready_ids.clear();
        self.detail = None;
        self.detail_id = None;
        self.detail_cache.clear();
        self.comment_draft.clear();
        self.comment_posting = false;
        self.comment_error = None;
        self.selected = 0;
        self.error = None;
        self.last_ok = None;
        self.loading = false;
        self.stale = false;
        self.flash_until.clear();
        self.previous_updated.clear();
        self.screen = Screen::Board;
        self.touch();
        self.persist();
    }

    fn move_selection(&mut self, delta: i32) {
        let count = self.visible_rows().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        let max = i32::try_from(count.saturating_sub(1)).unwrap_or(0);
        let current = i32::try_from(self.selected).unwrap_or(0);
        let shifted = current.checked_add(delta).unwrap_or(current);
        let next = shifted.clamp(0, max);
        let next = usize::try_from(next).unwrap_or(0);
        if next != self.selected {
            self.selected = next;
            self.touch();
        }
    }

    fn apply_detail_loaded(&mut self, repo: &Path, detail: Box<IssueDetail>) {
        // Ignore details fetched for a previously selected repo; ids can
        // collide across repos and would otherwise pollute the new cache.
        if self.repo.as_deref() != Some(repo) {
            return;
        }
        crate::cache::save_detail(repo, &detail);
        self.detail_cache
            .insert(detail.id.clone(), (*detail).clone());
        if self.detail_id.as_ref() == Some(&detail.id) {
            self.detail = Some(*detail);
            self.error = None;
            // A fresh load for the open issue resolves a pending post:
            // clear the spinner, the submitted draft, and any past error.
            // Plain refreshes also land here; the draft is only non-empty
            // while the user is typing, and a post round-trip always ends
            // with posting=true, so gate the draft clear on that flag to
            // avoid wiping in-progress typing on background refreshes.
            if self.comment_posting {
                self.comment_posting = false;
                self.comment_draft.clear();
                self.comment_error = None;
            }
            self.touch();
        }
    }

    fn apply_failed(&mut self, repo: Option<&Path>, message: String) {
        // Stale failures for a previous repo must not clear the loading
        // spinner or overwrite the error of the current repo.
        if let Some(repo) = repo {
            if self.repo.as_deref() != Some(repo) {
                return;
            }
        }
        self.error = Some(message.clone());
        self.loading = false;
        // A post round-trip surfaces here too: drop the spinner and show
        // the failure on the composer instead of leaving it stuck.
        if self.comment_posting {
            self.comment_posting = false;
            self.comment_error = Some(message);
        }
        self.touch();
    }

    fn apply_tick(&mut self) {
        let now = Instant::now();
        let before = self.flash_until.len();
        self.flash_until.retain(|_, until| *until > now);
        if self.flash_until.len() != before {
            self.touch();
        }
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) {
        // Ignore snapshots fetched for a previous repo (rapid A->B switch):
        // blindly overwriting would show A's tasks while B is selected.
        if self.repo.as_deref() != Some(snapshot.repo.as_path()) {
            return;
        }
        // Disk write (serialize + mtime walk + LRU enforce) runs on a
        // background thread; the UI thread keeps only the move into `self`.
        crate::cache::save_snapshot_async(snapshot.clone());
        let now = Instant::now();
        let flash_duration = Duration::from_millis(FLASH_MS);
        let mut next_updated = HashMap::new();

        for issue in &snapshot.issues {
            if let Some(updated) = issue.updated_at {
                if let Some(previous) = self.previous_updated.get(&issue.id) {
                    if *previous != updated {
                        if let Some(until) = now.checked_add(flash_duration) {
                            self.flash_until.insert(issue.id.clone(), until);
                        }
                    }
                }
                next_updated.insert(issue.id.clone(), updated);
            }
        }

        self.previous_updated = next_updated;
        self.issues = snapshot.issues;
        self.ready_ids = snapshot.ready_ids;
        self.last_ok = Some(snapshot.fetched_at);
        self.error = None;
        self.loading = false;
        self.stale = false;
        self.touch();
    }

    const fn touch(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn persist(&self) {
        // `state.json` write runs off the UI thread so rapid tab switches
        // never block first paint on serialization + fsync.
        let state = PersistedState {
            recents: self.recents.clone(),
            open_tabs: self.open_tabs.clone(),
            selected_tab: self.repo.clone(),
            collapsed_lanes: self
                .collapsed_lanes
                .iter()
                .map(|f| f.key().to_owned())
                .collect(),
        };
        std::thread::spawn(move || {
            save_state(&state);
        });
    }
}

fn state_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA");
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME").or_else(|| {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").into_os_string())
    });
    base.map(|b| PathBuf::from(b).join("bd-neon").join("state.json"))
}

fn load_state() -> PersistedState {
    let Some(path) = state_path() else {
        return PersistedState::default();
    };
    let Ok(bytes) = fs::read(path) else {
        return PersistedState::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn sanitize_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|p| p.join(".beads").is_dir())
        .cloned()
        .collect()
}

#[cfg(test)]
const fn save_state(_state: &PersistedState) {}

#[cfg(not(test))]
fn save_state(state: &PersistedState) {
    let Some(path) = state_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(state) {
        let _ = fs::write(path, bytes);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use chrono::Utc;

    use crate::model::{Action, Issue, IssueDetail, Snapshot, ALL_LANES};

    use super::Store;

    fn lock_cache_env() -> std::sync::MutexGuard<'static, ()> {
        crate::cache::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn snapshot_for(repo: &str, ids: &[&str]) -> Snapshot {
        Snapshot {
            repo: PathBuf::from(repo),
            issues: ids
                .iter()
                .map(|id| Issue {
                    id: (*id).to_owned(),
                    title: (*id).to_owned(),
                    status: "open".to_owned(),
                    priority: 2,
                    kind: "task".to_owned(),
                    assignee: None,
                    updated_at: None,
                    description: None,
                    acceptance_criteria: None,
                })
                .collect(),
            ready_ids: HashSet::new(),
            fetched_at: Utc::now(),
        }
    }

    fn detail_for(id: &str) -> IssueDetail {
        IssueDetail {
            id: id.to_owned(),
            title: id.to_owned(),
            description: None,
            acceptance_criteria: None,
            status: "open".to_owned(),
            priority: 2,
            kind: "task".to_owned(),
            assignee: None,
            owner: None,
            parent: None,
            created_at: None,
            updated_at: None,
            started_at: None,
            closed_at: None,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            dependency_count: 0,
            dependent_count: 0,
            comment_count: 0,
            comments: Vec::new(),
        }
    }

    #[test]
    fn open_detail_seeds_preview() {
        let _guard = lock_cache_env();
        let mut store = Store::new();
        store.issues.push(Issue {
            id: "bd-1".to_owned(),
            title: "one".to_owned(),
            status: "open".to_owned(),
            priority: 1,
            kind: "task".to_owned(),
            assignee: None,
            updated_at: None,
            description: None,
            acceptance_criteria: None,
        });
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        assert_eq!(store.detail.as_ref().map(|d| d.title.as_str()), Some("one"));
    }

    #[test]
    fn lanes_cover_all_filters() {
        let store = Store::new();
        let lanes = store.lanes();
        assert_eq!(lanes.len(), ALL_LANES.len());
    }

    #[test]
    fn revision_bumps_on_visible_mutations() {
        let mut store = Store::new();
        let rev0 = store.revision;
        store.reduce(Action::SetSearch("foo".to_owned()));
        assert!(store.revision > rev0);
    }

    #[test]
    fn recents_cap_and_dedupe() {
        let mut store = Store::new();
        store.recents = Vec::new();
        for i in 0..13 {
            store.push_recent(PathBuf::from(format!("r{i}")));
        }
        // Cap: 13 pushes keep only the 12 most recent.
        assert_eq!(store.recents.len(), 12);
        // MRU order: newest first, oldest evicted.
        assert_eq!(store.recents[0], PathBuf::from("r12"));
        assert_eq!(store.recents[11], PathBuf::from("r1"));
        assert!(!store.recents.contains(&PathBuf::from("r0")));
        // Dedupe: re-pushing moves to front without duplicating.
        store.push_recent(PathBuf::from("r5"));
        assert_eq!(store.recents.len(), 12);
        assert_eq!(store.recents[0], PathBuf::from("r5"));
        assert_eq!(
            store
                .recents
                .iter()
                .filter(|p| *p == &PathBuf::from("r5"))
                .count(),
            1
        );
    }

    #[test]
    fn open_tabs_add_keep_order_and_close() {
        let mut store = Store::new();
        store.open_tabs = Vec::new();
        for i in 0..9 {
            store.push_tab(PathBuf::from(format!("tab{i}")));
        }
        // Cap: 9 pushes keep only 8 tabs, oldest evicted.
        assert_eq!(store.open_tabs.len(), 8);
        assert!(!store.open_tabs.contains(&PathBuf::from("tab0")));
        assert_eq!(store.open_tabs[0], PathBuf::from("tab1"));
        assert_eq!(store.open_tabs[7], PathBuf::from("tab8"));
        assert_eq!(store.tab_entries().len(), 8);
    }

    #[test]
    fn open_detail_uses_list_markdown() {
        let _guard = lock_cache_env();
        let mut store = Store::new();
        store.issues.push(Issue {
            id: "bd-1".to_owned(),
            title: "one".to_owned(),
            status: "open".to_owned(),
            priority: 1,
            kind: "task".to_owned(),
            assignee: None,
            updated_at: None,
            description: Some("body markdown".to_owned()),
            acceptance_criteria: Some("criteria markdown".to_owned()),
        });
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        let detail = store.detail.as_ref().unwrap();
        assert_eq!(detail.description.as_deref(), Some("body markdown"));
        assert_eq!(
            detail.acceptance_criteria.as_deref(),
            Some("criteria markdown")
        );
    }

    fn tab_store(paths: &[&str], active: &str) -> Store {
        let mut store = Store::new();
        store.open_tabs = paths.iter().map(PathBuf::from).collect();
        store.repo = Some(PathBuf::from(active));
        store.closed_tab_stack = Vec::new();
        store
    }

    #[test]
    fn next_tab_wraps_sequentially() {
        let mut store = tab_store(&["a", "b", "c"], "c");
        store.reduce(Action::NextTab);
        assert_eq!(store.repo, Some(PathBuf::from("a")));
        store.reduce(Action::NextTab);
        assert_eq!(store.repo, Some(PathBuf::from("b")));
    }

    #[test]
    fn prev_tab_wraps_left() {
        let mut store = tab_store(&["a", "b", "c"], "a");
        store.reduce(Action::PrevTab);
        assert_eq!(store.repo, Some(PathBuf::from("c")));
        store.reduce(Action::PrevTab);
        assert_eq!(store.repo, Some(PathBuf::from("b")));
    }

    #[test]
    fn select_tab_picks_nth_and_clamps_slot_8_to_last() {
        let mut store = tab_store(
            &["t1", "t2", "t3", "t4", "t5", "t6", "t7", "t8", "t9"],
            "t1",
        );
        store.reduce(Action::SelectTab(3));
        assert_eq!(store.repo, Some(PathBuf::from("t3")));
        store.reduce(Action::SelectTab(8));
        assert_eq!(store.repo, Some(PathBuf::from("t9")));
        store.reduce(Action::SelectTab(7));
        assert_eq!(store.repo, Some(PathBuf::from("t7")));
    }

    #[test]
    fn select_tab_out_of_range_is_noop() {
        let mut store = tab_store(&["a", "b"], "a");
        store.reduce(Action::SelectTab(5));
        assert_eq!(store.repo, Some(PathBuf::from("a")));
        store.reduce(Action::SelectTab(0));
        assert_eq!(store.repo, Some(PathBuf::from("a")));
        store.reduce(Action::SelectTab(9));
        assert_eq!(store.repo, Some(PathBuf::from("a")));
    }

    #[test]
    fn close_pushes_stack_and_reopen_restores_position() {
        let mut store = tab_store(&["a", "b", "c"], "b");
        store.reduce(Action::CloseTab(PathBuf::from("b")));
        assert_eq!(store.closed_tab_stack.len(), 1);
        assert_eq!(store.closed_tab_stack.first().map(|c| c.index), Some(1));
        assert_eq!(store.repo, Some(PathBuf::from("c")));
        store.reduce(Action::ReopenTab);
        assert_eq!(
            store.open_tabs,
            vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c"),]
        );
        assert_eq!(store.repo, Some(PathBuf::from("b")));
        assert!(store.closed_tab_stack.is_empty());
    }

    #[test]
    fn reopen_empty_stack_is_noop() {
        let mut store = tab_store(&["a"], "a");
        let rev = store.revision;
        store.reduce(Action::ReopenTab);
        assert_eq!(store.open_tabs, vec![PathBuf::from("a")]);
        assert_eq!(store.repo, Some(PathBuf::from("a")));
        assert_eq!(store.revision, rev);
    }

    #[test]
    fn picker_toggle_open_close() {
        let mut store = tab_store(&["a"], "a");
        assert!(!store.picker_open);
        store.reduce(Action::TogglePicker);
        assert!(store.picker_open);
        store.reduce(Action::TogglePicker);
        assert!(!store.picker_open);
        store.reduce(Action::TogglePicker);
        assert!(store.picker_open);
        store.reduce(Action::ClosePicker);
        assert!(!store.picker_open);
    }

    #[test]
    fn picker_query_resets_selection_and_move_saturates() {
        let mut store = tab_store(&["a"], "a");
        store.reduce(Action::SetPickerQuery("neon".to_owned()));
        assert_eq!(store.picker_query, "neon");
        assert_eq!(store.picker_selected, 0);
        store.reduce(Action::MovePicker(3));
        assert_eq!(store.picker_selected, 3);
        store.reduce(Action::MovePicker(-10));
        assert_eq!(store.picker_selected, 0);
        store.reduce(Action::SetPickerQuery("other".to_owned()));
        assert_eq!(store.picker_selected, 0);
    }

    #[test]
    fn picker_open_resets_query_and_repo_pick_closes() {
        let mut store = tab_store(&["a"], "a");
        store.reduce(Action::SetPickerQuery("stale".to_owned()));
        store.reduce(Action::MovePicker(2));
        store.reduce(Action::TogglePicker);
        assert!(store.picker_open);
        assert_eq!(store.picker_query, String::new());
        assert_eq!(store.picker_selected, 0);
        store.reduce(Action::ClosePicker);
        assert!(!store.picker_open);
    }

    #[test]
    fn persisted_state_roundtrips_tabs_and_selection() {
        let state = super::PersistedState {
            recents: vec![PathBuf::from("a")],
            open_tabs: vec![PathBuf::from("a"), PathBuf::from("b")],
            selected_tab: Some(PathBuf::from("b")),
            collapsed_lanes: Vec::new(),
        };
        let bytes = serde_json::to_vec(&state).unwrap();
        let back: super::PersistedState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.open_tabs.len(), 2);
        assert_eq!(back.selected_tab, Some(PathBuf::from("b")));
    }

    #[test]
    fn rapid_switch_ignores_stale_snapshot() {
        // Switch A -> B fast: a late Snapshot for A must not overwrite B.
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::Snapshot(snapshot_for("b", &["b-1"])));
        assert_eq!(store.issues.len(), 1);
        assert_eq!(store.issues[0].id, "b-1");
        let rev = store.revision;
        store.reduce(Action::Snapshot(snapshot_for("a", &["a-1", "a-2"])));
        assert_eq!(store.issues.len(), 1);
        assert_eq!(store.issues[0].id, "b-1");
        assert_eq!(store.revision, rev);
    }

    #[test]
    fn snapshot_for_current_repo_applies() {
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::Snapshot(snapshot_for("b", &["b-1", "b-2"])));
        assert_eq!(store.issues.len(), 2);
        assert!(!store.loading);
        assert!(store.error.is_none());
    }

    #[test]
    fn stale_detail_loaded_is_ignored() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.reduce(Action::DetailLoaded(
            PathBuf::from("a"),
            Box::new(detail_for("bd-1")),
        ));
        assert!(!store.detail_cache.contains_key("bd-1"));
        store.reduce(Action::DetailLoaded(
            PathBuf::from("b"),
            Box::new(detail_for("bd-1")),
        ));
        assert!(store.detail_cache.contains_key("bd-1"));
        assert_eq!(store.detail.as_ref().map(|d| d.id.as_str()), Some("bd-1"));
    }

    #[test]
    fn stale_failed_does_not_clear_loading() {
        let mut store = tab_store(&["a", "b"], "b");
        store.loading = true;
        store.reduce(Action::Failed(
            Some(PathBuf::from("a")),
            "old boom".to_owned(),
        ));
        assert!(store.loading);
        assert!(store.error.is_none());
        store.reduce(Action::Failed(
            Some(PathBuf::from("b")),
            "new boom".to_owned(),
        ));
        assert!(!store.loading);
        assert_eq!(store.error.as_deref(), Some("new boom"));
    }

    #[test]
    fn comment_post_sets_spinner_and_success_clears_draft() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.comment_draft = "hello".to_owned();
        store.reduce(Action::PostComment);
        assert!(store.comment_posting);
        assert!(store.comment_error.is_none());
        store.reduce(Action::DetailLoaded(
            PathBuf::from("b"),
            Box::new(detail_for("bd-1")),
        ));
        assert!(!store.comment_posting);
        assert!(store.comment_draft.is_empty());
        assert!(store.comment_error.is_none());
    }

    #[test]
    fn comment_post_empty_sets_error() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.reduce(Action::PostComment);
        assert!(!store.comment_posting);
        assert!(store.comment_error.is_some());
    }

    #[test]
    fn comment_failed_clears_spinner_and_sets_composer_error() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.comment_draft = "hello".to_owned();
        store.reduce(Action::PostComment);
        assert!(store.comment_posting);
        store.reduce(Action::Failed(
            Some(PathBuf::from("b")),
            "bd exited 1: boom".to_owned(),
        ));
        assert!(!store.comment_posting);
        assert_eq!(store.comment_error.as_deref(), Some("bd exited 1: boom"));
        // Draft is kept so the user can retry.
        assert_eq!(store.comment_draft, "hello");
    }

    #[test]
    fn detail_refresh_while_typing_keeps_draft() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.comment_draft = "typing…".to_owned();
        store.reduce(Action::DetailLoaded(
            PathBuf::from("b"),
            Box::new(detail_for("bd-1")),
        ));
        assert_eq!(store.comment_draft, "typing…");
    }

    #[test]
    fn open_detail_resets_composer() {
        let _guard = lock_cache_env();
        let mut store = tab_store(&["a", "b"], "b");
        store.reduce(Action::OpenDetail("bd-1".to_owned()));
        store.comment_draft = "hello".to_owned();
        store.reduce(Action::PostComment);
        store.reduce(Action::OpenDetail("bd-2".to_owned()));
        assert!(store.comment_draft.is_empty());
        assert!(!store.comment_posting);
        assert!(store.comment_error.is_none());
    }
}
