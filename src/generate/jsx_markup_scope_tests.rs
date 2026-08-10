use super::{attribute_values, root_attribute};

/// The defect `root_attribute` exists to close. An attribute belongs to the element that
/// declares it, but the reader was handed the whole serialized subtree and took the first
/// match in document order. First-in-document-order and belongs-to-the-root coincide
/// exactly while the root declares the name, so the divergence only shows when it does not
/// — which is the ordinary shape of an icon, sized by CSS with only a `viewBox`.
#[test]
fn does_not_read_an_attribute_the_root_never_declared_from_a_descendant() {
    let svg = concat!(
        r#"<svg height={"120"} viewBox={"0 0 240 120"}>"#,
        r#"<rect width={"20"} height={"40"} /></svg>"#,
    );
    assert_eq!(root_attribute(svg, "height").as_deref(), Some("120"));
    assert_eq!(
        root_attribute(svg, "width"),
        None,
        "harvested the width of a child shape onto the element that has none"
    );
}

/// The mirror arm. Swapping which dimension the root declares must swap which one is
/// readable, so no name can be special-cased into or out of the rule.
#[test]
fn reads_whichever_dimension_the_root_itself_declares() {
    let svg = concat!(
        r#"<svg width={"240"} viewBox={"0 0 240 120"}>"#,
        r#"<rect width={"20"} height={"40"} /></svg>"#,
    );
    assert_eq!(root_attribute(svg, "width").as_deref(), Some("240"));
    assert_eq!(root_attribute(svg, "height"), None);
}

/// `className` and `aria-hidden` share the reader, so a dimension-only fix would still let
/// a decorative child's class decide which rules style the whole graphic, and its hidden
/// flag remove the graphic from the accessibility tree with no visual tell.
#[test]
fn does_not_read_meaning_carrying_attributes_from_a_descendant() {
    let svg = concat!(
        r#"<svg viewBox={"0 0 24 24"}>"#,
        r#"<path className={"r_decor"} aria-hidden={"true"} /></svg>"#,
    );
    assert_eq!(root_attribute(svg, "className"), None);
    assert_eq!(root_attribute(svg, "aria-hidden"), None);
}

/// Names match as whole names, not as suffixes. A `<marker>` legitimately carries
/// `markerWidth`, and a reader that recognised an attribute by the tail of its name would
/// read one as the other.
#[test]
fn does_not_confuse_an_attribute_whose_name_ends_with_the_sought_one() {
    let svg = r#"<svg markerWidth={"6"} viewBox={"0 0 24 24"}><path /></svg>"#;
    assert_eq!(root_attribute(svg, "width"), None);
    assert_eq!(root_attribute(svg, "markerWidth").as_deref(), Some("6"));
}

/// The bound is the root's own start tag, and a start tag ends at its first *unquoted*
/// `>`. Searching for the first `>` instead would cut the tag short here and start
/// silently dropping the root's real attributes — a wrong value traded for a missing one.
#[test]
fn ends_the_root_tag_at_an_unquoted_bracket_only() {
    let svg = r#"<svg data-label={"a > b"} height={"120"}><rect width={"20"} /></svg>"#;
    assert_eq!(root_attribute(svg, "height").as_deref(), Some("120"));
    assert_eq!(root_attribute(svg, "width"), None);
}

/// Leading text before the root must not be mistaken for it, and once the root tag closes
/// no later sibling may reopen the window.
#[test]
fn reads_only_the_first_element_even_when_a_sibling_follows() {
    let svg = r#" <svg height={"120"}><rect /></svg><svg width={"9"}></svg>"#;
    assert_eq!(root_attribute(svg, "height").as_deref(), Some("120"));
    assert_eq!(root_attribute(svg, "width"), None);
}

/// `classes` genuinely wants every class in the subtree, because the asset's stylesheet is
/// carved from what the whole relocated graphic references. The two accessors answer
/// different questions and both must keep answering their own.
#[test]
fn still_reports_every_value_in_the_subtree_when_the_subtree_is_the_question() {
    let svg = concat!(
        r#"<svg className={"r_root"}>"#,
        r#"<path className={"r_arc"} /></svg>"#,
    );
    assert_eq!(
        attribute_values(svg, "className"),
        vec!["r_root".to_string(), "r_arc".to_string()]
    );
}
