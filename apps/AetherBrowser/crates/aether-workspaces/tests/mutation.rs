use aether_session::TabId;
use aether_workspaces::{Workspace, WorkspaceId, WorkspaceSet};

#[test]
fn moving_a_tab_between_workspaces_preserves_unique_membership() {
    let first = Workspace::new(WorkspaceId::new(1), "Main", vec![TabId::new(10)]).unwrap();
    let second = Workspace::new(WorkspaceId::new(2), "Game", vec![TabId::new(20)]).unwrap();
    let mut set = WorkspaceSet::new(vec![first, second], WorkspaceId::new(1)).unwrap();
    set.move_tab(TabId::new(10), WorkspaceId::new(1), WorkspaceId::new(2))
        .unwrap();
    assert!(
        set.workspace(WorkspaceId::new(1))
            .unwrap()
            .tabs()
            .is_empty()
    );
    assert_eq!(
        set.workspace(WorkspaceId::new(2)).unwrap().tabs(),
        &[TabId::new(20), TabId::new(10)]
    );
}

#[test]
fn workspaces_can_be_added_and_renamed() {
    let first = Workspace::new(WorkspaceId::new(1), "Main", vec![]).unwrap();
    let mut set = WorkspaceSet::new(vec![first], WorkspaceId::new(1)).unwrap();
    set.add_workspace(Workspace::new(WorkspaceId::new(2), "Streaming", vec![]).unwrap())
        .unwrap();
    set.rename_workspace(WorkspaceId::new(2), "Live").unwrap();
    assert_eq!(set.workspace(WorkspaceId::new(2)).unwrap().name(), "Live");
}
