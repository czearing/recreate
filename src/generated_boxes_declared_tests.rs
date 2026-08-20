//! What a capture declares about a box it discovered but could not describe.
//!
//! A sub-module of `generated_boxes` so it reuses the same scripted document.

use super::evaluate;

/// A pseudo-element the page authored and the engine then declined to describe is a real loss,
/// and the filing's complaint was as much that nothing said so as that it happened. An empty
/// declaration block is the answer for a name the engine does not implement and for one it
/// keeps inside its own shadow tree, so the name is reported rather than either guessed at or
/// left to be inferred from an absence.
#[test]
fn declares_a_pseudo_element_the_engine_refused_to_describe() {
    let value = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker'];\
         globalThis.unsupported.add('::marker'); read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nrecreatePseudos(marked); recreatePseudoBlockers()')",
    );
    assert_eq!(
        value,
        serde_json::json!([
            "the engine reported no style for ::marker; authored rules for those \
             pseudo-elements are missing"
        ])
    );
}

/// Nothing is declared for a page that lost nothing. A blocker raised whenever a box happened
/// to match its baseline would fire on every page and tell a reader nothing.
#[test]
fn declares_nothing_when_the_engine_described_every_box() {
    let value = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker'];\
         globalThis.content.set('P#marked::marker', '\"x\"'); read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nrecreatePseudos(marked); recreatePseudoBlockers()')",
    );
    assert_eq!(value, serde_json::json!([]));
}

/// The reading has to survive, not merely be taken: a discovered box gets a baseline measured
/// under the revert sheet, exactly as a named one does, or every declaration the user agent
/// supplied it would be published as authored.
#[test]
fn records_the_discovered_box_against_its_own_baseline() {
    let value = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker']; read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\npseudoBaselineOf(marked, \"::marker\")')",
    );
    assert_eq!(
        value,
        serde_json::json!({ "color": "color=pseudo:P#marked::marker" })
    );
}
