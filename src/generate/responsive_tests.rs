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
