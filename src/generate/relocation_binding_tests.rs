//! The stage that answers what a relocated fragment still needs bound, tested against
//! captured trees rather than against a rendered page.

use super::relocated_current_color_tests::{declares_color, names_current_color};
use super::relocation_binding::rules;
use super::source_svg_assets::document;
use crate::model::{Node, PageState, Styles};
use std::collections::BTreeMap;

fn node(path: &str, parent: Option<&str>, styles: &[(&str, &str)]) -> Node {
    Node {
        path: path.into(),
        parent: parent.map(str::to_string),
        tag: path.rsplit('>').next().unwrap_or(path).into(),
        style: Styles::from_iter(
            styles
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into())),
        ),
        ..Default::default()
    }
}

/// A page shaped like the one that files this: the paint is declared on an ancestor, so
/// the differencing capture records it there and on no element below it.
fn page() -> (PageState, BTreeMap<String, String>) {
    let state = PageState {
        nodes: vec![
            node("body", None, &[]),
            node("body>span", Some("body"), &[("color", "rgb(220, 20, 60)")]),
            node(
                "body>span>svg",
                Some("body>span"),
                &[("fill", "rgb(220, 20, 60)")],
            ),
        ],
        ..Default::default()
    };
    let classes = BTreeMap::from_iter([
        ("body>span".to_string(), "r_host".to_string()),
        ("body>span>svg".to_string(), "r_icon".to_string()),
    ]);
    (state, classes)
}

/// The defect, at the stage that owns it. The icon's own rule resolves against a `color`
/// no rule of its own declares, and the value is not in the stylesheet's text at all — it
/// is on an intermediate ancestor, which is precisely the case `css_inheritance` refuses
/// to guess at. Read from the captured tree it is not a guess.
#[test]
fn binds_the_inherited_color_a_relocated_rule_resolves_against() {
    let (state, classes) = page();
    let bound = rules(
        &[(&state, &classes)],
        ".r_host{color:rgb(220, 20, 60);}\n.r_icon{fill:currentcolor;}\n",
    );
    assert_eq!(bound, ".r_icon{color:rgb(220, 20, 60);}\n");
}

/// The value has to come from wherever the engine resolved it, not from the element's
/// parent. A paint declared two levels up is the ordinary case and a one-step lookup
/// silently binds nothing.
#[test]
fn resolves_the_value_past_an_ancestor_that_records_nothing() {
    let state = PageState {
        nodes: vec![
            node("body", None, &[("color", "rgb(220, 20, 60)")]),
            node("body>span", Some("body"), &[]),
            node("body>span>svg", Some("body>span"), &[]),
        ],
        ..Default::default()
    };
    let classes = BTreeMap::from_iter([("body>span>svg".to_string(), "r_icon".to_string())]);
    let bound = rules(&[(&state, &classes)], ".r_icon{stroke:currentcolor;}\n");
    assert_eq!(bound, ".r_icon{color:rgb(220, 20, 60);}\n");
}

/// The empty case. A rule reading nothing must not grow a payload, which is what keeps a
/// page with no such keyword emitting exactly what it emitted before.
#[test]
fn binds_nothing_for_a_rule_that_reads_no_keyword() {
    let (state, classes) = page();
    let bound = rules(
        &[(&state, &classes)],
        ".r_host{color:rgb(220, 20, 60);}\n.r_icon{fill:rgb(0, 128, 0);}\n",
    );
    assert!(bound.is_empty(), "bound a value nothing reads: {bound}");
}

/// A rule already declaring the property it resolves against is complete, so re-declaring
/// it would write the same value twice and risk overriding the authored one.
#[test]
fn binds_nothing_when_the_rule_already_declares_the_property() {
    let (state, classes) = page();
    let bound = rules(
        &[(&state, &classes)],
        ".r_icon{fill:currentcolor;color:rgb(1, 2, 3);}\n",
    );
    assert!(bound.is_empty(), "re-declared a bound value: {bound}");
}

/// The keyword is seen wherever it is spelled, including inside a responsive arm, because
/// the emitter writes one rule per viewport for exactly these classes.
#[test]
fn sees_a_keyword_spelled_only_inside_a_grouping_at_rule() {
    let (state, classes) = page();
    let bound = rules(
        &[(&state, &classes)],
        "@media(max-width:320px){.r_icon{fill:currentcolor;}}\n",
    );
    assert_eq!(bound, ".r_icon{color:rgb(220, 20, 60);}\n");
}

/// End to end through the stage that writes the file: the bound rule is selected by the
/// closure like any other rule naming the fragment's class, so no second mechanism decides
/// what reaches the asset.
#[test]
fn the_bound_rule_reaches_the_emitted_asset() {
    let (state, classes) = page();
    let css = ".r_icon{fill:currentcolor;}\n";
    let asset = document(
        r#"<svg className={"r_icon"}><path /></svg>"#,
        &format!("{css}{}", rules(&[(&state, &classes)], css)),
    );
    assert!(names_current_color(&asset));
    assert!(declares_color(&asset), "{asset}");
    assert!(asset.contains("rgb(220, 20, 60)"), "{asset}");
}

/// The same unbound name in the other syntax the file carries. A presentation attribute
/// survives relocation as text and resolves in the destination document, so a fragment
/// whose only mention of the keyword is an attribute needs the same binding.
#[test]
fn binds_the_color_a_presentation_attribute_resolves_against() {
    let mut rect = node("body>span>svg>rect", Some("body>span>svg"), &[]);
    rect.attributes.insert("class".into(), "attr".into());
    rect.attributes.insert("fill".into(), "currentColor".into());
    let state = PageState {
        nodes: vec![
            node("body", None, &[]),
            node("body>span", Some("body"), &[("color", "rgb(220, 20, 60)")]),
            node("body>span>svg", Some("body>span"), &[]),
            rect,
        ],
        ..Default::default()
    };
    let classes = BTreeMap::from_iter([("body>span>svg>rect".to_string(), "r_rect".to_string())]);
    let bound = rules(&[(&state, &classes)], ".r_rect{fill:rgb(220, 20, 60);}\n");
    assert_eq!(bound, ".r_rect{color:rgb(220, 20, 60);}\n");
}

/// An attribute naming an ordinary colour reads nothing, so the attribute channel does not
/// become a licence to bind every element that has one.
#[test]
fn binds_nothing_for_an_attribute_naming_a_literal() {
    let mut rect = node("body>span>svg>rect", Some("body>span>svg"), &[]);
    rect.attributes.insert("fill".into(), "rgb(0, 128, 0)".into());
    let state = PageState {
        nodes: vec![
            node("body", None, &[("color", "rgb(220, 20, 60)")]),
            node("body>span", Some("body"), &[]),
            node("body>span>svg", Some("body>span"), &[]),
            rect,
        ],
        ..Default::default()
    };
    let classes = BTreeMap::from_iter([("body>span>svg>rect".to_string(), "r_rect".to_string())]);
    assert!(rules(&[(&state, &classes)], "").is_empty());
}