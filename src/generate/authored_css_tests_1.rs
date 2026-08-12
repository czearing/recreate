use super::{directly_targets_node, normalize, positive_integer_property};
use crate::model::{Node, Rect, Styles};

#[test]
fn compound_modifier_rules_require_every_class() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "section".into(),
        parent: None,
        tag: "section".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "items".into());
    assert!(directly_targets_node(".items", &node));
    assert!(!directly_targets_node(".items.list", &node));
    assert_eq!(
        positive_integer_property(
            &node,
            &[".items{-webkit-line-clamp:2}".into()],
            "-webkit-line-clamp"
        ),
        Some(2)
    );
    node.attributes.insert("class".into(), "items list".into());
    assert!(directly_targets_node(".items.list", &node));
}

#[test]
fn direct_tag_id_and_attribute_selectors_match_without_descendant_leaks() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "html>body>main".into(),
        parent: Some("html>body".into()),
        tag: "main".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes.insert("id".into(), "content".into());
    node.attributes.insert("hidden".into(), String::new());
    node.attributes.insert("role".into(), "button".into());
    assert!(directly_targets_node("main", &node));
    assert!(directly_targets_node("main#content[hidden]", &node));
    assert!(directly_targets_node("[role=\"button\"]", &node));
    assert!(!directly_targets_node("[role=\"dialog\"]", &node));
    assert!(!directly_targets_node("body main", &node));
}

#[test]
fn restores_authored_intrinsic_motion() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "button".into(),
        parent: None,
        tag: "button".into(),
        text: "Create".into(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 2.0,
            height: 28.0,
        },
        style: Styles::from([
            ("width".into(), "2px".into()),
            ("height".into(), "28px".into()),
            ("transition".into(), "all".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "create".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".create { max-width: 0; transition: opacity .2s, max-width .3s; }".into()],
    );
    assert!(!node.style.contains_key("width"));
    assert_eq!(node.style["height"], "28px");
    assert_eq!(node.style["transition"], "opacity .2s, max-width .3s");
    assert_eq!(node.style["max-width"], "0");
}

#[test]
fn keeps_resolved_motion_when_authored_variables_are_filtered() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "div".into(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 230.0,
            height: 180.0,
        },
        style: Styles::from([
            ("position".into(), "relative".into()),
            (
                "transition".into(),
                "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)".into(),
            ),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "card".into());
    let rules = [".card { position: relative; transition: transform var(--slow); }".into()];
    let captured = node.clone();

    normalize(&mut node.style, &captured, &rules);

    assert_eq!(
        node.style["transition"],
        "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)"
    );
}

#[test]
fn keeps_measured_width_for_ordinary_flex_items() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "article".into(),
        parent: None,
        tag: "article".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 253.0,
            height: 236.0,
        },
        style: Styles::from([
            ("width".into(), "253px".into()),
            ("height".into(), "185px".into()),
        ]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "card".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".card { flex: 0 0 auto; height: auto; transition: box-shadow .2s; }".into()],
    );
    assert_eq!(node.style["width"], "253px");
    assert_eq!(node.style["height"], "185px");
    assert_eq!(node.style["flex"], "0 0 auto");
}

#[test]
fn removes_measured_width_from_growing_flex_items() {
    let mut node = Node {
        disabled: false,
        rtl: false,
        path: "article>div".into(),
        parent: Some("article".into()),
        tag: "div".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        },
        style: Styles::from([("width".into(), "320px".into())]),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), "content".into());
    let captured = node.clone();
    normalize(
        &mut node.style,
        &captured,
        &[".content { flex: 1 1 0%; min-width: 0; }".into()],
    );
    assert!(!node.style.contains_key("width"));
    assert_eq!(node.style["flex"], "1 1 0%");
    assert_eq!(node.style["min-width"], "0");
}
