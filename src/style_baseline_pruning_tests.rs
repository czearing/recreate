//! A baseline reading that no consumer can reach is work the capture pays for and throws away.
//! The probe enumerates every element four times - live, `::before`, `::after` and the reverted
//! element - but a pseudo-element is only ever recorded when it generates a box, and the live
//! enumeration is the same one every consumer then repeats. These tests pin both prunings to the
//! condition that makes them safe rather than to a count: a pseudo baseline is skipped exactly
//! when the value the consumer tests says it would be discarded, and the handed-over live map is
//! the one the probe took, so a pruning that reached further would show up as a missing or wrong
//! recorded value rather than as a fast test.

use crate::style_baseline_double::evaluate;

/// The defect: every element paid for two full pseudo-element enumerations under a revert sheet
/// even though the recording is discarded unless the pseudo generates content. Almost no element
/// on a page does, so this is where the enumeration count lives.
#[test]
fn measures_no_pseudo_baseline_when_nothing_generates_content() {
    let seen = evaluate("read();", "[globalThis.pseudoMeasured, globalThis.sheets]");
    assert_eq!(seen, serde_json::json!([[], 0]));
}

/// The consumer asks for one pseudo name at a time and only after testing that name's content,
/// so measuring the sibling name records a baseline nothing can read.
#[test]
fn measures_only_the_pseudo_name_that_generates_content() {
    let seen = evaluate(
        "globalThis.content.set('P#marked::after', '\"x\"'); read();",
        "[globalThis.pseudoMeasured, globalThis.sheets]",
    );
    assert_eq!(seen, serde_json::json!([["P#marked::after"], 1]));
}

/// Pruning must not lose the reading. A pseudo that does generate content still needs a baseline
/// measured under the revert sheet, or every one of its declarations would be published as
/// authored when the user-agent supplied it.
#[test]
fn still_records_the_baseline_of_a_pseudo_that_generates_content() {
    let value = evaluate(
        "globalThis.content.set('P#marked::before', 'counter(step)'); read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\npseudoBaselineOf(marked, \"::before\")')",
    );
    assert_eq!(
        value,
        serde_json::json!({ "color": "color=pseudo:P#marked::before" })
    );
}

/// The reading this pruning is allowed to skip is decided by the value the consumer tests, so it
/// has to be taken from the same page the consumer sees: the restored one, after every style
/// attribute is back. Reverting an element drops its `animation` and `transition` declarations
/// and restoring them starts both over, so a value read before the pass describes a page the
/// capture never publishes. This is the defect that a byte comparison of the corpus caught while
/// a timing budget passed.
#[test]
fn tests_generated_content_only_after_the_page_is_restored() {
    let order = evaluate(
        "globalThis.content.set('P#marked::before', '\"x\"'); read();",
        "[globalThis.pseudoMeasured, globalThis.order.slice(-2)]",
    );
    assert_eq!(
        order,
        serde_json::json!([["P#marked::before"], ["restore", "pseudo"]])
    );
}

/// `all` does not reach custom properties, so a reverted element reports the same ones it reports
/// live and every comparison against the baseline already discards them. Enumerating them is the
/// largest variable-sized part of the read, because a design system declares its whole palette on
/// one inherited root.
#[test]
fn enumerates_no_custom_property_in_the_baseline() {
    let value = evaluate(
        "read();",
        "eval(SCRIPT + '\\nmeasureBaselines(documentElement, () => false);\
         \\nObject.keys(baselineOf(marked))')",
    );
    assert_eq!(value, serde_json::json!(["color"]));
}

/// A component framework declares `style` as a class field, which installs an own property over
/// the accessor `HTMLElement.prototype` supplies. Every reverted reading below is taken from an
/// element that has done exactly that, so the probe reaching through the instance for the
/// declaration block would end this run with a `TypeError` rather than a wrong value — and
/// because the probe is evaluated as one expression whose rejection is the capture's result, a
/// real page would produce no artifact at all. The baselines still arriving is the whole claim.
///
/// `plain` carries an authored `all: unset` of its own, which the measurement has to displace.
/// Writing the block through the attribute replaces it rather than merging into it, so this
/// pins that an element the author already styled inline is still measured under the
/// user-agent origin.
#[test]
fn measures_an_element_that_shadowed_the_style_accessor() {
    let value = evaluate(
        "globalThis.baseline = read('baselineOf(marked).color');",
        "[globalThis.measured, globalThis.baseline]",
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["HTML", "HEAD", "BODY", "P", "P#marked"],
            "color=revert:P#marked"
        ])
    );
}

/// Inheritance is one-way, so a level is reverted only after every level above it was measured
/// and put back. A pruning that reordered the walk would let a child inherit a reverted parent.
#[test]
fn reverts_parents_before_children_and_restores_every_element() {
    let value = evaluate(
        "read();",
        "[globalThis.measured, [documentElement, body, marked].map(node => node.reverted)]",
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["HTML", "HEAD", "BODY", "P", "P#marked"],
            [false, false, false]
        ])
    );
}
