pub mod attachment;
pub mod memory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub type ConversationId = Uuid;
pub type MessageId = Uuid;
pub type WorkspaceId = Uuid;
pub type ProjectId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConversationKind {
    #[default]
    Chat,
    Coding,
    Research,
    Project,
    Document,
    Analysis,
    AgentTask,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingPreset {
    Fast,
    Balanced,
    Deep,
    Maximum,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationSettings {
    pub preset: ProcessingPreset,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub seed: Option<u64>,
    pub context_tokens: u32,
    pub cpu_threads: Option<u16>,
    pub gpu_layers: Option<i16>,
}
impl GenerationSettings {
    pub fn from_preset(preset: ProcessingPreset) -> Self {
        match preset {
            ProcessingPreset::Fast => Self {
                preset,
                max_output_tokens: 1024,
                temperature: 0.7,
                top_p: 0.90,
                top_k: 40,
                seed: None,
                context_tokens: 4096,
                cpu_threads: None,
                gpu_layers: None,
            },
            ProcessingPreset::Balanced => Self {
                preset,
                max_output_tokens: 2048,
                temperature: 0.7,
                top_p: 0.95,
                top_k: 40,
                seed: None,
                context_tokens: 8192,
                cpu_threads: None,
                gpu_layers: None,
            },
            ProcessingPreset::Deep => Self {
                preset,
                max_output_tokens: 4096,
                temperature: 0.6,
                top_p: 0.95,
                top_k: 50,
                seed: None,
                context_tokens: 16384,
                cpu_threads: None,
                gpu_layers: None,
            },
            ProcessingPreset::Maximum => Self {
                preset,
                max_output_tokens: 8192,
                temperature: 0.5,
                top_p: 0.98,
                top_k: 80,
                seed: None,
                context_tokens: 32768,
                cpu_threads: None,
                gpu_layers: Some(-1),
            },
            ProcessingPreset::Custom => Self {
                preset,
                ..Self::from_preset(ProcessingPreset::Balanced)
            },
        }
    }
    pub fn validate(&self) -> Result<(), AetherCoreError> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(AetherCoreError::InvalidTemperature(self.temperature));
        }
        if !(0.0..=1.0).contains(&self.top_p) {
            return Err(AetherCoreError::InvalidTopP(self.top_p));
        }
        if self.max_output_tokens == 0 {
            return Err(AetherCoreError::ZeroOutputTokens);
        }
        if self.context_tokens == 0 {
            return Err(AetherCoreError::ZeroContextTokens);
        }
        Ok(())
    }
}
impl Default for GenerationSettings {
    fn default() -> Self {
        Self::from_preset(ProcessingPreset::Balanced)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NetworkPolicy {
    Off,
    #[default]
    Ask,
    On,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionKind {
    FileRead,
    FileWrite,
    ProcessExecute,
    GitDestructive,
    Network,
    Microphone,
    Screen,
    Camera,
    ComputerControl,
    Clipboard,
    BackgroundTasks,
    SystemIntegration,
    ReadAttachment,
    IndexRoot,
    WatchRoot,
    StoreEmbeddings,
    PersistProjectMemory,
    RevealSource,
    ExtractArchive,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PermissionDecision {
    Blocked,
    #[default]
    AskEveryTime,
    AllowThisChat,
    AllowThisProject,
    AlwaysAllow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationSettings {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub generation: GenerationSettings,
    #[serde(default = "default_true")]
    pub memory_enabled: bool,
    #[serde(default)]
    pub network_policy: NetworkPolicy,
    #[serde(default)]
    pub code_execution_enabled: bool,
    #[serde(default = "default_one")]
    pub agent_depth: u8,
    #[serde(default = "default_one")]
    pub concurrent_subagents: u8,
}
fn default_true() -> bool {
    true
}
fn default_one() -> u8 {
    1
}
impl Default for ConversationSettings {
    fn default() -> Self {
        Self {
            model: None,
            generation: GenerationSettings::default(),
            memory_enabled: true,
            network_policy: NetworkPolicy::Ask,
            code_execution_enabled: false,
            agent_depth: 1,
            concurrent_subagents: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRoot {
    pub path: String,
    #[serde(default)]
    pub writable: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    #[serde(default)]
    pub roots: Vec<WorkspaceRoot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
}
impl WorkspaceState {
    pub fn normalize_legacy(&mut self) {
        if let Some(root) = self.root.take() {
            if !self.roots.iter().any(|r| r.path == root) {
                self.roots.push(WorkspaceRoot {
                    path: root,
                    writable: false,
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub title: String,
    #[serde(default)]
    pub kind: ConversationKind,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub settings: ConversationSettings,
    #[serde(default)]
    pub workspace: Option<WorkspaceState>,
    #[serde(default)]
    pub messages: Vec<Message>,
}
impl Conversation {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            kind: ConversationKind::Chat,
            pinned: false,
            archived: false,
            created_at: now,
            updated_at: now,
            settings: ConversationSettings::default(),
            workspace: None,
            messages: vec![],
        }
    }
    pub fn normalize_legacy(&mut self) {
        if self.settings.model.as_deref() == Some("aetherai/mock") {
            self.settings.model = None;
        }
        if let Some(w) = &mut self.workspace {
            w.normalize_legacy();
        }
    }
    pub fn ensure_workspace(&mut self, name: impl Into<String>) -> WorkspaceId {
        let name = name.into();
        let id = match &mut self.workspace {
            Some(w) => {
                w.name = name;
                w.id
            }
            None => {
                let id = Uuid::new_v4();
                self.workspace = Some(WorkspaceState {
                    id,
                    name,
                    project_id: None,
                    roots: vec![],
                    root: None,
                });
                id
            }
        };
        self.updated_at = Utc::now();
        id
    }
    pub fn attach_workspace_root(
        &mut self,
        path: impl Into<String>,
        writable: bool,
    ) -> Result<(), AetherCoreError> {
        let path = path.into();
        if path.trim().is_empty() {
            return Err(AetherCoreError::EmptyWorkspaceRoot);
        }
        if self.workspace.is_none() {
            self.ensure_workspace("Workspace");
        }
        let w = self.workspace.as_mut().expect("created");
        if !w.roots.iter().any(|r| r.path == path) {
            w.roots.push(WorkspaceRoot { path, writable });
        }
        self.updated_at = Utc::now();
        Ok(())
    }
    pub fn push_message(&mut self, role: Role, content: impl Into<String>) -> MessageId {
        let m = Message::new(role, content);
        let id = m.id;
        self.messages.push(m);
        self.updated_at = Utc::now();
        id
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum AetherCoreError {
    #[error("temperature must be between 0.0 and 2.0, got {0}")]
    InvalidTemperature(f32),
    #[error("top_p must be between 0.0 and 1.0, got {0}")]
    InvalidTopP(f32),
    #[error("max_output_tokens must be greater than zero")]
    ZeroOutputTokens,
    #[error("context_tokens must be greater than zero")]
    ZeroContextTokens,
    #[error("workspace root cannot be empty")]
    EmptyWorkspaceRoot,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_has_no_fake_model_and_asks_for_network() {
        let c = Conversation::new("x");
        assert_eq!(c.settings.model, None);
        assert_eq!(c.settings.network_policy, NetworkPolicy::Ask);
    }
    #[test]
    fn ordinary_chat_gains_workspace_without_identity_change() {
        let mut c = Conversation::new("x");
        let id = c.id;
        c.attach_workspace_root("/tmp/demo", true).unwrap();
        assert_eq!(c.id, id);
        assert_eq!(c.workspace.unwrap().roots.len(), 1);
    }
    #[test]
    fn custom_preset_is_editable() {
        let mut g = GenerationSettings::from_preset(ProcessingPreset::Maximum);
        g.preset = ProcessingPreset::Custom;
        g.temperature = 0.2;
        assert!(g.validate().is_ok());
    }
    #[test]
    fn legacy_mock_is_sanitized() {
        let mut c = Conversation::new("x");
        c.settings.model = Some("aetherai/mock".into());
        c.normalize_legacy();
        assert_eq!(c.settings.model, None);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetrievalBudget {
    pub max_context_tokens: u32,
    pub max_sources: u16,
    pub max_memory_items: u16,
    pub allow_semantic: bool,
}

impl RetrievalBudget {
    pub fn from_preset(preset: ProcessingPreset) -> Self {
        match preset {
            ProcessingPreset::Fast => Self {
                max_context_tokens: 1024,
                max_sources: 4,
                max_memory_items: 2,
                allow_semantic: false,
            },
            ProcessingPreset::Balanced => Self {
                max_context_tokens: 3072,
                max_sources: 8,
                max_memory_items: 4,
                allow_semantic: true,
            },
            ProcessingPreset::Deep => Self {
                max_context_tokens: 6144,
                max_sources: 16,
                max_memory_items: 8,
                allow_semantic: true,
            },
            ProcessingPreset::Maximum => Self {
                max_context_tokens: 12288,
                max_sources: 32,
                max_memory_items: 16,
                allow_semantic: true,
            },
            ProcessingPreset::Custom => Self {
                max_context_tokens: 3072,
                max_sources: 8,
                max_memory_items: 4,
                allow_semantic: true,
            },
        }
    }
}

pub use attachment::*;
pub use memory::*;
