//! The inverse of a reversion: a key the output never declared cannot be withdrawn.
//! Every reset here would render identically and is therefore pure artifact, which the
//! tool is not allowed to emit.
use super::{band_rule, band_rule_with, box_node};

/// A reset cancels a declaration. A sampled measurement is never emitted as one - the
/// base rule strips it so the recreation reflows instead of freezing one viewport's
/// pixels - so resetting it cancels nothing and is inert output in every band.
#[test]
fn never_resets_a_measurement_the_base_rule_refused_to_declare() {
    let base = box_node("html>body>div", &[("width", "320px"), ("color", "red")]);
    let narrow = box_node("html>body>div", &[("color", "red")]);
    let css = band_rule(&base, &narrow);
    assert!(
        !css.contains("width:revert"),
        "no rule declares the sampled width, so there is nothing to revert: {css}"
    );
}

/// A replaced element's reserved box is a measurement the emitter keeps so the page does
/// not shift as the resource loads. Reserving it is not the page declaring a size, so
/// there is still nothing for a band to withdraw.
#[test]
fn never_resets_the_box_reserved_for_a_replaced_element() {
    let mut base = box_node("html>body>img", &[("width", "80px"), ("height", "40px")]);
    base.tag = "img".into();
    let mut narrow = box_node("html>body>img", &[]);
    narrow.tag = "img".into();
    let css = band_rule(&base, &narrow);
    assert!(!css.contains("revert"), "{css}");
}

/// A reset is applied after every normalizer, so no later stage can put the value it
/// withdraws back. The authored stage substitutes its own spelling for any key it owns,
/// and would restore the base value if it saw the reset.
#[test]
fn keeps_the_reset_that_a_later_stage_would_overwrite() {
    let rules = ["div{border-bottom-width:4px;}".to_string()];
    let base = box_node(
        "html>body>div",
        &[("border-bottom-width", "4px"), ("color", "red")],
    );
    let narrow = box_node("html>body>div", &[("color", "red")]);
    let css = band_rule_with(&base, &narrow, &rules);
    assert!(
        css.contains("border-bottom-width:revert"),
        "the authored width must not be substituted back over the reset: {css}"
    );
}

/// An authored size is re-emitted from the author's own spelling, so it already says the
/// right thing at every width the band covers. Withdrawing it would hand the box to the
/// flow at the narrow edge of the band, where the source still states a width.
#[test]
fn never_resets_a_size_the_source_authored() {
    let rules = ["div{width:320px;}".to_string()];
    let base = box_node("html>body>div", &[("width", "320px"), ("color", "red")]);
    let narrow = box_node("html>body>div", &[("color", "red")]);
    let css = band_rule_with(&base, &narrow, &rules);
    assert!(!css.contains("width:revert"), "{css}");
}
