use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use notify::{Config, Event, RecommendedWatcher, Watcher};

use crate::bd::{fetch_detail, fetch_snapshot, post_comment};
use crate::model::Action;

enum WorkerMsg {
    SetRepo(PathBuf),
    Clear,
    Refresh,
    FetchDetail(String),
    PostComment { id: String, body: String },
}

pub struct WatchService {
    worker_tx: Sender<WorkerMsg>,
    action_rx: Receiver<Action>,
    watcher: Option<RecommendedWatcher>,
    watched_repo: Option<PathBuf>,
}

pub type RepoWorker = WatchService;

impl WatchService {
    pub fn new() -> Self {
        let (worker_tx, worker_rx) = channel::<WorkerMsg>();
        let (action_tx, action_rx) = channel::<Action>();

        thread::spawn(move || {
            worker_loop(&worker_rx, &action_tx);
        });

        Self {
            worker_tx,
            action_rx,
            watcher: None,
            watched_repo: None,
        }
    }

    pub fn set_repo(&mut self, repo: &Path) {
        // Always forward SetRepo: the worker thread's `current_repo` lags
        // `watched_repo`, so an early-return on equality would permanently
        // desync the worker after a dropped/queued switch.
        self.watched_repo = Some(repo.to_path_buf());
        if let Err(err) = self.worker_tx.send(WorkerMsg::SetRepo(repo.to_path_buf())) {
            eprintln!("[worker] SetRepo send failed: {err:?}");
        }
        self.watcher = None;
        let beads = repo.join(".beads");
        if beads.is_dir() {
            let worker_tx = self.worker_tx.clone();
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    // Refresh only on content changes; ignore Access/Other
                    // metadata events to reduce refresh storms. (No debounce
                    // timer yet — coalescing in `drain_coalesced` covers
                    // bursts. KISS.)
                    let changed = matches!(
                        res,
                        Ok(ref event) if matches!(
                            event.kind,
                            notify::EventKind::Create(_)
                                | notify::EventKind::Modify(_)
                                | notify::EventKind::Remove(_)
                        )
                    );
                    if changed {
                        if let Err(err) = worker_tx.send(WorkerMsg::Refresh) {
                            eprintln!("[worker] Refresh send failed: {err:?}");
                        }
                    }
                },
                Config::default(),
            )
            .ok();
            if let Some(ref mut active) = watcher {
                if let Err(err) = active.watch(&beads, notify::RecursiveMode::Recursive) {
                    eprintln!("[worker] watch failed for {}: {err:?}", beads.display());
                }
            }
            self.watcher = watcher;
        }
    }

    pub fn schedule_refresh(&self) {
        if let Err(err) = self.worker_tx.send(WorkerMsg::Refresh) {
            eprintln!("[worker] Refresh send failed: {err:?}");
        }
    }

    pub fn request_detail(&self, id: &str) {
        if let Err(err) = self.worker_tx.send(WorkerMsg::FetchDetail(id.to_owned())) {
            eprintln!("[worker] FetchDetail send failed: {err:?}");
        }
    }

    pub fn post_comment(&self, id: &str, body: &str) {
        if let Err(err) = self.worker_tx.send(WorkerMsg::PostComment {
            id: id.to_owned(),
            body: body.to_owned(),
        }) {
            eprintln!("[worker] PostComment send failed: {err:?}");
        }
    }

    pub fn clear(&mut self) {
        self.watched_repo = None;
        self.watcher = None;
        // Tell the worker to drop its repo too; otherwise the next queued
        // Refresh would waste a fetch for a repo with no tabs open.
        if let Err(err) = self.worker_tx.send(WorkerMsg::Clear) {
            eprintln!("[worker] Clear send failed: {err:?}");
        }
    }

    pub fn drain_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Ok(action) = self.action_rx.try_recv() {
            actions.push(action);
        }
        actions
    }

    pub fn poll(&self, ctx: &eframe::egui::Context) -> Vec<Action> {
        let actions = self.drain_actions();
        if !actions.is_empty() {
            ctx.request_repaint();
        }
        actions
    }
}

impl Default for WatchService {
    fn default() -> Self {
        Self::new()
    }
}

/// Refresh `repo`, then serve every detail/comment coalesced behind it.
/// Shared by the `worker_loop` arms so the loop stays compact.
fn dispatch_pending(action_tx: &Sender<Action>, repo: &Path, pending: &Coalesced) {
    dispatch_refresh(action_tx, repo);
    for id in &pending.details {
        dispatch_detail(action_tx, repo, id);
    }
    for queued in &pending.comments {
        dispatch_post(action_tx, repo, &queued.id, &queued.body);
    }
}

/// Serve details/comments queued behind an already-handled message, then
/// refresh only if a `Refresh` arrived mid-flight (same coalescing the loop
/// always applied; factored out so `worker_loop` stays compact).
fn dispatch_queued(action_tx: &Sender<Action>, repo: &Path, pending: &Coalesced) {
    for id in &pending.details {
        dispatch_detail(action_tx, repo, id);
    }
    for queued in &pending.comments {
        dispatch_post(action_tx, repo, &queued.id, &queued.body);
    }
    if pending.refresh_seen {
        dispatch_refresh(action_tx, repo);
    }
}

