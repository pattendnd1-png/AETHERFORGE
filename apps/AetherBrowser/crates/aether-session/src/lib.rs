#![forbid(unsafe_code)]
//! Engine-neutral browser session state and crash-recovery snapshot rules.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TabId(u64);

impl TabId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(u64);

impl WindowId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabState {
    id: TabId,
    url: String,
    title: String,
    pinned: bool,
    protected: bool,
}

impl TabState {
    #[must_use]
    pub fn new(id: TabId, url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id,
            url: url.into(),
            title: title.into(),
            pinned: false,
            protected: false,
        }
    }
    #[must_use]
    pub const fn id(&self) -> TabId {
        self.id
    }
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }
    #[must_use]
    pub const fn protected(&self) -> bool {
        self.protected
    }
    pub fn set_pinned(&mut self, value: bool) {
        self.pinned = value;
    }
    pub fn set_protected(&mut self, value: bool) {
        self.protected = value;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    DuplicateTabId(TabId),
    ActiveTabMissing(TabId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowSession {
    id: WindowId,
    private: bool,
    tabs: Vec<TabState>,
    active_tab: Option<TabId>,
}

impl WindowSession {
    pub fn new(id: WindowId, private: bool, tabs: Vec<TabState>) -> Result<Self, SessionError> {
        let mut seen = HashSet::new();
        for tab in &tabs {
            if !seen.insert(tab.id()) {
                return Err(SessionError::DuplicateTabId(tab.id()));
            }
        }
        let active_tab = tabs.first().map(TabState::id);
        Ok(Self {
            id,
            private,
            tabs,
            active_tab,
        })
    }

    #[must_use]
    pub const fn id(&self) -> WindowId {
        self.id
    }
    #[must_use]
    pub const fn is_private(&self) -> bool {
        self.private
    }
    #[must_use]
    pub fn tabs(&self) -> &[TabState] {
        &self.tabs
    }
    #[must_use]
    pub const fn active_tab(&self) -> Option<TabId> {
        self.active_tab
    }

    pub fn set_active_tab(&mut self, id: TabId) -> Result<(), SessionError> {
        if self.tabs.iter().any(|tab| tab.id() == id) {
            self.active_tab = Some(id);
            Ok(())
        } else {
            Err(SessionError::ActiveTabMissing(id))
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionSnapshot {
    windows: Vec<WindowSession>,
}

impl SessionSnapshot {
    #[must_use]
    pub fn restorable_from(windows: Vec<WindowSession>) -> Self {
        Self {
            windows: windows
                .into_iter()
                .filter(|window| !window.is_private())
                .collect(),
        }
    }

    #[must_use]
    pub fn windows(&self) -> &[WindowSession] {
        &self.windows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentlyClosedTab {
    pub url: String,
    pub title: String,
    pub closed_unix_seconds: u64,
}
