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
    );
    assert!(!css.contains("width:212px"));
}

#[test]
fn keeps_the_width_of_a_row_flex_item_that_cannot_grow() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut link = node("a", 230.0, 230.0);
    link.style.extend([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "row".into()),
        ("align-items".into(), "normal".into()),
    ]);
    let mut card = node("div", 230.0, 230.0);
    card.attributes.insert("class".into(), "card".into());
    card.style.extend([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "column".into()),
        ("flex-grow".into(), "0".into()),
        ("position".into(), "relative".into()),
        ("width".into(), "230px".into()),
    ]);
    let rules = [".card{width:230px;height:180px;}".to_string()];
    let css = base_declarations(
        &card,
        Some(&link),
        &viewport,
        &Default::default(),
        &rules,
        false,
    );
    // The card's content is absolutely positioned, so dropping the width collapses it to
    // nothing, and `100%` would make a fixed-size card shrink with its grid track.
    assert!(css.contains("width:230px"), "{css}");
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
    let css = base_declarations(&surface, None, &viewport, &Default::default(), &[], false);
    assert!(css.contains("left:auto"));
    assert!(css.contains("right:12px"));
}

/// Reconciling an over-constrained absolute box to its nearer edge is about the insets,
/// not the size: the authored width is what survives.
#[test]
fn anchors_an_overconstrained_absolute_control_to_its_nearer_edge() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("div", 475.0, 230.0);
    parent.style.insert("position".into(), "relative".into());
    let mut menu = node("div", 657.0, 28.0);
    menu.style.extend([
        ("position".into(), "absolute".into()),
        ("left".into(), "182px".into()),
        ("right".into(), "20px".into()),
        ("width".into(), "28px".into()),
    ]);
    let css = base_declarations(
        &menu,
        Some(&parent),
        &viewport,
        &Default::default(),
        &["*{width:28px;}".to_string()],
        false,
    );
    assert!(css.contains("left:auto"), "{css}");
    assert!(css.contains("right:20px"), "{css}");
    assert!(css.contains("width:28px"), "{css}");
}

#[test]
fn keeps_edges_that_do_not_reconcile_against_the_containing_block() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("div", 0.0, 96.0);
    parent.style.insert("position".into(), "relative".into());
    let mut item = node("div", 1344.0, 96.0);
    item.style.extend([
        ("position".into(), "absolute".into()),
        ("left".into(), "1344px".into()),
        ("right".into(), "0px".into()),
        ("width".into(), "96px".into()),
    ]);
    let css = base_declarations(
        &item,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(css.contains("left:1344px"), "{css}");
}

#[test]
fn keeps_both_edges_when_no_explicit_size_overconstrains_them() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let parent = node("div", 475.0, 230.0);
    let mut title = node("p", 495.0, 190.0);
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
    );
    assert!(css.contains("left:20px"), "{css}");
    assert!(css.contains("right:20px"), "{css}");
}

/// A flex item that fills its parent does so because the source authored `100%` (or a
/// grow). That authored value round-trips; the emitter never invents a percentage to
/// stand in for a width it did not find, because the guess is wrong on every other page.
#[test]
fn keeps_the_authored_fill_of_a_row_flex_item() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("div", 0.0, 1440.0);
    parent.style.insert("display".into(), "flex".into());
    let mut shell = node("div", 0.0, 1440.0);
    shell.style.extend([
        ("width".into(), "1440px".into()),
        ("flex-grow".into(), "0".into()),
        ("overflow-x".into(), "hidden".into()),
    ]);
    let css = base_declarations(
        &shell,
        Some(&parent),
        &viewport,
        &Default::default(),
        &["*{width:100%;}".to_string()],
        false,
    );
    assert!(css.contains("width:100%"), "{css}");
    assert!(!css.contains("width:1440px"), "{css}");
}

/// With nothing authored there is no width to recover, and a sampled 1440px would pin the
/// shell to the captured viewport on every screen.
#[test]
fn drops_an_unauthored_row_flex_item_width() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("div", 0.0, 1440.0);
    parent.style.insert("display".into(), "flex".into());
    let mut shell = node("div", 0.0, 1440.0);
    shell.style.extend([
        ("width".into(), "1440px".into()),
        ("flex-grow".into(), "0".into()),
        ("overflow-x".into(), "hidden".into()),
    ]);
    let css = base_declarations(
        &shell,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:1440px"), "{css}");
}

/// A growing flex item already fills the row, so it must keep the fluid behaviour of
/// having no width rather than being pinned to a percentage.
#[test]
fn leaves_a_growing_row_flex_item_without_a_width() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("div", 0.0, 1440.0);
    parent.style.insert("display".into(), "flex".into());
    let mut shell = node("div", 0.0, 1440.0);
    shell.style.extend([
        ("width".into(), "1440px".into()),
        ("flex-grow".into(), "1".into()),
    ]);
    let css = base_declarations(
        &shell,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
    );
    assert!(!css.contains("width:100%"), "{css}");
    assert!(!css.contains("width:1440px"), "{css}");
}
