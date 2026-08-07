use super::{is_blocking_overlay, js_predicate};
use crate::model::{Node, Viewport};
use serde_json::{Value, json};

fn viewport() -> Viewport {
    Viewport {
        width: 1000,
        height: 1000,
        dpr: 1.0,
    }
}

fn node(width: f64, height: f64, style: Value) -> Value {
    json!({
        "path": "0",
        "parent": null,
        "tag": "div",
        "text": "",
        "attributes": {},
        "rect": { "x": 0.0, "y": 0.0, "width": width, "height": height },
        "style": style,
        "before": null,
        "after": null
    })
}

/// A covering element that satisfies every clause, so each case below can disable exactly
/// one of them and show which clause carried the verdict.
fn covering(overrides: Value) -> Value {
    let mut style = json!({
        "position": "fixed",
        "z-index": "100",
        "pointer-events": "auto",
        "display": "block",
        "visibility": "visible"
    });
    for (name, value) in overrides.as_object().unwrap() {
        style[name] = value.clone();
    }
    node(1000.0, 1000.0, style)
}

/// The named cases, each paired with the verdict the rule must reach.
fn cases() -> Vec<(&'static str, Value, bool)> {
    vec![
        (
            "covers the viewport above the page",
            covering(json!({})),
            true,
        ),
        (
            "stacked below the page",
            covering(json!({ "z-index": "10" })),
            false,
        ),
        (
            "exactly at the stacking threshold",
            covering(json!({ "z-index": "50" })),
            true,
        ),
        (
            "no stacking level at all",
            covering(json!({ "z-index": "auto" })),
            false,
        ),
        (
            "left in normal flow",
            covering(json!({ "position": "static" })),
            false,
        ),
        (
            "absolutely positioned",
            covering(json!({ "position": "absolute" })),
            true,
        ),
        // The clause the startup-node copy had lost: a curtain that passes pointer input
        // through is not blocking, and selecting it as startup content captured a layer the
        // page had already finished with.
        (
            "passes pointer input through",
            covering(json!({ "pointer-events": "none" })),
            false,
        ),
        (
            "not displayed",
            covering(json!({ "display": "none" })),
            false,
        ),
        (
            "not visible",
            covering(json!({ "visibility": "hidden" })),
            false,
        ),
        (
            "covers only part of the viewport",
            node(
                1000.0,
                500.0,
                json!({
                    "position": "fixed", "z-index": "100"
                }),
            ),
            false,
        ),
        (
            "exactly at the area threshold",
            node(
                1000.0,
                900.0,
                json!({
                    "position": "fixed", "z-index": "100"
                }),
            ),
            true,
        ),
    ]
}

/// Runs the shipped JavaScript rendering of the rule over the same fixture.
fn js_verdicts(fixture: &Value) -> Vec<bool> {
    let script = format!(
        "globalThis.innerWidth = 1000; globalThis.innerHeight = 1000;\n\
         globalThis.getComputedStyle = element => new Proxy({{}}, {{ get: (_, name) =>\n\
           element.declarations[String(name).replace(/[A-Z]/g, c => '-' + c.toLowerCase())]\n\
         }});\n\
         const blocking = {predicate};\n\
         console.log(JSON.stringify({fixture}.map(entry => blocking({{\n\
           getBoundingClientRect: () => entry.rect, declarations: entry.style\n\
         }}))));",
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

/// Both renderings must reach the stated verdict. Asserting agreement alone would pass if
/// the two copies drifted together, and asserting the expected verdicts alone would let one
/// runtime drift unnoticed, so the test asserts both.
#[test]
fn the_rule_reaches_the_same_verdict_in_both_runtimes() {
    let cases = cases();
    let fixture = Value::Array(cases.iter().map(|(_, node, _)| node.clone()).collect());
    let js = js_verdicts(&fixture);
    assert_eq!(js.len(), cases.len());
    for (index, (name, value, expected)) in cases.iter().enumerate() {
        let parsed: Node = serde_json::from_value(value.clone()).unwrap();
        let rust = is_blocking_overlay(&parsed, &viewport());
        assert_eq!(rust, *expected, "rust disagreed for {name}");
        assert_eq!(js[index], *expected, "javascript disagreed for {name}");
    }
}

/// A viewport-relative rule must not be satisfied by absolute size, or a page captured at a
/// narrow viewport would report every wide element as a curtain.
#[test]
fn coverage_is_measured_against_the_viewport_and_not_absolute_size() {
    let full = covering(json!({}));
    let parsed: Node = serde_json::from_value(full).unwrap();
    let larger = Viewport {
        width: 2000,
        height: 2000,
        dpr: 1.0,
    };
    assert!(is_blocking_overlay(&parsed, &viewport()));
    assert!(!is_blocking_overlay(&parsed, &larger));
}
