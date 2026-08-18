//! A scoped rule must not reach the look-alike the author excluded.
//!
//! The tree here is the shape scoping exists for: two structurally identical subtrees, one
//! inside the scoping wrapper and one outside it, painting identically. Because they paint
//! identically the generator gives each pair one shared class, which is correct and is what
//! keeps the sheet small. So the paint class cannot tell the two subtrees apart, and any
//! rewrite built from paint classes reaches both.

#[path = "selector_marker_exactness_tests.rs"]
mod exactness;

use super::selector_marker::{apply, name as marker};
use super::selector_scope::Scope;
use crate::model::{Attributes, Node, Rect};
use std::collections::{BTreeMap, BTreeSet};

const WRAPPER: &str = "w";
const CARD: &str = "c";

/// `ALPHA` inside `.theme`, `BRAVO` outside it, everything else equal.
fn tree() -> (Vec<Node>, BTreeMap<String, String>) {
    let nodes = vec![
        element("html>body", None, ""),
        element("html>body>div:nth-of-type(1)", Some("html>body"), "theme"),
        element(
            "html>body>div:nth-of-type(1)>p:nth-of-type(1)",
            Some("html>body>div:nth-of-type(1)"),
            "card",
        ),
        element("html>body>div:nth-of-type(2)", Some("html>body"), ""),
        element(
            "html>body>div:nth-of-type(2)>p:nth-of-type(1)",
            Some("html>body>div:nth-of-type(2)"),
            "card",
        ),
    ];
    let classes = BTreeMap::from([
        (nodes[0].path.clone(), "page".to_string()),
        (nodes[1].path.clone(), WRAPPER.to_string()),
        (nodes[2].path.clone(), CARD.to_string()),
        (nodes[3].path.clone(), WRAPPER.to_string()),
        (nodes[4].path.clone(), CARD.to_string()),
    ]);
    (nodes, classes)
}

fn element(path: &str, parent: Option<&str>, class: &str) -> Node {
    Node {
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
        ..Default::default()
    }
}

const ALPHA: usize = 2;
const BRAVO: usize = 4;

/// Emit the authored rule for `subject`, then carry the markers it named.
fn generate(subject: usize, rule: &str) -> (Vec<String>, BTreeMap<String, String>) {
    let (nodes, classes) = tree();
    let mut compounds = BTreeSet::new();
    let emitted = {
        let scope = Scope::new(&nodes, &classes, "r");
        super::authored_conditions::rules(
            &nodes[subject],
            &scope,
            &[rule.to_string()],
            &mut compounds,
        )
        .iter()
        .map(super::authored_conditions::Emitted::text)
        .collect::<Vec<_>>()
    };
    let mut classes = classes;
    apply(&compounds, &nodes, "r", &mut classes);
    (emitted, classes)
}

const SCOPED_RULE: &str =
    "@media (prefers-color-scheme: dark) { .theme .card { color: rgb(0, 128, 0); } }";

fn tokens(classes: &BTreeMap<String, String>, index: usize) -> Vec<String> {
    let (nodes, _) = tree();
    classes[&nodes[index].path]
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// The defect. Every class token the rewritten selector names is also carried by the
/// element outside the scope, so the rule the author scoped paints the control too.
///
/// This is asserted on token identity between the rule and the control's own classes, which
/// is decidable from the two emitted files and needs no rendering.
#[test]
fn keeps_a_scoped_rule_off_the_look_alike_outside_the_scope() {
    let (rules, classes) = generate(ALPHA, SCOPED_RULE);
    let [rule] = rules.as_slice() else {
        panic!("{rules:?}");
    };
    let selector = rule
        .split_once('{')
        .and_then(|(_, rest)| rest.split_once('{'))
        .map(|(selector, _)| selector.to_string())
        .expect("a rewritten selector");

    let subject = selector.rsplit(' ').next().expect("a subject compound");
    let ancestor = selector.split(' ').next().expect("an ancestor compound");
    assert!(
        tokens(&classes, ALPHA).contains(&subject.trim_start_matches('.').to_string()),
        "the scoped card must answer to the rule: {selector} against {:?}",
        tokens(&classes, ALPHA)
    );
    assert!(
        !tokens(&classes, BRAVO).contains(&subject.trim_start_matches('.').to_string())
            || !tokens(&classes, 3).contains(&ancestor.trim_start_matches('.').to_string()),
        "the rule reaches the card the author excluded: {selector} against {:?} under {:?}",
        tokens(&classes, BRAVO),
        tokens(&classes, 3)
    );
}

/// The scoping wrapper is the end that collides most easily, because the theming idiom puts
/// the token on an element with no visual styles of its own. Both wrappers here share one
/// paint class, so only a marker can separate them.
#[test]
fn marks_the_scoping_wrapper_and_not_its_undecorated_twin() {
    let (_, classes) = generate(ALPHA, SCOPED_RULE);
    let theme = marker("r", ".theme");

    assert!(
        tokens(&classes, 1).contains(&theme),
        "{:?}",
        tokens(&classes, 1)
    );
    assert!(
        !tokens(&classes, 3).contains(&theme),
        "{:?}",
        tokens(&classes, 3)
    );
}

/// Identity is added, never substituted. Both cards must keep the one shared paint class,
/// or the repair has bought correctness by deleting the deduplication.
#[test]
fn leaves_both_look_alikes_sharing_one_paint_class() {
    let (_, classes) = generate(ALPHA, SCOPED_RULE);

    assert_eq!(tokens(&classes, ALPHA)[0], CARD);
    assert_eq!(tokens(&classes, BRAVO)[0], CARD);
    assert_eq!(tokens(&classes, 1)[0], WRAPPER);
    assert_eq!(tokens(&classes, 3)[0], WRAPPER);
}

/// A compound names every element it matches, so one marker serves both cards and the rule
/// is emitted once. Marking per node instead would emit one rule per element and reintroduce
/// on the sheet the growth the paint class exists to prevent.
#[test]
fn gives_look_alikes_inside_and_outside_the_scope_one_subject_marker() {
    let (_, classes) = generate(ALPHA, SCOPED_RULE);
    let card = marker("r", ".card");

    assert!(tokens(&classes, ALPHA).contains(&card));
    assert!(tokens(&classes, BRAVO).contains(&card));
}

/// A selector of one compound expresses no relationship, so it needs no identity and must
/// mint no marker. Without this every page would grow, not just the ones being scoped.
#[test]
fn adds_no_marker_for_a_selector_carrying_no_combinator() {
    let (rules, classes) = generate(
        ALPHA,
        "@media (prefers-color-scheme: dark) { .card { color: rgb(0, 128, 0); } }",
    );

    assert_eq!(
        rules,
        vec![format!(
            "@media (prefers-color-scheme: dark){{.{CARD}{{color: rgb(0, 128, 0);}}}}"
        )]
    );
    assert_eq!(classes, tree().1);
}
