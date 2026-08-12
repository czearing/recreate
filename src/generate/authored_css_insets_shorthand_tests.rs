use super::super::normalize;
use super::tests::node;
use crate::model::Styles;
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
/// something else entirely. Counting nesting instead reads it as the one value it is, so a
/// shorthand carrying a function is expanded like any other and the function reaches its
/// edge intact. Declining to expand it was the old way of not tearing it, and it cost the
/// arbiter its view of the edges.
#[test]
fn a_function_inside_a_shorthand_reaches_its_edge_whole() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("inset".into(), "calc(100% - 10px) 5px 0px 20px".into());
    node.style = styles.clone();
    normalize(
        &mut styles,
        &node,
        &[".marker{position:absolute;right:30%;}".into()],
    );
    assert!(!styles.contains_key("inset"));
    // Torn on whitespace this would read as four values starting `calc(100%`, and the top
    // edge would carry a fragment that parses as nothing.
    assert_eq!(styles["top"], "calc(100% - 10px)");
    assert_eq!(styles["bottom"], "0px");
    // The authored longhand already claims its edge and is at least as specific, so the
    // expansion fills the gaps rather than overwriting them.
    assert_eq!(styles["right"], "30%");
    // The author anchored the right edge alone, so the left one is the derived pixel this
    // stage exists to drop.
    assert!(!styles.contains_key("left"));
}

/// A logical axis shorthand covers BOTH of its edges. The arbiter reads the edges to
/// decide which one the engine derived, so an expansion that reached only the first would
/// leave the second sitting under a name no query looks for — invisible to the arbiter and
/// still carrying the shorthand's value on one side only.
///
/// Neither block edge is authored in the rules here, so the arbiter has nothing to
/// suppress and every value in the map is the expansion's own work.
#[test]
fn a_logical_axis_shorthand_reaches_both_of_its_edges() {
    let mut node = node("marker", "absolute");
    let mut styles = Styles::new();
    styles.insert("position".into(), "absolute".into());
    styles.insert("inset-block".into(), "10px 20px".into());
    node.style = styles.clone();
    normalize(&mut styles, &node, &[".marker{position:absolute;}".into()]);
    assert!(!styles.contains_key("inset-block"));
    assert_eq!(styles["top"], "10px");
    assert_eq!(styles["bottom"], "20px");
}
