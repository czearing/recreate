use super::normalize;
use crate::model::{Node, Rect, Styles};

#[test]
fn ignores_declarations_for_descendants() {
    let mut node = Node {
        disabled: false,
        path: "button".into(),
        parent: None,
        tag: "button".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 36.0,
        },
        style: Styles::from([
            ("width".into(), "200px".into()),
            ("height".into(), "36px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "menu-row".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[
            ".menu-row { width: 100%; height: 36px; display: flex; }".into(),
            ".menu-row svg { width: 20px; height: 20px; flex-shrink: 0; }".into(),
        ],
    );
    assert_eq!(node.style["width"], "100%");
    assert_eq!(node.style["height"], "36px");
    assert!(!node.style.contains_key("flex-shrink"));
}

#[test]
fn restores_centered_max_width_container() {
    let mut node = Node {
        disabled: false,
        path: "main".into(),
        parent: None,
        tag: "main".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 177.0,
            y: 0.0,
            width: 1076.0,
            height: 1000.0,
        },
        style: Styles::from([
            ("width".into(), "1076px".into()),
            ("margin-left".into(), "0px".into()),
            ("margin-right".into(), "0px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "content".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".content { max-width: 1076px; margin: 0 auto; }".into()],
    );
    assert!(!node.style.contains_key("width"));
    assert_eq!(node.style["max-width"], "1076px");
    assert_eq!(node.style["margin-left"], "auto");
    assert_eq!(node.style["margin-right"], "auto");
}

#[test]
fn keeps_the_last_active_responsive_declaration() {
    let mut node = Node {
        disabled: false,
        path: "section".into(),
        parent: None,
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 760.0,
            height: 100.0,
        },
        style: Styles::from([("grid-template-columns".into(), "372px 372px".into())]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "grid".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[
            ".grid { display: grid; grid-template-columns: repeat(2, 1fr); }".into(),
            ".grid { grid-template-columns: 1fr; }".into(),
        ],
    );
    assert_eq!(node.style["display"], "grid");
    assert_eq!(node.style["grid-template-columns"], "1fr");
}
