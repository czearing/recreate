//! The four regression cases the filed item required alongside the fix. They share the
//! fixtures of the primary suite because they exercise the same predicate.
use crate::node_alignment_tests::{division, insertion_of, paragraph, state, PARENT};

/// Regression 1: inserted between two unlike siblings, not at the front.
#[test]
fn anchors_an_insertion_between_two_unlike_siblings() {
    let baseline = state(vec![
        paragraph(1, "First.", ""),
        division(1, "Last.", "tail"),
    ]);
    let subject = state(vec![
        paragraph(1, "First.", ""),
        division(1, "Note", "note"),
        division(2, "Last.", "tail"),
    ]);
    let inserted = format!("{PARENT}>div:nth-of-type(1)");

    let insertion = insertion_of(&inserted, &subject, &baseline).expect("middle insertion");

    assert_eq!(
        insertion.before.as_deref(),
        Some(format!("{PARENT}>div:nth-of-type(1)").as_str())
    );
}

/// Regression 2: the same interaction annotates the sibling it displaced. Whole-map
/// attribute equality fails, but the sibling is still a survivor, not a second insertion.
#[test]
fn anchors_an_insertion_whose_displaced_sibling_was_also_annotated() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let mut subject = state(vec![
        division(1, "Note", "note"),
        paragraph(1, "Existing body text.", ""),
    ]);
    let annotated = format!("{PARENT}>p:nth-of-type(1)");
    subject
        .nodes
        .iter_mut()
        .find(|node| node.path == annotated)
        .expect("displaced sibling")
        .attributes
        .insert("aria-expanded".into(), "true".into());
    let inserted = format!("{PARENT}>div:nth-of-type(1)");

    let insertion = insertion_of(&inserted, &subject, &baseline).expect("annotated-sibling case");

    assert_eq!(insertion.before.as_deref(), Some(annotated.as_str()));
}

/// Regression 3: two elements inserted ahead of the same sibling. Both anchor to it, and
/// `insertBefore` then stacks them in attachment order.
#[test]
fn anchors_both_of_two_elements_inserted_before_one_sibling() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let subject = state(vec![
        division(1, "First note", "note"),
        division(2, "Second note", "note"),
        paragraph(1, "Existing body text.", ""),
    ]);
    let anchor = format!("{PARENT}>p:nth-of-type(1)");

    for ordinal in [1, 2] {
        let inserted = format!("{PARENT}>div:nth-of-type({ordinal})");
        let insertion = insertion_of(&inserted, &subject, &baseline).expect("both are insertions");
        assert_eq!(
            insertion.before.as_deref(),
            Some(anchor.as_str()),
            "{inserted}"
        );
    }
}

/// Regression 4: appending at the end must keep producing no anchor, because there is no
/// survivor after it and appending is what the portal does anyway.
#[test]
fn leaves_an_appended_element_unanchored() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let subject = state(vec![
        paragraph(1, "Existing body text.", ""),
        division(1, "Note", "note"),
    ]);
    let inserted = format!("{PARENT}>div:nth-of-type(1)");

    let insertion = insertion_of(&inserted, &subject, &baseline).expect("append is an insertion");

    assert_eq!(insertion.before, None);
    assert_eq!(insertion.displaced, None);
}

/// An unchanged tree has no insertions at all, so nothing is portalled or excluded.
#[test]
fn reports_no_insertion_when_the_children_are_unchanged() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let same = state(vec![paragraph(1, "Existing body text.", "")]);

    assert!(insertion_of(&format!("{PARENT}>p:nth-of-type(1)"), &same, &baseline).is_none());
}
