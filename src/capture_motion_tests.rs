//! The invariant every reader of a resting page depends on, stated without naming a property or
//! a kind of motion: the style a reading records is the style the page computes when nothing is
//! in flight, whatever happens to be moving while the reading is taken.
//!
//! A transition on a paint-only property moves no geometry, so no stillness signature built
//! from boxes can see it and no downstream stage can tell an interpolated value from a written
//! one. Worse, a transition that runs from load leaves the property at its initial value, which
//! is exactly what an unauthored property looks like, so the declaration is not merely wrong,
//! it is pruned and gone.

/// A page whose motion is only what the test says it is. `finish()` is the platform's own
/// contract — seek to the end of the active interval — so the double lands the property on the
/// value the transition was travelling to and stops being reported as running, which is what
/// makes "the record survives" a claim the tests can check rather than assume.
///
/// `style` is what the cascade produced and `computed` is what a reader sees: an attached
/// effect answers for the property it drives, exactly as the animation origin outranks the
/// author origin on a real page, so a value read while motion applies is the frame and not the
/// resting value unless the read holds the motion out.
use crate::capture_motion_double::evaluate;

/// The defect itself. A value still travelling is not a value the page rests at, and the only
/// reading that is safe is one taken after it has arrived.
#[test]
fn every_transition_in_flight_reaches_its_resting_value() {
    let read = evaluate(
        "transition('paint', 'background-color', 'rgb(0, 0, 255)', {});\
         \ntransition('other', 'opacity', '0.25', {});\
         \narriveTransitions(root);",
        "[globalThis.style, names()]",
    );
    assert_eq!(
        read,
        serde_json::json!([
            { "background-color": "rgb(0, 0, 255)", "opacity": "0.25" },
            []
        ])
    );
}

/// Not a list of property names. The rule is what the two kinds of motion mean, so a property
/// the repair was never written against is carried by the same statement.
#[test]
fn a_property_the_rule_never_names_arrives_the_same_way() {
    let read = evaluate(
        "transition('f', 'filter', 'blur(4px)', {});\narriveTransitions(root);",
        "globalThis.style",
    );
    assert_eq!(read, serde_json::json!({ "filter": "blur(4px)" }));
}

/// Advancing a transition is not licence to advance an animation. An animation applies over the
/// base style rather than travelling towards it, so its end value is not a value the element
/// rests at. The separation is the platform's own type, so widening the rule to anything that
/// merely offers `finish()` fails here.
#[test]
fn an_animation_is_never_advanced_to_its_end() {
    let read = evaluate(
        "scripted('keyframed', 'opacity', { to: '1' });\narriveTransitions(root);",
        "[globalThis.style, names()]",
    );
    assert_eq!(read, serde_json::json!([{}, ["keyframed"]]));
}

/// A page with nothing in motion is not touched at all. A repair that wrote values rather than
/// advancing motion would show up here as work on a page that needed none.
#[test]
fn a_page_with_nothing_in_flight_is_left_alone() {
    let read = evaluate("arriveTransitions(root);", "[globalThis.style, names()]");
    assert_eq!(read, serde_json::json!([{}, []]));
}

/// A transition with no resolved end has no resting value to bring forward, and says so by
/// throwing. Skipping it is the whole response; letting it escape would abandon the rest of the
/// page mid-flight, which is the defect again for every element after it.
#[test]
fn a_transition_without_a_resting_value_does_not_abandon_the_rest() {
    let read = evaluate(
        "transition('endless', 'color', 'red', { endless: true });\
         \ntransition('paint', 'opacity', '0.5', {});\
         \narriveTransitions(root);",
        "[globalThis.style, names()]",
    );
    assert_eq!(read, serde_json::json!([{ "opacity": "0.5" }, ["endless"]]));
}

/// The measurement in the middle of a resting read is the page's largest source of transitions,
/// because reverting an element and putting its style attribute back are both style changes an
/// element that declares a transition answers by starting one. Suspending them for the duration
/// is what makes every value the pass reads, and every value read after it, a resting one; a
/// policy that only tidied up afterwards would still have measured baselines mid-flight.
#[test]
fn a_resting_read_measures_with_transitions_suspended() {
    let seen = evaluate(
        "restingRead(() => { globalThis.during = globalThis.sheets.slice(); \
         transition('provoked', 'color', 'red', {}); });",
        "[globalThis.during, globalThis.sheets, globalThis.style]",
    );
    assert_eq!(
        seen[0]
            .as_array()
            .and_then(|during| during.first())
            .and_then(|text| text.as_str())
            .map(|text| text.contains("*,*::before,*::after{transition-property:none !important}")),
        Some(true),
        "the read is taken with transitions declared away: {seen}"
    );
    assert_eq!(
        (&seen[1], &seen[2]),
        (
            &serde_json::json!([]),
            &serde_json::json!({ "color": "red" })
        ),
        "the suspension lasts exactly as long as the read: {seen}"
    );
}

/// A read of one moment leaves the page exactly as it found it, moving. The first-paint reading
/// is that read: every entry transition the page declares is in flight during it, and ending
/// those reads a later page than the one asked for and destroys the record of the entry motion.
#[test]
fn a_moving_read_leaves_the_page_moving() {
    let seen = evaluate(
        "transition('entry', 'opacity', '1', {});\nmovingRead(() => {});",
        "[globalThis.sheets, globalThis.style, names()]",
    );
    assert_eq!(seen, serde_json::json!([[], {}, ["entry"]]));
}
