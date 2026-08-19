//! Which grouped rules a walk contributes, and with what condition still attached.
//!
//! A grouping rule is evidence twice over: the declarations it holds and the condition that
//! guards them. Recording one without the other publishes a rule the author never wrote.

use super::{recorded, scene, walk};
/// A `@container` block the page's *current* layout cannot satisfy still declares something,
/// because the recreation is a live document whose container may be resized. What must not
/// happen is the fabrication: keeping the declarations while dropping the condition, which
/// publishes as unconditional a rule the author guarded. So the block travels, wrapped.
#[test]
fn an_unsatisfied_container_block_keeps_the_condition_that_guards_it() {
    let rules = recorded(&walk(scene()));
    let guarded: Vec<_> = rules
        .iter()
        .filter(|rule| rule.contains("width: 100%"))
        .collect();
    assert_eq!(guarded.len(), 1, "{rules:?}");
    assert!(
        guarded[0].starts_with("@container panelwrap (min-width: 900px)"),
        "a container condition was baked away, publishing an unconditional rule: {rules:?}"
    );
}

/// `@supports` keeps its nested rules in the CSSOM when the condition is false, so a walk
/// that reads them without evaluating the condition records styles no browser applies.
#[test]
fn a_false_supports_block_contributes_no_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        !rules.iter().any(|rule| rule.contains("max-width: 50%")),
        "recorded a dead @supports declaration: {rules:?}"
    );
}

/// The inverse of the two tests above. Discarding every grouped rule would satisfy them
/// while destroying the styles that do apply, so both directions are asserted.
#[test]
fn a_satisfied_condition_still_contributes_its_authored_rule() {
    let rules = recorded(&walk(scene()));
    assert!(
        rules.iter().any(|rule| rule.starts_with(".grid")),
        "lost a live @supports declaration: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with("@media (min-width: 0px)") && rule.contains(".wide")),
        "lost a declaration nested in two satisfied conditions: {rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule.starts_with(".panel") && rule.contains("padding")),
        "lost an unconditional declaration: {rules:?}"
    );
}
