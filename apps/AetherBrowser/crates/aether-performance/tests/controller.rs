use aether_performance::{
    ActivityState, BudgetError, GlobalResourcePolicy, ProtectionFlags, ResourceBudget,
    ResourceDecision, TabPriority, TabResourceController, TabResourceState,
};

fn state(activity: ActivityState) -> TabResourceState {
    TabResourceState {
        activity,
        priority: TabPriority::Background,
        protection: ProtectionFlags::default(),
        budget: ResourceBudget::default(),
    }
}

#[test]
fn controller_reconciles_background_tabs_using_active_policy() {
    let mut controller = TabResourceController::new(GlobalResourcePolicy::aggressive_gaming());
    controller
        .upsert(1, state(ActivityState::Foreground))
        .unwrap();
    controller
        .upsert(2, state(ActivityState::Background))
        .unwrap();

    let decisions = controller.reconcile();
    assert_eq!(
        decisions
            .iter()
            .find(|entry| entry.tab_id == 1)
            .unwrap()
            .decision,
        ResourceDecision::Run
    );
    assert_eq!(
        decisions
            .iter()
            .find(|entry| entry.tab_id == 2)
            .unwrap()
            .decision,
        ResourceDecision::Freeze
    );
}

#[test]
fn controller_rejects_impossible_percent_budgets() {
    let mut bad = state(ActivityState::Background);
    bad.budget.cpu_percent = Some(101);
    let mut controller = TabResourceController::new(GlobalResourcePolicy::default());
    assert_eq!(controller.upsert(9, bad), Err(BudgetError::CpuPercent(101)));
}
