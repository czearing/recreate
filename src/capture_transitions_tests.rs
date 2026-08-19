//! The invariant every reader of a resting page depends on, stated without naming a property:
//! when computed style is read, no transition is in flight, so every value read is a value the
//! page rests at.
//!
//! A transition on a paint-only property moves no geometry, so no stillness signature built
//! from boxes can see it and no downstream stage can tell an interpolated value from a written
//! one. Worse, a transition that runs from load leaves the property at its initial value, which
//! is exactly what an unauthored property looks like, so the declaration is not merely wrong,
//! it is pruned and gone.

use crate::node_eval;

/// A page whose motion is only what the test says it is. `finish()` is the platform's own
/// contract — seek to the end of the active interval — so the double lands the property on the
/// value the transition was travelling to and stops being reported as running, which is what
/// makes "the record survives" a claim the tests can check rather than assume.
const DOUBLE: &str = r#"
globalThis.style = {};
globalThis.running = [];
class Animation {
  constructor(name, property, to, endless) {
    this.name = name; this.property = property; this.to = to; this.endless = endless;
    globalThis.running.push(this);
  }
  finish() {
    if (this.endless) throw new Error('unresolved end time');
    globalThis.style[this.property] = this.to;
    globalThis.running = globalThis.running.filter(other => other !== this);
  }
}
class CSSTransition extends Animation {}
const root = { getAnimations: options => (options && options.subtree ? globalThis.running : []) };
const names = () => globalThis.running.map(animation => animation.name);
class CSSStyleSheet {
  replaceSync(text) { this.text = text; }
}
globalThis.CSSStyleSheet = CSSStyleSheet;
const document = {
  querySelectorAll: () => [],
  styleSheets: [],
  adoptedStyleSheets: [],
  getAnimations: options => root.getAnimations(options)
};
// What the page is being read under, named as the rule texts in force rather than as the
// carrier that delivers them, so the assertions survive a change of carrier.
Object.defineProperty(globalThis, 'sheets', {
  get: () => document.adoptedStyleSheets.map(sheet => sheet.text)
});
"#;

fn evaluate(body: &str, expression: &str) -> serde_json::Value {
    node_eval::evaluate(
        &format!(
            "{DOUBLE}\n{}\n{}\n{body}",
            crate::scoped_rules::SOURCE,
            crate::capture_transitions::SOURCE
        ),
        expression,
    )
}

/// The defect itself. A value still travelling is not a value the page rests at, and the only
/// reading that is safe is one taken after it has arrived.
#[test]
fn every_transition_in_flight_reaches_its_resting_value() {
    let read = evaluate(
        "new CSSTransition('paint', 'background-color', 'rgb(0, 0, 255)');\
         \nnew CSSTransition('other', 'opacity', '0.25');\
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
        "new CSSTransition('f', 'filter', 'blur(4px)');\narriveTransitions(root);",
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
        "new Animation('keyframed', 'opacity', '1');\narriveTransitions(root);",
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
        "new CSSTransition('endless', 'color', 'red', true);\
         \nnew CSSTransition('paint', 'opacity', '0.5');\
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
         new CSSTransition('provoked', 'color', 'red'); });",
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
        "new CSSTransition('entry', 'opacity', '1');\nmovingRead(() => {});",
        "[globalThis.sheets, globalThis.style, names()]",
    );
    assert_eq!(seen, serde_json::json!([[], {}, ["entry"]]));
}

/// Which reader gets which policy. Every settled viewport reading and every reading taken after
/// an interaction is a resting one; the first-paint reading is the only moving one.
#[test]
fn each_reader_is_wired_to_the_policy_its_promise_requires() {
    let settled = crate::page_script::source_without_assets();
    let moment = crate::page_script::source_at_first_paint();
    assert!(settled.contains("restingRead(() => measureBaselines"));
    assert!(moment.contains("movingRead(() => measureBaselines"));
    assert!(!moment.contains("restingRead(() => measureBaselines"));
    assert!(crate::interaction_script::source().contains("restingRead(() => measureBaselines"));
}
