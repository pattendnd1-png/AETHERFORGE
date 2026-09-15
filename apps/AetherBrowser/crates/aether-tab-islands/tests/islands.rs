use aether_session::TabId;
use aether_tab_islands::{TabIsland, TabIslandError, TabIslandId, TabIslandSet};

#[test]
fn tab_islands_keep_each_tab_in_at_most_one_island() {
    let first = TabIsland::new(
        TabIslandId::new(1),
        "Docs",
        vec![TabId::new(1), TabId::new(2)],
    )
    .unwrap();
    let second = TabIsland::new(TabIslandId::new(2), "Video", vec![TabId::new(3)]).unwrap();
    let mut set = TabIslandSet::new(vec![first, second]).unwrap();
    assert_eq!(
        set.add_tab(TabIslandId::new(2), TabId::new(2)),
        Err(TabIslandError::TabAlreadyGrouped(TabId::new(2)))
    );
    set.move_tab(TabId::new(2), TabIslandId::new(1), TabIslandId::new(2))
        .unwrap();
    assert_eq!(
        set.island(TabIslandId::new(2)).unwrap().tabs(),
        &[TabId::new(3), TabId::new(2)]
    );
}
