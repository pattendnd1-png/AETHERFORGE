#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use aether_core::PermissionDecision;
use aether_tools::{MemoryActivitySink, ProcessRequest, ProcessRunner, ToolError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_RECENT_COMMITS: usize = 8;
const MAX_TOUCHED_PATHS_PER_COMMIT: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitContextState {
    Available,
    NotRepository,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitSummary {
    pub hash: String,
    pub subject: String,
    pub touched_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitContextSnapshot {
    pub state: GitContextState,
    pub root: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub modified_paths: Vec<PathBuf>,
    pub untracked_paths: Vec<PathBuf>,
    pub recent_commits: Vec<GitCommitSummary>,
}

impl GitContextSnapshot {
    fn unavailable(root: PathBuf, reason: impl Into<String>) -> Self {
        Self {
            state: GitContextState::Unavailable(reason.into()),
            root,
            branch: None,
            head: None,
            modified_paths: Vec::new(),
            untracked_paths: Vec::new(),
            recent_commits: Vec::new(),
        }
    }

    fn not_repository(root: PathBuf) -> Self {
        Self {
            state: GitContextState::NotRepository,
            root,
            branch: None,
            head: None,
            modified_paths: Vec::new(),
            untracked_paths: Vec::new(),
            recent_commits: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GitContextError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("git root is outside approved roots: {0}")]
    OutsideApprovedRoot(PathBuf),
    #[error(transparent)]
    Tool(#[from] ToolError),
}

#[derive(Debug, Clone)]
pub struct PermissionedGitContextProvider {
    approved_roots: Vec<PathBuf>,
    decision: PermissionDecision,
}

impl PermissionedGitContextProvider {
    pub fn new(approved_roots: Vec<PathBuf>, decision: PermissionDecision) -> Self {
        Self {
            approved_roots,
            decision,
        }
    }

    pub fn snapshots(
        &self,
        requested_roots: &[PathBuf],
    ) -> Result<Vec<GitContextSnapshot>, GitContextError> {
        requested_roots
            .iter()
            .map(|root| git_context_snapshot(root, &self.approved_roots, self.decision))
            .collect()
    }
}

pub fn git_context_snapshot(
    root: &Path,
    approved_roots: &[PathBuf],
    decision: PermissionDecision,
) -> Result<GitContextSnapshot, GitContextError> {
    let canonical_root = fs::canonicalize(root)?;
    if !is_approved(&canonical_root, approved_roots) {
        return Err(GitContextError::OutsideApprovedRoot(canonical_root));
    }

    let runner = ProcessRunner::new(MemoryActivitySink::default());

    let probe = match run_git(
        &runner,
        &canonical_root,
        approved_roots,
        decision,
        ["rev-parse", "--is-inside-work-tree"],
    ) {
        Ok(result) => result,
        Err(ToolError::Spawn(reason)) => {
            return Ok(GitContextSnapshot::unavailable(canonical_root, reason));
        }
        Err(error) => return Err(error.into()),
    };

    if probe.exit_code != 0 || probe.stdout.trim() != "true" {
        return Ok(GitContextSnapshot::not_repository(canonical_root));
    }

    let branch = successful_stdout(run_git(
        &runner,
        &canonical_root,
        approved_roots,
        decision,
        ["branch", "--show-current"],
    )?);
    let head = successful_stdout(run_git(
        &runner,
        &canonical_root,
        approved_roots,
        decision,
        ["rev-parse", "HEAD"],
    )?);

    let status = run_git(
        &runner,
        &canonical_root,
        approved_roots,
        decision,
        ["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let (modified_paths, untracked_paths) = if status.exit_code == 0 {
        parse_status(&status.stdout)
    } else {
        (Vec::new(), Vec::new())
    };

    let max_recent = MAX_RECENT_COMMITS.to_string();
    let log = run_git(
        &runner,
        &canonical_root,
        approved_roots,
        decision,
        ["log", "-n", &max_recent, "--pretty=format:%H%x09%s"],
    )?;

    let mut recent_commits = Vec::new();
    if log.exit_code == 0 {
        for line in log.stdout.lines().take(MAX_RECENT_COMMITS) {
            let Some((hash, subject)) = line.split_once('\t') else {
                continue;
            };
            let touched = run_git(
                &runner,
                &canonical_root,
                approved_roots,
                decision,
                ["diff-tree", "--no-commit-id", "--name-only", "-r", hash],
            )?;
            let mut touched_paths = touched
                .stdout
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(MAX_TOUCHED_PATHS_PER_COMMIT)
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            touched_paths.sort();
            touched_paths.dedup();
            recent_commits.push(GitCommitSummary {
                hash: hash.to_string(),
                subject: subject.to_string(),
                touched_paths,
            });
        }
    }

    Ok(GitContextSnapshot {
        state: GitContextState::Available,
        root: canonical_root,
        branch,
        head,
        modified_paths,
        untracked_paths,
        recent_commits,
    })
}

fn is_approved(root: &Path, approved_roots: &[PathBuf]) -> bool {
    approved_roots.iter().any(|approved| {
        fs::canonicalize(approved)
            .map(|approved| root.starts_with(approved))
            .unwrap_or(false)
    })
}

fn run_git<S, I>(
    runner: &ProcessRunner<MemoryActivitySink>,
    root: &Path,
    approved_roots: &[PathBuf],
    decision: PermissionDecision,
    args: I,
) -> Result<aether_tools::ProcessResult, ToolError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let request = ProcessRequest::new("git", args, root).approved_roots(approved_roots.to_vec());
    runner.run(request, decision)
}

fn successful_stdout(result: aether_tools::ProcessResult) -> Option<String> {
    if result.exit_code != 0 {
        return None;
    }
    let value = result.stdout.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn parse_status(text: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut modified = BTreeSet::new();
    let mut untracked = BTreeSet::new();

    for record in text.split('\0') {
        let bytes = record.as_bytes();
        if bytes.len() < 4 || bytes[2] != b' ' {
            continue;
        }
        let status = &record[..2];
        let path = record[3..].trim();
        if path.is_empty() {
            continue;
        }
        if status == "??" {
            untracked.insert(PathBuf::from(path));
        } else {
            modified.insert(PathBuf::from(path));
        }
    }

    (
        modified.into_iter().collect(),
        untracked.into_iter().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use uuid::Uuid;

    fn td() -> PathBuf {
        let path = std::env::temp_dir().join(format!("aether-workspace-git-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn setup_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git fixture command failed: {args:?}");
    }

    #[test]
    fn git_snapshot_reports_branch_head_changes_and_recent_commit_paths() {
        let root = td();

        if !git_available() {
            let snapshot = git_context_snapshot(
                &root,
                std::slice::from_ref(&root),
                PermissionDecision::AllowThisProject,
            )
            .unwrap();
            assert!(matches!(snapshot.state, GitContextState::Unavailable(_)));
            let _ = fs::remove_dir_all(root);
            return;
        }

        setup_git(&root, &["init", "-q"]);
        setup_git(&root, &["config", "user.name", "AetherAI Test"]);
        setup_git(&root, &["config", "user.email", "aetherai@example.invalid"]);

        fs::write(root.join("a.txt"), "one\n").unwrap();
        setup_git(&root, &["add", "a.txt"]);
        setup_git(&root, &["commit", "-qm", "first"]);

        fs::write(root.join("b.txt"), "two\n").unwrap();
        setup_git(&root, &["add", "b.txt"]);
        setup_git(&root, &["commit", "-qm", "second"]);

        fs::write(root.join("a.txt"), "one changed\n").unwrap();
        fs::write(root.join("untracked.txt"), "new\n").unwrap();

        let snapshot = git_context_snapshot(
            &root,
            std::slice::from_ref(&root),
            PermissionDecision::AllowThisProject,
        )
        .unwrap();

        assert_eq!(snapshot.state, GitContextState::Available);
        assert!(
            snapshot
                .branch
                .as_deref()
                .is_some_and(|branch| !branch.is_empty())
        );
        assert!(snapshot.head.as_deref().is_some_and(|head| head.len() >= 7));
        assert!(snapshot.modified_paths.contains(&PathBuf::from("a.txt")));
        assert!(
            snapshot
                .untracked_paths
                .contains(&PathBuf::from("untracked.txt"))
        );
        assert!(snapshot.recent_commits.iter().any(|commit| {
            commit.subject == "second" && commit.touched_paths.contains(&PathBuf::from("b.txt"))
        }));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn git_snapshot_never_executes_outside_approved_roots() {
        let root = td();
        let other = td();

        let result = git_context_snapshot(
            &root,
            std::slice::from_ref(&other),
            PermissionDecision::AllowThisProject,
        );
        assert!(matches!(
            result,
            Err(GitContextError::OutsideApprovedRoot(_))
        ));

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(other);
    }
}
