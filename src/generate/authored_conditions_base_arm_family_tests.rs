//! The same invariant, over families whose grammars divide by different rules entirely.
//!
//! Each is a family the repository names nowhere, in no table, in either direction. They
//! divide because the division is performed by the engine that parsed the sheet and merely
//! read here, which is the only form of the answer that can be right about a family nobody
//! listed. A repair that needed a row per family would fail one of these.

use super::{divisions, node, restored, scene};
use crate::generate::shorthand::Shorthands;
/// A different arity and a different division rule: `margin` gives its first component to the
/// block edges and its second to the inline ones. Nothing here reads that rule — the shares
/// come from the engine that parsed the sheet — so a family with a different grammar needs no
/// second entry anywhere.
#[test]
fn a_family_that_divides_by_edge_returns_to_its_own_base_arm() {
    let card = node("card", &[("margin-top", "20px"), ("margin-left", "30px")]);
    let rules = vec![
        ".card { margin: 4px 12px; }".to_string(),
        "@media (min-width: 500px){.card { margin: 20px 30px; }}".to_string(),
    ];
    let shorthands = divisions(&[
        (
            "margin: 4px 12px;",
            &[
                ("margin-bottom", "4px"),
                ("margin-left", "12px"),
                ("margin-right", "12px"),
                ("margin-top", "4px"),
            ],
        ),
        (
            "margin: 20px 30px;",
            &[
                ("margin-bottom", "20px"),
                ("margin-left", "30px"),
                ("margin-right", "30px"),
                ("margin-top", "20px"),
            ],
        ),
    ]);

    let styles = restored(&card, &rules, &shorthands);

    assert_eq!(styles["margin-top"], "4px");
    assert_eq!(styles["margin-left"], "12px");
}

/// `border` divides by component *kind* rather than by position, and its name prefixes
/// `border-radius`, which it does not set. The engine's own record of what the block stored
/// is what refuses that pair, so a corner the author never touched is never rewritten.
#[test]
fn a_family_that_divides_by_kind_leaves_a_prefixed_stranger_alone() {
    let card = node(
        "card",
        &[
            ("border-top-width", "8px"),
            ("border-top-color", "rgb(0, 255, 0)"),
            ("border-radius", "4px"),
        ],
    );
    let rules = vec![
        ".card { border: 2px solid rgb(255, 0, 0); border-radius: 4px; }".to_string(),
        "@media (min-width: 500px){.card { border: 8px solid rgb(0, 255, 0); }}".to_string(),
    ];
    let shorthands = divisions(&[
        (
            "border: 2px solid rgb(255, 0, 0); border-radius: 4px;",
            &[
                ("border-top-color", "rgb(255, 0, 0)"),
                ("border-top-style", "solid"),
                ("border-top-width", "2px"),
            ],
        ),
        (
            "border: 8px solid rgb(0, 255, 0);",
            &[
                ("border-top-color", "rgb(0, 255, 0)"),
                ("border-top-style", "solid"),
                ("border-top-width", "8px"),
            ],
        ),
    ]);

    let styles = restored(&card, &rules, &shorthands);

    assert_eq!(styles["border-top-width"], "2px");
    assert_eq!(styles["border-top-color"], "rgb(255, 0, 0)");
    assert_eq!(
        styles["border-radius"], "4px",
        "a corner the shorthand never set was rewritten from its prefix"
    );
}

/// A family no part of this repository names, in any table, in either direction. It divides
/// because the division is performed by the engine and merely read here, which is the only
/// form of the answer that can be right about a family nobody thought of.
#[test]
fn a_family_the_tool_has_never_heard_of_divides_correctly() {
    let card = node(
        "note",
        &[
            ("text-decoration-line", "underline"),
            ("text-decoration-color", "rgb(0, 255, 0)"),
        ],
    );
    let rules = vec![
        ".note { text-decoration: underline dotted rgb(255, 0, 0); }".to_string(),
        "@container (min-width: 500px){.note { text-decoration: underline wavy rgb(0, 255, 0); }}"
            .to_string(),
    ];
    let shorthands = divisions(&[
        (
            "text-decoration: underline dotted rgb(255, 0, 0);",
            &[
                ("text-decoration-color", "rgb(255, 0, 0)"),
                ("text-decoration-line", "underline"),
                ("text-decoration-style", "dotted"),
            ],
        ),
        (
            "text-decoration: underline wavy rgb(0, 255, 0);",
            &[
                ("text-decoration-color", "rgb(0, 255, 0)"),
                ("text-decoration-line", "underline"),
                ("text-decoration-style", "wavy"),
            ],
        ),
    ]);

    let styles = restored(&card, &rules, &shorthands);

    assert_eq!(styles["text-decoration-color"], "rgb(255, 0, 0)");
    assert_eq!(styles["text-decoration-line"], "underline");
}

/// A base arm holding `var()` divides into longhands the engine itself cannot settle, and it
/// says so by storing them present and blank. That blank is not "no arm was written": deleting
/// the property would publish an initial value the source never takes, so the measured value
/// stands and the declaration is left alone.
#[test]
fn a_base_arm_the_engine_could_not_settle_deletes_nothing() {
    let card = node("card", &[("padding-top", "40px")]);
    let rules = vec![
        ".card { padding: 0px var(--gutter); }".to_string(),
        "@media (min-width: 500px){.card { padding: 40px; }}".to_string(),
    ];
    let shorthands = divisions(&[
        (
            "padding: 0px var(--gutter);",
            &[
                ("padding-bottom", ""),
                ("padding-left", ""),
                ("padding-right", ""),
                ("padding-top", ""),
            ],
        ),
        (
            "padding: 40px;",
            &[
                ("padding-bottom", "40px"),
                ("padding-left", "40px"),
                ("padding-right", "40px"),
                ("padding-top", "40px"),
            ],
        ),
    ]);

    let styles = restored(&card, &rules, &shorthands);

    assert_eq!(styles["padding-top"], "40px");
}

/// An artifact that recorded no division at all — one written before the capture recorded
/// them, or a fixture written by hand. Nothing can be divided, so nothing is withdrawn and
/// nothing is deleted: the stage fails closed rather than guessing a share.
#[test]
fn an_artifact_carrying_no_division_withdraws_nothing_and_deletes_nothing() {
    let wide = node("multi", &[("background-color", "rgb(0, 255, 0)")]);

    let styles = restored(&wide, &scene(), &Shorthands::new());

    assert_eq!(styles["background-color"], "rgb(0, 255, 0)");
}
