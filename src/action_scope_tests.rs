//! What one action is allowed to be blamed for, run under Node against a DOM double.
//!
//! The scope recorder decides which nodes an interaction's state is rebased onto. Reaching a
//! control that sits below the fold means scrolling to it, and that scroll is the harness
//! moving rather than the page reacting — so it must not land in the scope, or every
//! interaction with an offscreen control records the harness's own viewport offset as page
//! geometry. That loss hid for as long as the document element resolved to a malformed path.

use crate::node_eval::evaluate;

const HARNESS: &str = include_str!("action_scope_harness.js");

/// Opens a scope over the button, runs `scene` against it, and returns the paths it collected.
fn scope(scene: &str) -> Vec<String> {
    let preamble = format!(
        "{}\nconst scope = ({})(TRIGGER);\n{scene}",
        crate::node_path::embed(HARNESS),
        crate::interactions::interactions_scripts::action_scope()
    );
    serde_json::from_value(evaluate(
        &preamble,
        "[...globalThis.__recreateCaptureScope.paths]",
    ))
    .unwrap()
}

/// The scroll the harness performs to bring a control into view reaches the scope recorder as
/// an ordinary scroll event on the scrolling element, and it arrives long after the approach
/// returned — a scroll event is delivered asynchronously. What identifies it is that the
/// scroller has not moved since the approach left it, so the approach reports that position
/// and everything still sitting at it is recognised as the harness's own.
#[test]
fn discards_what_the_approach_did_to_reach_the_control() {
    let paths = scope(
        "document.scrollingElement.scrollTop = 640;
         globalThis.__recreateCaptureScope.settle([document.scrollingElement]);
         globalThis.__recreateCaptureScope.scroll({ target: document });",
    );
    assert_eq!(paths, ["html>body:nth-of-type(1)>button:nth-of-type(1)"]);
}

/// The negative control. Without the approach's report the document element is in the scope,
/// which is the state that put the harness's scroll offset into the recorded geometry of every
/// interaction with an offscreen control. It also proves the scroll hook resolves `html` to a
/// real key: while it produced a malformed one, this assertion would have failed silently.
#[test]
fn a_scroll_reaches_the_recorder_as_the_document_element() {
    let paths = scope(
        "document.scrollingElement.scrollTop = 640;
         globalThis.__recreateCaptureScope.scroll({ target: document });",
    );
    assert!(
        paths.contains(&"html".to_string()),
        "the scroll hook did not resolve the scrolling element: {paths:?}"
    );
}

/// A page that scrolls itself in response to the action moves the scroller off the position
/// the approach reported, which is what tells the two apart. Discarding by time instead would
/// lose this, because the harness's own event has not even been delivered yet.
#[test]
fn keeps_a_scroll_the_page_itself_performed_afterwards() {
    let paths = scope(
        "document.scrollingElement.scrollTop = 640;
         globalThis.__recreateCaptureScope.settle([document.scrollingElement]);
         document.scrollingElement.scrollTop = 900;
         globalThis.__recreateCaptureScope.scroll({ target: document });",
    );
    assert!(paths.contains(&"html".to_string()), "{paths:?}");
}

/// Every pass that positions an element does it through the one approach, so none can leave
/// the scope holding its own scrolling. A pass that scrolled on its own would pass every
/// assertion above and still poison the geometry of the interaction it was reaching for.
#[test]
fn every_approach_reports_the_scrollers_it_moved() {
    let approach = crate::interactions_approach::approach_script();
    assert!(
        approach.contains("scrollIntoView"),
        "the approach no longer positions the element"
    );
    assert!(
        approach.contains("__recreateCaptureScope?.settle("),
        "the approach does not report the scrollers it moved"
    );
    let scripts = crate::interactions_approach::positioning_scripts();
    for (name, script) in &scripts {
        assert_eq!(
            script.matches("scrollIntoView").count(),
            script.matches("__recreateCaptureScope?.settle(").count(),
            "{name} scrolls without reporting it"
        );
        assert!(script.contains(approach), "{name} positions on its own");
    }
    assert!(scripts.len() >= 2, "the positioning passes went missing");
}
