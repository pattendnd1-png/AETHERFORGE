#![forbid(unsafe_code)]
//! Named workspace state with stable tab membership.

use aether_session::TabId;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceId(u64);
impl WorkspaceId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceError {
    EmptyName,
    DuplicateTab(TabId),
    DuplicateWorkspace(WorkspaceId),
    UnknownWorkspace(WorkspaceId),
    TabNotFound(TabId),
    TabAlreadyAssigned(TabId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    tabs: Vec<TabId>,
}

impl Workspace {
    pub fn new(
        id: WorkspaceId,
        name: impl Into<String>,
        tabs: Vec<TabId>,
    ) -> Result<Self, WorkspaceError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        let mut seen = HashSet::new();
        for tab in &tabs {
            if !seen.insert(*tab) {
                return Err(WorkspaceError::DuplicateTab(*tab));
            }
        }
        Ok(Self { id, name, tabs })
    }

    #[must_use]
    pub const fn id(&self) -> WorkspaceId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn tabs(&self) -> &[TabId] {
        &self.tabs
    }

    fn rename(&mut self, name: String) -> Result<(), WorkspaceError> {
        if name.trim().is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        self.name = name;
        Ok(())
    }

    fn contains(&self, tab: TabId) -> bool {
        self.tabs.contains(&tab)
    }

    fn add_tab(&mut self, tab: TabId) -> Result<(), WorkspaceError> {
        if self.contains(tab) {
            return Err(WorkspaceError::TabAlreadyAssigned(tab));
        }
        self.tabs.push(tab);
        Ok(())
    }

    fn remove_tab(&mut self, tab: TabId) -> Result<(), WorkspaceError> {
        let Some(index) = self.tabs.iter().position(|candidate| *candidate == tab) else {
            return Err(WorkspaceError::TabNotFound(tab));
        };
        self.tabs.remove(index);
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSet {
    workspaces: Vec<Workspace>,
    active: WorkspaceId,
}

impl WorkspaceSet {
    pub fn new(workspaces: Vec<Workspace>, active: WorkspaceId) -> Result<Self, WorkspaceError> {
        let mut workspace_ids = HashSet::new();
        let mut tabs = HashSet::new();
        for workspace in &workspaces {
            if !workspace_ids.insert(workspace.id()) {
                return Err(WorkspaceError::DuplicateWorkspace(workspace.id()));
            }
            for &tab in workspace.tabs() {
                if !tabs.insert(tab) {
                    return Err(WorkspaceError::TabAlreadyAssigned(tab));
                }
            }
        }
        if !workspaces.iter().any(|workspace| workspace.id() == active) {
            return Err(WorkspaceError::UnknownWorkspace(active));
        }
        Ok(Self { workspaces, active })
    }

    pub fn switch(&mut self, id: WorkspaceId) -> Result<(), WorkspaceError> {
        if self.workspaces.iter().any(|workspace| workspace.id() == id) {
            self.active = id;
            Ok(())
        } else {
            Err(WorkspaceError::UnknownWorkspace(id))
        }
    }

    pub fn add_workspace(&mut self, workspace: Workspace) -> Result<(), WorkspaceError> {
        if self
            .workspaces
            .iter()
            .any(|entry| entry.id() == workspace.id())
        {
            return Err(WorkspaceError::DuplicateWorkspace(workspace.id()));
        }
        for &tab in workspace.tabs() {
            if self.workspaces.iter().any(|entry| entry.contains(tab)) {
                return Err(WorkspaceError::TabAlreadyAssigned(tab));
            }
        }
        self.workspaces.push(workspace);
        Ok(())
    }

    pub fn rename_workspace(
        &mut self,
        id: WorkspaceId,
        name: impl Into<String>,
    ) -> Result<(), WorkspaceError> {
        let Some(workspace) = self
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.id() == id)
        else {
            return Err(WorkspaceError::UnknownWorkspace(id));
        };
        workspace.rename(name.into())
    }

    pub fn move_tab(
        &mut self,
        tab: TabId,
        from: WorkspaceId,
        to: WorkspaceId,
    ) -> Result<(), WorkspaceError> {
        let Some(from_index) = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id() == from)
        else {
            return Err(WorkspaceError::UnknownWorkspace(from));
        };
        let Some(to_index) = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id() == to)
        else {
            return Err(WorkspaceError::UnknownWorkspace(to));
        };
        if !self.workspaces[from_index].contains(tab) {
            return Err(WorkspaceError::TabNotFound(tab));
        }
        if from_index == to_index {
            return Ok(());
        }
        if self.workspaces[to_index].contains(tab) {
            return Err(WorkspaceError::TabAlreadyAssigned(tab));
        }
        self.workspaces[from_index].remove_tab(tab)?;
        self.workspaces[to_index].add_tab(tab)
    }

    #[must_use]
    pub const fn active_id(&self) -> WorkspaceId {
        self.active
    }

    #[must_use]
    pub fn active(&self) -> Option<&Workspace> {
        self.workspace(self.active)
    }

    #[must_use]
    pub fn workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id() == id)
    }

    #[must_use]
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }
}
