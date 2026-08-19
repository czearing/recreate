//! What a capture records about an element the engine was painting from the top layer.
//!
//! Stated over the promotion rather than over dialogs: the record must say that the page had
//! this element in the top layer, and say which call put it there, whatever the element is.
//! A test named for dialogs cannot fail for a popover, which is how the defect survived.

use crate::node_eval;

/// A page whose top-layer membership is only what the test says it is, answered the way the
/// engine answers it — one pseudo-class per way in, with a popover deliberately outside
/// `:modal` because the page behind a popover stays interactive.
fn reason(matching: &[&str]) -> serde_json::Value {
    let set = matching
        .iter()
        .map(|selector| format!("'{selector}'"))
        .collect::<Vec<_>>()
        .join(",");
    node_eval::evaluate(
        &format!(
            "{}\nconst element = {{ matches: selector => [{set}].includes(selector) }};",
            super::SOURCE
        ),
        "recreateTopLayer(element)",
    )
}

/// The defect. An open popover is in the top layer and is never `:modal`, so a record built
/// from inertness says it was an ordinary in-flow box — and an element the page painted at
/// 300x82 then serialises identically to one that was never invoked.
#[test]
fn an_open_popover_is_recorded_as_promoted() {
    assert_eq!(reason(&[":popover-open"]), serde_json::json!("popover"));
}

/// The control the repair must not disturb: a modal dialog keeps answering `modal`, so it
/// keeps being replayed with `showModal()`.
#[test]
fn a_modal_dialog_still_records_the_call_that_opened_it() {
    assert_eq!(reason(&[":modal"]), serde_json::json!("modal"));
}

/// The clause that forces a reason rather than a wider predicate. `:modal` is true of the
/// fullscreen element as well, so an answer that stopped at "in the top layer" would report
/// fullscreen as something the recreation may call `showModal()` on.
#[test]
fn a_fullscreen_element_is_never_reported_as_a_dialog_or_a_popover() {
    assert_eq!(
        reason(&[":fullscreen", ":modal"]),
        serde_json::json!("fullscreen")
    );
}

/// An element the page never promoted says so, and says it as the absence of a reason rather
/// than as a reason of its own.
#[test]
fn an_ordinary_element_carries_no_promotion() {
    assert_eq!(reason(&[]), serde_json::json!(""));
}

/// An engine that does not know one of the selectors has still promoted nothing for that
/// reason, so the remaining ways in are still asked. Throwing would cost the page every
/// promotion, including the ones the engine does implement.
#[test]
fn a_selector_the_engine_refuses_does_not_cost_the_other_promotions() {
    let answer = node_eval::evaluate(
        &format!(
            "{}\nconst element = {{ matches: selector => {{ \
             if (selector === ':popover-open') throw new SyntaxError('unknown'); \
             return selector === ':modal'; }} }};",
            super::SOURCE
        ),
        "recreateTopLayer(element)",
    );
    assert_eq!(answer, serde_json::json!("modal"));
}

/// Which call the recreation makes to put the element back. Fullscreen has none — it needs
/// transient user activation a recreation rendering itself does not have — and that is the
/// difference a boolean cannot carry: it must be recordable without being replayable.
#[test]
fn each_promotion_replays_as_its_own_call_and_fullscreen_as_none() {
    use super::Promotion;

    let replay = |reason: &str| Promotion(reason.into()).replay();

    assert_eq!(replay("modal"), Some("showModal"));
    assert_eq!(replay("popover"), Some("showPopover"));
    assert_eq!(replay("fullscreen"), None);
    assert_eq!(replay(""), None);
    assert!(Promotion("popover".into()).promoted());
    assert!(!Promotion(String::new()).promoted());
}

/// Both node-record producers and the generated-box gate ask one question, so a promotion one
/// of them can see is one all of them can. The gate is the reason this matters beyond
/// bookkeeping: `::backdrop` is generated for a top-layer element whatever promoted it, so a
/// gate asking about inertness never looks for a scrim the engine really generated.
#[test]
fn every_reader_of_a_promotion_asks_the_same_question() {
    let capture = crate::page_script::source_without_assets();
    let interaction = crate::interaction_script::source();

    for source in [&capture, &interaction] {
        assert!(
            source.contains("promotion: recreateTopLayer(element)"),
            "a node-record producer asks the engine for the promotion's reason"
        );
        assert!(
            !source.contains("modal: element.matches(':modal')"),
            "no producer records inertness in place of the promotion"
        );
        // The attribute-shaped substitutes, each silent on the promotion: `open` is set by
        // `show()` as well as `showModal()`, and `aria-modal` is an authored claim.
        assert!(!source.contains("promotion: element.open"));
        assert!(!source.contains("aria-modal"));
    }
    assert!(
        crate::generated_boxes::SOURCE
            .contains("'::backdrop': element => recreateTopLayer(element)"),
        "the scrim exists for the top layer, not for the elements that inert the page"
    );
}
