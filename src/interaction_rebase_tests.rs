use crate::model::PageState;
use crate::node_alignment_tests::{PARENT, paragraph, state};

fn text_at(page: &PageState, ordinal: usize) -> Option<&str> {
    let path = format!("{PARENT}>p:nth-of-type({ordinal})>#text(1)");
    page.nodes
        .iter()
        .find(|node| node.path == path)
        .map(|node| node.text.as_str())
}

/// `unchanged` exists so a value the interaction did not touch is reported with its
/// baseline value, which lets a later diff against the baseline stay silent about it.
/// Rebasing must therefore pair a state node with the baseline node it actually is.
/// A path is a chain of `:nth-of-type()` ordinals, so once an earlier sibling is removed
/// every following same-tag sibling shifts up one and the path names a different element.
/// Pairing by raw path writes the departed sibling's text over its successor, which is
/// the same aliasing the alignment exists to prevent.
#[test]
fn keeps_the_survivor_text_when_an_earlier_sibling_was_removed() {
    let baseline = state(vec![
        paragraph(1, "ALPHA", ""),
        paragraph(2, "BRAVO", ""),
        paragraph(3, "CHARLIE", ""),
    ]);
    // The second activation is idempotent, so this state's DOM equals the previous one.
    let previous = state(vec![paragraph(1, "ALPHA", ""), paragraph(2, "CHARLIE", "")]);
    let mut current = previous.clone();

    crate::interaction_rebase::unchanged(&mut current, &previous, &baseline);

    assert_eq!(text_at(&current, 1), Some("ALPHA"), "{:?}", current.nodes);
    assert_eq!(
        text_at(&current, 2),
        Some("CHARLIE"),
        "the surviving sibling must keep its own text, not inherit the departed one's"
    );
}

/// The rebase must still do its job where the paths do line up: a value equal to the
/// previous state's value is reported with the baseline's, so the diff stays silent.
#[test]
fn restores_the_baseline_value_for_an_untouched_sibling() {
    let baseline = state(vec![paragraph(1, "ALPHA", "idle")]);
    let previous = state(vec![paragraph(1, "ALPHA", "busy")]);
    let mut current = previous.clone();

    crate::interaction_rebase::unchanged(&mut current, &previous, &baseline);

    let path = format!("{PARENT}>p:nth-of-type(1)");
    let node = current
        .nodes
        .iter()
        .find(|node| node.path == path)
        .expect("subject");
    assert_eq!(
        node.attributes.get("class").map(String::as_str),
        Some("idle")
    );
}

/// A value this interaction did change must survive the rebase untouched, or the edit
/// it represents would be erased before anything could emit it.
#[test]
fn preserves_a_value_this_interaction_changed() {
    let baseline = state(vec![paragraph(1, "ALPHA", "idle")]);
    let previous = baseline.clone();
    let mut current = state(vec![paragraph(1, "ALPHA", "active")]);

    crate::interaction_rebase::unchanged(&mut current, &previous, &baseline);

    let path = format!("{PARENT}>p:nth-of-type(1)");
    let node = current
        .nodes
        .iter()
        .find(|node| node.path == path)
        .expect("subject");
    assert_eq!(
        node.attributes.get("class").map(String::as_str),
        Some("active")
    );
}
