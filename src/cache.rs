use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{Issue, IssueDetail, Snapshot};

const MAX_ENTRIES: usize = 12;
const MAX_BYTES: u64 = 10 * 1024 * 1024;
const TTL_SECS: i64 = 7 * 24 * 3600;

/// Process-wide serialisation for the on-disk cache index
/// (`index.json` load-modify-save in [`touch_index`]/[`enforce_caps`]).
/// Background touches run on spawned threads, so without this two saves can
/// interleave and drop each other's entries. Poison recovery reuses the
/// `TEST_ENV_LOCK` pattern below.
static INDEX_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Monotonic suffix for [`atomic_write`] temp files so concurrent saves
/// never share a temp path.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Serializes every test that touches the process-global `BD_NEON_CACHE_DIR`
/// (cache tests set it per-test; store tests use fake repos whose cache dir
/// would otherwise collide with an active cache-test temp root when run in
/// parallel). Hold the guard for the whole test body.
#[cfg(test)]
#[allow(clippy::redundant_pub_crate)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Disk snapshot stored at `cache/<slug-hash>/snapshot.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSnapshot {
    pub repo: PathBuf,
    pub fetched_at: DateTime<Utc>,
    pub beads_mtime: Option<u64>,
    pub issues: Vec<Issue>,
    pub ready_ids: HashSet<String>,
}

/// Cache hit with staleness (`true` when `.beads` changed since fetch).
#[derive(Debug, Clone)]
pub struct LoadedCache {
    pub issues: Vec<Issue>,
    pub ready_ids: HashSet<String>,
    pub fetched_at: DateTime<Utc>,
    pub stale: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CacheIndex {
    #[serde(default)]
    entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    dirname: String,
    repo: PathBuf,
    last_used: DateTime<Utc>,
    size_bytes: u64,
}

#[cfg(test)]
pub fn cache_root() -> Option<PathBuf> {
    // In tests never touch the real APPDATA/XDG cache unless the test
    // explicitly opts in via `BD_NEON_CACHE_DIR`.
    std::env::var("BD_NEON_CACHE_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
}

#[cfg(not(test))]
pub fn cache_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("BD_NEON_CACHE_DIR") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA");
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME").or_else(|| {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").into_os_string())
    });
    base.map(|b| PathBuf::from(b).join("bd-neon").join("cache"))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn canonical_string(repo: &Path) -> String {
    fs::canonicalize(repo).map_or_else(
        |_| repo.to_string_lossy().into_owned(),
        |canonical| canonical.to_string_lossy().into_owned(),
    )
}

fn slug_for(repo: &Path) -> String {
    let raw = repo
        .file_name()
        .map_or_else(|| "repo".to_owned(), |n| n.to_string_lossy().into_owned());
    let mut slug = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 32 {
            break;
        }
    }
    let trimmed = slug.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "repo".to_owned()
    } else {
        trimmed
    }
}

pub fn dirname_for(repo: &Path) -> String {
    let canonical = canonical_string(repo);
    let hash = fnv1a64(canonical.as_bytes());
    format!("{}-{hash:016x}", slug_for(repo))
}

fn repo_dir(repo: &Path) -> Option<PathBuf> {
    cache_root().map(|root| root.join(dirname_for(repo)))
}

fn snapshot_path(repo: &Path) -> Option<PathBuf> {
    repo_dir(repo).map(|d| d.join("snapshot.json"))
}

fn detail_path(repo: &Path, id: &str) -> Option<PathBuf> {
    repo_dir(repo).map(|d| d.join("details").join(format!("{}.json", sanitize_id(id))))
}

fn sanitize_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_".to_owned()
    } else {
        out
    }
}

/// Best-effort atomic file write via a unique sibling temp file + rename.
/// The temp name carries the process id plus a monotonic counter so
/// concurrent saves never share (and clobber) one fixed `.tmp` path.
/// No `fsync`: this is a UI cache — a torn write on crash/power-loss just
/// falls back to a cache miss, so rename atomicity is enough.
fn atomic_write(dest: &Path, bytes: &[u8]) -> bool {
    let Some(parent) = dest.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let count = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let tmp = dest.with_extension(format!("json.tmp.{pid}.{count}"));
    if fs::write(&tmp, bytes).is_err() {
        return false;
    }
    fs::rename(&tmp, dest).is_ok()
}

/// Max `modified` mtime (unix secs) over all files under `repo/.beads`.
pub fn beads_mtime(repo: &Path) -> Option<u64> {
    let beads = repo.join(".beads");
    if !beads.is_dir() {
        return None;
    }
    let mut stack = vec![beads];
    let mut max: Option<u64> = None;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = modified.duration_since(UNIX_EPOCH) {
                        let secs = age.as_secs();
                        max = Some(max.map_or(secs, |m: u64| m.max(secs)));
                    }
                }
            }
        }
    }
    max
}

