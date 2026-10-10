use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Board,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Kanban,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Filter {
    Ready,
    Open,
    Wip,
    Blocked,
    Done,
}

pub const ALL_LANES: [Filter; 5] = [
    Filter::Ready,
    Filter::Open,
    Filter::Wip,
    Filter::Blocked,
    Filter::Done,
];

impl Filter {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Open => "open",
            Self::Wip => "wip",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Open => "Open",
            Self::Wip => "WIP",
            Self::Blocked => "Blocked",
            Self::Done => "Done",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "ready" => Some(Self::Ready),
            "open" => Some(Self::Open),
            "wip" => Some(Self::Wip),
            "blocked" => Some(Self::Blocked),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: u8,
    pub kind: String,
    pub assignee: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub acceptance_criteria: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepNode {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub priority: Option<u8>,
    pub kind: Option<String>,
    pub dep_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub author: Option<String>,
    pub body: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueDetail {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub status: String,
    pub priority: u8,
    pub kind: String,
    pub assignee: Option<String>,
    pub owner: Option<String>,
    pub parent: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub dependencies: Vec<DepNode>,
    pub dependents: Vec<DepNode>,
    pub dependency_count: usize,
    pub dependent_count: usize,
    pub comment_count: usize,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: u8,
    pub kind: String,
    pub flashed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanLane {
    pub filter: Filter,
    pub label: &'static str,
    pub cards: Vec<IssueRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRepo {
    pub path: PathBuf,
    pub name: String,
    pub parent: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub repo: PathBuf,
    pub issues: Vec<Issue>,
    pub ready_ids: HashSet<String>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedTab {
    pub path: PathBuf,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub enum Action {
    RepoPicked(PathBuf),
    PickRecent(PathBuf),
    SetFilter(Filter),
    SetView(ViewMode),
    ToggleLane(Filter),
    SetSearch(String),
    FocusSearch,
    MoveSelection(i32),
    OpenSelected,
    OpenDetail(String),
    Back,
    Refresh,
    Snapshot(Snapshot),
    DetailLoaded(PathBuf, Box<IssueDetail>),
    Failed(Option<PathBuf>, String),
    Tick,
    CloseTab(PathBuf),
    NextTab,
    PrevTab,
    SelectTab(u8),
    ReopenTab,
    ClosePicker,
    TogglePicker,
    SetPickerQuery(String),
    MovePicker(i32),
    PostComment,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueJson {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShowIssueJson {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_dep_list")]
    pub dependencies: Vec<DepNode>,
    #[serde(default, deserialize_with = "deserialize_dep_list")]
    pub dependents: Vec<DepNode>,
    #[serde(default)]
    pub dependency_count: Option<usize>,
    #[serde(default)]
    pub dependent_count: Option<usize>,
    #[serde(default)]
    pub comment_count: Option<usize>,
    #[serde(default)]
    pub comments: Vec<CommentJson>,
}

/// Single comment as returned by `bd show --json --include-comments`.
/// Wire shape uses `text` for the body (see `bd comments <id> --json`).
#[derive(Debug, Clone, Deserialize)]
pub struct CommentJson {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "text")]
    pub body: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl CommentJson {
    pub fn into_comment(self) -> Comment {
        Comment {
            author: nonempty(self.author),
            body: self.body.unwrap_or_default(),
            created_at: parse_time(self.created_at.as_deref()),
        }
    }
}

pub const DEFAULT_PRIORITY: u8 = 2;
pub const DEFAULT_KIND: &str = "task";

fn default_priority(v: Option<u8>) -> u8 {
    v.unwrap_or(DEFAULT_PRIORITY)
}

fn default_kind(v: Option<String>) -> String {
    v.unwrap_or_else(|| DEFAULT_KIND.to_owned())
}

impl ShowIssueJson {
    pub fn into_detail(self) -> IssueDetail {
        let dep_count = self.dependency_count.unwrap_or(self.dependencies.len());
        let dependent_count = self.dependent_count.unwrap_or(self.dependents.len());
        let comments: Vec<Comment> = self
            .comments
            .into_iter()
            .map(CommentJson::into_comment)
            .collect();
        let comment_count = self.comment_count.unwrap_or(comments.len());
        IssueDetail {
            id: self.id,
            title: self.title,
            description: nonempty(self.description),
            acceptance_criteria: nonempty(self.acceptance_criteria),
            status: self.status,
            priority: default_priority(self.priority),
            kind: default_kind(self.kind),
            assignee: nonempty(self.assignee),
            owner: nonempty(self.owner),
            parent: nonempty(self.parent),
            created_at: parse_time(self.created_at.as_deref()),
            updated_at: parse_time(self.updated_at.as_deref()),
            started_at: parse_time(self.started_at.as_deref()),
            closed_at: parse_time(self.closed_at.as_deref()),
            dependencies: self.dependencies,
            dependents: self.dependents,
            dependency_count: dep_count,
            dependent_count,
            comment_count,
            comments,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DepJson {
    Id(String),
    Object(DepFields),
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepFields {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub dep_type: Option<String>,
}

impl DepJson {
    pub fn into_node(self) -> DepNode {
        match self {
            Self::Id(id) => DepNode {
                id,
                title: String::new(),
                status: None,
                priority: None,
                kind: None,
                dep_type: None,
            },
            Self::Object(fields) => DepNode {
                id: fields.id,
                title: fields.title.unwrap_or_default(),
                status: fields.status,
                priority: fields.priority,
                kind: fields.kind,
                dep_type: fields.dep_type,
            },
        }
    }
}

pub fn deserialize_dep_list<'de, D>(deserializer: D) -> Result<Vec<DepNode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<Vec<DepJson>>::deserialize(deserializer)?;
    Ok(raw
        .unwrap_or_default()
        .into_iter()
        .map(DepJson::into_node)
        .collect())
}

impl IssueJson {
    pub fn into_issue(self) -> Issue {
        Issue {
            id: self.id,
            title: self.title,
            status: self.status,
            priority: default_priority(self.priority),
            kind: default_kind(self.kind),
            assignee: nonempty(self.assignee),
            updated_at: parse_time(self.updated_at.as_deref()),
            description: nonempty(self.description),
            acceptance_criteria: nonempty(self.acceptance_criteria),
        }
    }
}

fn parse_time(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    })
}

fn nonempty(val: Option<String>) -> Option<String> {
    val.and_then(|s| {
        let trimmed = s.trim();
        (!trimmed.is_empty()).then_some(trimmed.to_owned())
    })
}

fn to_local(value: DateTime<Utc>) -> DateTime<chrono::Local> {
    value.with_timezone(&chrono::Local)
}

pub fn format_local(value: DateTime<Utc>) -> String {
    to_local(value).format("%d.%m.%Y %H:%M").to_string()
}

pub fn format_clock(value: DateTime<Utc>) -> String {
    to_local(value).format("%H:%M:%S").to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueIdParts {
    pub prefix: String,
    pub number: String,
}

pub fn split_issue_id(id: &str) -> IssueIdParts {
    if let Some((prefix, num)) = id.rsplit_once('-') {
        IssueIdParts {
            prefix: prefix.to_owned(),
            number: num.to_owned(),
        }
    } else {
        IssueIdParts {
            prefix: id.to_owned(),
            number: String::new(),
        }
    }
}

pub fn matches_filter(issue: &Issue, filter: Filter, ready_ids: &HashSet<String>) -> bool {
    match filter {
        Filter::Ready => ready_ids.contains(&issue.id),
        Filter::Open => issue.status == "open",
        Filter::Wip => issue.status == "in_progress",
        Filter::Blocked => issue.status == "blocked",
        Filter::Done => issue.status == "closed",
    }
}

pub fn matches_search(issue: &Issue, query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }
    let q = trimmed.to_ascii_lowercase();
    issue.title.to_ascii_lowercase().contains(&q) || issue.id.to_ascii_lowercase().contains(&q)
}

pub fn recent_from_path(path: PathBuf) -> RecentRepo {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let parent = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    RecentRepo { path, name, parent }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_prefix_body_rev() {
        let parts = split_issue_id("bd-neon-12");
        assert_eq!(parts.prefix, "bd-neon");
        assert_eq!(parts.number, "12");
    }

    #[test]
    fn split_prefix_body() {
        let parts = split_issue_id("custom");
        assert_eq!(parts.prefix, "custom");
        assert_eq!(parts.number, "");
    }

    #[test]
    fn parse_dep_objects_and_ids() {
        let raw = r#"[{"id":"bd-1","title":"first","type":"task","status":"open","priority":1,"dep_type":"blocks"},{"id":"bd-2"}]"#;
        let deps: Vec<DepNode> = serde_json::from_str::<Vec<DepJson>>(raw)
            .expect("valid")
            .into_iter()
            .map(DepJson::into_node)
            .collect();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps.first().map(|d| d.id.as_str()), Some("bd-1"));
        assert_eq!(deps.first().map(|d| d.title.as_str()), Some("first"));
        assert_eq!(deps.get(1).map(|d| d.id.as_str()), Some("bd-2"));
    }

    #[test]
    fn parse_show_comments_text_shape() {
        let raw = r#"[{"id":"bd-1","title":"t","status":"open","comment_count":2,"comments":[{"id":"c1","issue_id":"bd-1","author":"alice","text":"first **bold**","created_at":"2026-10-09T14:35:49Z"},{"id":"c2","issue_id":"bd-1","author":"","text":"second","created_at":null}]}]"#;
        let items: Vec<ShowIssueJson> = serde_json::from_str(raw).expect("valid");
        let detail = items.into_iter().next().expect("one").into_detail();
        assert_eq!(detail.comment_count, 2);
        assert_eq!(detail.comments.len(), 2);
        assert_eq!(
            detail.comments.first().map(|c| c.author.as_deref()),
            Some(Some("alice"))
        );
        assert_eq!(
            detail.comments.first().map(|c| c.body.as_str()),
            Some("first **bold**")
        );
        assert!(detail.comments.first().unwrap().created_at.is_some());
        // Empty author normalizes to None.
        assert_eq!(detail.comments.get(1).and_then(|c| c.author.clone()), None);
    }

    #[test]
    fn old_detail_cache_without_comments_defaults() {
        // Disk caches written before comments existed lack the field.
        let raw = r#"{"id":"bd-1","title":"t","description":null,"acceptance_criteria":null,"status":"open","priority":2,"kind":"task","assignee":null,"owner":null,"parent":null,"created_at":null,"updated_at":null,"started_at":null,"closed_at":null,"dependencies":[],"dependents":[],"dependency_count":0,"dependent_count":0,"comment_count":0}"#;
        let detail: IssueDetail = serde_json::from_str(raw).expect("valid");
        assert!(detail.comments.is_empty());
    }
}
