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

/// The same, for the half of the definition that is allowed to answer "nothing holds this".
fn holders(scene: &str, expression: &str) -> Vec<Option<String>> {
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

/// The parentage half of the address, which is the half the artifact is assembled from.
///
/// Every top node of a shadow tree has a null `parentElement`, so a record derived from that
/// property alone reports no holder at all for them — while their paths stay perfectly
/// correct. The tree builder attaches a node only under a holder it was given, so the whole
/// shadow subtree detaches at its top and disappears from the output, addresses and all.
#[test]
fn holds_the_top_of_a_shadow_tree_under_the_root_that_opened_it() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const card = element('X-CARD', body);
      const root = attachShadow(card, 'open');
      const frame = inShadow(root, 'DIV');
      const nested = element('SPAN', frame);
    ";
    assert_eq!(
        holders(scene, "[holderPath(frame), holderPath(nested)]"),
        [
            Some(
                "html>body:nth-of-type(1)>x-card:nth-of-type(1)>::shadow-root(open)".to_string()
            ),
            Some(
                "html>body:nth-of-type(1)>x-card:nth-of-type(1)>::shadow-root(open)>div:nth-of-type(1)"
                    .to_string()
            ),
        ]
    );
}

/// The unchanged case, and the one value that is genuinely absent. A holder of `null` has to
/// keep meaning "nothing holds this", or the document root acquires a parent and the tree
/// builder loses its only entry point.
#[test]
fn reports_no_holder_only_for_the_element_that_has_none() {
    let scene = "
      const body = element('BODY', document.documentElement);
      const div = element('DIV', body);
    ";
    assert_eq!(
        holders(
            scene,
            "[holderPath(document.documentElement), holderPath(body), holderPath(div)]"
        ),
        [
            None,
            Some("html".to_string()),
            Some("html>body:nth-of-type(1)".to_string()),
        ]
    );
}

#[cfg(test)]
#[path = "node_path_seam_tests.rs"]
mod seam;
