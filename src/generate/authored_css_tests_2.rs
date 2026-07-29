use super::normalize;
use crate::model::{Node, Rect, Styles};

#[test]
fn removes_measured_height_from_growing_flex_items() {
    let mut node = Node {
        path: "form>div".into(),
        parent: Some("form".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 34.0,
        },
        style: Styles::from([
            ("width".into(), "320px".into()),
            ("height".into(), "34px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "editor".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".editor { flex: 1 1 0%; min-height: 0; }".into()],
    );
    assert!(!node.style.contains_key("height"));
    assert_eq!(node.style["flex"], "1 1 0%");
    assert_eq!(node.style["min-height"], "0");
}

#[test]
fn keeps_used_height_for_percentage_sized_textareas() {
    let mut node = Node {
        path: "form>textarea".into(),
        parent: Some("form".into()),
        tag: "textarea".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 48.0,
        },
        style: Styles::from([
            ("width".into(), "320px".into()),
            ("height".into(), "48px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "editor".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".editor { flex: 1 1 0%; width: 100%; height: 100%; }".into()],
    );
    assert_eq!(node.style["width"], "100%");
    assert_eq!(node.style["height"], "48px");
}

#[test]
fn removes_sampled_height_from_intrinsic_flex_cards() {
    let mut node = Node {
        path: "article>button".into(),
        parent: Some("article".into()),
        tag: "button".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 369.0,
            height: 245.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("height".into(), "245px".into()),
            ("overflow".into(), "hidden".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "task-card".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".task-card { display: flex; min-height: 132px; overflow: hidden; }".into()],
    );
    assert!(!node.style.contains_key("height"));
    assert_eq!(node.style["min-height"], "132px");
    assert_eq!(node.style["overflow"], "hidden");
}

#[test]
fn removes_resolved_grid_rows_without_authored_tracks() {
    let mut node = Node {
        path: "main>section".into(),
        parent: Some("main".into()),
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 389.0,
            height: 536.0,
        },
        style: Styles::from([
            ("display".into(), "grid".into()),
            ("grid-template-columns".into(), "389px".into()),
            ("grid-template-rows".into(), "168px 164px 164px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "cards".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".cards { display: grid; grid-template-columns: 1fr; gap: 20px; }".into()],
    );

    assert!(!node.style.contains_key("grid-template-rows"));
    assert_eq!(node.style["grid-template-columns"], "1fr");
}

#[test]
fn rejects_authored_layout_values_from_inactive_media_rules() {
    let mut node = Node {
        path: "header".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        },
        style: Styles::from([
            ("display".into(), "flex".into()),
            ("flex-direction".into(), "row".into()),
            ("gap".into(), "normal".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "header".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".header { flex-direction: column; gap: 4px; }".into()],
    );
    assert_eq!(node.style["flex-direction"], "row");
    assert_eq!(node.style["gap"], "normal");
}
