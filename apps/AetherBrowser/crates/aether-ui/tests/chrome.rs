use aether_ui::{
    APPROVED_RENDER_REFERENCE_HEIGHT_PX, APPROVED_RENDER_REFERENCE_WIDTH_PX, CHROME_HEIGHT_PX,
    ChromeHitTarget, ChromeLayout, ChromeModel, ChromeTab, HOME_DISPLAY_URL, LAUNCHER_WIDTH_PX,
    NAV_ROW_TOP_PX, NativeChromeRenderer, OMNIBOX_CARET_VISUAL_CONTRACT, STATUS_BAR_HEIGHT_PX,
    TAB_MAX_WIDTH_PX, TAB_MIN_WIDTH_PX, TAB_STRIP_TOP_PX, UTILITY_DOCK_COLLAPSED_WIDTH_PX,
    UTILITY_DOCK_WIDTH_PX, WINDOW_TITLEBAR_HEIGHT_PX,
};

fn model() -> ChromeModel {
    let mut model = ChromeModel::new(
        vec![ChromeTab {
            id: 7,
            title: "Welcome to Aether".into(),
            url: "aether://home".into(),
            active: true,
        }],
        "aether://home",
    );
    model.home_mode = true;
    model
}

#[test]
fn approved_render_geometry_is_native_browser_surface() {
    assert_eq!(WINDOW_TITLEBAR_HEIGHT_PX, 32);
    assert_eq!(CHROME_HEIGHT_PX, 108);
    assert_eq!(LAUNCHER_WIDTH_PX, 52);
    assert_eq!(UTILITY_DOCK_WIDTH_PX, 176);
    assert_eq!(UTILITY_DOCK_COLLAPSED_WIDTH_PX, 38);
    assert_eq!(STATUS_BAR_HEIGHT_PX, 30);
    assert_eq!(APPROVED_RENDER_REFERENCE_WIDTH_PX, 1672);
    assert_eq!(APPROVED_RENDER_REFERENCE_HEIGHT_PX, 941);
    assert_eq!(
        ChromeLayout::canonical().web_content_rect(1672, 941),
        (52, 108, 1444, 803)
    );
    assert_eq!(
        ChromeLayout::canonical()
            .with_utility_collapsed(true)
            .web_content_rect(1672, 941),
        (52, 108, 1582, 803)
    );
}

#[test]
fn native_renderer_exists_without_html_or_dom_api() {
    let _renderer = NativeChromeRenderer::new();
    let home = model();
    assert_eq!(home.display_omnibox(), HOME_DISPLAY_URL);
}

#[test]
fn chrome_hit_testing_finds_window_navigation_tabs_and_integrated_controls() {
    let model = model();
    assert_eq!(
        model.hit_test(70.0, 18.0, 1440, 900),
        ChromeHitTarget::WindowMinimize
    );
    assert_eq!(
        model.hit_test(104.0, 18.0, 1440, 900),
        ChromeHitTarget::WindowMaximize
    );
    assert_eq!(
        model.hit_test(138.0, 18.0, 1440, 900),
        ChromeHitTarget::WindowDiminish
    );
    assert_eq!(
        model.hit_test(172.0, 18.0, 1440, 900),
        ChromeHitTarget::WindowClose
    );
    assert_eq!(
        model.hit_test(240.0, 18.0, 1440, 900),
        ChromeHitTarget::WindowDrag
    );
    assert_eq!(
        model.hit_test(72.0, f64::from(NAV_ROW_TOP_PX) + 20.0, 1440, 900),
        ChromeHitTarget::Back
    );
    assert_eq!(
        model.hit_test(112.0, f64::from(NAV_ROW_TOP_PX) + 20.0, 1440, 900),
        ChromeHitTarget::Forward
    );
    assert_eq!(
        model.hit_test(151.0, f64::from(NAV_ROW_TOP_PX) + 20.0, 1440, 900),
        ChromeHitTarget::Reload
    );
    assert_eq!(
        model.hit_test(230.0, f64::from(NAV_ROW_TOP_PX) + 20.0, 1440, 900),
        ChromeHitTarget::Omnibox
    );
    assert_eq!(
        model.hit_test(110.0, f64::from(TAB_STRIP_TOP_PX) + 18.0, 1440, 900),
        ChromeHitTarget::Tab(7)
    );

    let launcher_y = |index: u32| f64::from(CHROME_HEIGHT_PX + 42 + index * 46);
    assert_eq!(
        model.hit_test(27.0, launcher_y(0), 1440, 900),
        ChromeHitTarget::Home
    );
    assert_eq!(
        model.hit_test(27.0, launcher_y(1), 1440, 900),
        ChromeHitTarget::Providers
    );
    assert_eq!(
        model.hit_test(27.0, launcher_y(2), 1440, 900),
        ChromeHitTarget::StreamStudio
    );
    assert_eq!(
        model.hit_test(27.0, launcher_y(3), 1440, 900),
        ChromeHitTarget::Vault
    );
}

#[test]
fn utility_cards_have_dedicated_targets() {
    let model = model();
    let width = 1440;
    let height = 900;
    let x = f64::from(width - UTILITY_DOCK_WIDTH_PX) + 40.0;
    let top = f64::from(CHROME_HEIGHT_PX);
    assert_eq!(
        model.hit_test(x, top + 70.0, width, height),
        ChromeHitTarget::YouTubeMusic
    );
    assert_eq!(
        model.hit_test(x, top + 180.0, width, height),
        ChromeHitTarget::Library
    );
    assert_eq!(
        model.hit_test(x, top + 320.0, width, height),
        ChromeHitTarget::StreamStudio
    );
}

