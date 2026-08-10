use super::source_svg_assets::document;

/// The stylesheet a relocated SVG is carved from, shaped like the emitter's own output:
/// baked class rules on one line each, and a multi-line `@keyframes` block, which is how
/// a definition rule re-serialised from `cssText` actually arrives.
const CSS: &str = concat!(
    ".r_outside{animation-name:arcspin;}\n",
    "@keyframes arcspin {\n  0% {\n    opacity: 1;\n  }\n  100% {\n    opacity: 0.25;\n  }\n}\n",
    ".r_root{animation-name:arcspin;}\n",
    ".r_arc{animation-name:arcspin;fill:none;}\n",
);

const SVG: &str = r#"<svg className={"r_root"}><circle className={"r_arc"} /></svg>"#;

fn style_block(asset: &str) -> String {
    let start = asset.find("<style>").expect("asset carries a style block") + "<style>".len();
    let end = asset.find("</style>").expect("style block is closed");
    asset[start..end].to_string()
}

fn balanced(text: &str) -> bool {
    text.matches('{').count() == text.matches('}').count()
}

/// The defect. An SVG referenced through `<img src>` is an independent document, so a
/// `@keyframes` block left in the page stylesheet can never reach it. The asset must both
/// name and define the animation.
///
/// Asserted as a conjunction on purpose. The reference half rules out "the declaration was
/// dropped upstream", which would make the missing definition unattributable; the
/// definition half is the loss. Either half alone passes for the wrong reason — the first
/// under the defect, the second if the whole stylesheet were copied wholesale.
#[test]
fn carries_the_keyframes_a_relocated_element_names() {
    let asset = document(SVG, CSS);
    let styles = style_block(&asset);
    assert!(
        styles.contains("animation-name:arcspin"),
        "the relocated element must keep the declaration naming the animation: {styles}"
    );
    assert!(
        styles.contains("@keyframes arcspin"),
        "the asset names an animation it does not define: {styles}"
    );
    assert!(
        styles.contains("opacity: 0.25"),
        "the keyframes block must arrive whole, not as its opening line: {styles}"
    );
    assert!(balanced(&styles), "unbalanced style block: {styles}");
}

/// A definition nothing in the relocated subtree names stays behind. Without this the
/// "fix" of copying the entire stylesheet would pass every other test here while making
/// each asset as large as the page.
#[test]
fn leaves_behind_a_definition_the_relocated_subtree_never_names() {
    let css = format!("{CSS}@keyframes unused {{\n  0% {{\n    opacity: 0;\n  }}\n}}\n");
    let styles = style_block(&document(SVG, &css));
    assert!(styles.contains("@keyframes arcspin"));
    assert!(
        !styles.contains("unused"),
        "carried a definition nothing references: {styles}"
    );
}

/// Only the classes the SVG actually carries. A rule for an element that stayed in the
/// page is not the relocated subtree's business.
#[test]
fn leaves_behind_a_rule_for_an_element_that_stayed_in_the_page() {
    let styles = style_block(&document(SVG, CSS));
    assert!(styles.contains("r_arc"));
    assert!(
        !styles.contains("r_outside"),
        "carried a rule for an element outside the SVG: {styles}"
    );
}

/// The same severing, one document boundary and one at-rule further out. `@font-face` is
/// named by a descriptor inside its own block rather than by a prelude identifier, so a
/// fix that reads only the prelude closes the animation case and leaves this one open.
#[test]
fn carries_the_font_face_a_relocated_element_names() {
    let css = concat!(
        "@font-face {\n  font-family: \"Sceneface\";\n  src: url(a.woff2);\n}\n",
        ".r_arc{font-family:Sceneface;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(
        styles.contains("@font-face"),
        "the asset names a font family it does not define: {styles}"
    );
    assert!(balanced(&styles));
}

/// A counter style, reached through a different property again. One closure covers every
/// definition at-rule, so no per-property branch decides which of these survives.
#[test]
fn carries_the_counter_style_a_relocated_element_names() {
    let css = concat!(
        "@counter-style scenecount {\n  system: cyclic;\n  symbols: \"x\";\n}\n",
        ".r_arc{list-style-type:scenecount;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(styles.contains("@counter-style scenecount"), "{styles}");
    assert!(balanced(&styles));
}

/// A definition that only becomes reachable through another definition. One pass over the
/// stylesheet carries the keyframes and stops, leaving the face the keyframes names behind,
/// so the traversal has to run to a fixed point.
#[test]
fn follows_a_reference_a_carried_definition_introduces() {
    let css = concat!(
        "@font-face {\n  font-family: \"Deepface\";\n  src: url(a.woff2);\n}\n",
        "@keyframes arcspin {\n  0% {\n    font-family: Deepface;\n  }\n}\n",
        ".r_arc{animation-name:arcspin;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(styles.contains("@keyframes arcspin"));
    assert!(
        styles.contains("@font-face"),
        "stopped after one pass instead of closing over what it carried: {styles}"
    );
}

/// A class rule nested in a grouping at-rule is a real shape in this emitter's output —
/// the responsive stages write per-viewport rules for exactly these classes. Copied by
/// line it yields `@media(...) {` and the selector's opening brace with no close, which
/// corrupts every rule after it in the asset rather than dropping one.
#[test]
fn carries_a_grouped_rule_as_a_balanced_block() {
    let css = concat!(
        "@media(max-width:320px) {\n  .r_arc {\n    opacity:0.5;\n  }\n",
        "  .r_outside {\n    opacity:0.1;\n  }\n}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(balanced(&styles), "unbalanced style block: {styles}");
    assert!(styles.contains("opacity:0.5"), "{styles}");
    assert!(
        !styles.contains("r_outside"),
        "re-wrapped the group without filtering it: {styles}"
    );
}

/// Names are matched as identifiers, not as substrings, so a definition whose name merely
/// occurs inside a longer one is not dragged along.
#[test]
fn does_not_carry_a_definition_whose_name_is_a_substring_of_another() {
    let css = concat!(
        "@keyframes spin {\n  0% {\n    opacity: 0;\n  }\n}\n",
        "@keyframes arcspin {\n  0% {\n    opacity: 1;\n  }\n}\n",
        ".r_arc{animation-name:arcspin;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(styles.contains("@keyframes arcspin"));
    assert!(
        !styles.contains("@keyframes spin"),
        "matched a name by substring: {styles}"
    );
}

#[test]
fn converts_jsx_svg_to_xml() {
    let svg = super::source_svg_assets::to_xml(
        r#"<svg className={"icon"} viewBox={"0 0 1 1"}><path strokeWidth={"1"} /></svg>"#,
    );
    assert!(svg.contains("class=\"icon\""));
    assert!(svg.contains("stroke-width=\"1\""));
}

#[test]
fn decodes_svg_image_sources() {
    let matches = super::source_svg_assets::encoded_svg_sources(
        r#"<img src={"data:image/svg+xml;utf8,%3Csvg%20xmlns%3D%22x%22%3E%3C%2Fsvg%3E"} />"#,
    );
    assert_eq!(matches[0].2, r#"<svg xmlns="x"></svg>"#);
}
