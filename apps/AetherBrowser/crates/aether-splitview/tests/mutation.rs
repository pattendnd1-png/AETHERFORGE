use aether_session::TabId;
use aether_splitview::{SplitLayout, SplitView};

#[test]
fn split_view_can_focus_and_replace_a_pane() {
    let mut split = SplitView::new(
        SplitLayout::Three,
        vec![TabId::new(1), TabId::new(2), TabId::new(3)],
    )
    .unwrap();
    split.focus_pane(1).unwrap();
    split.replace_pane(1, TabId::new(4)).unwrap();
    assert_eq!(split.focused_pane(), 1);
    assert_eq!(split.tabs(), &[TabId::new(1), TabId::new(4), TabId::new(3)]);
}
