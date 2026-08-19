//! The invariant every measuring stage depends on, stated without naming shadow DOM or a
//! pseudo-element: a condition declared in order to measure the page is in force in every tree
//! scope the capture reads, and the page is left holding exactly what it was found holding.
//!
//! A rule is in force in the scope whose stylesheets hold it and in no other. A stage that
//! declares its rule in the document alone therefore measures every shadow tree as though it
//! had declared nothing - and says so to nobody, because the read still succeeds and hands back
//! the page's own live values, which is what a page with nothing to say looks like.

use super::double::evaluate;

/// The defect. A rule installed for the duration of a read has to hold wherever that read
/// looks, and a shadow tree is reached by no sheet of the document's.
#[test]
fn a_declared_rule_is_in_force_in_every_tree_scope_during_the_read() {
    let seen = evaluate(
        "const seen = underRules('R', () => globalThis.inForce());",
        "seen",
    );
    let scopes = seen.as_array().expect("one reading per scope");
    assert_eq!(scopes.len(), 4, "{seen}");
    for scope in scopes {
        let held = scope.as_array().expect("the sheets one scope holds");
        assert_eq!(held.len(), 1, "one carrier per scope: {seen}");
        assert!(
            held[0].as_str().expect("sheet text").contains('R'),
            "a scope the read reaches but the rule does not is measured as though nothing was \
             declared: {seen}"
        );
    }
}

/// A root inside a root is a scope like any other. Delivery that walked one level would repair
/// the filed page and leave every component that composes components unmeasured.
#[test]
fn a_scope_nested_inside_a_scope_is_reached_by_the_same_walk() {
    let names = evaluate("null;", "treeScopes().map(scope => scope.name)");
    assert_eq!(
        names,
        serde_json::json!(["document", "outer", "inner", "sibling"])
    );
}

/// Every scope is under the rule before the read begins. A read that ran while the walk was
/// still installing would measure the scopes reached so far under one condition and the rest
/// under another, which is the same wrong answer arriving intermittently.
#[test]
fn every_scope_is_under_the_rule_before_the_read_begins() {
    let installed = evaluate(
        "const seen = underRules('R', () => globalThis.log.slice());",
        "seen.length",
    );
    assert_eq!(installed, serde_json::json!(4));
}

/// A probe that measures a page must leave it as it found it. Removing the rule from the
/// document alone would leave every shadow tree permanently reverted, which every later stage
/// then reads as the page's own styling.
#[test]
fn every_scope_is_left_holding_what_it_was_found_holding() {
    let after = evaluate(
        "const kept = new CSSStyleSheet(); kept.replaceSync('K');\
         \nouter.adoptedStyleSheets = [kept];\
         \nunderRules('R', () => null);",
        "globalThis.inForce()",
    );
    assert_eq!(after, serde_json::json!([[], ["K"], [], []]));
}

/// What is withdrawn is this reading's own sheet, identified by which sheet it is rather than
/// by where it sat when it was installed. A read runs the page's own code - a resize handler,
/// an observer, a component upgrading - and any of it may adopt a sheet of the page's own while
/// the read is in flight. Withdrawing by position discards that sheet and leaves the
/// measurement rule behind, which is both halves of the damage at once.
#[test]
fn a_sheet_the_page_adopts_during_the_read_outlives_the_reading() {
    let after = evaluate(
        "underRules('R', () => {\
         \n  const late = new CSSStyleSheet(); late.replaceSync('L');\
         \n  inner.adoptedStyleSheets = [...inner.adoptedStyleSheets, late];\
         \n});",
        "globalThis.inForce()",
    );
    assert_eq!(after, serde_json::json!([[], [], ["L"], []]));
}

/// A read that throws is still a read that ended, and a page left under a measurement rule is
/// a page no later stage can read. The restoration is owed unconditionally.
#[test]
fn a_read_that_throws_still_restores_every_scope() {
    let after = evaluate(
        "try { underRules('R', () => { throw new Error('read failed'); }); } catch (failed) {}",
        "globalThis.inForce()",
    );
    assert_eq!(after, serde_json::json!([[], [], [], []]));
}

/// The carrier is a constructed sheet adopted per scope, not an element appended per root.
/// Appending would insert a node into a tree the page observes, and this capture's own settle
/// gate is driven by mutation records, so the probe would be answering its own perturbation.
#[test]
fn the_carrier_inserts_no_node_into_any_tree() {
    let source = crate::scoped_rules::SOURCE;
    assert!(source.contains("new CSSStyleSheet()"), "{source}");
    assert!(!source.contains("createElement"), "{source}");
    assert!(!source.contains("appendChild"), "{source}");
}

/// The walk is the whole document, and every reader of it wants the same answer about the same
/// page state, so a reading traverses the page once however many readers ask. Without this a
/// large page pays a full traversal per reader on every viewport it is read at.
#[test]
fn one_reading_enumerates_the_page_once_however_many_readers_ask() {
    let walks = evaluate(
        "globalThis.walks = 0;\
         \nfor (const each of [document, outer, inner, sibling]) {\
         \n  const own = each.querySelectorAll;\
         \n  each.querySelectorAll = () => { globalThis.walks += 1; return own(); };\
         \n}\
         \nunderRules('R', () => underRules('S', () => null));\
         \ntreeScopes();",
        "globalThis.walks",
    );
    assert_eq!(walks, serde_json::json!(4));
}
