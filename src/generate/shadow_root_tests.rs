//! The sentinel-to-construct translation, at the level of the rule.

use super::{TAG, element, is_root, mode};
use crate::model::Node;

fn shadow(path: &str) -> Node {
    Node {
        path: path.into(),
        tag: TAG.into(),
        ..Default::default()
    }
}

/// The mode is part of the address because a host may hold either kind, and the two are
/// different tree scopes. It cannot be recovered from the host afterwards either: a closed
/// root is unreachable through `host.shadowRoot`, so losing it here loses it for good.
#[test]
fn reads_the_mode_the_address_records() {
    assert_eq!(
        mode("html>body:nth-of-type(1)>::shadow-root(open)"),
        Some("open")
    );
    assert_eq!(mode("x>y>::shadow-root(closed)"), Some("closed"));
}

/// Only the last segment of an address opens a tree. An element *inside* a shadow tree
/// carries the same substring in its path, and treating it as a root would open a second,
/// empty tree on it and portal its own subtree out of view.
#[test]
fn refuses_an_address_that_merely_passes_through_a_shadow_tree() {
    assert_eq!(mode("x>::shadow-root(open)>div:nth-of-type(1)"), None);
    assert_eq!(mode("html>body:nth-of-type(1)"), None);
}

/// The whole point of the translation: the sentinel is a name in the address grammar, so it
/// must leave the artifact entirely rather than be sanitised into a tag the page never had.
#[test]
fn emits_a_component_rather_than_the_sentinel() {
    let rendered = element(&shadow("x>::shadow-root(open)"), "    <div/>\n", "  ");
    assert_eq!(
        rendered,
        "  <ShadowRoot mode={\"open\"}>\n    <div/>\n  </ShadowRoot>\n"
    );
    assert!(!rendered.contains(TAG));
}

/// A shadow root has no box and no class, so anything a class would carry belongs to the
/// host. Emitting one would also make the boundary look like a styled element in the output.
#[test]
fn carries_the_mode_and_nothing_else() {
    let rendered = element(&shadow("x>::shadow-root(closed)"), "", "");
    assert_eq!(rendered, "<ShadowRoot mode={\"closed\"}>\n</ShadowRoot>\n");
}

/// Elements are translated by tag, so the judgement has to be the tag rather than the path;
/// an element whose own address passes through a shadow tree is still an element.
#[test]
fn recognises_a_shadow_root_by_the_tag_the_walk_gave_it() {
    assert!(is_root(&shadow("x>::shadow-root(open)")));
    assert!(!is_root(&Node {
        path: "x>::shadow-root(open)>div:nth-of-type(1)".into(),
        tag: "div".into(),
        ..Default::default()
    }));
}
