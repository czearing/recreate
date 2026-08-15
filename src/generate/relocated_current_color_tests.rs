//! The file-level invariant a relocated graphic has to satisfy, asserted on the emitted
//! asset text rather than on the styles that produced it.
//!
//! Referenced through `<img src>` a graphic becomes its own document, and an image
//! document inherits nothing from its host, so `currentcolor` there resolves against
//! `color`'s initial value. Whatever binds it, the artifact has to show the binding.

use super::relocation_binding::rules;
use super::source_svg_assets::document;
use crate::model::{Node, PageState, Styles};
use std::collections::BTreeMap;

const SVG: &str = r#"<svg className={"r_icon"}><path className={"r_path"} /></svg>"#;

/// Whether `text` names `currentcolor`, case-insensitively, because `css_rules` lowercases
/// the keyword while an attribute keeps the author's spelling.
pub(super) fn names_current_color(text: &str) -> bool {
    text.to_ascii_lowercase().contains("currentcolor")
}

/// Whether `text` declares `color` as a property: a declaration boundary, the name, then a
/// colon. Written independently of the production code, and deliberately not a search for
/// bare `color`, which `currentColor`, `-webkit-text-fill-color` and `stop-color` all
/// satisfy.
pub(super) fn declares_color(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.match_indices("color")
        .filter(|(index, _)| {
            text[..*index]
                .chars()
                .next_back()
                .is_none_or(|character| matches!(character, '{' | ';' | ' ' | '\n' | '\t'))
        })
        .any(|(index, _)| text[index + "color".len()..].trim_start().starts_with(':'))
}

#[test]
fn the_predicate_separates_a_binding_from_a_keyword() {
    assert!(names_current_color("fill:currentColor;"));
    assert!(!declares_color(".a{fill:currentcolor;}"));
    assert!(!declares_color(".a{-webkit-text-fill-color:red;}"));
    assert!(!declares_color(".a{stop-color:red;}"));
    assert!(declares_color(".a{color:red;}"));
    assert!(declares_color(".a{fill:currentcolor;color:red;}"));
}

/// The defect, at the file level. An asset whose carried text resolves against `color` has
/// to declare it, or the icon paints in `canvastext` where the page painted it crimson.
#[test]
fn a_relocated_graphic_naming_current_color_declares_the_color_it_resolves_against() {
    let asset = document(
        SVG,
        ".r_icon{fill:currentcolor;color:rgb(220, 20, 60);}\n.r_path{d:path(\"M 0 0\");}\n",
    );
    assert!(names_current_color(&asset));
    assert!(
        declares_color(&asset),
        "the asset resolves against a colour it never declares: {asset}"
    );
    assert!(
        asset.contains("rgb(220, 20, 60)"),
        "the binding must carry the host's resolved value: {asset}"
    );
}

/// `stroke` reaches `color` through the same keyword and must arrive by the same route.
#[test]
fn a_relocated_graphic_stroking_with_current_color_declares_it_too() {
    let asset = document(
        SVG,
        ".r_icon{fill:none;stroke:currentcolor;color:rgb(30, 60, 200);}\n",
    );
    assert!(names_current_color(&asset));
    assert!(declares_color(&asset), "{asset}");
}

/// The empty case. A graphic that names no `currentcolor` must not grow a fabricated
/// payload, so the invariant holds vacuously rather than by over-carrying.
#[test]
fn a_relocated_graphic_with_a_literal_fill_gains_no_color_declaration() {
    let asset = document(SVG, ".r_icon{fill:rgb(0, 128, 0);}\n");
    assert!(!names_current_color(&asset));
    assert!(
        !declares_color(&asset),
        "carried a binding nothing reads: {asset}"
    );
}

/// The rule reaches the asset root, which is what makes one declaration enough for every
/// descendant. A `color` written only inside a responsive arm would satisfy a naive grep
/// while binding nothing at the base viewport.
#[test]
fn the_binding_lands_on_the_relocated_root_not_only_inside_a_responsive_arm() {
    let asset = document(
        SVG,
        concat!(
            ".r_icon{fill:currentcolor;color:rgb(220, 20, 60);}\n",
            "@media(max-width:320px) {\n  .r_icon {\n    fill:currentcolor;\n  }\n}\n",
        ),
    );
    let base = asset.split("@media").next().unwrap_or_default();
    assert!(
        declares_color(base),
        "the binding is only reachable through a media arm: {asset}"
    );
}
