//! The tree every scoped-selector test resolves against, built once and shared.
//!
//! Keeping it in one place is what lets each test file stay cheap: the files run in
//! parallel, so a fixture copied into each of them is setup paid once per file.
use super::selector_marker::name as marker;
use super::selector_scope::Scope;
use crate::model::{Attributes, Node, Rect};
use std::collections::{BTreeMap, BTreeSet};

/// `.theme > .card` plus a second `.card` outside it.
///
/// Both cards compute the same style, so the generator gives them one shared class. That is
/// the point: it makes the emitted selector the only thing that can tell them apart, so a
/// rewrite that collapses to the subject alone cannot pass by accident.
pub(super) fn tree() -> (Vec<Node>, BTreeMap<String, String>) {
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
        rtl: false,
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

pub(super) fn emit(subject: usize, rule: &str) -> Vec<String> {
    let (nodes, classes) = tree();
    let scope = Scope::new(&nodes, &classes, "r");
    let mut compounds = BTreeSet::new();
    super::authored_media::rules(&nodes[subject], &scope, &[rule.to_string()], &mut compounds)
}

/// The selector a correct rewrite of these compounds produces: each authored compound
/// replaced by its own marker, the combinators between them untouched.
pub(super) fn scoped(compounds: &[&str], combinators: &[char]) -> String {
    let mut selector = format!(".{}", marker("r", compounds[0]));
    for (compound, combinator) in compounds[1..].iter().zip(combinators) {
        selector.push(*combinator);
        selector.push_str(&format!(".{}", marker("r", compound)));
    }
    selector
}
