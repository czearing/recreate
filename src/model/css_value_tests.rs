use super::components;

/// The ordinary case the shorthands rely on: whitespace separates components.
#[test]
fn separates_components_at_top_level_whitespace() {
    assert_eq!(components("5% 15%"), vec!["5%", "15%"]);
    assert_eq!(components("10%"), vec!["10%"]);
    assert_eq!(
        components("  1px   solid   red  "),
        vec!["1px", "solid", "red"]
    );
}

/// A value with no components at all must not invent one, or a shorthand would resolve an
/// empty declaration onto both edges of its axis.
#[test]
fn an_empty_value_has_no_components() {
    assert!(components("").is_empty());
    assert!(components("   ").is_empty());
}

/// The whole reason this is not `split_whitespace`. `calc(1rem + 2px)` is one value; read as
/// three, a one-value shorthand becomes a two-value one and the second edge receives `+`.
#[test]
fn whitespace_inside_a_function_does_not_separate_components() {
    assert_eq!(components("calc(1rem + 2px)"), vec!["calc(1rem + 2px)"]);
    assert_eq!(
        components("calc(1rem + 2px) 4px"),
        vec!["calc(1rem + 2px)", "4px"]
    );
    assert_eq!(
        components("minmax(min(10px, 2%), 1fr)"),
        vec!["minmax(min(10px, 2%), 1fr)"]
    );
}

/// Nesting is counted rather than merely detected, so an inner call closing does not release
/// the outer one and expose its spaces.
#[test]
fn a_closed_inner_call_does_not_reopen_the_outer_one() {
    assert_eq!(
        components("clamp(1px, calc(2px + 3px), 4px) 8px"),
        vec!["clamp(1px, calc(2px + 3px), 4px)", "8px"]
    );
}

/// An unbalanced value is authored rubbish, and the scanner must not panic or wrap around on
/// it. Treating the stray close as depth zero keeps the rest of the value readable.
#[test]
fn an_unbalanced_value_neither_panics_nor_wraps() {
    assert_eq!(components("a) b"), vec!["a)", "b"]);
    assert_eq!(components("calc(1px 2px"), vec!["calc(1px 2px"]);
}
