//! The invariant the base-arm stage exists to guarantee, stated without naming a shorthand.
//!
//! > For every element and every property, a declaration in force only inside a
//! > document-answered condition is emitted only inside that condition, and the value the
//! > element takes when the condition is false is emitted outside it.
//!
//! The stage's earlier tests are written in the vocabulary of its implementation — they
//! assert a withdrawal happens where the stage can name the value a declaration set — so no
//! arrangement of inputs can make them notice a population it declines to name. These are
//! written in the vocabulary of the page: an authored sheet, the sample one paint produced,
//! and the arm the element must be able to return to. How the author spelled the declaration
//! is varied *within* that statement rather than being its subject.

use super::restore_unconditional;
use crate::generate::authored_css_index::{Authored, Index};
use crate::generate::shorthand::Shorthands;
use crate::model::{Attributes, Node, Rect, Styles};

fn node(classes: &str, style: &[(&str, &str)]) -> Node {
    Node {
        path: String::new(),
        tag: "p".into(),
        attributes: Attributes::from([("class".into(), classes.into())]),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style: style
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
        ..Default::default()
    }
}

/// The division a capture records, in the shape the artifact carries: block text to the
/// longhands the engine stored for it, each with the share it gave.
fn divisions(blocks: &[(&str, &[(&str, &str)])]) -> Shorthands {
    blocks
        .iter()
        .map(|(block, shares)| {
            (
                (*block).to_string(),
                shares
                    .iter()
                    .map(|(name, share)| ((*name).to_string(), (*share).to_string()))
                    .collect(),
            )
        })
        .collect()
}

fn restored(node: &Node, rules: &[String], shorthands: &Shorthands) -> Styles {
    let mut styles = node.style.clone();
    restore_unconditional(
        &mut styles,
        node,
        &Index::new(Authored { rules, shorthands }),
    );
    styles
}

/// One property family, one condition, one pair of colours, three spellings. Every card takes
/// green while its container is wide and red when it is not, so the three must agree.
fn scene() -> Vec<String> {
    vec![
        ".multi { background: padding-box padding-box rgb(255, 0, 0); }".into(),
        ".single { background: rgb(255, 0, 0); }".into(),
        ".longhand { background-color: rgb(255, 0, 0); }".into(),
        "@container (min-width: 500px){.multi { background: border-box border-box rgb(0, 255, 0); }}"
            .into(),
        "@container (min-width: 500px){.single { background: rgb(0, 255, 0); }}".into(),
        "@container (min-width: 500px){.longhand { background-color: rgb(0, 255, 0); }}".into(),
    ]
}

/// The shares the engine stored for the scene's blocks. A component the author omitted is
/// reported as `initial`, which declares nothing and is not recorded.
fn scene_divisions() -> Shorthands {
    divisions(&[
        (
            "background: padding-box padding-box rgb(255, 0, 0);",
            &[
                ("background-clip", "padding-box"),
                ("background-color", "rgb(255, 0, 0)"),
                ("background-origin", "padding-box"),
            ],
        ),
        (
            "background: border-box border-box rgb(0, 255, 0);",
            &[
                ("background-clip", "border-box"),
                ("background-color", "rgb(0, 255, 0)"),
                ("background-origin", "border-box"),
            ],
        ),
        (
            "background: rgb(255, 0, 0);",
            &[("background-color", "rgb(255, 0, 0)")],
        ),
        (
            "background: rgb(0, 255, 0);",
            &[("background-color", "rgb(0, 255, 0)")],
        ),
    ])
}

/// The filed defect. The wide card measured green, and green is what the condition declares,
/// so green belongs inside the condition and red — the arm the author wrote outside it — is
/// the value the card must publish unconditionally. Spelling the declaration with two
/// components changes nothing about which arm the card takes at which width.
#[test]
fn a_card_returns_to_its_base_arm_however_the_override_was_spelled() {
    for class in ["multi", "single", "longhand"] {
        let wide = node(
            class,
            &[
                ("background-color", "rgb(0, 255, 0)"),
                ("background-clip", "border-box"),
            ],
        );

        let styles = restored(&wide, &scene(), &scene_divisions());

        assert_eq!(
            styles["background-color"], "rgb(255, 0, 0)",
            "the .{class} card can never become red: green was published unconditionally"
        );
    }
}

/// The other instance of the same component, in the same paint, on the other branch. It
/// already measured the base arm, so its conditional declaration disagrees with its sample
/// and there is nothing to withdraw.
#[test]
fn the_instance_on_the_false_branch_keeps_what_it_measured() {
    for class in ["multi", "single", "longhand"] {
        let narrow = node(class, &[("background-color", "rgb(255, 0, 0)")]);

        let styles = restored(&narrow, &scene(), &scene_divisions());

        assert_eq!(styles["background-color"], "rgb(255, 0, 0)");
    }
}

#[path = "authored_conditions_base_arm_family_tests.rs"]
mod families;
