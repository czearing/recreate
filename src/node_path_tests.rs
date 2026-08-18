//! The address rule, run under Node against a DOM double.
//!
//! A path is the key every capture pass writes into one map, so the questions here are about
//! agreement: the same element must resolve the same way whichever pass met it, and an
//! element that lives in a shadow tree must get an address rather than abort the run.

use crate::node_eval::evaluate;

const HARNESS: &str = include_str!("node_path_harness.js");

/// Builds the scene once and asks the shipped definition for the paths named by `expression`.
fn paths(scene: &str, expression: &str) -> Vec<String> {
    let preamble = format!("{}\n{scene}", crate::node_path::embed(HARNESS));
    serde_json::from_value(evaluate(&preamble, expression)).unwrap()
}

/// A page with no shadow tree at all: the path is the chain of elements, numbered per tag.
#[test]
fn addresses_a_light_dom_element_by_its_position_among_same_tag_siblings() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const first = element('DIV', body);
      const span = element('SPAN', body);
      const second = element('DIV', body);
    ";
    assert_eq!(
        paths(scene, "[pathOf(first), pathOf(span), pathOf(second)]"),
        [
            "html>body:nth-of-type(1)>div:nth-of-type(1)",
            "html>body:nth-of-type(1)>span:nth-of-type(1)",
            "html>body:nth-of-type(1)>div:nth-of-type(2)",
        ]
    );
}

/// The defect, at the level of the rule. A slot lives in the shadow tree, so its parent node
/// is a `ShadowRoot` and `parentElement` is null; the light-DOM-only twin read `null.children`
/// and threw, which aborted the whole capture and left no artifact behind.
///
/// The address it must produce instead names the tree it belongs to, through the host, so two
/// hosts on one page cannot both claim `slot:nth-of-type(1)`.
#[test]
fn addresses_a_slot_through_the_shadow_tree_that_holds_it() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const card = element('X-CARD', body);
      const slot = inShadow(attachShadow(card, 'open'), 'SLOT');
    ";
    assert_eq!(
        paths(scene, "[pathOf(slot)]"),
        ["html>body:nth-of-type(1)>x-card:nth-of-type(1)>::shadow-root(open)>slot:nth-of-type(1)"]
    );
}

/// Two instances of one component, which is what content projection is for. Their slots are
/// distinguished only by the host, so a path that stopped at the shadow boundary would give
/// both the same key and silently merge two elements' records into one.
#[test]
fn distinguishes_the_same_slot_in_two_instances_of_one_component() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const first = element('X-CARD', body);
      const second = element('X-CARD', body);
      const a = inShadow(attachShadow(first, 'open'), 'SLOT');
      const b = inShadow(attachShadow(second, 'open'), 'SLOT');
    ";
    let paths = paths(scene, "[pathOf(a), pathOf(b)]");
    assert_ne!(paths[0], paths[1]);
    assert!(paths[1].contains("x-card:nth-of-type(2)>::shadow-root(open)"));
}

/// A shadow tree numbers its own children, and the numbering is taken over the root rather
/// than over a parent element, because the top nodes of a shadow tree have no parent element
/// to enumerate. Reading `element.parentElement.children` here is the crash; falling back to
/// the document's numbering instead would be the silent wrong key.
#[test]
fn numbers_the_top_nodes_of_a_shadow_tree_within_that_tree() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const card = element('X-CARD', body);
      const root = attachShadow(card, 'open');
      const frame = inShadow(root, 'DIV');
      const second = inShadow(root, 'DIV');
      const nested = element('DIV', frame);
    ";
    let paths = paths(scene, "[pathOf(frame), pathOf(second), pathOf(nested)]");
    assert!(paths[0].ends_with("::shadow-root(open)>div:nth-of-type(1)"));
    assert!(paths[1].ends_with("::shadow-root(open)>div:nth-of-type(2)"));
    assert!(paths[2].ends_with("::shadow-root(open)>div:nth-of-type(1)>div:nth-of-type(1)"));
}

/// The mode is part of the address because a host may hold either kind, and a closed root is
/// a different tree scope from an open one. Recording the tree without it would make the two
/// indistinguishable in the artifact.
#[test]
fn records_the_mode_of_the_tree_the_element_lives_in() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const card = element('X-CARD', body);
      const slot = inShadow(attachShadow(card, 'closed'), 'SLOT');
    ";
    assert!(paths(scene, "[pathOf(slot)]")[0].contains("::shadow-root(closed)"));
}

/// Every script that resolves an element to a path, named once.
///
/// A path is a key in one map, so these must agree byte for byte; the list is here rather
/// than in each caller so a new producer is added by one line and cannot be half-registered.
fn producers() -> Vec<(&'static str, String)> {
    vec![
        (
            "resting capture",
            crate::page_script::source_without_assets(),
        ),
        ("interaction capture", crate::interaction_script::source()),
        ("lifecycle recorder", crate::lifecycle_script::source()),
        (
            "interaction discovery",
            crate::interactions::interactions_scripts::candidates(),
        ),
        (
            "action scope",
            crate::interactions::interactions_scripts::action_scope(),
        ),
        (
            "comparison animations",
            crate::compare_capture::animations_script(),
        ),
        (
            "focused element",
            crate::interactions_input::focused_script(),
        ),
    ]
}

/// The seam, which is the part a behavioural test cannot pin: a producer that grew its own
/// copy would still resolve light-DOM paths correctly and would still pass every assertion
/// above, right up to the moment it met a shadow tree and aborted the run.
#[test]
fn every_producer_of_a_path_carries_the_one_definition() {
    for (name, script) in producers() {
        assert!(
            crate::node_path::embedded(&script),
            "{name} does not carry the shared path definition"
        );
        assert_eq!(
            script.matches("const pathOf").count(),
            1,
            "{name} defines a path function of its own"
        );
        assert!(
            !script.contains(crate::node_path::PLACEHOLDER),
            "{name} ships the placeholder unsubstituted"
        );
    }
}

/// No producer may reach a parent's children without the branch that survives a shadow root.
/// This is the exact expression that threw, and its absence is what the whole repair buys.
#[test]
fn no_producer_reads_children_through_an_unguarded_parent_element() {
    for (name, script) in producers() {
        assert!(
            !script.contains("parentElement.children"),
            "{name} enumerates a parent element that is null inside a shadow tree"
        );
        assert!(
            script.contains("root instanceof ShadowRoot"),
            "{name} cannot tell a shadow boundary from the document root"
        );
    }
}
