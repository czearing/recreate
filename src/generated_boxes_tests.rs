//! Which generated boxes a capture looks for, and on which elements.
//!
//! The engine generates a handful of boxes on its own terms and those can be named in the
//! source. Every other pseudo-element exists because an author wrote a rule for one, so a
//! capture that names those too is a capture that reproduces exactly the pseudo-elements
//! someone thought to list — which is how `::marker`, `::placeholder` and `::selection` came
//! to be captured into `css_rules[]` and dropped before the recreation. These tests pin the
//! admission to the authored rule's *form* rather than to its name, and pin the cost of doing
//! so to the authored rules rather than to the size of the page.

use crate::style_baseline_double::evaluate;

/// The defect. `::marker` is named nowhere in the tool's source and no engine rule generates
/// it on the tool's terms, so before this it was invisible however plainly the page asked for
/// one. The subject decides who gets probed, which is also what a bare name could never say.
#[test]
fn measures_a_pseudo_element_the_source_never_names() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker']; read();",
        "globalThis.pseudoMeasured",
    );
    assert_eq!(seen, serde_json::json!(["P#marked::marker"]));
}

/// Admission is by the `::` form, so a pseudo-element invented after this code was written
/// reaches the recreation without an edit here. `::first-line` stands in for that: it is not a
/// generated-content box, so no `content` test could ever have found it.
#[test]
fn admits_by_form_rather_than_by_an_enumeration_of_names() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked::first-line', '#marked::spelling-error'];\
         read();",
        "globalThis.pseudoMeasured",
    );
    assert_eq!(
        seen,
        serde_json::json!(["P#marked::first-line", "P#marked::spelling-error"])
    );
}

/// The subject is what keeps this affordable. Every element the page has would otherwise pay a
/// style resolution and a whole property enumeration for every name any rule anywhere
/// mentioned, which on a page authoring a handful of them is the whole run budget. The ten
/// reads below are the `content` probes the two engine-generated names already cost on this
/// five-element document; the authored name adds exactly one, for the one element it selects.
#[test]
fn probes_only_the_elements_the_authoring_rule_selects() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker']; read();",
        "[globalThis.pseudoMeasured.length, globalThis.pseudoReads]",
    );
    assert_eq!(seen, serde_json::json!([1, 11]));
}

/// A page authoring no pseudo-element rule pays nothing for the ability to find them, which is
/// the property that lets the search be unbounded in what it will admit: no baseline measured,
/// no revert sheet inserted, and not one read beyond the ten the engine-generated names cost
/// before any of this existed.
#[test]
fn costs_nothing_on_a_page_that_authors_no_pseudo_element_rule() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked', '.stage p']; read();",
        "[globalThis.pseudoMeasured, globalThis.sheets, globalThis.pseudoReads]",
    );
    assert_eq!(seen, serde_json::json!([[], 0, 10]));
}

/// A rule with no subject conditions the whole document rather than any element in it, so
/// there is no element whose record could carry it and nothing here may guess one. Admitting it
/// against the universal selector would put a copy of it on every element of the page.
#[test]
fn declines_a_rule_that_names_no_subject() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['::selection']; read();",
        "[globalThis.pseudoMeasured, globalThis.sheets]",
    );
    assert_eq!(seen, serde_json::json!([[], 0]));
}

/// A state the element is not in now cannot be matched now, but the box the rule describes is
/// still one this element has. Stripping the state keeps the subject matchable at rest instead
/// of losing the box to a selector that cannot fire.
#[test]
fn matches_a_subject_written_behind_a_dynamic_state() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked:hover::marker']; read();",
        "globalThis.pseudoMeasured",
    );
    assert_eq!(seen, serde_json::json!(["P#marked::marker"]));
}

/// The names the engine generates on its own terms keep their own conditions. A page authoring
/// `::before` must not start receiving one on every element it selects regardless of whether
/// the engine generated content there, which is what a subject match would say.
#[test]
fn leaves_an_engine_generated_box_to_its_own_condition() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['#marked::before']; read();",
        "globalThis.pseudoMeasured",
    );
    assert_eq!(seen, serde_json::json!([]));
}

/// A subject is cut out of authored selector text, where a comma inside `:is()` or an
/// attribute value splits into a fragment that parses as neither. Testing the subjects one at
/// a time is what stops one such fragment throwing away every other element the name was
/// authored for — the same reason the revert sheet emits one rule per name.
#[test]
fn keeps_a_subject_that_parses_beside_one_that_does_not() {
    let seen = evaluate(
        "globalThis.authoredSelectors = ['((::marker', '#marked::marker']; read();",
        "globalThis.pseudoMeasured",
    );
    assert_eq!(seen, serde_json::json!(["P#marked::marker"]));
}

/// A box that reduces to nothing emits no declaration, so recording it can only make two
/// elements the output cannot tell apart differ in the record the generated class is keyed on.
/// An engine answering a lookup for a pseudo-element it does not implement returns exactly
/// this, so leaving it in would split a class for every such name a page happened to mention.
#[test]
fn records_no_box_for_a_pseudo_element_that_reduces_to_nothing() {
    let value = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker']; read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nrecreatePseudos(marked)')",
    );
    assert_eq!(value, serde_json::json!({}));
}

/// Suppression must not reach a box that does say something. The reduction runs first, so a
/// discovered box whose style survives it is recorded exactly as a named one is.
#[test]
fn still_records_a_discovered_box_whose_style_survives_reduction() {
    let value = evaluate(
        "globalThis.authoredSelectors = ['#marked::marker'];\
         globalThis.content.set('P#marked::marker', '\"x\"'); read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nrecreatePseudos(marked)[\"::marker\"].content')",
    );
    assert_eq!(value, serde_json::json!("\"x\""));
}

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