fn worker_loop(rx: &Receiver<WorkerMsg>, action_tx: &Sender<Action>) {
    let mut current_repo: Option<PathBuf> = None;
    while let Ok(msg) = rx.recv() {
        match msg {
            WorkerMsg::SetRepo(repo) => {
                current_repo = Some(repo.clone());
                // Coalesce everything queued while this message waited:
                // Refresh bursts collapse into the SetRepo fetch, the latest
                // SetRepo wins, stale FetchDetails for older repos are dropped.
                let pending = drain_coalesced(rx);
                if pending.clear_seen && pending.latest_repo.is_none() {
                    // A Clear arrived after this SetRepo: the repo is gone,
                    // so drop the fetch instead of wasting it.
                    current_repo = None;
                } else if let Some(ref last) = pending.latest_repo {
                    current_repo = Some(last.clone());
                    dispatch_pending(action_tx, last, &pending);
                } else {
                    dispatch_pending(action_tx, &repo, &pending);
                }
            }
            WorkerMsg::Clear => {
                // Last-tab-close: forget the repo so queued Refresh bursts
                // are dropped instead of fetched.
                let pending = drain_coalesced(rx);
                if let Some(ref last) = pending.latest_repo {
                    // A SetRepo queued after the Clear still wins.
                    current_repo = Some(last.clone());
                    dispatch_pending(action_tx, last, &pending);
                } else {
                    current_repo = None;
                }
            }
            WorkerMsg::Refresh => {
                // Coalesce Refresh bursts but never drop SetRepo/FetchDetail.
                let pending = drain_coalesced(rx);
                if pending.clear_seen {
                    // Cleared while this Refresh waited: only a SetRepo
                    // queued after the Clear justifies a fetch.
                    current_repo.clone_from(&pending.latest_repo);
                    if let Some(ref last) = current_repo {
                        dispatch_pending(action_tx, last, &pending);
                    }
                } else if let Some(ref last) = pending.latest_repo {
                    current_repo = Some(last.clone());
                    dispatch_pending(action_tx, last, &pending);
                } else if let Some(ref repo) = current_repo.clone() {
                    dispatch_pending(action_tx, repo, &pending);
                }
            }
            WorkerMsg::FetchDetail(id) => {
                let pending = drain_coalesced(rx);
                if pending.clear_seen && pending.latest_repo.is_none() {
                    // Cleared while this request waited: drop it.
                    current_repo = None;
                } else if let Some(ref last) = pending.latest_repo {
                    // The requested id belongs to the previous repo; drop it
                    // and serve only details queued after the latest SetRepo.
                    current_repo = Some(last.clone());
                    dispatch_pending(action_tx, last, &pending);
                } else if let Some(ref repo) = current_repo.clone() {
                    dispatch_detail(action_tx, repo, &id);
                    dispatch_queued(action_tx, repo, &pending);
                }
            }
            WorkerMsg::PostComment { id, body } => {
                let pending = drain_coalesced(rx);
                if pending.clear_seen && pending.latest_repo.is_none() {
                    // Cleared while this post waited: drop it.
                    current_repo = None;
                } else if let Some(ref last) = pending.latest_repo {
                    // Comment targets the previous repo; drop it and serve
                    // only what was queued after the latest SetRepo.
                    current_repo = Some(last.clone());
                    dispatch_pending(action_tx, last, &pending);
                } else if let Some(ref repo) = current_repo.clone() {
                    dispatch_post(action_tx, repo, &id, &body);
                    dispatch_queued(action_tx, repo, &pending);
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct QueuedComment {
    id: String,
    body: String,
}

#[derive(Debug, Default)]
struct Coalesced {
    latest_repo: Option<PathBuf>,
    refresh_seen: bool,
    clear_seen: bool,
    details: Vec<String>,
    comments: Vec<QueuedComment>,
}

/// Drain the queue without loss: `Refresh` messages coalesce, only the latest
/// `SetRepo` is kept, and `FetchDetail`/`PostComment` queued before a
/// `SetRepo` are stale and dropped. `Clear` acts as a barrier: everything
/// queued before it (pending refresh/details/comments and any earlier
/// `SetRepo`) is dropped; only messages queued after it survive. Never
/// blocks; returns what was pending at call time.
fn drain_coalesced(rx: &Receiver<WorkerMsg>) -> Coalesced {
    let mut out = Coalesced::default();
    while let Ok(pending) = rx.try_recv() {
        match pending {
            WorkerMsg::SetRepo(repo) => {
                out.latest_repo = Some(repo);
                out.details.clear();
                out.comments.clear();
                out.refresh_seen = false;
            }
            WorkerMsg::Clear => {
                out.clear_seen = true;
                out.latest_repo = None;
                out.details.clear();
                out.comments.clear();
                out.refresh_seen = false;
            }
            WorkerMsg::Refresh => {
                out.refresh_seen = true;
            }
            WorkerMsg::FetchDetail(id) => {
                out.details.push(id);
            }
            WorkerMsg::PostComment { id, body } => {
                out.comments.push(QueuedComment { id, body });
            }
        }
    }
    out
}

fn dispatch_refresh(action_tx: &Sender<Action>, path: &Path) {
    match fetch_snapshot(path) {
        Ok(snapshot) => {
            if let Err(err) = action_tx.send(Action::Snapshot(snapshot)) {
                eprintln!("[worker] Snapshot send failed: {err:?}");
            }
        }
        Err(err) => {
            if let Err(send_err) =
                action_tx.send(Action::Failed(Some(path.to_path_buf()), err.to_string()))
            {
                eprintln!("[worker] Failed send failed: {send_err:?}");
            }
        }
    }
}

fn dispatch_detail(action_tx: &Sender<Action>, path: &Path, id: &str) {
    match fetch_detail(path, id) {
        Ok(detail) => {
            if let Err(err) =
                action_tx.send(Action::DetailLoaded(path.to_path_buf(), Box::new(detail)))
            {
                eprintln!("[worker] DetailLoaded send failed: {err:?}");
            }
        }
        Err(err) => {
            if let Err(send_err) =
                action_tx.send(Action::Failed(Some(path.to_path_buf()), err.to_string()))
            {
                eprintln!("[worker] Failed send failed: {send_err:?}");
            }
        }
    }
}

fn dispatch_post(action_tx: &Sender<Action>, path: &Path, id: &str, body: &str) {
    match post_comment(path, id, body) {
        Ok(detail) => {
            if let Err(err) =
                action_tx.send(Action::DetailLoaded(path.to_path_buf(), Box::new(detail)))
            {
                eprintln!("[worker] DetailLoaded send failed: {err:?}");
            }
        }
        Err(err) => {
            if let Err(send_err) =
                action_tx.send(Action::Failed(Some(path.to_path_buf()), err.to_string()))
            {
                eprintln!("[worker] Failed send failed: {send_err:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::{drain_coalesced, WorkerMsg};

    #[test]
    fn rapid_switch_keeps_latest_setrepo() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::SetRepo("a".into())).unwrap();
        tx.send(WorkerMsg::SetRepo("b".into())).unwrap();
        let pending = drain_coalesced(&rx);
        assert_eq!(pending.latest_repo, Some("b".into()));
    }

    #[test]
    fn refresh_burst_does_not_drop_setrepo() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::Refresh).unwrap();
        tx.send(WorkerMsg::Refresh).unwrap();
        tx.send(WorkerMsg::SetRepo("b".into())).unwrap();
        tx.send(WorkerMsg::Refresh).unwrap();
        let pending = drain_coalesced(&rx);
        // The queued SetRepo must survive the Refresh burst.
        assert_eq!(pending.latest_repo, Some("b".into()));
        assert!(pending.refresh_seen);
    }

    #[test]
    fn refresh_burst_without_setrepo_coalesces() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::Refresh).unwrap();
        tx.send(WorkerMsg::Refresh).unwrap();
        tx.send(WorkerMsg::FetchDetail("bd-1".to_owned())).unwrap();
        let pending = drain_coalesced(&rx);
        assert_eq!(pending.latest_repo, None);
        assert!(pending.refresh_seen);
        assert_eq!(pending.details, vec!["bd-1".to_owned()]);
    }

    #[test]
    fn stale_details_before_setrepo_are_dropped() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::FetchDetail("old".to_owned())).unwrap();
        tx.send(WorkerMsg::SetRepo("b".into())).unwrap();
        tx.send(WorkerMsg::FetchDetail("new".to_owned())).unwrap();
        let pending = drain_coalesced(&rx);
        assert_eq!(pending.latest_repo, Some("b".into()));
        assert_eq!(pending.details, vec!["new".to_owned()]);
    }

    #[test]
    fn clear_drops_queued_refresh() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::Refresh).unwrap();
        tx.send(WorkerMsg::FetchDetail("old".to_owned())).unwrap();
        tx.send(WorkerMsg::Clear).unwrap();
        let pending = drain_coalesced(&rx);
        // Everything queued before the Clear is dropped: no wasted fetch.
        assert!(pending.clear_seen);
        assert!(!pending.refresh_seen);
        assert_eq!(pending.latest_repo, None);
        assert_eq!(pending.details, Vec::<String>::new());
        assert!(pending.comments.is_empty());
    }

    #[test]
    fn stale_comments_before_setrepo_are_dropped() {
        let (tx, rx) = channel::<WorkerMsg>();
        tx.send(WorkerMsg::PostComment {
            id: "old".to_owned(),
            body: "stale".to_owned(),
        })
        .unwrap();
        tx.send(WorkerMsg::SetRepo("b".into())).unwrap();
        tx.send(WorkerMsg::PostComment {
            id: "new".to_owned(),
            body: "fresh".to_owned(),
        })
        .unwrap();
        let pending = drain_coalesced(&rx);
        assert_eq!(pending.latest_repo, Some("b".into()));
        assert_eq!(pending.details, Vec::<String>::new());
        assert_eq!(pending.comments.len(), 1);
        assert_eq!(pending.comments[0].id, "new");
        assert_eq!(pending.comments[0].body, "fresh");
    }
}
