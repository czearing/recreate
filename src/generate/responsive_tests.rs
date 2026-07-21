use super::*;
use crate::model::{Attributes, Rect};

fn node(tag: &str, x: f64, width: f64) -> Node {
    let mut attributes = Attributes::new();
    if tag == "root" {
        attributes.insert("id".into(), "root".into());
    }
    Node {
        path: tag.into(),
        parent: None,
        tag: if tag == "root" { "div" } else { tag }.into(),
        text: String::new(),
        attributes,
        rect: Rect {
            x,
            y: 0.0,
            width,
            height: 40.0,
        },
        style: Styles::from([("width".into(), format!("{width}px"))]),
        before: None,
        after: None,
    }
}

fn filtered_width(node: Node, viewport: &Viewport) -> Option<String> {
    let mut styles = node.style.clone();
    normalize_viewport_width(&mut styles, &node, viewport, None);
    styles.get("width").cloned()
}

#[test]
fn omits_fluid_root_width_but_keeps_content_width() {
    let viewport = Viewport {
        width: 390,
        height: 844,
        dpr: 1.0,
    };
    assert_eq!(filtered_width(node("body", 8.0, 374.0), &viewport), None);
    assert_eq!(
        filtered_width(node("main", 8.0, 374.0), &viewport).as_deref(),
        Some("374px")
    );
}

#[test]
fn preserves_centered_fixed_width_root() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    assert_eq!(
        filtered_width(node("root", 320.0, 800.0), &viewport).as_deref(),
        Some("800px")
    );
}

#[test]
fn preserves_compact_control_width_when_it_fills_its_parent() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let control = node("button", 0.0, 28.0);
    let parent = node("div", 0.0, 28.0);
    let css = base_declarations(
        &control,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:28px"));
}

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

#[test]
fn preserves_intrinsic_svg_aspect_width() {
    let viewport = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let image = node("svg", 0.0, 174.5);
    let parent = node("div", 0.0, 174.5);
    let css = base_declarations(
        &image,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );
    assert!(css.contains("width:174.5px"));
}

#[test]
fn removes_measured_width_when_border_box_fills_parent_content() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("parent", 4.0, 760.0);
    parent.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "758px".into()),
        ("border-left-width".into(), "1px".into()),
        ("border-right-width".into(), "1px".into()),
    ]);
    let mut child = node("child", 5.0, 758.0);
    child.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "714px".into()),
        ("padding-left".into(), "22px".into()),
        ("padding-right".into(), "22px".into()),
    ]);

    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );

    assert!(!css.contains("width:714px"));
}

#[test]
fn removes_measured_width_when_parent_uses_border_shorthand() {
    let viewport = Viewport {
        width: 1920,
        height: 1080,
        dpr: 1.0,
    };
    let mut parent = node("parent", 465.0, 980.0);
    parent.style.extend([
        ("box-sizing".into(), "border-box".into()),
        ("width".into(), "980px".into()),
        ("border".into(), "1px solid rgb(0, 0, 0)".into()),
    ]);
    let mut child = node("child", 466.0, 978.0);
    child.style.extend([
        ("box-sizing".into(), "border-box".into()),
        ("width".into(), "978px".into()),
        ("padding-left".into(), "16px".into()),
        ("padding-right".into(), "16px".into()),
    ]);

    let css = base_declarations(
        &child,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        false,
    );

    assert!(!css.contains("width:978px"));
}

#[test]
fn removes_measured_height_for_evidence_backed_content_reflow() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut card = node("button", 0.0, 369.0);
    card.style.insert("height".into(), "245px".into());
    let css = base_declarations(
        &card,
        None,
        &viewport,
        &Default::default(),
        &[],
        true,
        false,
    );
    assert!(!css.contains("height:245px"));
}

#[test]
fn removes_measured_width_from_intrinsic_column_flex_text() {
    let viewport = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let mut parent = node("section", 0.0, 768.0);
    parent.style.extend([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "column".into()),
    ]);
    let mut subtitle = node("div", 0.0, 475.765625);
    subtitle.text = "Bring everyone together".into();
    subtitle.style.extend([
        ("display".into(), "block".into()),
        ("position".into(), "static".into()),
        ("width".into(), "475.765625px".into()),
    ]);
    let css = base_declarations(
        &subtitle,
        Some(&parent),
        &viewport,
        &Default::default(),
        &[],
        false,
        true,
    );
    assert!(!css.contains("width:475.765625px"));
}

#[test]
fn resets_captured_width_when_child_becomes_parent_filling() {
    let wide = Viewport {
        width: 1440,
        height: 900,
        dpr: 1.0,
    };
    let narrow = Viewport {
        width: 768,
        height: 900,
        dpr: 1.0,
    };
    let base = node("child", 204.0, 1076.0);
    let mut current = node("child", 5.0, 758.0);
    current.style.insert("width".into(), "714px".into());
    let mut parent = node("parent", 4.0, 760.0);
    parent.style.extend([
        ("box-sizing".into(), "content-box".into()),
        ("width".into(), "758px".into()),
        ("border-left-width".into(), "1px".into()),
        ("border-right-width".into(), "1px".into()),
    ]);
    let mut changed = changed_styles(&base.style, &current.style);

    crate::generate::responsive_geometry::normalize(
        &mut changed,
        &current,
        Some(&parent),
        &narrow,
        Some((&base, &wide)),
    );

    assert_eq!(changed.get("width").map(String::as_str), Some("auto"));
}

#[test]
fn writes_auto_when_centered_fixed_root_becomes_fluid() {
    let wide = Viewport {
        width: 1200,
        height: 800,
        dpr: 1.0,
    };
    let narrow = Viewport {
        width: 600,
        height: 600,
        dpr: 1.0,
    };
    let base = node("root", 200.0, 800.0);
    let current = node("root", 0.0, 600.0);
    let mut changed = changed_styles(&base.style, &current.style);
    normalize_viewport_width(&mut changed, &current, &narrow, Some((&base, &wide)));
    assert_eq!(changed.get("width").map(String::as_str), Some("auto"));
}

#[test]
fn writes_auto_when_fluid_root_matches_fixed_base_width() {
    let wide = Viewport {
        width: 1200,
        height: 800,
        dpr: 1.0,
    };
    let narrow = Viewport {
        width: 720,
        height: 800,
        dpr: 1.0,
    };
    let base = node("root", 240.0, 720.0);
    let current = node("root", 0.0, 720.0);
    let mut changed = changed_styles(&base.style, &current.style);
    assert!(!changed.contains_key("width"));
    normalize_viewport_width(&mut changed, &current, &narrow, Some((&base, &wide)));
    assert_eq!(changed.get("width").map(String::as_str), Some("auto"));
}

#[test]
fn sparse_capture_owns_widths_until_next_capture() {
    assert_eq!(band(390, None, 1440, true), (None, 1439));
}

#[test]
fn preserves_measured_multi_layout_bands() {
    assert_eq!(band(1440, Some(768), 1920, false), (Some(769), 1440));
    assert_eq!(band(768, Some(390), 1440, false), (Some(391), 768));
    assert_eq!(band(390, Some(320), 768, false), (Some(321), 390));
    assert_eq!(band(320, None, 390, false), (None, 320));
}
