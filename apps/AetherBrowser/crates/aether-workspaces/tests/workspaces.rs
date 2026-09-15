use aether_session::TabId;
use aether_workspaces::{Workspace, WorkspaceId, WorkspaceSet};

#[test]
fn switching_workspace_preserves_tab_membership() {
    let first = Workspace::new(WorkspaceId::new(1), "Main", vec![TabId::new(10)]).unwrap();
    let second = Workspace::new(WorkspaceId::new(2), "Game", vec![TabId::new(20)]).unwrap();
    let mut set = WorkspaceSet::new(vec![first, second], WorkspaceId::new(1)).unwrap();

    set.switch(WorkspaceId::new(2)).unwrap();

    assert_eq!(set.active_id(), WorkspaceId::new(2));
    assert_eq!(set.active().unwrap().tabs(), &[TabId::new(20)]);
}
