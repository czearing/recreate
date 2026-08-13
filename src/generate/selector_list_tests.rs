use super::selector_list::{members, static_member};
use crate::model::{Attributes, Node, Rect};

fn node(classes: &str) -> Node {
    Node {
        writing_mode: Default::default(),
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
        before: None,
        after: None,
    }
}

fn media(node: &Node, rules: &[&str]) -> Vec<String> {
    let rules = rules
        .iter()
        .map(|rule| (*rule).to_string())
        .collect::<Vec<_>>();
    let nodes = [node.clone()];
    let classes = std::collections::BTreeMap::from([(node.path.clone(), "generated".to_string())]);
    super::authored_conditions::rules(
        node,
        &super::selector_scope::Scope::new(&nodes, &classes, "r"),
        &rules,
        &mut std::collections::BTreeSet::new(),
    )
}

/// A selector list is a logical OR of independent selectors, so a member that carries no
/// state at all keeps applying when a sibling member does. Judging the list by whether any
/// character in it is a colon discards the static member along with the stateful one.
#[test]
fn keeps_the_static_member_of_a_list_that_also_carries_state() {
    let rules = media(
        &node("dropped"),
        &["@media (max-width: 733px) { .dropped, .dropped:hover { padding-left: 40px; } }"],
    );

    assert_eq!(
        rules,
        vec!["@media (max-width: 733px){.generated{padding-left: 40px;}}".to_string()]
    );
}

/// The stateful member belongs to the state pipeline. Emitting it here would apply a hover
/// declaration at rest, because the rewritten selector no longer carries the state test.
#[test]
fn never_emits_a_stateful_member() {
    let rules = media(
        &node("dropped"),
        &["@media (max-width: 733px) { .dropped:hover { padding-left: 40px; } }"],
    );

    assert!(rules.is_empty(), "{rules:?}");
}

/// Every member is stateful, so nothing survives and the rule is discarded whole - the
/// behaviour the colon test was reaching for, now expressed per member.
#[test]
fn discards_a_list_whose_members_are_all_stateful() {
    let rules = media(
        &node("card"),
        &["@media (max-width: 733px) { .card:hover, .card:focus { color: red; } }"],
    );

    assert!(rules.is_empty(), "{rules:?}");
}

/// The generated class is shared by every node with the same computed-style signature, so a
/// positional selector rewritten onto it would reach siblings that do not occupy that
/// position. Structural is not the same as class-expressible.
#[test]
fn discards_structural_pseudo_classes_and_pseudo_elements() {
    for selector in [".card:first-child", ".card:not(.wide)", ".card::before"] {
        let rules = media(
            &node("card"),
            &[&format!(
                "@media (max-width: 733px) {{ {selector} {{ color: red; }} }}"
            )],
        );

        assert!(rules.is_empty(), "{selector} -> {rules:?}");
    }
}

/// `:is()` and `:where()` say nothing about state: they match on structure alone, so the
/// compound they wrap is fully expressed by the facts the generated class already encodes.
#[test]
fn keeps_a_member_wrapped_in_a_forgiving_pseudo_class() {
    let rules = media(
        &node("root size-medium"),
        &["@media (max-width: 733px) { .root:where(.size-medium) { padding: 4px; } }"],
    );

    assert_eq!(
        rules,
        vec!["@media (max-width: 733px){.generated{padding: 4px;}}".to_string()]
    );
}

/// A selector list is separated by top-level commas only. Cutting a comma inside `:is()`
/// leaves the fragment `.tall)`, which reads as a bare class and would match a node that
/// the whole member does not, so a naive split can admit a selector the correct split
/// rejects.
#[test]
fn splits_only_at_top_level_commas() {
    assert_eq!(
        members(".card:is(.wide, .tall), .rail").collect::<Vec<_>>(),
        vec![".card:is(.wide, .tall)", ".rail"]
    );
    assert_eq!(
        members("[title=\"a,b\"], .rail").collect::<Vec<_>>(),
        vec!["[title=\"a,b\"]", ".rail"]
    );
    assert_eq!(members(".only").collect::<Vec<_>>(), vec![".only"]);
}

/// A wrapper holding a list would have to expand into several selectors to stay correct, so
/// the member is left alone rather than flattened into something narrower than it matches.
#[test]
fn refuses_to_flatten_a_wrapper_holding_a_list() {
    assert_eq!(static_member(".card:is(.wide, .tall)"), None);
    assert_eq!(static_member(".card:where(.a)").as_deref(), Some(".card.a"));
    assert_eq!(static_member(".card").as_deref(), Some(".card"));
    assert_eq!(static_member(".card:hover"), None);
}

/// A quoted attribute value is delimited by its quotes, so Selectors 4 lets it hold any
/// character including a colon — only an unquoted value would need escaping. Such a member
/// names no pseudo-class and no state; it is the same exact-value test the generated class
/// already encodes, so refusing it drops an ordinary authored rule whole.
#[test]
fn keeps_a_colon_inside_a_quoted_attribute_value() {
    let mut node = node("slot");
    node.attributes.insert("data-when".into(), "09:00".into());

    assert!(
        matches!(
            static_member("[data-when=\"09:00\"]"),
            Some(std::borrow::Cow::Borrowed("[data-when=\"09:00\"]"))
        ),
        "a member the grammar finds no colon in is returned untouched, never rewritten"
    );
    assert_eq!(
        media(
            &node,
            &["@media (max-width: 733px) { [data-when=\"09:00\"] { color: red; } }"]
        ),
        vec!["@media (max-width: 733px){.generated{color: red;}}".to_string()]
    );
}

/// The authored-value index carried its own copy of this rule and vetoed the same way, so a
/// list mixing a static member with a stateful one lost the authored value entirely.
#[test]
fn indexes_the_static_member_of_a_mixed_list() {
    let mut node = node("card");
    node.style.insert("padding".into(), "12px".into());
    let rules = [".card, .card:hover { padding: 12px; }".to_string()];

    let index = crate::generate::authored_css_index::Index::new(&rules);

    assert_eq!(index.authored_value(&node, "padding"), Some("12px".into()));
}
