use super::selector_scope_fixture::{emit, scoped};

const SCOPED: usize = 2;
const LOOSE: usize = 4;

/// A sibling combinator names a preceding sibling, which the subject here does not have.
/// Without this the walk would silently fall back to the ancestor chain.
#[test]
fn refuses_a_sibling_combinator_with_no_preceding_sibling() {
    let rules = emit(
        SCOPED,
        "@media (min-width: 100000px) { .theme + .card { color: red; } }",
    );

    assert!(rules.is_empty(), "{rules:?}");
}

/// A preceding sibling that does match resolves onto its own generated class, so the
/// relationship is carried rather than flattened.
#[test]
fn resolves_a_sibling_combinator_onto_the_preceding_sibling() {
    let rules = emit(
        LOOSE,
        "@media (min-width: 100000px) { .spacer + .card { color: red; } }",
    );

    assert_eq!(
        rules,
        vec![format!(
            "@media (min-width: 100000px){{{}{{color: red;}}}}",
            scoped(&[".spacer", ".card"], &['+'])
        )]
    );
}

/// `+` names the immediately preceding sibling and `~` names any of them. A walk that
/// searched backwards for either would make the two indistinguishable, so the same tree
/// must reject one and accept the other: `.theme` precedes this card but `.spacer` is
/// between them.
#[test]
fn distinguishes_the_adjacent_sibling_from_a_general_one() {
    let adjacent = emit(
        LOOSE,
        "@media (min-width: 100000px) { .theme + .card { color: red; } }",
    );
    let general = emit(
        LOOSE,
        "@media (min-width: 100000px) { .theme ~ .card { color: red; } }",
    );

    assert!(adjacent.is_empty(), "{adjacent:?}");
    assert_eq!(
        general,
        vec![format!(
            "@media (min-width: 100000px){{{}{{color: red;}}}}",
            scoped(&[".theme", ".card"], &['~'])
        )]
    );
}

/// An ancestor compound is matched against the ancestor, not against the subject. Without
/// this the walk would accept any selector whose leading compounds happen to describe the
/// element being emitted for.
#[test]
fn matches_each_compound_against_the_node_it_names() {
    let rules = emit(
        SCOPED,
        "@media (min-width: 100000px) { .card .card { color: red; } }",
    );

    assert!(rules.is_empty(), "{rules:?}");
}
