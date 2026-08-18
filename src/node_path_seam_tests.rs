//! The seam a behavioural test cannot pin: which scripts carry the one path definition.
//!
//! A producer that regrew its own copy resolves every light-DOM element identically and
//! diverges only at a shadow boundary, so these enumerate the producers instead.

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

/// The seam for parentage. A producer that spells the fallback itself resolves every
/// light-DOM parent identically to the shared one and diverges only at a shadow boundary,
/// which is exactly the divergence that erased the subtree — so no behavioural test on a
/// light-DOM page can tell the two apart, and the copy has to be refused by name.
#[test]
fn no_producer_derives_an_element_holder_of_its_own() {
    for (name, script) in producers() {
        assert_eq!(
            script.matches("const holderPath").count(),
            1,
            "{name} does not carry the one definition of an element's holder"
        );
        assert!(
            !script.contains("element.parentElement ? pathOf(element.parentElement)"),
            "{name} derives a holder itself, and so answers null inside a shadow tree"
        );
    }
}
