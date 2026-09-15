use aether_session::TabId;
use aether_splitview::{SplitError, SplitLayout, SplitView};

#[test]
fn split_layouts_require_exact_pane_counts() {
    assert!(SplitView::new(SplitLayout::Two, vec![TabId::new(1), TabId::new(2)]).is_ok());
    assert!(
        SplitView::new(
            SplitLayout::Three,
            vec![TabId::new(1), TabId::new(2), TabId::new(3)]
        )
        .is_ok()
    );
    assert!(
        SplitView::new(
            SplitLayout::Four,
            vec![TabId::new(1), TabId::new(2), TabId::new(3), TabId::new(4)]
        )
        .is_ok()
    );
    assert_eq!(
        SplitView::new(SplitLayout::Four, vec![TabId::new(1), TabId::new(2)]),
        Err(SplitError::WrongPaneCount {
            expected: 4,
            actual: 2
        })
    );
}

#[test]
fn a_tab_cannot_occupy_multiple_panes() {
    assert_eq!(
        SplitView::new(SplitLayout::Two, vec![TabId::new(1), TabId::new(1)]),
        Err(SplitError::DuplicateTab(TabId::new(1)))
    );
}