fn dir_size(dir: &Path) -> u64 {
    let mut total: u64 = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                // Skip `atomic_write` orphan temp files (`*.tmp.*`).
                let is_tmp_orphan = entry.file_name().to_string_lossy().contains(".tmp.");
                if !is_tmp_orphan {
                    total = total.saturating_add(meta.len());
                }
            }
        }
    }
    total
}

fn index_path() -> Option<PathBuf> {
    cache_root().map(|r| r.join("index.json"))
}

fn load_index() -> CacheIndex {
    let Some(path) = index_path() else {
        return CacheIndex::default();
    };
    let Ok(bytes) = fs::read(&path) else {
        return CacheIndex::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_index(index: &CacheIndex) {
    let Some(path) = index_path() else {
        return;
    };
    if let Ok(bytes) = serde_json::to_vec(index) {
        let _ = atomic_write(&path, &bytes);
    }
}

fn touch_index(dirname: &str, repo: &Path, size_bytes: u64) {
    let Some(root) = cache_root() else {
        return;
    };
    // Serialise the whole load-modify-save so concurrent background touches
    // cannot interleave and drop each other's entries.
    let _guard = INDEX_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut index = load_index();
    let now = Utc::now();
    if let Some(entry) = index.entries.iter_mut().find(|e| e.dirname == dirname) {
        entry.repo = repo.to_path_buf();
        entry.last_used = now;
        entry.size_bytes = size_bytes;
    } else {
        index.entries.push(IndexEntry {
            dirname: dirname.to_owned(),
            repo: repo.to_path_buf(),
            last_used: now,
            size_bytes,
        });
    }
    save_index(&index);
    enforce_caps(&root, &mut index);
}

/// Evict expired/over-cap entries. Caller must hold `INDEX_LOCK` (currently
/// only `touch_index`, which keeps one guard across load-modify-save-enforce
/// so concurrent background touches cannot interleave).
fn enforce_caps(root: &Path, index: &mut CacheIndex) {
    let now = Utc::now();
    // TTL: drop entries unused for >7 days.
    let mut expired: Vec<String> = Vec::new();
    for entry in &index.entries {
        let age = now.signed_duration_since(entry.last_used).num_seconds();
        if age > TTL_SECS {
            expired.push(entry.dirname.clone());
        }
    }
    for dirname in &expired {
        let _ = fs::remove_dir_all(root.join(dirname));
    }
    index.entries.retain(|e| !expired.contains(&e.dirname));
    // Sort oldest-first for eviction.
    index.entries.sort_by_key(|entry| entry.last_used);
    // Entry-count cap.
    while index.entries.len() > MAX_ENTRIES {
        if let Some(oldest) = index.entries.first().cloned() {
            let _ = fs::remove_dir_all(root.join(&oldest.dirname));
            index.entries.remove(0);
        } else {
            break;
        }
    }
    // Byte cap: refresh sizes from disk before evicting.
    for entry in &mut index.entries {
        let size = dir_size(&root.join(&entry.dirname));
        entry.size_bytes = size;
    }
    let mut total: u64 = 0;
    for entry in &index.entries {
        total = total.saturating_add(entry.size_bytes);
    }
    while total > MAX_BYTES {
        if let Some(oldest) = index.entries.first().cloned() {
            let _ = fs::remove_dir_all(root.join(&oldest.dirname));
            total = total.saturating_sub(oldest.size_bytes);
            index.entries.remove(0);
        } else {
            break;
        }
    }
    save_index(index);
}

/// Persist a fresh snapshot to disk (best-effort, never fails the UI).
pub fn save_snapshot(repo: &Path, snapshot: &Snapshot) {
    let Some(dest) = snapshot_path(repo) else {
        return;
    };
    let cached = CachedSnapshot {
        repo: repo.to_path_buf(),
        fetched_at: snapshot.fetched_at,
        beads_mtime: beads_mtime(repo),
        issues: snapshot.issues.clone(),
        ready_ids: snapshot.ready_ids.clone(),
    };
    let Ok(bytes) = serde_json::to_vec(&cached) else {
        return;
    };
    if !atomic_write(&dest, &bytes) {
        return;
    }
    if let Some(dir) = dest.parent() {
        touch_index(&dirname_for(repo), repo, dir_size(dir));
    }
}

/// Shared read/parse/TTL/corrupt-delete logic for both cache loaders.
/// Returns the parsed snapshot plus the dirname and size hint for the LRU
/// touch. Callers compute staleness themselves.
fn load_inner(repo: &Path) -> Option<(CachedSnapshot, String, u64)> {
    let dirname = dirname_for(repo);
    let dest = cache_root().map(|root| root.join(&dirname).join("snapshot.json"))?;
    let bytes = fs::read(&dest).ok()?;
    let size_hint = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let cached: CachedSnapshot = serde_json::from_slice(&bytes).ok().or_else(|| {
        let _ = fs::remove_file(&dest);
        None
    })?;
    let age = Utc::now()
        .signed_duration_since(cached.fetched_at)
        .num_seconds();
    if age > TTL_SECS {
        let _ = fs::remove_file(&dest);
        return None;
    }
    Some((cached, dirname, size_hint))
}

/// Load a cached snapshot. Returns `None` on miss/corrupt/TTL-expiry.
/// Staleness is computed against the live `.beads` mtime but the cache is
/// still returned (stale-while-revalidate).
/// Hot-path cost is a single `snapshot.json` read + parse plus a `.beads`
/// mtime walk; the LRU index read/write and directory-size rescan are
/// deferred to a background thread so project switches stay under ~50ms.
///
/// Used by tests and background revalidation; the UI switch hot path uses
/// [`load_cached_fast`] (no mtime walk).
#[cfg(test)]
pub fn load_for_repo(repo: &Path) -> Option<LoadedCache> {
    let (cached, dirname, size_hint) = load_inner(repo)?;
    let current = beads_mtime(repo);
    let stale = current != cached.beads_mtime;
    touch_index_async(dirname, repo.to_path_buf(), size_hint);
    Some(LoadedCache {
        issues: cached.issues,
        ready_ids: cached.ready_ids,
        fetched_at: cached.fetched_at,
        stale,
    })
}

/// Hot-path cache seed for `Store::set_repo`: a single `snapshot.json`
/// read + parse with no `.beads` mtime walk, no directory-size scan, and no
/// blocking index I/O. Staleness is left as `true` so the worker refresh
/// always revalidates; the LRU touch runs on a background thread.
pub fn load_cached_fast(repo: &Path) -> Option<LoadedCache> {
    let (cached, dirname, size_hint) = load_inner(repo)?;
    touch_index_async(dirname, repo.to_path_buf(), size_hint);
    Some(LoadedCache {
        issues: cached.issues,
        ready_ids: cached.ready_ids,
        fetched_at: cached.fetched_at,
        stale: true,
    })
}

/// Best-effort LRU touch without blocking the UI thread. Size is
/// reconciled from disk inside the worker (`enforce_caps` rescans), so the
/// hot path passes only the cheap snapshot byte length.
fn touch_index_async(dirname: String, repo: PathBuf, size_hint: u64) {
    std::thread::spawn(move || {
        touch_index(&dirname, &repo, size_hint);
    });
}

/// Persist a snapshot from a background thread (serialization, `.beads`
/// mtime walk, and LRU enforce all run off the UI thread).
pub fn save_snapshot_async(snapshot: Snapshot) {
    std::thread::spawn(move || {
        let repo = snapshot.repo.clone();
        save_snapshot(&repo, &snapshot);
    });
}

pub fn load_detail(repo: &Path, id: &str) -> Option<IssueDetail> {
    let dest = detail_path(repo, id)?;
    let bytes = fs::read(&dest).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_detail(repo: &Path, detail: &IssueDetail) {
    let Some(dest) = detail_path(repo, &detail.id) else {
        return;
    };
    if let Ok(bytes) = serde_json::to_vec(detail) {
        let size_hint = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if atomic_write(&dest, &bytes) {
            // Refresh the LRU timestamp off the hot path; the next
            // snapshot save/enforce pass reconciles sizes from disk.
            touch_index_async(dirname_for(repo), repo.to_path_buf(), size_hint);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::MutexGuard;
    use std::time::{Duration, SystemTime};

    use chrono::Utc;

    use crate::model::{Issue, IssueDetail, Snapshot};

    use super::TEST_ENV_LOCK as ENV_LOCK;
    use super::{beads_mtime, dirname_for, load_detail, load_for_repo, save_detail, save_snapshot};

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn temp_root(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join("bd-neon-cache-tests").join(name);
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn issue(id: &str) -> Issue {
        Issue {
            id: id.to_owned(),
            title: id.to_owned(),
            status: "open".to_owned(),
            priority: 2,
            kind: "task".to_owned(),
            assignee: None,
            updated_at: None,
            description: None,
            acceptance_criteria: None,
        }
    }

    fn snapshot(repo: &str, ids: &[&str]) -> Snapshot {
        Snapshot {
            repo: PathBuf::from(repo),
            issues: ids.iter().map(|id| issue(id)).collect(),
            ready_ids: HashSet::new(),
            fetched_at: Utc::now(),
        }
    }

    fn detail(id: &str) -> IssueDetail {
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

    fn repo_with_beads(root: &std::path::Path, name: &str) -> PathBuf {
        let repo = root.join(name);
        std::fs::create_dir_all(repo.join(".beads")).unwrap();
        std::fs::write(repo.join(".beads").join("issues.jsonl"), "{}\n").unwrap();
        repo
    }

    #[test]
    fn roundtrip_snapshot_and_detail() {
        let _guard = lock_env();
        let root = temp_root("roundtrip");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        let repo = repo_with_beads(&root, "repo-a");
        let snap = snapshot("x", &["a-1", "a-2"]);
        let snap = Snapshot {
            repo: repo.clone(),
            ..snap
        };
        save_snapshot(&repo, &snap);
        let loaded = load_for_repo(&repo).expect("cache hit");
        assert_eq!(loaded.issues.len(), 2);
        assert_eq!(loaded.issues[0].id, "a-1");

        let det = detail("a-1");
        save_detail(&repo, &det);
        let back = load_detail(&repo, "a-1").expect("detail hit");
        assert_eq!(back.id, "a-1");
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }

    #[test]
    fn corrupt_snapshot_falls_back_to_none() {
        let _guard = lock_env();
        let root = temp_root("corrupt");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        let repo = repo_with_beads(&root, "repo-b");
        let snap = Snapshot {
            repo: repo.clone(),
            ..snapshot("x", &["b-1"])
        };
        save_snapshot(&repo, &snap);
        let path = super::snapshot_path(&repo).unwrap();
        std::fs::write(&path, "{not json").unwrap();
        assert!(load_for_repo(&repo).is_none());
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }

    #[test]
    fn stale_then_fresh_replace() {
        let _guard = lock_env();
        let root = temp_root("stale-fresh");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        let repo = repo_with_beads(&root, "repo-c");
        let old = Snapshot {
            repo: repo.clone(),
            ..snapshot("x", &["old-1"])
        };
        save_snapshot(&repo, &old);
        let loaded = load_for_repo(&repo).expect("old cached");
        assert_eq!(loaded.issues[0].id, "old-1");
        let fresh = Snapshot {
            repo: repo.clone(),
            ..snapshot("x", &["new-1", "new-2"])
        };
        save_snapshot(&repo, &fresh);
        let reloaded = load_for_repo(&repo).expect("fresh cached");
        assert_eq!(reloaded.issues.len(), 2);
        assert_eq!(reloaded.issues[0].id, "new-1");
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }

    #[test]
    fn eviction_caps_entries() {
        let _guard = lock_env();
        let root = temp_root("evict");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        for i in 0..15 {
            let repo = repo_with_beads(&root, &format!("repo-{i}"));
            // Canonicalize-dependent dirname: real on-disk path differs per
            // repo so each save lands in its own cache entry.
            let snap = Snapshot {
                repo: repo.clone(),
                ..snapshot("x", &["e-1"])
            };
            save_snapshot(&repo, &snap);
            std::thread::sleep(Duration::from_millis(5));
        }
        let index_bytes = std::fs::read(root.join("index.json")).expect("index exists");
        let index: super::CacheIndex = serde_json::from_slice(&index_bytes).unwrap();
        assert!(index.entries.len() <= super::MAX_ENTRIES);
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }

    #[test]
    fn mtime_change_marks_stale_but_still_serves() {
        let _guard = lock_env();
        let root = temp_root("mtime");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        let repo = repo_with_beads(&root, "repo-m");
        let snap = Snapshot {
            repo: repo.clone(),
            ..snapshot("x", &["m-1"])
        };
        save_snapshot(&repo, &snap);
        let first = load_for_repo(&repo).expect("hit");
        assert!(!first.stale);
        // Bump the beads mtime into the future so it differs from stored.
        let future = SystemTime::now() + Duration::from_secs(60);
        let handle = std::fs::File::options()
            .write(true)
            .open(repo.join(".beads").join("issues.jsonl"))
            .unwrap();
        handle.set_modified(future).unwrap();
        drop(handle);
        assert!(beads_mtime(&repo).is_some());
        let second = load_for_repo(&repo).expect("stale-while-revalidate");
        assert_eq!(second.issues.len(), 1);
        assert!(second.stale);
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }

    #[test]
    fn no_cross_repo_pollution() {
        let _guard = lock_env();
        let root = temp_root("nopollute");
        unsafe { std::env::set_var("BD_NEON_CACHE_DIR", &root) };
        let repo_a = repo_with_beads(&root, "repo-pa");
        let repo_b = repo_with_beads(&root, "repo-pb");
        let snap_a = Snapshot {
            repo: repo_a.clone(),
            ..snapshot("x", &["only-a"])
        };
        save_snapshot(&repo_a, &snap_a);
        assert_ne!(dirname_for(&repo_a), dirname_for(&repo_b));
        assert!(load_for_repo(&repo_b).is_none());
        let hit_a = load_for_repo(&repo_a).expect("a hit");
        assert_eq!(hit_a.issues[0].id, "only-a");
        unsafe { std::env::remove_var("BD_NEON_CACHE_DIR") };
    }
}
