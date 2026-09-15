#![forbid(unsafe_code)]
//! Two-, three-, and four-pane browsing layout contracts.

use aether_session::TabId;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitLayout {
    Two,
    Three,
    Four,
}
impl SplitLayout {
    #[must_use]
    pub const fn pane_count(self) -> usize {
        match self {
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SplitError {
    WrongPaneCount { expected: usize, actual: usize },
    DuplicateTab(TabId),
    UnknownPane(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitView {
    layout: SplitLayout,
    tabs: Vec<TabId>,
    focused_pane: usize,
}

impl SplitView {
    pub fn new(layout: SplitLayout, tabs: Vec<TabId>) -> Result<Self, SplitError> {
        let expected = layout.pane_count();
        if tabs.len() != expected {
            return Err(SplitError::WrongPaneCount {
                expected,
                actual: tabs.len(),
            });
        }
        let mut seen = HashSet::new();
        for tab in &tabs {
            if !seen.insert(*tab) {
                return Err(SplitError::DuplicateTab(*tab));
            }
        }
        Ok(Self {
            layout,
            tabs,
            focused_pane: 0,
        })
    }

    pub fn focus_pane(&mut self, pane: usize) -> Result<(), SplitError> {
        if pane >= self.tabs.len() {
            return Err(SplitError::UnknownPane(pane));
        }
        self.focused_pane = pane;
        Ok(())
    }

    pub fn replace_pane(&mut self, pane: usize, tab: TabId) -> Result<TabId, SplitError> {
        if pane >= self.tabs.len() {
            return Err(SplitError::UnknownPane(pane));
        }
        if self
            .tabs
            .iter()
            .enumerate()
            .any(|(index, candidate)| index != pane && *candidate == tab)
        {
            return Err(SplitError::DuplicateTab(tab));
        }
        let previous = std::mem::replace(&mut self.tabs[pane], tab);
        Ok(previous)
    }

    #[must_use]
    pub const fn layout(&self) -> SplitLayout {
        self.layout
    }

    #[must_use]
    pub fn tabs(&self) -> &[TabId] {
        &self.tabs
    }

    #[must_use]
    pub const fn focused_pane(&self) -> usize {
        self.focused_pane
    }
}
