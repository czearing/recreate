use super::declarations;

/// The top-level authoring form. The block wraps a style rule that carries its own selector.
#[test]
fn reads_the_top_level_form_through_the_selector_of_the_rule_it_wraps() {
    let rules = vec!["@starting-style{.panel { opacity: 0; translate: 0px 24px; }}".to_string()];
    assert_eq!(
        declarations(&rules),
        vec![(
            ".panel".to_string(),
            " opacity: 0; translate: 0px 24px; ".to_string()
        )]
    );
}

/// The nested authoring form, in the exact shape the capture records it: the block sits
/// inside a style rule, its declarations are bare, and the enclosing rule's prelude is the
/// only selector that reaches them. A reader keyed on the outermost prelude skips this
/// entirely and the entry motion for every keyword-initial property flattens.
#[test]
fn reads_the_nested_form_through_the_selector_of_the_rule_enclosing_it() {
    let rules = vec![
        concat!(
            ".card {\n  opacity: 1; translate: 0px; transition: translate 120ms linear;\n",
            "  @starting-style {\n  opacity: 0; translate: 0px 24px;\n}\n}"
        )
        .to_string(),
    ];
    let found = declarations(&rules);
    assert_eq!(found.len(), 1, "expected exactly one block: {found:?}");
    assert_eq!(found[0].0, ".card");
    assert!(
        found[0].1.contains("translate: 0px 24px"),
        "nested declarations were not read: {found:?}"
    );
}

/// The enclosing rule's own declarations are not before-change style. Returning them would
/// seed every opening frame with the resting value and erase the movement.
#[test]
fn excludes_the_enclosing_rules_own_declarations() {
    let rules = vec![
        ".card {\n  translate: 0px;\n  @starting-style {\n  translate: 0px 24px;\n}\n}".to_string(),
    ];
    let found = declarations(&rules);
    assert!(
        !found[0].1.contains("translate: 0px;"),
        "the resting declaration leaked into the before-change set: {found:?}"
    );
}

/// A grouping at-rule names no element, so descending through one must leave the selector
/// context alone rather than adopting its prelude as a selector.
#[test]
fn keeps_the_selector_context_across_a_grouping_at_rule() {
    let rules = vec![
        "@media (min-width: 1px){.card {\n  @starting-style {\n  opacity: 0;\n}\n}}".to_string(),
    ];
    let found = declarations(&rules);
    assert_eq!(found.len(), 1, "expected exactly one block: {found:?}");
    assert_eq!(found[0].0, ".card");
}

/// Declarations may sit on either side of a nested rule, so they are collected rather than
/// sliced up to the first brace.
#[test]
fn collects_declarations_written_on_both_sides_of_a_nested_rule() {
    let rules = vec![
        "@starting-style{.card { opacity: 0; .inner { scale: 2 } translate: 0px 24px; }}"
            .to_string(),
    ];
    let found = declarations(&rules);
    let card = found
        .iter()
        .find(|(selector, _)| selector == ".card")
        .unwrap_or_else(|| panic!("no .card entry: {found:?}"));
    assert!(card.1.contains("opacity: 0"), "{found:?}");
    assert!(card.1.contains("translate: 0px 24px"), "{found:?}");
    assert!(
        found
            .iter()
            .any(|(selector, body)| selector == ".inner" && body.contains("scale: 2")),
        "the rule nested inside the block was dropped: {found:?}"
    );
}

/// A rule with no `@starting-style` anywhere yields nothing and costs one substring test,
/// which is every page but the rare one that authors the construct.
#[test]
fn yields_nothing_for_a_page_that_authors_no_before_change_style() {
    let rules = vec![
        ".card { opacity: 1 }".to_string(),
        "@media screen{.a{b:c}}".to_string(),
    ];
    assert!(declarations(&rules).is_empty());
}

/// The defect, end to end: the nested authoring form must seed the opening frame exactly as
/// the top-level form does. Asserted through the emitted CSS rather than the reader alone,
/// because a reader that returns the declarations is worthless if nothing joins them to an
/// animation.
#[test]
fn seeds_the_opening_frame_from_the_nested_authoring_form() {
    use super::super::before_change_fixture::{emit, entry_animation, panel};
    let css = emit(
        &[panel()],
        &super::super::before_change_fixture::nested_authored_rules(),
        &[entry_animation()],
    );
    assert!(
        css.contains("translate:0px 24px"),
        "nested form lost the authored start distance: {css}"
    );
    assert!(
        !css.contains("translate:none"),
        "nested form still carries the initial keyword: {css}"
    );
    assert!(
        css.contains("clip-path:inset(40px)"),
        "nested form lost the authored clip-path start: {css}"
    );
}

/// A grouping at-rule written *inside* the style rule is the shape that separates keeping
/// the selector context from adopting each prelude in turn: the enclosing .card is the
/// only selector present, and taking the @media prelude instead attributes the
/// declarations to a selector that names no element.
#[test]
fn keeps_the_enclosing_selector_across_a_grouping_rule_nested_inside_it() {
    let rules =
        vec![concat!(
        ".card {\n  translate: 0px;\n",
        "  @media (min-width: 1px) {\n    @starting-style {\n  translate: 0px 24px;\n}\n  }\n}"
    )
    .to_string()];
    let found = declarations(&rules);
    assert_eq!(found.len(), 1, "expected exactly one block: {found:?}");
    assert_eq!(found[0].0, ".card");
}

/// Declarations no selector reaches are attributed to nothing. An empty selector is not a
/// universal one: directly_targets_node requires no class of a compound that names none,
/// so admitting the empty context would seed every animated element on the page with one
/// stray block's start values.
#[test]
fn attributes_nothing_when_no_selector_reaches_the_declarations() {
    let rules = vec!["@starting-style{ translate: 0px 24px; }".to_string()];
    assert!(
        declarations(&rules).is_empty(),
        "an unreachable block was given a selector: {:?}",
        declarations(&rules)
    );
}
