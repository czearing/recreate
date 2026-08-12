use super::selector_scope_fixture::{emit, scoped};

const SCOPED: usize = 2;
const LOOSE: usize = 4;

/// The defect. A rule whose selector requires an ancestor is discarded outright, so an
/// authored condition the viewport sweep cannot resample leaves no trace at all.
#[test]
fn keeps_a_rule_whose_selector_requires_an_ancestor() {
    let rules = emit(
        SCOPED,
        "@media (prefers-color-scheme: dark) { .theme .card { background-color: #d4e5f6; } }",
    );

    assert_eq!(
        rules,
        vec![format!(
            "@media (prefers-color-scheme: dark){{{}{{background-color: #d4e5f6;}}}}",
            scoped(&[".theme", ".card"], &[' '])
        )]
    );
}

/// The ancestor requirement has to survive as a requirement. Rewriting only the subject
/// would style every node sharing its class, turning a silent omission into a silent
/// over-application - and dropping specificity from two compounds to one.
#[test]
fn leaves_a_node_that_lacks_the_required_ancestor_unstyled() {
    let rules = emit(
        LOOSE,
        "@media (prefers-color-scheme: dark) { .theme .card { background-color: #d4e5f6; } }",
    );

    assert!(rules.is_empty(), "{rules:?}");
}

/// Each combinator constrains a different relationship, so each must resolve against a
/// different part of the tree. One walk covers all four; none is a special case.
#[test]
fn resolves_every_combinator_against_the_relationship_it_names() {
    for (selector, compounds, combinators) in [
        (".theme>.card", vec![".theme", ".card"], vec!['>']),
        (".theme .card", vec![".theme", ".card"], vec![' ']),
        ("body .card", vec!["body", ".card"], vec![' ']),
        (
            "body>.theme .card",
            vec!["body", ".theme", ".card"],
            vec!['>', ' '],
        ),
    ] {
        let rules = emit(
            SCOPED,
            &format!("@media (min-width: 100000px) {{ {selector} {{ color: red; }} }}"),
        );

        assert_eq!(
            rules,
            vec![format!(
                "@media (min-width: 100000px){{{}{{color: red;}}}}",
                scoped(&compounds, &combinators)
            )],
            "{selector}"
        );
    }
}

/// A child combinator names the parent and nothing further up. Resolving it against any
/// ancestor would make `>` mean the same as a descendant space.
#[test]
fn refuses_a_child_combinator_that_names_a_higher_ancestor() {
    let rules = emit(
        SCOPED,
        "@media (min-width: 100000px) { body > .card { color: red; } }",
    );

    assert!(rules.is_empty(), "{rules:?}");
}