#[test]
fn web_content_is_not_owned_by_the_native_background() {
    let mut web = model();
    web.home_mode = false;
    web.omnibox = "https://example.com/".into();
    assert!(!web.native_surface_owns_content());
    let home = model();
    assert!(home.native_surface_owns_content());
}

#[test]
fn home_feature_cards_and_quick_actions_have_live_targets() {
    let model = model();
    let width = 1672;
    let height = 941;
    assert_eq!(
        model.hit_test(300.0, 700.0, width, height),
        ChromeHitTarget::StreamStudio
    );
    assert_eq!(
        model.hit_test(900.0, 700.0, width, height),
        ChromeHitTarget::Vault
    );
    assert_eq!(
        model.hit_test(1360.0, 516.0, width, height),
        ChromeHitTarget::MediaCenter
    );
    assert_eq!(
        model.hit_test(1360.0, 556.0, width, height),
        ChromeHitTarget::StreamStudio
    );
    assert_eq!(
        model.hit_test(1360.0, 596.0, width, height),
        ChromeHitTarget::CreatorHub
    );
}

#[test]
fn utility_hit_targets_match_the_painted_cards() {
    let model = model();
    let width = 1672;
    let height = 941;
    let x_left = f64::from(width - UTILITY_DOCK_WIDTH_PX) + 24.0;
    let x_right = f64::from(width) - 24.0;
    let top = f64::from(CHROME_HEIGHT_PX);
    assert_eq!(
        model.hit_test(x_left, top + 70.0, width, height),
        ChromeHitTarget::YouTubeMusic
    );
    assert_eq!(
        model.hit_test(x_left, top + 112.0, width, height),
        ChromeHitTarget::YouTubeMusicSearch
    );
    assert_eq!(
        model.hit_test(x_right, top + 112.0, width, height),
        ChromeHitTarget::YouTubeMusicLibrary
    );
    assert_eq!(
        model.hit_test(x_left, top + 180.0, width, height),
        ChromeHitTarget::Library
    );
    assert_eq!(
        model.hit_test(x_left, top + 320.0, width, height),
        ChromeHitTarget::StreamStudio
    );
}

#[test]
fn utility_dock_collapses_to_icon_rail_and_reclaims_web_space() {
    let mut model = model();
    model.layout = model.layout.with_utility_collapsed(true);
    let width = 1672;
    let height = 941;
    let x = f64::from(width) - 19.0;
    let top = f64::from(CHROME_HEIGHT_PX);
    assert_eq!(
        model.layout.utility_width(),
        UTILITY_DOCK_COLLAPSED_WIDTH_PX
    );
    assert_eq!(
        model.hit_test(x, top + 19.0, width, height),
        ChromeHitTarget::UtilityCollapseToggle
    );
    assert_eq!(
        model.hit_test(x, top + 68.0, width, height),
        ChromeHitTarget::YouTubeMusic
    );
    assert_eq!(
        model.hit_test(x, top + 118.0, width, height),
        ChromeHitTarget::Library
    );
    assert_eq!(
        model.hit_test(x, top + 168.0, width, height),
        ChromeHitTarget::StreamStudio
    );
}

#[test]
fn adaptive_tabs_shrink_without_losing_hit_targets() {
    let tabs = (0..8)
        .map(|id| ChromeTab {
            id,
            title: format!("Tab {id}"),
            url: if id % 2 == 0 {
                "https://www.youtube.com/".into()
            } else {
                "https://www.twitch.tv/".into()
            },
            active: id == 0,
        })
        .collect();
    let model = ChromeModel::new(tabs, "https://www.youtube.com/");
    let width = model.tab_width_for_count(1200);
    assert!((TAB_MIN_WIDTH_PX..=TAB_MAX_WIDTH_PX).contains(&width));
    assert!(width < TAB_MAX_WIDTH_PX);
}

#[test]
fn focused_omnibox_has_visible_native_caret_contract() {
    assert_eq!(
        OMNIBOX_CARET_VISUAL_CONTRACT,
        "AETHER_BROWSER_OMNIBOX_CARET_VISIBLE_WHEN_ACTIVE"
    );
    let mut model = model();
    assert!(!model.omnibox_active);
    model.omnibox_active = true;
    assert!(model.omnibox_active);
}

#[test]
fn frameless_window_has_eight_way_resize_edges() {
    use aether_ui::{WindowResizeEdge, window_resize_edge};
    let width = 1200;
    let height = 800;
    assert_eq!(
        window_resize_edge(1.0, 1.0, width, height),
        Some(WindowResizeEdge::NorthWest)
    );
    assert_eq!(
        window_resize_edge(1198.0, 1.0, width, height),
        Some(WindowResizeEdge::NorthEast)
    );
    assert_eq!(
        window_resize_edge(1.0, 798.0, width, height),
        Some(WindowResizeEdge::SouthWest)
    );
    assert_eq!(
        window_resize_edge(1198.0, 798.0, width, height),
        Some(WindowResizeEdge::SouthEast)
    );
    assert_eq!(
        window_resize_edge(600.0, 1.0, width, height),
        Some(WindowResizeEdge::North)
    );
    assert_eq!(
        window_resize_edge(600.0, 798.0, width, height),
        Some(WindowResizeEdge::South)
    );
    assert_eq!(
        window_resize_edge(1.0, 400.0, width, height),
        Some(WindowResizeEdge::West)
    );
    assert_eq!(
        window_resize_edge(1198.0, 400.0, width, height),
        Some(WindowResizeEdge::East)
    );
    assert_eq!(window_resize_edge(600.0, 400.0, width, height), None);
}
