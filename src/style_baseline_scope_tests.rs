//! What a baseline is measured against, wherever the walk reached the thing being measured.
//!
//! The probe rolls the page back to the user-agent origin and reads it there, so that a value
//! equal to that baseline can be pruned as unauthored. An element carries that rollback on its
//! own style attribute, which is a property of the node and so travels with it into any tree
//! scope. A pseudo-element has no node, so its rollback can only be a rule - and a rule reaches
//! the scope whose sheets hold it and no other. When the two halves of one measurement have two
//! reaches, everything the walk found past the shorter one is compared against its own live
//! value, the difference is empty, and every declaration on it is pruned as unauthored.
//!
//! Nothing reports that. The read succeeds, a box that carries content is still recorded because
//! of the content, and a box that carries none is dropped without being counted as declined.

use crate::style_baseline_double::evaluate;

/// The page these tests measure: the same authored boxes on one element per tree scope, at three
/// depths, so the only thing that varies between them is which scope the walk reached them
/// through. `::marker` is authored on all three and carries no content, which is the box a
/// repair aimed at the two content-carrying names would leave behind.
const SCOPES: &str = r#"
globalThis.authoredSelectors = ['P::marker'];
const host = body.add(new Element('PROBE-CARD'));
const shadow = host.attachShadow();
const shallow = shadow.add(new Element('P'));
shallow.setAttribute('id', 'shallow');
const innerHost = shadow.add(new Element('PROBE-INNER'));
const nested = innerHost.attachShadow().add(new Element('P'));
nested.setAttribute('id', 'nested');
for (const name of ['P#marked', 'P#shallow', 'P#nested']) {
  globalThis.content.set(name + '::before', '"x"');
  globalThis.pseudoAuthored.add(name + '::before').add(name + '::marker');
}
"#;

/// Runs the walk over that page and reports what `probe` evaluates to under it.
fn measured(probe: &str) -> serde_json::Value {
    evaluate(
        &format!("{SCOPES}\nread();"),
        &format!("eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\\n{probe}')"),
    )
}

/// The invariant, stated over the whole walk rather than over one scope: no baseline the probe
/// recorded may be a reading taken outside the reach of the revert. A carrier that reaches only
/// some of the scopes the walk enters fails here without any test naming shadow DOM.
#[test]
fn every_baseline_the_walk_records_is_taken_under_the_user_agent_origin() {
    let unreached = measured(
        "globalThis.walked.flatMap(element => generatedBoxTests() \
         .flatMap(([name]) => Object.values(pseudoBaselineOf(element, name)) \
         .filter(value => value.includes(\"authored\")).map(() => element.name + name))).sort()",
    );
    assert_eq!(
        unreached,
        serde_json::json!([]),
        "a baseline taken outside the revert is the live value, so everything on it is pruned"
    );
}

/// The filed case, pinned to a value rather than to an absence: the box inside a shadow root
/// records the same declarations its light-DOM twin does.
#[test]
fn a_box_inside_a_shadow_root_records_what_its_light_dom_twin_records() {
    let seen =
        measured("[recreatePseudos(marked)[\"::before\"], recreatePseudos(shallow)[\"::before\"]]");
    assert_eq!(
        seen,
        serde_json::json!([
            { "content": "\"x\"", "style": { "color": "color=pseudo-authored:P#marked::before" } },
            { "content": "\"x\"", "style": { "color": "color=pseudo-authored:P#shallow::before" } }
        ])
    );
}

/// A root inside a root. Delivery to the roots the document hosts would close the filed scene
/// and leave every component built out of components measured against nothing.
#[test]
fn a_box_inside_a_nested_shadow_root_is_measured_the_same_way() {
    let seen = measured("recreatePseudos(nested)[\"::before\"].style");
    assert_eq!(
        seen,
        serde_json::json!({ "color": "color=pseudo-authored:P#nested::before" })
    );
}

/// A box carrying no content is kept by its declarations alone, so an unreached baseline does
/// not merely strip it - the box disappears, and no blocker is reported because the live read
/// was not empty. Its light-DOM twin proves the loss is the scope and not the box.
#[test]
fn a_box_that_carries_no_content_survives_inside_a_shadow_root() {
    let seen =
        measured("[Object.keys(recreatePseudos(marked)), Object.keys(recreatePseudos(nested))]");
    assert_eq!(
        seen,
        serde_json::json!([["::before", "::marker"], ["::before", "::marker"]])
    );
}

/// Reaching further must not keep anything that was correctly pruned. A box the page authored
/// nothing for reads its user-agent value in every scope, so it reduces to nothing in every
/// scope, and the repair is a change of reach rather than of what counts as authored.
#[test]
fn a_box_the_page_authored_nothing_for_is_still_pruned_in_every_scope() {
    let seen = evaluate(
        &format!("{SCOPES}\nglobalThis.pseudoAuthored.clear();\nread();"),
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\n[recreatePseudos(marked), recreatePseudos(nested)]')",
    );
    assert_eq!(
        seen,
        serde_json::json!([
            { "::before": { "content": "\"x\"", "style": {} } },
            { "::before": { "content": "\"x\"", "style": {} } }
        ])
    );
}

/// The reach is not bought by declaring the rule again per scope. One constructed sheet serves
/// every scope, so a page of any shape costs the same single parse it cost when it reached one.
#[test]
fn reaching_every_scope_costs_one_declaration_of_the_rule() {
    let seen = evaluate(
        &format!("{SCOPES}\nread();"),
        "[globalThis.sheets, globalThis.pseudoMeasured.length]",
    );
    assert_eq!(seen, serde_json::json!([1, 7]));
}
