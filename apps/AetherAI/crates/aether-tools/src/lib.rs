use aether_core::PermissionDecision;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Process,
    FileChange,
    ModelLoad,
    Inference,
    Download,
    Discovery,
    System,
    Index,
    Retrieval,
    Memory,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityStatus {
    Running,
    Passed,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: Uuid,
    pub kind: ActivityKind,
    pub status: ActivityStatus,
    pub command: Option<String>,
    pub cwd: Option<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub details_json: Option<String>,
}
pub trait ActivitySink: Send + Sync {
    fn record(&self, event: ActivityEvent) -> Result<(), ToolError>;
}
#[derive(Default)]
pub struct MemoryActivitySink {
    events: Mutex<Vec<ActivityEvent>>,
}
impl MemoryActivitySink {
    pub fn events(&self) -> Vec<ActivityEvent> {
        self.events.lock().map(|v| v.clone()).unwrap_or_default()
    }
}
impl ActivitySink for MemoryActivitySink {
    fn record(&self, event: ActivityEvent) -> Result<(), ToolError> {
        self.events
            .lock()
            .map_err(|_| ToolError::Activity("activity lock poisoned".into()))?
            .push(event);
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ProcessRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub approved_roots: Vec<PathBuf>,
}
impl ProcessRequest {
    pub fn new<I, S>(command: impl Into<String>, args: I, cwd: impl Into<PathBuf>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let cwd = cwd.into();
        Self {
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
            approved_roots: vec![cwd.clone()],
            cwd,
        }
    }
    pub fn approved_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.approved_roots = roots;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
pub struct ProcessRunner<S: ActivitySink> {
    sink: S,
}
impl<S: ActivitySink> ProcessRunner<S> {
    pub fn new(sink: S) -> Self {
        Self { sink }
    }
    pub fn run(
        &self,
        request: ProcessRequest,
        decision: PermissionDecision,
    ) -> Result<ProcessResult, ToolError> {
        if !matches!(
            decision,
            PermissionDecision::AllowThisChat
                | PermissionDecision::AllowThisProject
                | PermissionDecision::AlwaysAllow
        ) {
            return Err(ToolError::PermissionDenied(
                "process execution is not approved".into(),
            ));
        }
        let cwd = canonical_existing(&request.cwd)?;
        let mut approved = false;
        for root in &request.approved_roots {
            if let Ok(root) = canonical_existing(root) {
                if cwd.starts_with(root) {
                    approved = true;
                    break;
                }
            }
        }
        if !approved {
            return Err(ToolError::OutsideWorkspace(cwd));
        }
        let id = Uuid::new_v4();
        let started = Utc::now();
        let cmdline = std::iter::once(request.command.as_str())
            .chain(request.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        self.sink.record(ActivityEvent {
            id,
            kind: ActivityKind::Process,
            status: ActivityStatus::Running,
            command: Some(cmdline.clone()),
            cwd: Some(cwd.clone()),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            started_at: started,
            finished_at: None,
            details_json: None,
        })?;
        let output = Command::new(&request.command)
            .args(&request.args)
            .current_dir(&cwd)
            .output()
            .map_err(|e| ToolError::Spawn(e.to_string()))?;
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let status = if output.status.success() {
            ActivityStatus::Passed
        } else {
            ActivityStatus::Failed
        };
        self.sink.record(ActivityEvent {
            id,
            kind: ActivityKind::Process,
            status,
            command: Some(cmdline),
            cwd: Some(cwd),
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            exit_code: Some(code),
            started_at: started,
            finished_at: Some(Utc::now()),
            details_json: None,
        })?;
        Ok(ProcessResult {
            stdout,
            stderr,
            exit_code: code,
        })
    }
}
fn canonical_existing(p: &Path) -> Result<PathBuf, ToolError> {
    std::fs::canonicalize(p).map_err(|e| ToolError::InvalidCwd(format!("{}: {e}", p.display())))
}
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("working directory is outside approved roots: {0}")]
    OutsideWorkspace(PathBuf),
    #[error("invalid working directory: {0}")]
    InvalidCwd(String),
    #[error("process spawn failed: {0}")]
    Spawn(String),
    #[error("activity error: {0}")]
    Activity(String),
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    struct Shared(Arc<MemoryActivitySink>);
    impl ActivitySink for Shared {
        fn record(&self, e: ActivityEvent) -> Result<(), ToolError> {
            self.0.record(e)
        }
    }
    fn td() -> PathBuf {
        let p = std::env::temp_dir().join(format!("aether-tools-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn blocked_never_spawns() {
        let sink = MemoryActivitySink::default();
        let r = ProcessRunner::new(sink);
        let d = td();
        let req = ProcessRequest::new("definitely-not-a-command", Vec::<String>::new(), &d);
        assert!(matches!(
            r.run(req, PermissionDecision::Blocked),
            Err(ToolError::PermissionDenied(_))
        ));
        let _ = std::fs::remove_dir_all(d);
    }
    #[test]
    fn outside_root_never_spawns() {
        let sink = MemoryActivitySink::default();
        let r = ProcessRunner::new(sink);
        let d = td();
        let other = td();
        let req = ProcessRequest::new("true", Vec::<String>::new(), &d)
            .approved_roots(vec![other.clone()]);
        assert!(matches!(
            r.run(req, PermissionDecision::AlwaysAllow),
            Err(ToolError::OutsideWorkspace(_))
        ));
        let _ = std::fs::remove_dir_all(d);
        let _ = std::fs::remove_dir_all(other);
    }
    #[test]
    fn real_process_is_audited() {
        let sink = Arc::new(MemoryActivitySink::default());
        let r = ProcessRunner::new(Shared(sink.clone()));
        let d = td();
        let req = ProcessRequest::new("sh", ["-c", "printf AETHERAI_PROCESS_PASS"], &d);
        let result = r.run(req, PermissionDecision::AllowThisProject).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "AETHERAI_PROCESS_PASS");
        let e = sink.events();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].status, ActivityStatus::Running);
        assert_eq!(e[1].status, ActivityStatus::Passed);
        assert_eq!(e[1].stdout, "AETHERAI_PROCESS_PASS");
        let _ = std::fs::remove_dir_all(d);
    }
}
