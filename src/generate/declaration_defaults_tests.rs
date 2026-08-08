use super::remove_defaults;
use crate::model::Styles;

fn styles(pairs: &[(&str, &str)]) -> Styles {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect()
}

/// `visibility` is inherited and re-overridable, so a badge inside a hidden panel
/// stays on screen only because it declares `visibility: visible`. That declaration
/// looks exactly like the initial value, which is what overriding back to the default
/// has to look like — dropping it makes the badge inherit `hidden` and disappear.
#[test]
fn keeps_an_inherited_declaration_that_overrides_a_non_initial_parent() {
    let mut badge = styles(&[("visibility", "visible")]);
    let panel = styles(&[("visibility", "hidden")]);

    remove_defaults(&mut badge, Some(&panel));

    assert_eq!(badge.get("visibility").map(String::as_str), Some("visible"));
}

/// The defect is a class, not a `visibility` special case: every inherited property
/// can be overridden back to its initial value against a non-initial ancestor.
#[test]
fn keeps_every_inherited_override_against_a_non_initial_parent() {
    for (name, initial, ancestor) in [
        ("text-transform", "none", "uppercase"),
        ("white-space", "normal", "pre"),
        ("word-break", "normal", "break-all"),
        ("cursor", "auto", "pointer"),
        ("font-style", "normal", "italic"),
        ("pointer-events", "auto", "none"),
        ("border-collapse", "separate", "collapse"),
        ("text-rendering", "auto", "optimizeLegibility"),
    ] {
        let mut child = styles(&[(name, initial)]);
        let parent = styles(&[(name, ancestor)]);

        remove_defaults(&mut child, Some(&parent));

        assert_eq!(
            child.get(name).map(String::as_str),
            Some(initial),
            "{name} override against {ancestor} was pruned"
        );
    }
}

/// The positive control. `position` does not inherit, so an undeclared `position`
/// starts over at `static` no matter what the parent set — pruning it is correct and
/// must keep happening. A fix that disabled the table wholesale would pass the tests
/// above and fail this one.
#[test]
fn still_prunes_a_non_inherited_default_under_a_non_initial_parent() {
    let mut child = styles(&[("position", "static"), ("z-index", "auto")]);
    let parent = styles(&[("position", "relative"), ("z-index", "5")]);

    remove_defaults(&mut child, Some(&parent));

    assert!(child.is_empty(), "{child:?}");
}

/// An inherited declaration that merely restates what the parent already computes is
/// genuinely redundant, so the parent diff must still drop it. Without this the fix
/// would trade a dropped override for bloated output.
#[test]
fn prunes_an_inherited_declaration_that_matches_the_parent() {
    let mut child = styles(&[("text-transform", "uppercase"), ("visibility", "hidden")]);
    let parent = styles(&[("text-transform", "uppercase"), ("visibility", "hidden")]);

    remove_defaults(&mut child, Some(&parent));

    assert!(child.is_empty(), "{child:?}");
}

/// The root has no parent, so its inheritance chain terminates in the initial value.
/// An inherited property equal to that value is redundant there and must still go.
#[test]
fn prunes_an_inherited_default_at_the_root_where_there_is_no_parent() {
    let mut root = styles(&[("visibility", "visible"), ("text-transform", "none")]);

    remove_defaults(&mut root, None);

    assert!(root.is_empty(), "{root:?}");
}

/// A parent that never declares the property computes the initial value for it, so a
/// child restating the initial value is still redundant.
#[test]
fn prunes_an_inherited_default_when_the_parent_leaves_it_at_the_initial_value() {
    let mut child = styles(&[("visibility", "visible")]);
    let parent = styles(&[("color", "rgb(0, 0, 0)")]);

    remove_defaults(&mut child, Some(&parent));

    assert!(child.is_empty(), "{child:?}");
}

/// Two elements with byte-identical styles emit different declarations when their
/// parents disagree on an inherited property, so a dedupe keyed on style alone would
/// merge them and give one the other's rule. The signature fragment must separate them.
#[test]
fn separates_identical_styles_whose_parents_differ_on_an_inherited_property() {
    let uppercase = styles(&[("text-transform", "uppercase")]);
    let plain = styles(&[("text-transform", "none")]);

    assert_ne!(
        super::inherited_context(Some(&uppercase)),
        super::inherited_context(Some(&plain))
    );
}

/// The fragment must ignore non-inherited properties, or every element under a
/// positioned parent gets its own class and the output stops deduping.
#[test]
fn ignores_non_inherited_parent_values_in_the_dedupe_fragment() {
    let relative = styles(&[("position", "relative"), ("z-index", "5")]);
    let statik = styles(&[("position", "static"), ("z-index", "auto")]);

    assert_eq!(
        super::inherited_context(Some(&relative)),
        super::inherited_context(Some(&statik))
    );
}

/// A non-inherited property is unaffected by the parent in either direction: a
/// non-default value is kept even when the parent happens to share it.
#[test]
fn keeps_a_non_default_value_that_the_parent_also_declares() {
    let mut child = styles(&[("position", "absolute")]);
    let parent = styles(&[("position", "absolute")]);

    remove_defaults(&mut child, Some(&parent));

    assert_eq!(child.get("position").map(String::as_str), Some("absolute"));
}
