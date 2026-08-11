use super::css_pseudo_identity_tests::span;
use super::source_svg_assets::document;
use crate::model::{PageState, Specification, Viewport};
use std::collections::BTreeMap;

const SVG: &str = r#"<svg className={"r_root"}><circle className={"r_arc"} /></svg>"#;

fn style_block(asset: &str) -> String {
    let start = asset.find("<style>").expect("asset carries a style block") + "<style>".len();
    let end = asset.find("</style>").expect("style block is closed");
    asset[start..end].to_string()
}

fn balanced(text: &str) -> bool {
    text.matches('{').count() == text.matches('}').count()
}

/// The condition still wrapping the definition, rather than merely present somewhere in the
/// file. A fix that lifts the definition out of its group satisfies a `@font-face` grep
/// while publishing a font the source declared only under a condition.
fn wrapped(styles: &str, prelude: &str, definition: &str) -> bool {
    let Some(start) = styles.find(prelude) else {
        return false;
    };
    let rest = &styles[start + prelude.len()..];
    let Some(end) = rest.find('}') else {
        return false;
    };
    rest[..end].contains(definition)
}

/// The defect. A definition inside a conditional group reaches neither half of the closure:
/// the consumer half descends into the group but declines definitions, and the definition
/// half would take it but iterates only the top-level rule list.
#[test]
fn carries_a_definition_nested_in_a_conditional_group() {
    let css = concat!(
        "@media (min-width: 1px) {\n@font-face { font-family: \"Vorplish\"; src: url(a.woff2); }\n}\n",
        ".r_arc{font-family:Vorplish;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(
        styles.contains("font-family:Vorplish"),
        "the reference must survive or the absence is unattributable: {styles}"
    );
    assert!(
        styles.contains("@font-face"),
        "the asset names a font family it does not define: {styles}"
    );
    assert!(balanced(&styles), "unbalanced style block: {styles}");
}

/// The group is rebuilt around the definition, never flattened. An unconditional definition
/// published where the source wrote a conditional one is a fabrication, and a worse outcome
/// than the omission it replaces.
#[test]
fn keeps_the_condition_a_nested_definition_was_authored_under() {
    let css = concat!(
        "@media (prefers-reduced-motion: no-preference) {\n@keyframes spin { 0% { rotate: 0deg; } }\n}\n",
        ".r_arc{animation-name:spin;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(
        wrapped(
            &styles,
            "@media (prefers-reduced-motion: no-preference) {",
            "@keyframes spin"
        ),
        "flattened the definition out of its condition: {styles}"
    );
}

/// A group holding both a style rule for a relocated class and a definition. Marking each
/// top-level rule carried-or-not claims this group on its first member and then skips it,
/// so the definition never arrives however deeply a second traversal recurses. Only one
/// unified pass reaches it.
#[test]
fn carries_a_definition_beside_a_style_rule_in_the_same_group() {
    let css = concat!(
        "@media (min-width: 1px) {\n",
        ".r_arc { font-family: Vorplish; }\n",
        "@font-face { font-family: \"Vorplish\"; src: url(a.woff2); }\n",
        "}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(styles.contains("font-family: Vorplish"), "{styles}");
    assert!(
        styles.contains("@font-face"),
        "the group was claimed by its style rule and its definition skipped: {styles}"
    );
    assert!(balanced(&styles), "unbalanced style block: {styles}");
}

/// Two levels, both nesting orders. A walk that descends one level closes the reported case
/// and leaves these open.
#[test]
fn carries_a_definition_two_groups_deep_in_either_order() {
    let layer_inside = concat!(
        "@media (min-width: 1px) {\n@layer motion {\n@font-face { font-family: \"Vorplish\"; src: url(a.woff2); }\n}\n}\n",
        ".r_arc{font-family:Vorplish;}\n",
    );
    let styles = style_block(&document(SVG, layer_inside));
    assert!(styles.contains("@font-face"), "layer inside media: {styles}");
    assert!(balanced(&styles));

    let media_inside = concat!(
        "@layer motion {\n@media (min-width: 1px) {\n@font-face { font-family: \"Quazitic\"; src: url(a.woff2); }\n}\n}\n",
        ".r_arc{font-family:Quazitic;}\n",
    );
    let styles = style_block(&document(SVG, media_inside));
    assert!(styles.contains("@font-face"), "media inside layer: {styles}");
    assert!(balanced(&styles));
}

/// Widening reach must not become "carry every definition", which would inline unrelated
/// fonts into every asset.
#[test]
fn leaves_behind_a_nested_definition_nothing_references() {
    let css = concat!(
        "@media (min-width: 1px) {\n@font-face { font-family: \"Unwanted\"; src: url(a.woff2); }\n}\n",
        ".r_arc{fill:none;}\n",
    );
    let styles = style_block(&document(SVG, css));
    assert!(styles.contains("fill:none"), "{styles}");
    assert!(
        !styles.contains("Unwanted"),
        "carried a definition nothing references: {styles}"
    );
}

/// A group whose only members stayed in the page contributes nothing, so no empty condition
/// is published.
#[test]
fn leaves_behind_a_group_none_of_whose_members_reach_the_fragment() {
    let css = "@media (min-width: 1px) {\n.r_outside { opacity: 0.1; }\n}\n.r_arc{fill:none;}\n";
    let styles = style_block(&document(SVG, css));
    assert!(
        !styles.contains("@media"),
        "published an empty condition: {styles}"
    );
}

/// The same blind spot, in the second place that reads it: the main stylesheet re-emits
/// definitions verbatim because no baked computed style can stand in for one, and a
/// definition inside a condition is no less a definition. The condition must come with it.
#[test]
fn the_main_stylesheet_keeps_a_definition_nested_in_a_condition() {
    let specification = Specification {
        schema_version: 1,
        requested_url: String::new(),
        captured_url: String::new(),
        states: vec![PageState {
            viewport: Viewport {
                width: 1920,
                height: 1080,
                dpr: 1.0,
            },
            nodes: vec![span(1)],
            css_rules: vec![
                "@media (min-width: 1px) {\n@font-face { font-family: \"Vorplish\"; src: url(a.woff2); }\n}".into(),
                "@media (min-width: 1px) {\n.stayed { color: red; }\n}".into(),
            ],
            ..Default::default()
        }],
        interactions: Vec::new(),
        transitions: Vec::new(),
    };
    let assets = BTreeMap::new();
    let css = super::css_base::build(super::css_base::Request {
        specification: &specification,
        assets: &assets,
        prefix: "r",
        include_interactions: true,
        reuse: None,
        cache: None,
        path_override: None,
        timing: &|_: &str| {},
    })
    .css;

    assert!(
        wrapped(&css, "@media (min-width: 1px) {", "@font-face"),
        "the conditioned definition was dropped or flattened: {css}"
    );
    assert!(
        !css.contains("color: red"),
        "re-emitted a style rule the baked classes already carry: {css}"
    );
}
