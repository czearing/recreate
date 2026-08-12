use super::selector_list::static_member;

/// Quoted text is data, not grammar, and flattening does not change that. A wrapper removed
/// from around a compound leaves the quoted values beside it untouched, so a colon one of
/// them contains is no more the selector's own after flattening than it was before.
#[test]
fn keeps_a_quoted_colon_that_survives_flattening() {
    assert_eq!(
        static_member(".card:where(.a)[title=\"a:b\"]").as_deref(),
        Some(".card.a[title=\"a:b\"]")
    );
}

/// A value spelling out a forgiving wrapper must not be flattened as though it were one:
/// rewriting it would change the exact value the attribute matcher compares, so the rule
/// would go on to reach elements the author never named.
#[test]
fn never_flattens_a_wrapper_a_quoted_value_merely_spells() {
    assert_eq!(
        static_member(".card:where(.a)[data-x=\":is(y)\"]").as_deref(),
        Some(".card.a[data-x=\":is(y)\"]")
    );
}

/// A parenthesis inside a quoted value opens nothing. Counting it leaves a balanced wrapper
/// looking unterminated, which takes the whole member down rather than mangling it — the
/// same total loss, arrived at from the other direction.
#[test]
fn never_counts_a_parenthesis_a_quoted_value_merely_spells() {
    assert_eq!(
        static_member(".card:where([title=\"a(b\"])").as_deref(),
        Some(".card[title=\"a(b\"]")
    );
}

/// Flattening lifts a wrapper's argument into the compound, so a colon the argument hid is
/// argument hid is the selector's own once the wrapper is gone. Asking only before flattening
/// would admit a state test the rewritten class cannot carry.
#[test]
fn refuses_a_pseudo_class_a_forgiving_wrapper_hid() {
    assert_eq!(static_member(".card:where(:hover)"), None);
    assert_eq!(static_member(".card:is(:first-child)"), None);
}
