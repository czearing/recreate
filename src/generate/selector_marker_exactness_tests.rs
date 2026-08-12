//! The lone compound the paint class cannot serve.
//!
//! The sibling file's tree is reused: it already holds two elements that paint alike and
//! that the author distinguished, which is the only shape in which the question arises.

use super::{generate, marker, tokens};

/// The lone-compound case the paint class cannot serve. `.theme` and the bare wrapper beside
/// it paint identically, so they share one paint class — and a rewrite that borrowed it would
/// hand the unwrapped element the authored declarations the author gave only to `.theme`.
/// The rewrite is exact or it is a marker; it is never a guess.
#[test]
fn mints_a_marker_when_the_paint_class_names_an_element_the_compound_excludes() {
    let (rules, classes) = generate(
        1,
        "@media (prefers-color-scheme: dark) { .theme { color: rgb(0, 128, 0); } }",
    );
    let theme = marker("r", ".theme");

    assert_eq!(
        rules,
        vec![format!(
            "@media (prefers-color-scheme: dark){{.{theme}{{color: rgb(0, 128, 0);}}}}"
        )],
        "borrowed a paint class shared with an element the compound excludes"
    );
    assert!(
        tokens(&classes, 1).contains(&theme),
        "the marker never reached the element the compound names"
    );
    assert!(
        !tokens(&classes, 3).contains(&theme),
        "the marker reached the look-alike the compound excludes"
    );
}
