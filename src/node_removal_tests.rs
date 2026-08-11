//! Departure, the axis the alignment had no word for. The insertion cases live beside these
//! in `sibling_alignment_regression_tests.rs`; every one of them must keep passing, because
//! a walk that reports a changed element as a removal plus an insertion would remount it and
//! throw away its identity.
use crate::model::PageState;
use crate::node_alignment::{self, Removal};
use crate::node_alignment_tests::{PARENT, division, paragraph, state, text_child};

fn removals(state: &PageState, baseline: &PageState) -> Vec<(String, String)> {
    node_alignment::of(state, baseline)
        .removals()
        .iter()
        .filter(|removal: &&Removal| removal.tag != "#text")
        .map(|removal| (removal.path.clone(), removal.parent.clone()))
        .collect()
}

fn counterpart_text(path: &str, state: &PageState, baseline: &PageState) -> Option<String> {
    let alignment = node_alignment::of(state, baseline);
    let node = alignment.counterpart(path)?;
    baseline
        .nodes
        .iter()
        .find(|child| child.parent.as_deref() == Some(node.path.as_str()))
        .map(|child| child.text.clone())
}

/// The filed defect. Removing the middle of three like siblings renumbers the survivor into
/// its predecessor's path, and with no word for departure the walk paired the survivor to the
/// element that left.
#[test]
fn reports_a_removed_middle_sibling_as_a_departure() {
    let baseline = state(vec![
        paragraph(1, "ALPHA", ""),
        paragraph(2, "BRAVO", ""),
        paragraph(3, "CHARLIE", ""),
    ]);
    let subject = state(vec![paragraph(1, "ALPHA", ""), paragraph(2, "CHARLIE", "")]);

    assert_eq!(
        removals(&subject, &baseline),
        vec![(format!("{PARENT}>p:nth-of-type(2)"), PARENT.to_string())],
        "the element that left is the one whose content is gone, not the last path"
    );
}

/// The other half of the same failure, and the half that fabricates. The survivor must be
/// paired with the element it actually is; pairing it with its predecessor emits an edit
/// rewriting that predecessor's content and leaves the survivor's own copy in place, so the
/// after-state shows the same content twice.
#[test]
fn pairs_the_survivor_with_the_element_it_actually_is() {
    let baseline = state(vec![
        paragraph(1, "ALPHA", ""),
        paragraph(2, "BRAVO", ""),
        paragraph(3, "CHARLIE", ""),
    ]);
    let subject = state(vec![paragraph(1, "ALPHA", ""), paragraph(2, "CHARLIE", "")]);

    assert_eq!(
        counterpart_text(&format!("{PARENT}>p:nth-of-type(2)"), &subject, &baseline).as_deref(),
        Some("CHARLIE"),
        "the survivor is the baseline element with its own content, not the one it displaced"
    );
}

/// The inverse guard. An element whose content merely changed must stay a survivor. Reporting
/// it as a removal plus an insertion would remount it and discard its identity.
#[test]
fn does_not_report_a_changed_element_as_a_departure() {
    let baseline = state(vec![paragraph(1, "ALPHA", ""), paragraph(2, "BRAVO", "")]);
    let subject = state(vec![
        paragraph(1, "ALPHA", ""),
        paragraph(2, "BRAVO EDITED", ""),
    ]);

    assert!(removals(&subject, &baseline).is_empty());
    assert_eq!(
        counterpart_text(&format!("{PARENT}>p:nth-of-type(2)"), &subject, &baseline).as_deref(),
        Some("BRAVO")
    );
}

/// The other inverse guard. An insertion is not a departure, and the walk must still prefer
/// reading a new element as new rather than as a replacement for the one it displaced.
#[test]
fn does_not_report_a_departure_when_an_element_was_inserted() {
    let baseline = state(vec![paragraph(1, "ALPHA", "")]);
    let subject = state(vec![division(1, "NEW", "note"), paragraph(1, "ALPHA", "")]);

    assert!(removals(&subject, &baseline).is_empty());
}

/// A departure must be named even when it is the last child, since nothing follows it to
/// renumber and the walk simply runs out of state children while candidates remain.
#[test]
fn reports_a_removed_last_sibling_as_a_departure() {
    let baseline = state(vec![paragraph(1, "ALPHA", ""), paragraph(2, "BRAVO", "")]);
    let subject = state(vec![paragraph(1, "ALPHA", "")]);

    assert_eq!(
        removals(&subject, &baseline),
        vec![(format!("{PARENT}>p:nth-of-type(2)"), PARENT.to_string())]
    );
}

/// Removing every child of a parent must report each departure once, and must not report the
/// parent itself, which survived.
#[test]
fn reports_each_departure_once_when_the_whole_list_empties() {
    let baseline = state(vec![
        paragraph(1, "ALPHA", ""),
        paragraph(2, "BRAVO", ""),
        paragraph(3, "CHARLIE", ""),
    ]);
    let subject = state(vec![]);

    let mut paths = removals(&subject, &baseline)
        .into_iter()
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(
        paths,
        vec![
            format!("{PARENT}>p:nth-of-type(1)"),
            format!("{PARENT}>p:nth-of-type(2)"),
            format!("{PARENT}>p:nth-of-type(3)"),
        ]
    );
}

/// A text node has no children, so `child_signature` can say nothing about it and its own
/// value is the only identity it has. Removing the middle of three like text siblings is
/// therefore the one case where dropping the text comparison from `same` mis-pairs.
#[test]
fn reports_a_removed_text_sibling_by_the_value_that_left() {
    let baseline = state(vec![
        text_child(1, "ALPHA"),
        text_child(2, "BRAVO"),
        text_child(3, "CHARLIE"),
    ]);
    let subject = state(vec![text_child(1, "ALPHA"), text_child(2, "CHARLIE")]);

    let departed: Vec<_> = node_alignment::of(&subject, &baseline)
        .removals()
        .iter()
        .map(|removal| removal.path.clone())
        .collect();

    assert_eq!(
        departed,
        vec![format!("{PARENT}>#text(2)")],
        "the text that left is BRAVO, not whichever ordinal fell off the end"
    );
}
