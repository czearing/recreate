use super::*;

#[test]
fn stretches_absolute_content_between_captured_edges() {
    let viewport = Viewport {
        width: 390,
        height: 844,
        dpr: 1.0,
    };
    let parent = node("card", 40.0, 300.0);
    let mut title = node("p", 60.0, 260.0);
    title.style.extend([
        ("position".into(), "absolute".into()),
        ("left".into(), "20px".into()),
        ("right".into(), "20px".into()),
    ]);
    let css = base_declarations(
        &title,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(!css.contains("width:260px"));
    assert!(css.contains("left:20px"));
    assert!(css.contains("right:20px"));
}

#[test]
fn resets_frozen_width_when_responsive_edges_take_over() {
    let wide = Viewport {
        width: 1920,
        height: 1080,
        dpr: 1.0,
    };
    let narrow = Viewport {
        width: 320,
        height: 568,
        dpr: 1.0,
    };
    let narrow_parent = node("body", 0.0, 320.0);
    let mut base = node("section", 384.0, 1152.0);
    base.style.extend([
        ("position".into(), "fixed".into()),
        ("left".into(), "384px".into()),
        ("right".into(), "384px".into()),
    ]);
    let mut current = node("section", 64.0, 192.0);
    current.style.extend([
        ("position".into(), "fixed".into()),
        ("left".into(), "64px".into()),
        ("right".into(), "64px".into()),
    ]);
    let mut changed = changed_styles(&base.style, &current.style);
    normalize_viewport_width(
        &mut changed,
        &current,
        Some(&narrow_parent),
        &narrow,
        Some((&base, &wide)),
    );
    assert_eq!(changed.get("width").map(String::as_str), Some("auto"));
}

#[test]
fn stretches_grid_items_across_responsive_tracks() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("grid", 242.0, 946.0);
    parent.style.insert("display".into(), "grid".into());
    let mut card = node("article", 242.0, 212.0);
    card.style.extend([
        ("display".into(), "flex".into()),
        ("position".into(), "static".into()),
        ("justify-self".into(), "normal".into()),
    ]);
    let css = base_declarations(
        &card,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(!css.contains("width:212px"));
}

#[test]
fn anchors_fixed_surfaces_to_the_nearest_viewport_edge() {
    let viewport = Viewport {
        width: 1920,
        height: 1080,
        dpr: 1.0,
    };
    let mut surface = node("div", 1360.0, 548.0);
    surface.style.extend([
        ("position".into(), "fixed".into()),
        ("left".into(), "1360px".into()),
        ("right".into(), "12px".into()),
        ("inset".into(), "44px 12px 470px 1360px".into()),
    ]);
    let css = base_declarations(
        &surface,
        None,
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("left:auto"));
    assert!(css.contains("right:12px"));
}
