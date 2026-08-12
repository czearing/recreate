use super::super::normalize;
use crate::model::{Node, Rect, Styles};

fn node(class: &str, position: &str) -> Node {
    let mut node = Node {
        writing_mode: Default::default(),
        disabled: false,
        rtl: false,
        path: "html>body>div>aside".into(),
        parent: Some("html>body>div".into()),
        tag: "aside".into(),
        text: String::new(),
        attributes: Default::default(),
        rect: Rect {
            x: 648.0,
            y: 0.0,
            width: 360.0,
            height: 24.0,
        },
        style: Styles::new(),
        before: None,
        after: None,
    };
    node.attributes.insert("class".into(), class.into());
    node.style.insert("position".into(), position.into());
    node
}

/// `physical_property` maps only the size logicals, so a lookup for `right` alone reports
/// an authored `inset-inline-end` as unauthored — which reads exactly like having no
/// anchor and leaves the frozen pixel in place, deleting the author's declaration.
#[test]
fn a_logical_inset_is_recognised_as_an_anchor() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "648px".into());
    styles.insert("width".into(), "360px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;inset-inline-end:30%;width:25%;}".into()],
    );
    assert!(!styles.contains_key("left"));
}

/// Under `direction: rtl` the inline-start edge is `right`, so a mapping written for LTR
/// would arbitrate the wrong edge and delete the anchor it was meant to protect.
///
/// The direction is read from the engine's answer rather than from the node's authored
/// declarations, and the box here carries none — which is the ordinary shape, because
/// `direction` is inherited and a real page declares it once at the root. A test that
/// hand-placed `direction` in the style map would assert the mapping is right while
/// saying nothing about whether it ever runs, which is how this defect stayed green.
#[test]
fn the_inline_axis_follows_direction() {
    let mut node = node("marker", "absolute");
    node.rtl = true;
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "648px".into());
    styles.insert("right".into(), "432px".into());
    node.style.extend(styles.clone());
    assert!(!node.style.contains_key("direction"));
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;inset-inline-start:30%;}".into()],
    );
    // `inset-inline-start` is `right` in RTL, so the derived edge is `left`.
    assert!(!styles.contains_key("left"));
    // And the anchor that survives is the author's own percentage, not the pixel sampled
    // beside it. A logical name reaches the physical edge it stands for, so `right` is
    // authored here in every sense that matters; asserting `432px` would be asserting that
    // the authored value was thrown away.
    assert_eq!(styles["right"], "30%");
}

/// A shorthand re-states every edge it covers, so removing the longhand while leaving
/// `inset` beside it removes nothing. The media-band rules carry exactly this shape.
#[test]
fn a_shorthand_cannot_smuggle_the_derived_edge_back() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("inset".into(), "0px 432px 48px 648px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;right:30%;top:0;width:25%;}".into()],
    );
    assert!(!styles.contains_key("inset"));
    assert!(!styles.contains_key("left"));
    assert!(!styles.contains_key("bottom"));
    assert_eq!(styles["right"], "30%");
    assert_eq!(styles["top"], "0");
}

/// Splitting on whitespace would tear `calc(100% - 10px)` into three values that mean
/// something else entirely, so a shorthand carrying a function is left intact.
#[test]
fn a_shorthand_containing_a_function_is_left_alone() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("inset".into(), "0px calc(100% - 10px) 0px 5px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;right:30%;}".into()],
    );
    assert_eq!(styles["inset"], "0px calc(100% - 10px) 0px 5px");
}

/// Where the author anchored both edges the cascade is entirely theirs, so there is no
/// derived value to arbitrate and the rule must not touch either side.
#[test]
fn both_edges_authored_are_reproduced_untouched() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "648px".into());
    styles.insert("right".into(), "432px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;left:10%;right:30%;}".into()],
    );
    assert_eq!(styles["left"], "10%");
    assert_eq!(styles["right"], "30%");
}

/// The media-band caller passes a diff of the base styles, and `position` rarely differs
/// between viewports, so it is absent there. Reading the scheme from the diff alone would
/// silently skip every band — which is where the frozen pixel is most visible.
#[test]
fn the_scheme_is_read_from_the_captured_style_not_the_diff() {
    let mut node = node("marker", "absolute");
    node.style.insert("left".into(), "648px".into());
    let mut changed = Styles::new();
    changed.insert("left".into(), "648px".into());
    normalize(
        &mut changed,
        &node,
        &[".marker{position:absolute;right:30%;width:25%;}".into()],
    );
    assert!(!changed.contains_key("left"));
}

/// A `fixed` box is positioned against the viewport and over-constrains identically.
#[test]
fn a_fixed_box_is_arbitrated_too() {
    let mut node = node("marker", "fixed");
    let mut styles = Styles::new();
    styles.insert("position".into(), "fixed".into());
    styles.insert("left".into(), "648px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:fixed;right:30%;}".into()],
    );
    assert!(!styles.contains_key("left"));
}

/// A percentage anchor is the whole point, but a non-px derived value is not a sampled
/// pixel and is left where it is.
#[test]
fn only_a_sampled_pixel_is_removed() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "auto".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;right:30%;}".into()],
    );
    assert_eq!(styles["left"], "auto");
}

/// A logical shorthand anchors both edges of its axis, and it has no single physical name
/// to be renamed to. A resolver that reported only renameable names would answer "not
/// authored" to a query for inset-inline — and the edge the author reset with it, then
/// overrode on one side, would be judged engine-derived and deleted.
#[test]
fn a_logical_shorthand_still_answers_a_query_for_itself() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("left".into(), "648px".into());
    styles.insert("right".into(), "432px".into());
    node.style.extend(styles.clone());
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;inset-inline:0px;left:30%;}".into()],
    );
    assert_eq!(styles["left"], "30%");
    assert_eq!(styles["right"], "432px");
}