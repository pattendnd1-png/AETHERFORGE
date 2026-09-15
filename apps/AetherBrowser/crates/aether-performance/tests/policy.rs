use aether_performance::{
    ActivityState, GlobalResourcePolicy, ProtectionFlags, ResourceBudget, ResourceDecision,
    TabPriority, TabResourceState, evaluate_tab_policy,
};

fn background(flags: ProtectionFlags) -> TabResourceState {
    TabResourceState {
        activity: ActivityState::Background,
        priority: TabPriority::Background,
        protection: flags,
        budget: ResourceBudget::default(),
    }
}

#[test]
fn protected_background_activity_is_never_silently_frozen() {
    let aggressive = GlobalResourcePolicy::aggressive_gaming();
    for flags in [
        ProtectionFlags {
            audible: true,
            ..ProtectionFlags::default()
        },
        ProtectionFlags {
            webrtc: true,
            ..ProtectionFlags::default()
        },
        ProtectionFlags {
            active_download: true,
            ..ProtectionFlags::default()
        },
        ProtectionFlags {
            user_protected: true,
            ..ProtectionFlags::default()
        },
    ] {
        let record = evaluate_tab_policy(&background(flags), &aggressive);
        assert_ne!(record.decision, ResourceDecision::Freeze);
        assert_ne!(record.decision, ResourceDecision::Sleep);
    }
}

#[test]
fn aggressive_gaming_may_freeze_unprotected_idle_background_tabs() {
    let record = evaluate_tab_policy(
        &background(ProtectionFlags::default()),
        &GlobalResourcePolicy::aggressive_gaming(),
    );
    assert_eq!(record.decision, ResourceDecision::Freeze);
}

#[test]
fn foreground_tabs_remain_runnable() {
    let state = TabResourceState {
        activity: ActivityState::Foreground,
        priority: TabPriority::Normal,
        protection: ProtectionFlags::default(),
        budget: ResourceBudget::default(),
    };
    assert_eq!(
        evaluate_tab_policy(&state, &GlobalResourcePolicy::aggressive_gaming()).decision,
        ResourceDecision::Run
    );
}

#[test]
fn resource_budgets_round_trip_without_hidden_changes() {
    let budget = ResourceBudget {
        cpu_percent: Some(35),
        memory_mib: Some(3072),
        network_kib_per_sec: Some(4096),
        gpu_percent: Some(25),
    };
    let state = TabResourceState {
        activity: ActivityState::VisibleBackground,
        priority: TabPriority::Low,
        protection: ProtectionFlags::default(),
        budget,
    };
    assert_eq!(state.budget, budget);
}
