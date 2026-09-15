#![forbid(unsafe_code)]
//! Browser-native CPU, memory, network, GPU, and tab lifecycle policy contracts.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivityState {
    Foreground,
    VisibleBackground,
    Background,
    Sleeping,
    Frozen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProtectionFlags {
    pub audible: bool,
    pub webrtc: bool,
    pub active_download: bool,
    pub user_protected: bool,
}

impl ProtectionFlags {
    #[must_use]
    pub const fn any(self) -> bool {
        self.audible || self.webrtc || self.active_download || self.user_protected
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceBudget {
    pub cpu_percent: Option<u8>,
    pub memory_mib: Option<u32>,
    pub network_kib_per_sec: Option<u32>,
    pub gpu_percent: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetError {
    CpuPercent(u8),
    GpuPercent(u8),
    ZeroMemory,
    ZeroNetwork,
}

impl ResourceBudget {
    pub fn validate(self) -> Result<(), BudgetError> {
        if let Some(value) = self.cpu_percent
            && value > 100
        {
            return Err(BudgetError::CpuPercent(value));
        }
        if let Some(value) = self.gpu_percent
            && value > 100
        {
            return Err(BudgetError::GpuPercent(value));
        }
        if matches!(self.memory_mib, Some(0)) {
            return Err(BudgetError::ZeroMemory);
        }
        if matches!(self.network_kib_per_sec, Some(0)) {
            return Err(BudgetError::ZeroNetwork);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabResourceState {
    pub activity: ActivityState,
    pub priority: TabPriority,
    pub protection: ProtectionFlags,
    pub budget: ResourceBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlobalResourcePolicy {
    pub aggressive_gaming: bool,
    pub background_sleep: bool,
    pub background_freeze: bool,
}

impl Default for GlobalResourcePolicy {
    fn default() -> Self {
        Self {
            aggressive_gaming: false,
            background_sleep: true,
            background_freeze: false,
        }
    }
}

impl GlobalResourcePolicy {
    #[must_use]
    pub const fn aggressive_gaming() -> Self {
        Self {
            aggressive_gaming: true,
            background_sleep: true,
            background_freeze: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceDecision {
    Run,
    Throttle,
    Sleep,
    Freeze,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceDecisionRecord {
    pub decision: ResourceDecision,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabDecision {
    pub tab_id: u64,
    pub decision: ResourceDecision,
    pub reason: &'static str,
}

#[derive(Clone, Debug)]
pub struct TabResourceController {
    policy: GlobalResourcePolicy,
    tabs: BTreeMap<u64, TabResourceState>,
}

impl TabResourceController {
    #[must_use]
    pub fn new(policy: GlobalResourcePolicy) -> Self {
        Self {
            policy,
            tabs: BTreeMap::new(),
        }
    }

    pub fn set_policy(&mut self, policy: GlobalResourcePolicy) {
        self.policy = policy;
    }

    pub fn upsert(
        &mut self,
        tab_id: u64,
        state: TabResourceState,
    ) -> Result<Option<TabResourceState>, BudgetError> {
        state.budget.validate()?;
        Ok(self.tabs.insert(tab_id, state))
    }

    pub fn remove(&mut self, tab_id: u64) -> Option<TabResourceState> {
        self.tabs.remove(&tab_id)
    }

    #[must_use]
    pub fn decision(&self, tab_id: u64) -> Option<ResourceDecisionRecord> {
        self.tabs
            .get(&tab_id)
            .map(|state| evaluate_tab_policy(state, &self.policy))
    }

    #[must_use]
    pub fn reconcile(&self) -> Vec<TabDecision> {
        self.tabs
            .iter()
            .map(|(&tab_id, state)| {
                let record = evaluate_tab_policy(state, &self.policy);
                TabDecision {
                    tab_id,
                    decision: record.decision,
                    reason: record.reason,
                }
            })
            .collect()
    }
}

#[must_use]
pub const fn evaluate_tab_policy(
    state: &TabResourceState,
    policy: &GlobalResourcePolicy,
) -> ResourceDecisionRecord {
    if matches!(state.activity, ActivityState::Foreground) {
        return ResourceDecisionRecord {
            decision: ResourceDecision::Run,
            reason: "foreground",
        };
    }

    if state.protection.any() || matches!(state.priority, TabPriority::Critical | TabPriority::High)
    {
        return ResourceDecisionRecord {
            decision: ResourceDecision::Throttle,
            reason: "protected background activity",
        };
    }

    if matches!(state.activity, ActivityState::VisibleBackground) {
        return ResourceDecisionRecord {
            decision: ResourceDecision::Throttle,
            reason: "visible background",
        };
    }

    if policy.aggressive_gaming
        && policy.background_freeze
        && matches!(state.activity, ActivityState::Background)
    {
        return ResourceDecisionRecord {
            decision: ResourceDecision::Freeze,
            reason: "aggressive gaming",
        };
    }

    if policy.background_sleep && matches!(state.activity, ActivityState::Background) {
        return ResourceDecisionRecord {
            decision: ResourceDecision::Sleep,
            reason: "idle background",
        };
    }

    match state.activity {
        ActivityState::Sleeping => ResourceDecisionRecord {
            decision: ResourceDecision::Sleep,
            reason: "already sleeping",
        },
        ActivityState::Frozen => ResourceDecisionRecord {
            decision: ResourceDecision::Freeze,
            reason: "already frozen",
        },
        _ => ResourceDecisionRecord {
            decision: ResourceDecision::Run,
            reason: "no limiting action",
        },
    }
}
