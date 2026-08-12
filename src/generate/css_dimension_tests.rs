use super::units;

/// The defect this reader exists to end. A viewport length nested inside `calc()` is the
/// value that was lost, and it must be found wherever it sits in the value.
#[test]
fn a_viewport_length_is_read_through_the_function_that_contains_it() {
    assert_eq!(units("calc(var(--gutter) + 30vw)"), ["vw"]);
    assert_eq!(units("30vw"), ["vw"]);
    assert_eq!(units("calc(var(--gutter) + 30%)"), ["%"]);
}

/// The over-match that stops a substring scan from ever holding a unit. None of these spells
/// a dimension, because none of the unit letters follows a number.
#[test]
fn letters_that_merely_spell_a_unit_are_not_a_dimension() {
    assert!(
        units("calc(var(--vwrap) + 40px)")
            .iter()
            .all(|unit| *unit != "vw")
    );
    assert_eq!(units("var(--vwrap)"), Vec::<&str>::new());
    assert!(units("Vwide, sans-serif").is_empty());
}

/// A number that continues an identifier starts no token, so a name carrying digits cannot
/// invent a unit out of the letters that follow them.
#[test]
fn a_digit_inside_an_identifier_does_not_begin_a_dimension() {
    assert!(units("Roboto2Vwide").is_empty());
    assert_eq!(units("url(a%20b.png)"), ["b"]);
}

/// Quoted text is data. A font name may hold anything, including something shaped like a
/// dimension, and reading it as one would keep a value that is entirely static.
#[test]
fn a_dimension_spelled_inside_a_quoted_value_is_not_read() {
    assert_eq!(units("\"30vw\", sans-serif"), Vec::<&str>::new());
    assert_eq!(units("calc(10px + 5vh) \"30vw\""), ["px", "vh"]);
}

/// The number grammar decides where the unit starts, so a fraction and an exponent must not
/// leave digits behind to be mistaken for one.
#[test]
fn the_whole_number_is_consumed_before_the_unit_is_read() {
    assert_eq!(units("12.5vw"), ["vw"]);
    assert_eq!(units("1e3vw"), ["vw"]);
    assert_eq!(units("1.5e-2vmin"), ["vmin"]);
}
