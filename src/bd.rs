use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use rayon::prelude::*;

use crate::model::{Issue, IssueDetail, IssueJson, ShowIssueJson, Snapshot};

#[derive(Debug)]
pub enum BdError {
    Spawn(String),
    Exit(i32, String),
    Parse(String),
}

impl std::fmt::Display for BdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(msg) => write!(f, "failed to run bd: {msg}"),
            Self::Exit(code, msg) => write!(f, "bd exited {code}: {msg}"),
            Self::Parse(msg) => write!(f, "bd json parse failed: {msg}"),
        }
    }
}

pub fn fetch_snapshot(repo: &Path) -> Result<Snapshot, BdError> {
    let (issues, ready) = rayon::join(|| list_all(repo), || ready_ids(repo));
    match (issues, ready) {
        (Ok(issues), Ok(ready_ids)) => Ok(Snapshot {
            repo: repo.to_path_buf(),
            issues,
            ready_ids,
            fetched_at: Utc::now(),
        }),
        (Err(err), _) | (_, Err(err)) => Err(err),
    }
}

pub fn fetch_detail(repo: &Path, id: &str) -> Result<IssueDetail, BdError> {
    let output = run_bd(
        repo,
        &[
            "show",
            id,
            "--json",
            "--brief-deps",
            "--include-dependents",
            "--include-comments",
        ],
    )?;
    let items = parse_show_json(&output)?;
    let first = items
        .into_iter()
        .next()
        .ok_or_else(|| BdError::Parse("empty show response".to_owned()))?;
    Ok(first.into_detail())
}

/// Post a comment as the default `bd` identity (no author picker) and
/// return the refreshed detail. Empty bodies are rejected before spawning.
pub fn post_comment(repo: &Path, id: &str, body: &str) -> Result<IssueDetail, BdError> {
    if body.trim().is_empty() {
        return Err(BdError::Parse("comment is empty".to_owned()));
    }
    // `bd comment <id> [text...]`: body travels as one trailing arg so
    // spaces and newlines survive without shell quoting.
    run_bd(repo, &["comment", id, body.trim()])?;
    fetch_detail(repo, id)
}

fn list_all(repo: &Path) -> Result<Vec<Issue>, BdError> {
    let output = run_bd(repo, &["list", "--json", "--flat", "--all", "--limit", "0"])?;
    let items = parse_json_array(&output)?;
    Ok(items.into_par_iter().map(IssueJson::into_issue).collect())
}

fn ready_ids(repo: &Path) -> Result<HashSet<String>, BdError> {
    let output = run_bd(repo, &["ready", "--json", "--brief"])?;
    let items = parse_json_array(&output)?;
    Ok(items.into_iter().map(|item| item.id).collect())
}

fn run_bd(repo: &Path, args: &[&str]) -> Result<String, BdError> {
    let mut command = Command::new("bd");
    command.arg("-C").arg(repo).args(args);
    let output = command
        .output()
        .map_err(|err| BdError::Spawn(err.to_string()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let message = if stderr.is_empty() { stdout } else { stderr };
        let code = output.status.code().unwrap_or(-1);
        Err(BdError::Exit(code, message))
    }
}

fn parse_array<T>(raw: &str) -> Result<Vec<T>, BdError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw).map_err(|err| BdError::Parse(err.to_string()))
}

fn parse_json_array(raw: &str) -> Result<Vec<IssueJson>, BdError> {
    parse_array(raw)
}

fn parse_show_json(raw: &str) -> Result<Vec<ShowIssueJson>, BdError> {
    parse_array(raw)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::fetch_snapshot;

    #[test]
    fn fabrik_snapshot() {
        let repo = Path::new(r"C:\Arbeit\fabrik");
        if !repo.join(".beads").is_dir() {
            return;
        }
        let snapshot = fetch_snapshot(repo).expect("fabrik snapshot");
        assert_ne!(snapshot.issues, Vec::<crate::model::Issue>::new());
    }

    #[test]
    fn docstool_snapshot() {
        let repo = Path::new(r"C:\Arbeit\docstool-app");
        if !repo.join(".beads").is_dir() {
            return;
        }
        let snapshot = fetch_snapshot(repo).expect("docstool snapshot");
        assert_eq!(snapshot.issues.len(), 7);
    }
}
