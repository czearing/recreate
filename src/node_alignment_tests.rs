use crate::model::{Attributes, Node, PageState, Rect};
use crate::node_alignment::{self as node_alignment, Insertion};

/// Queries the one alignment the pipeline builds per state, keeping each test a single
/// call against the behaviour under test.
pub(super) fn insertion_of(
    path: &str,
    state: &PageState,
    baseline: &PageState,
) -> Option<Insertion> {
    let alignment = node_alignment::of(state, baseline);
    let insertion = alignment.insertion(path)?;
    Some(Insertion {
        before: insertion.before.clone(),
        displaced: insertion.displaced.clone(),
    })
}

pub(super) const PARENT: &str = "html>body:nth-of-type(1)>div:nth-of-type(1)";

fn node(path: &str, parent: Option<&str>, tag: &str, text: &str, class: &str) -> Node {
    let mut attributes = Attributes::default();
    if !class.is_empty() {
        attributes.insert("class".into(), class.into());
    }
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: tag.into(),
        text: text.into(),
        attributes,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
        style: Default::default(),
        ..Default::default()
    }
}

/// An element with one text child, as the capture records it.
fn element(tag: &str, ordinal: usize, text: &str, class: &str) -> Vec<Node> {
    let path = format!("{PARENT}>{tag}:nth-of-type({ordinal})");
    let child = node(&format!("{path}>#text(1)"), Some(&path), "#text", text, "");
    vec![node(&path, Some(PARENT), tag, "", class), child]
}

pub(super) fn paragraph(ordinal: usize, text: &str, class: &str) -> Vec<Node> {
    element("p", ordinal, text, class)
}

pub(super) fn division(ordinal: usize, text: &str, class: &str) -> Vec<Node> {
    element("div", ordinal, text, class)
}

/// A bare text child, as a parent that mixes text with markup records it. A text node has
/// no children of its own, so its own value is the only thing that can identify it.
pub(super) fn text_child(ordinal: usize, value: &str) -> Vec<Node> {
    vec![node(
        &format!("{PARENT}>#text({ordinal})"),
        Some(PARENT),
        "#text",
        value,
        "",
    )]
}

/// A capture is rooted at `html` and every node names its parent, so the alignment can
/// walk down from a root whose identity is not in question.
pub(super) fn state(children: Vec<Vec<Node>>) -> PageState {
    let mut nodes = vec![
        node("html", None, "html", "", ""),
        node("html>body:nth-of-type(1)", Some("html"), "body", "", ""),
        node(PARENT, Some("html>body:nth-of-type(1)"), "div", "", "body"),
    ];
    nodes.extend(children.into_iter().flatten());
    PageState {
        nodes,
        ..Default::default()
    }
}

/// The filed defect. A `div` inserted ahead of a `p` moves no same-tag ordinal, so the
/// tag-scoped probe saw no displacement and emitted no anchor.
#[test]
fn anchors_an_insertion_whose_tag_no_sibling_shares() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let subject = state(vec![
        division(1, "Note", "note"),
        paragraph(1, "Existing body text.", ""),
    ]);
    let inserted = format!("{PARENT}>div:nth-of-type(1)");

    let insertion = insertion_of(&inserted, &subject, &baseline).expect("div is an insertion");

    assert_eq!(
        insertion.before.as_deref(),
        Some(format!("{PARENT}>p:nth-of-type(1)").as_str())
    );
}

/// The same-tag case, which the filed item expected to already work. It does not: the
/// insertion takes the displaced element's path, so the probe looked one slot past the end.
#[test]
fn anchors_an_insertion_that_took_the_displaced_element_s_path() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let control = state(vec![
        paragraph(1, "Note", "note"),
        paragraph(2, "Existing body text.", ""),
    ]);
    let inserted = format!("{PARENT}>p:nth-of-type(1)");

    let insertion = insertion_of(&inserted, &control, &baseline).expect("note is an insertion");

    assert_eq!(
        insertion.before.as_deref(),
        Some(format!("{PARENT}>p:nth-of-type(1)").as_str()),
        "anchor must be the survivor's BASELINE path"
    );
    assert_eq!(
        insertion.displaced.as_deref(),
        Some(format!("{PARENT}>p:nth-of-type(2)").as_str()),
        "the survivor now lives one slot down"
    );
}

/// The survivor is not the insertion. Asking about the displaced element must not report it
/// as inserted, or the overlay would render it twice.
#[test]
fn does_not_report_the_displaced_survivor_as_an_insertion() {
    let baseline = state(vec![paragraph(1, "Existing body text.", "")]);
    let control = state(vec![
        paragraph(1, "Note", "note"),
        paragraph(2, "Existing body text.", ""),
    ]);

    let survivor = format!("{PARENT}>p:nth-of-type(2)");

    assert!(insertion_of(&survivor, &control, &baseline).is_none());
}
