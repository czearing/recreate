//! Whether a replayed value progression comes to rest, driven against the shipped runtime.
//!
//! The runtime module is imported and executed here rather than searched for its own words,
//! and time is supplied by a virtual clock, so a scenario spanning simulated minutes runs in
//! milliseconds and never opens a browser.

use crate::node_eval;
use serde_json::Value;

/// Drives the shipped `runtime/sequence.mjs` against a fake element and a virtual clock.
///
/// The clock runs whatever the runtime schedules, up to a fixed number of steps, so a
/// sequence that never stops is distinguishable from one that does: a terminating sequence
/// leaves the queue empty with steps to spare, and a looping one exhausts the budget.
fn replay(sequence: &str, captured: &str, ticks: u32) -> Value {
    let module = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("runtime")
        .join("sequence.mjs")
        .display()
        .to_string()
        .replace('\\', "/");
    node_eval::evaluate(
        &format!(
            r#"import {{ startSequence }} from 'file:///{module}';
const seen = [];
const element = {{
  childNodes: [],
  textContent: {captured},
  get attributes() {{ return {{}}; }}
}};
Object.defineProperty(element, 'textContent', {{
  get: () => seen.at(-1),
  set: value => seen.push(value),
  configurable: true
}});
seen.push({captured});
let queued = null;
const clock = {{
  setTimeout: (handler, delay) => {{ queued = handler; return 1; }},
  clearTimeout: () => {{ queued = null; }}
}};
startSequence(element, {sequence}, clock);
let ran = 0;
while (queued && ran < {ticks}) {{ const next = queued; queued = null; next(); ran++; }}
const stopped = queued === null;
"#
        ),
        "({ seen, stopped })",
    )
}

/// The emitted shape for a progression whose values never came back round.
const ONE_SHOT: &str = r#"{
  attribute: 'textContent', repeats: false,
  steps: [
    { value: 'Draft', delay_ms: 300 },
    { value: 'Reviewing', delay_ms: 300 },
    { value: 'Final', delay_ms: 300 }
  ]
}"#;

/// The emitted shape for a progression observed to repeat, identical on every other axis.
const CYCLIC: &str = r#"{
  attribute: 'textContent', repeats: true,
  steps: [
    { value: 'Alpha', delay_ms: 300 },
    { value: 'Bravo', delay_ms: 300 },
    { value: 'Charlie', delay_ms: 300 }
  ]
}"#;

/// A progression captured at its last observed value has nothing left to play, so the
/// recreation must simply hold that value. Looping it walks forward into values observed
/// strictly BEFORE the capture: a finished answer reverting to its half-written prefix and
/// then to its skeleton, permanently.
#[test]
fn a_progression_that_never_repeated_rests_on_the_value_capture_caught() {
    let result = replay(ONE_SHOT, "'Final'", 12);
    assert_eq!(result["seen"], serde_json::json!(["Final"]));
    assert_eq!(result["stopped"], Value::Bool(true));
}

/// Capture can catch a one-shot midway. Those later values were genuinely observed after the
/// captured one, so playing them forward completes the progression — the repair is "stop at
/// the end", not "play nothing".
#[test]
fn a_progression_caught_midway_plays_forward_to_its_last_value_and_stops() {
    let result = replay(ONE_SHOT, "'Draft'", 12);
    assert_eq!(
        result["seen"],
        serde_json::json!(["Draft", "Reviewing", "Final"])
    );
    assert_eq!(result["stopped"], Value::Bool(true));
}

/// The positive control, and the reason a blunt "stop looping" is a regression rather than a
/// fix. This twin clears every capture gate identically to the one above and differs in one
/// property only, so nothing but the recorded repetition fact can separate them.
#[test]
fn a_progression_observed_to_repeat_still_loops_forever() {
    let result = replay(CYCLIC, "'Alpha'", 7);
    assert_eq!(
        result["seen"],
        serde_json::json!([
            "Alpha", "Bravo", "Charlie", "Alpha", "Bravo", "Charlie", "Alpha", "Bravo"
        ])
    );
    assert_eq!(result["stopped"], Value::Bool(false));
}

/// A sequence emitted before this fact was recorded carries no `repeats` key. Treating that
/// absence as "does not repeat" would silently stop motion the tool used to reproduce, so
/// the unknown case keeps the old behaviour and only a recorded `false` terminates.
#[test]
fn a_sequence_that_records_no_repetition_fact_keeps_looping() {
    let legacy = r#"{
      attribute: 'textContent',
      steps: [
        { value: 'Alpha', delay_ms: 300 },
        { value: 'Bravo', delay_ms: 300 },
        { value: 'Charlie', delay_ms: 300 }
      ]
    }"#;
    assert_eq!(replay(legacy, "'Alpha'", 5)["stopped"], Value::Bool(false));
}
