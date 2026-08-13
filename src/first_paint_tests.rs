//! What decides that a page had a first-paint phase.
//!
//! Every fixture in the suite that touches `startup_nodes` builds it by hand, so the suite
//! tested what happens once the phase is recorded and never what admits one. These do the
//! opposite: they call the selector and the reader directly and never construct a phase.

use super::startup_nodes;
use crate::model::{Node, PageState, Rect};
use std::collections::BTreeSet;

/// The whole defect in one assertion. This skeleton is `position: static`, carries no
/// `z-index`, and is a card rather than a viewport, so it fails the blocking-overlay
/// predicate on clauses that are categorical rather than numeric — no threshold could admit
/// it. It must still be recorded, because it was on the page and then was not.
#[test]
fn an_unpositioned_placeholder_that_is_replaced_is_recorded_as_the_first_paint_phase() {
    let first = page(&[
        ("html>body>div", None),
        ("html>body>div>div", Some("html>body>div")),
        ("html>body>div>div>#text(1)", Some("html>body>div>div")),
    ]);
    let settled = paths(&["html>body>div", "html>body>div>p"]);

    let recorded = startup_nodes(&first, &settled);

    assert_eq!(
        recorded.iter().map(|n| n.path.as_str()).collect::<Vec<_>>(),
        vec![
            "startup>html>body>div>div",
            "startup>html>body>div>div>#text(1)"
        ],
        "the replaced subtree, and only it, is the phase"
    );
    let root = &recorded[0];
    assert_eq!(
        root.parent, None,
        "the surviving wrapper is not part of the phase, so the phase has a root to portal"
    );
    assert_eq!(
        recorded[1].parent.as_deref(),
        Some("startup>html>body>div>div"),
        "a descendant keeps its parent so the phase rebuilds as a tree, not a flat list"
    );
    assert!(
        recorded.iter().all(|n| !n.blocking_overlay),
        "nothing here matches the overlay predicate; that is the point"
    );
}

/// The negative control, as a unit. A page that only appends has every earlier path still
/// present, so the removed set is empty and no phase is invented. Without this, a fix could
/// satisfy every positive assertion by emitting a startup layer for every page.
#[test]
fn a_page_that_only_adds_nodes_has_no_first_paint_phase() {
    let first = page(&[("html>body>p:nth-of-type(1)", None)]);
    let settled = paths(&["html>body>p:nth-of-type(1)", "html>body>p:nth-of-type(2)"]);

    assert!(startup_nodes(&first, &settled).is_empty());
}

/// A prepended sibling shifts every later index, and the paths of the nodes that were already
/// there are among the shifted ones. The set of paths still only grows, so this must not read
/// as a removal — otherwise every list that gains a row at the top reports a phantom phase.
#[test]
fn prepending_a_sibling_is_not_read_as_a_removal() {
    let first = page(&[
        ("html>body>li:nth-of-type(1)", None),
        ("html>body>li:nth-of-type(2)", None),
    ]);
    let settled = paths(&[
        "html>body>li:nth-of-type(1)",
        "html>body>li:nth-of-type(2)",
        "html>body>li:nth-of-type(3)",
    ]);

    assert!(startup_nodes(&first, &settled).is_empty());
}

/// A phase that is removed wholesale contributes exactly one root. Two roots would portal two
/// fragments for one phase, and zero roots suppresses the overlay CSS entirely.
#[test]
fn a_wholly_removed_subtree_contributes_one_root() {
    let first = page(&[
        ("html>body", None),
        ("html>body>div", Some("html>body")),
        ("html>body>div>span", Some("html>body>div")),
    ]);
    let settled = paths(&["html>body"]);

    let recorded = startup_nodes(&first, &settled);

    assert_eq!(
        recorded.iter().filter(|n| n.parent.is_none()).count(),
        1,
        "the highest removed node is the only root"
    );
    assert_eq!(recorded.len(), 2);
}

fn paths(paths: &[&str]) -> BTreeSet<&'static str> {
    paths
        .iter()
        .map(|path| Box::leak(path.to_string().into_boxed_str()) as &'static str)
        .collect()
}

fn page(nodes: &[(&str, Option<&str>)]) -> PageState {
    PageState {
        nodes: nodes
            .iter()
            .map(|(path, parent)| node(path, *parent))
            .collect(),
        ..Default::default()
    }
}

fn node(path: &str, parent: Option<&str>) -> Node {
    Node {
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: path.rsplit('>').next().unwrap_or(path).into(),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 96.0,
        },
        ..Default::default()
    }
}
