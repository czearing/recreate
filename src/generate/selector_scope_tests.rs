use super::selector_scope::Scope;
use crate::model::{Attributes, Node, Rect};
use std::collections::BTreeMap;

/// `.theme > .card` plus a second `.card` outside it.
///
/// Both cards compute the same style, so the generator gives them one shared class. That is
/// the point: it makes the emitted selector the only thing that can tell them apart, so a
/// rewrite that collapses to the subject alone cannot pass by accident.
fn tree() -> (Vec<Node>, BTreeMap<String, String>) {
    let nodes = vec![
        element("html>body", None, "page"),
        element("html>body>div:nth-of-type(1)", Some("html>body"), "theme"),
        element(
            "html>body>div:nth-of-type(1)>div:nth-of-type(1)",
            Some("html>body>div:nth-of-type(1)"),
            "card",
        ),
        element("html>body>p:nth-of-type(1)", Some("html>body"), "spacer"),
        element("html>body>div:nth-of-type(2)", Some("html>body"), "card"),
    ];
    let classes = BTreeMap::from([
        (nodes[0].path.clone(), "page".to_string()),
        (nodes[1].path.clone(), "theme".to_string()),
        (nodes[2].path.clone(), "card".to_string()),
        (nodes[3].path.clone(), "spacer".to_string()),
        (nodes[4].path.clone(), "card".to_string()),
    ]);
    (nodes, classes)
}

fn element(path: &str, parent: Option<&str>, class: &str) -> Node {
    Node {
        disabled: false,
        path: path.into(),
        parent: parent.map(Into::into),
        tag: path
            .rsplit('>')
            .next()
            .and_then(|segment| segment.split(':').next())
            .unwrap_or("div")
            .into(),
        text: String::new(),
        attributes: Attributes::from([("class".into(), class.into())]),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
        },
        style: Default::default(),
        before: None,
        after: None,
    }
}

fn emit(subject: usize, rule: &str) -> Vec<String> {
    let (nodes, classes) = tree();
    let scope = Scope::new(&nodes, &classes);
    super::authored_media::rules(&nodes[subject], &scope, &[rule.to_string()])
}

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
        vec![
            "@media (prefers-color-scheme: dark){.theme .card{background-color: #d4e5f6;}}"
                .to_string()
        ]
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
    for (selector, expected) in [
        (".theme>.card", ".theme>.card"),
        (".theme .card", ".theme .card"),
        ("body .card", ".page .card"),
        ("body>.theme .card", ".page>.theme .card"),
    ] {
        let rules = emit(
            SCOPED,
            &format!("@media (min-width: 100000px) {{ {selector} {{ color: red; }} }}"),
        );

        assert_eq!(
            rules,
            vec![format!(
                "@media (min-width: 100000px){{{expected}{{color: red;}}}}"
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
        vec!["@media (min-width: 100000px){.spacer+.card{color: red;}}".to_string()]
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
        vec!["@media (min-width: 100000px){.theme~.card{color: red;}}".to_string()]
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
