use super::js_predicate;
use serde_json::{Value, json};

const HARNESS: &str = concat!(
    include_str!("dom_style_harness.js"),
    include_str!("blocking_overlay_harness.js")
);

/// An element under test, with the ancestors above it named outermost first.
fn entry(ancestors: Value, style: Value, width: f64, height: f64) -> Value {
    json!({
        "ancestors": ancestors,
        "style": style,
        "rect": { "width": width, "height": height }
    })
}

/// A covering element that satisfies every clause, so each case below can disable exactly
/// one of them and show which clause carried the verdict.
fn covering(ancestors: Value, overrides: Value) -> Value {
    let mut style = json!({
        "position": "fixed",
        "z-index": "100",
        "pointer-events": "auto"
    });
    for (name, value) in overrides.as_object().unwrap() {
        style[name] = value.clone();
    }
    entry(ancestors, style, 1000.0, 1000.0)
}

/// The named cases, each paired with the verdict the rule must reach.
fn cases() -> Vec<(&'static str, Value, bool)> {
    vec![
        (
            "covers the viewport above the page",
            covering(json!([]), json!({})),
            true,
        ),
        (
            "stacked below the page",
            covering(json!([]), json!({ "z-index": "10" })),
            false,
        ),
        (
            "exactly at the stacking threshold",
            covering(json!([]), json!({ "z-index": "50" })),
            true,
        ),
        (
            "no stacking level at all",
            covering(json!([]), json!({ "z-index": "auto" })),
            false,
        ),
        (
            "left in normal flow",
            covering(json!([]), json!({ "position": "static" })),
            false,
        ),
        (
            "absolutely positioned",
            covering(json!([]), json!({ "position": "absolute" })),
            true,
        ),
        // The clause the startup-node copy had lost: a curtain that passes pointer input
        // through is not blocking, and selecting it as startup content captured a layer the
        // page had already finished with.
        (
            "passes pointer input through",
            covering(json!([]), json!({ "pointer-events": "none" })),
            false,
        ),
        (
            "not displayed",
            covering(json!([]), json!({ "display": "none" })),
            false,
        ),
        (
            "not visible",
            covering(json!([]), json!({ "visibility": "hidden" })),
            false,
        ),
        (
            "not visible because its opacity is zero",
            covering(json!([]), json!({ "opacity": "0" })),
            false,
        ),
        // The defect: a parked dialog hides its whole subtree, and the backdrop inside it
        // declares nothing of its own. Five clauses hold; the page shows no curtain.
        (
            "hidden by an ancestor rather than by itself",
            covering(json!([{ "visibility": "hidden" }]), json!({})),
            false,
        ),
        // The same shape one property along. `opacity` does not inherit, so the backdrop
        // computes `1` here and only the ancestor walk can answer.
        (
            "faded out by an ancestor rather than by itself",
            covering(json!([{ "opacity": "0" }]), json!({})),
            false,
        ),
        (
            "inside an ancestor that is not displayed",
            covering(json!([{ "display": "none" }]), json!({})),
            false,
        ),
        // A hidden ancestor higher up must not blind the rule to a curtain that a nearer
        // ancestor puts back on screen. `visibility` inherits, so a descendant can re-show
        // itself, and the capture browser agrees this element is visible.
        (
            "under an ancestor chain that hides nothing",
            covering(
                json!([{ "visibility": "hidden" }, { "visibility": "visible" }]),
                json!({}),
            ),
            true,
        ),
        // `opacity` is the opposite: it composites the subtree away and no descendant can
        // undo it, so the same chain shape reaches the opposite verdict. Reading either
        // property the way the other resolves is wrong in one direction or the other, which
        // is the whole reason the rule asks the engine rather than the declarations.
        (
            "faded out above an ancestor that sets its own opacity back",
            covering(json!([{ "opacity": "0" }, { "opacity": "1" }]), json!({})),
            false,
        ),
        (
            "covers only part of the viewport",
            entry(
                json!([]),
                json!({ "position": "fixed", "z-index": "100" }),
                1000.0,
                500.0,
            ),
            false,
        ),
        (
            "exactly at the area threshold",
            entry(
                json!([]),
                json!({ "position": "fixed", "z-index": "100" }),
                1000.0,
                900.0,
            ),
            true,
        ),
    ]
}

/// Runs the shipped rule over the fixture in Node, so the rule the page receives is the one
/// under test and no browser is launched.
fn verdicts_at(width: u32, height: u32, fixture: &Value) -> Vec<bool> {
    let script = format!(
        "globalThis.innerWidth = {width}; globalThis.innerHeight = {height};\n\
         {HARNESS}\n\
         const blocking = {predicate};\n\
         console.log(JSON.stringify(buildOverlayFixture({fixture}).map(blocking)));",
        predicate = js_predicate(),
        fixture = fixture
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("overlay.js");
    std::fs::write(&path, script).unwrap();
    let output = std::process::Command::new("node")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "overlay predicate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn verdicts(fixture: &Value) -> Vec<bool> {
    verdicts_at(1000, 1000, fixture)
}

#[test]
fn the_rule_reaches_the_stated_verdict_for_every_case() {
    let cases = cases();
    let fixture = Value::Array(cases.iter().map(|(_, entry, _)| entry.clone()).collect());
    let reached = verdicts(&fixture);
    assert_eq!(reached.len(), cases.len());
    for (index, (name, _, expected)) in cases.iter().enumerate() {
        assert_eq!(reached[index], *expected, "disagreed for {name}");
    }
}

/// A viewport-relative rule must not be satisfied by absolute size, or a page captured at a
/// narrow viewport would report every wide element as a curtain.
#[test]
fn coverage_is_measured_against_the_viewport_and_not_absolute_size() {
    let full = Value::Array(vec![covering(json!([]), json!({}))]);
    assert_eq!(verdicts(&full), vec![true]);
    assert_eq!(verdicts_at(2000, 2000, &full), vec![false]);
}
