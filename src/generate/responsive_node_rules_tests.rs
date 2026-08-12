use super::*;
use crate::model::Rect;

fn heading(font_size: &str, width: f64) -> Node {
    Node {
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: "html>body>h1".into(),
        parent: Some("html>body".into()),
        tag: "h1".into(),
        text: "Heading".into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height: 100.0,
        },
        style: Styles::from([
            ("font-size".into(), font_size.into()),
            ("width".into(), format!("{width}px")),
        ]),
        before: None,
        after: None,
    }
}

#[test]
fn emits_responsive_clamped_typography() {
    let base = heading("68px", 760.0);
    let current = heading("53.76px", 603.0);
    let css = append_node_rules(
        &base,
        &current,
        None,
        (
            &Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
            &Viewport {
                width: 768,
                height: 1024,
                dpr: 1.0,
            },
        ),
        "heading",
        &Default::default(),
        &["h1{font-size:clamp(36px,7vw,68px);max-width:16ch;}".into()],
        false,
        false,
    );
    assert!(css.contains("font-size:53.76px"), "{css}");
}

fn shrinking_title(width: f64) -> Node {
    Node {
        writing_mode: Default::default(),
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: "html>body>div>span".into(),
        parent: Some("html>body>div".into()),
        tag: "span".into(),
        text: "Untitled notebook".into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height: 22.0,
        },
        style: Styles::from([
            ("flex-shrink".into(), "1".into()),
            ("width".into(), format!("{width}px")),
        ]),
        before: None,
        after: None,
    }
}

fn shrinking_title_row(width: f64) -> Node {
    let mut parent = shrinking_title(width);
    parent.path = "html>body>div".into();
    parent.tag = "div".into();
    parent.style = Styles::from([
        ("display".into(), "flex".into()),
        ("flex-direction".into(), "row".into()),
    ]);
    parent
}

fn shrunk_title_rule() -> String {
    append_node_rules(
        &shrinking_title(234.0),
        &shrinking_title(170.65625),
        Some(&shrinking_title_row(420.0)),
        (
            &Viewport {
                width: 1440,
                height: 900,
                dpr: 1.0,
            },
            &Viewport {
                width: 768,
                height: 1024,
                dpr: 1.0,
            },
        ),
        "title",
        &Default::default(),
        &[],
        false,
        false,
    )
}

#[test]
fn lets_a_shrunk_flex_item_shrink_instead_of_pinning_its_sampled_width() {
    let css = shrunk_title_rule();
    assert!(css.contains("min-width:0"), "{css}");
}

#[test]
fn never_freezes_a_sampled_pixel_width_on_a_shrunk_flex_item() {
    let css = shrunk_title_rule();
    assert!(!css.contains("max-width:170.65625px"), "{css}");
    assert!(!css.contains("width:170.65625px"), "{css}");
}
