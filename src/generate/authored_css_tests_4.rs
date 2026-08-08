//! Selector shape must not decide whether an authored declaration is real.
//!
//! Every twin here is reached by the same declaration `fill: currentColor` through a
//! different selector shape, so any difference between them is caused by the shape alone.

use super::Index;
use crate::model::{Node, Rect, Styles};

/// The scene's four `<svg>` twins: identical markup, one distinguishing width each so the
/// emitter's computed-signature dedupe cannot collapse them and mask a difference.
fn icon(class: Option<&str>, id: Option<&str>, attribute: Option<&str>) -> Node {
    let mut node = Node {
        disabled: false,
        path: "html>body>main>svg".into(),
        parent: Some("html>body>main".into()),
        tag: "svg".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,
        },
        style: Styles::from([("fill".into(), "rgb(0, 128, 64)".into())]),
        before: None,
        after: None,
    };
    if let Some(class) = class {
        node.attributes.insert("class".into(), class.into());
    }
    if let Some(id) = id {
        node.attributes.insert("id".into(), id.into());
    }
    if let Some(attribute) = attribute {
        node.attributes.insert(attribute.into(), String::new());
    }
    node
}

fn scene_rules() -> Vec<String> {
    vec![
        ".tray{color:rgb(0, 128, 64);}".into(),
        "svg{fill:currentColor;height:20px;}".into(),
        ".icon-a{fill:currentColor;width:20px;}".into(),
        ".probe-b{width:22px;}".into(),
        "#icon-c{fill:currentColor;width:24px;}".into(),
        "[data-icon-d]{fill:currentColor;width:26px;}".into(),
    ]
}

#[test]
fn restores_authored_paint_through_every_selector_shape() {
    let rules = scene_rules();
    let index = Index::new(&rules);
    for (shape, node) in [
        ("class", icon(Some("icon-a"), None, None)),
        ("type", icon(Some("probe-b"), None, None)),
        ("id", icon(None, Some("icon-c"), None)),
        ("attribute", icon(None, None, Some("data-icon-d"))),
    ] {
        assert_eq!(
            index.inherited_value(&node, "fill"),
            Some("currentColor".into()),
            "{shape} selector lost the authored value",
        );
    }
}

/// A classless twin reached only by a type selector is the canonical reset idiom, and is the
/// case a class-keyed index cannot see at all.
#[test]
fn restores_authored_paint_for_a_node_with_no_attributes() {
    let rules = vec!["svg{fill:currentColor;}".into()];
    assert_eq!(
        Index::new(&rules).inherited_value(&icon(None, None, None), "fill"),
        Some("currentColor".into())
    );
}

/// A rule whose ancestor condition cannot be checked must not be claimed. The node below
/// carries `control` but has no `.parent` ancestor, so `.parent .control` does not match it.
#[test]
fn refuses_a_descendant_rule_whose_ancestor_is_unverified() {
    let rules = vec![".parent .control{fill:currentColor;}".into()];
    let mut node = icon(Some("control"), None, None);
    node.parent = None;
    assert_eq!(Index::new(&rules).inherited_value(&node, "fill"), None);
}

/// The declaration must belong to a rule that reaches this node, not merely to any rule
/// mentioning one of its axes.
#[test]
fn refuses_a_rule_that_targets_a_different_node() {
    let rules = scene_rules();
    let index = Index::new(&rules);
    let mut stranger = icon(None, Some("icon-z"), None);
    stranger.tag = "span".into();
    assert_eq!(index.inherited_value(&stranger, "fill"), None);
}

/// Disagreeing candidates still abandon restoration rather than guessing a winner.
#[test]
fn refuses_restoration_when_candidates_disagree() {
    let rules = vec![
        "svg{fill:currentColor;}".into(),
        "#icon-c{fill:rgb(1, 2, 3);}".into(),
    ];
    assert_eq!(
        Index::new(&rules).inherited_value(&icon(None, Some("icon-c"), None), "fill"),
        None
    );
}
