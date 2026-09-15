#![forbid(unsafe_code)]
//! Tab-island grouping with unique tab membership.

use aether_session::TabId;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TabIslandId(u64);

impl TabIslandId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TabIslandError {
    EmptyName,
    DuplicateIsland(TabIslandId),
    DuplicateTab(TabId),
    UnknownIsland(TabIslandId),
    TabAlreadyGrouped(TabId),
    TabNotFound(TabId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabIsland {
    id: TabIslandId,
    name: String,
    tabs: Vec<TabId>,
}

impl TabIsland {
    pub fn new(
        id: TabIslandId,
        name: impl Into<String>,
        tabs: Vec<TabId>,
    ) -> Result<Self, TabIslandError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(TabIslandError::EmptyName);
        }
        let mut seen = HashSet::new();
        for tab in &tabs {
            if !seen.insert(*tab) {
                return Err(TabIslandError::DuplicateTab(*tab));
            }
        }
        Ok(Self { id, name, tabs })
    }

    #[must_use]
    pub const fn id(&self) -> TabIslandId {
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
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TabIslandSet {
    islands: Vec<TabIsland>,
}

impl TabIslandSet {
    pub fn new(islands: Vec<TabIsland>) -> Result<Self, TabIslandError> {
        let mut ids = HashSet::new();
        let mut tabs = HashSet::new();
        for island in &islands {
            if !ids.insert(island.id()) {
                return Err(TabIslandError::DuplicateIsland(island.id()));
            }
            for &tab in island.tabs() {
                if !tabs.insert(tab) {
                    return Err(TabIslandError::TabAlreadyGrouped(tab));
                }
            }
        }
        Ok(Self { islands })
    }

    pub fn add_island(&mut self, island: TabIsland) -> Result<(), TabIslandError> {
        if self.islands.iter().any(|entry| entry.id() == island.id()) {
            return Err(TabIslandError::DuplicateIsland(island.id()));
        }
        for &tab in island.tabs() {
            if self.contains_tab(tab) {
                return Err(TabIslandError::TabAlreadyGrouped(tab));
            }
        }
        self.islands.push(island);
        Ok(())
    }

    pub fn add_tab(&mut self, island_id: TabIslandId, tab: TabId) -> Result<(), TabIslandError> {
        if self.contains_tab(tab) {
            return Err(TabIslandError::TabAlreadyGrouped(tab));
        }
        let Some(island) = self
            .islands
            .iter_mut()
            .find(|island| island.id() == island_id)
        else {
            return Err(TabIslandError::UnknownIsland(island_id));
        };
        island.tabs.push(tab);
        Ok(())
    }

    pub fn move_tab(
        &mut self,
        tab: TabId,
        from: TabIslandId,
        to: TabIslandId,
    ) -> Result<(), TabIslandError> {
        let Some(from_index) = self.islands.iter().position(|island| island.id() == from) else {
            return Err(TabIslandError::UnknownIsland(from));
        };
        let Some(to_index) = self.islands.iter().position(|island| island.id() == to) else {
            return Err(TabIslandError::UnknownIsland(to));
        };
        let Some(tab_index) = self.islands[from_index]
            .tabs
            .iter()
            .position(|candidate| *candidate == tab)
        else {
            return Err(TabIslandError::TabNotFound(tab));
        };
        if from_index == to_index {
            return Ok(());
        }
        self.islands[from_index].tabs.remove(tab_index);
        self.islands[to_index].tabs.push(tab);
        Ok(())
    }

    fn contains_tab(&self, tab: TabId) -> bool {
        self.islands.iter().any(|island| island.tabs.contains(&tab))
    }

    #[must_use]
    pub fn island(&self, id: TabIslandId) -> Option<&TabIsland> {
        self.islands.iter().find(|island| island.id() == id)
    }

    #[must_use]
    pub fn islands(&self) -> &[TabIsland] {
        &self.islands
    }
}
