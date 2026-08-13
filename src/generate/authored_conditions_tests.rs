use super::*;
use crate::model::{Attributes, Rect};

#[path = "authored_conditions_carrier_tests.rs"]
mod carrier;

fn node(classes: &str) -> Node {
    Node {
        writing_mode: Default::default(),
        scrollbar_gutter: 0.0,
        blocking_overlay: false,
        disabled: false,
        rtl: false,
        path: String::new(),
        parent: None,
        tag: "div".into(),
        text: String::new(),
        attributes: Attributes::from([("class".into(), classes.into())]),
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

fn emitted(node: &Node, captured: &[String]) -> Vec<String> {
    let nodes = [node.clone()];
    let classes = std::collections::BTreeMap::from([(node.path.clone(), "generated".to_string())]);
    rules(
        node,
        &Scope::new(&nodes, &classes, "r"),
        captured,
        &mut BTreeSet::new(),
    )
}
#[test]
fn remaps_direct_authored_media_rules_to_generated_classes() {
    let node = node("rail card");
    let captured = vec![
        "@media (max-width: 1023px) { .rail { padding: 0 40px; } }".into(),
        "@media (max-width: 479px) { .card { grid-template-columns: 1fr; } }".into(),
        "@media (max-width: 479px) { .card:hover { color: red; } }".into(),
    ];
    let rules = emitted(&node, &captured);

    assert_eq!(rules.len(), 2);
    assert!(rules[0].contains(".generated"));
    assert!(rules.iter().any(|rule| rule.contains("padding: 0 40px")));
    assert!(
        rules
            .iter()
            .any(|rule| rule.contains("grid-template-columns: 1fr"))
    );
}

/// The defect. A brace inside a quoted value is data, but a body split on the byte cuts
/// the rule there: the declarations become an unterminated `font-family: "A`, the rest of
/// the rule is lost, and the emitted text carries a string that never closes. Because each
/// rule is written on its own line, the newline ends that string as a bad-string-token —
/// taking the two braces that should close the declaration block and the at-rule with it,
/// so every later rule in the stylesheet is absorbed into a colour-scheme condition.
#[test]
fn a_quoted_brace_neither_truncates_a_rule_nor_leaves_a_string_open() {
    let subject = node("alpha");
    let captured: Vec<String> = vec![
        r#"@media (prefers-color-scheme: dark) { .alpha { font-family: "A}B"; color: red; } }"#
            .into(),
    ];
    let rules = emitted(&subject, &captured);

    assert_eq!(rules.len(), 1);
    assert!(rules[0].contains(r#"font-family: "A}B""#), "{}", rules[0]);
    assert!(rules[0].contains("color: red"), "{}", rules[0]);
    assert_eq!(rules[0].matches('"').count() % 2, 0, "{}", rules[0]);
}

/// The control, one character apart. A bracket is untouched by a split on `}`, so the two
/// must already have agreed; asserting the pair is what proves the brace is the variable.
#[test]
fn a_quoted_bracket_is_treated_exactly_as_the_quoted_brace_is() {
    let brace = emitted(
        &node("alpha"),
        &[
            r#"@media (prefers-color-scheme: dark) { .alpha { font-family: "A}B"; color: red; } }"#
                .into(),
        ],
    );
    let bracket = emitted(
        &node("alpha"),
        &[
            r#"@media (prefers-color-scheme: dark) { .alpha { font-family: "A]B"; color: red; } }"#
                .into(),
        ],
    );
    assert_eq!(brace[0].replace('}', "]"), bracket[0].replace('}', "]"));
}

/// A media body holds more than one rule, and reading the body as a single block would
/// keep the first and silently drop the rest.
#[test]
fn every_rule_in_one_media_body_is_read() {
    let rules = emitted(
        &node("rail card"),
        &["@media (max-width: 99px) { .rail { color: red; } .card { color: blue; } }".into()],
    );
    assert_eq!(rules.len(), 2);
    assert!(rules.iter().any(|rule| rule.contains("color: red")));
    assert!(rules.iter().any(|rule| rule.contains("color: blue")));
}

/// The condition is read with the same scanner as the body. A fix that tokenized only the
/// body would still cut a rule whose condition quotes a brace.
#[test]
fn a_brace_quoted_in_the_condition_does_not_open_the_body_early() {
    let rules = emitted(
        &node("alpha"),
        &[r#"@media (min-width: 99px) and (font-family: "{") { .alpha { color: red; } }"#.into()],
    );
    assert_eq!(rules.len(), 1);
    assert!(rules[0].contains("color: red"), "{}", rules[0]);
    assert!(
        rules[0].starts_with(r#"@media (min-width: 99px) and (font-family: "{")"#),
        "{}",
        rules[0]
    );
}

/// A cascade layer is a carrier, so a media rule the author placed inside one reaches this
/// stage still wrapped in it. Testing only the outermost prelude answers a question about
/// the wrapper rather than about the rule, and drops it. The layer is settled elsewhere, so
/// this stage reads through it exactly as `css::global_rule` does — which is also why the
/// emitted text carries no `@layer`.
#[test]
fn claims_an_authored_media_rule_a_cascade_layer_wraps() {
    let node = node("subject");
    let captured = vec![
        "@layer theme { @media (min-width: 100000px) { .subject { letter-spacing: 13px; } } }"
            .into(),
    ];
    let rules = emitted(&node, &captured);

    assert_eq!(
        rules.len(),
        1,
        "dropped a layer-wrapped media rule: {rules:?}"
    );
    assert!(
        rules[0].starts_with("@media (min-width: 100000px)"),
        "did not lift the media condition to the front: {rules:?}"
    );
    assert!(
        rules[0].contains("letter-spacing: 13px") && rules[0].contains(".generated"),
        "lost the declaration or its remapped class: {rules:?}"
    );
    assert!(
        !rules[0].contains("@layer"),
        "re-emitted the cascade layer this stage does not own: {rules:?}"
    );
}
