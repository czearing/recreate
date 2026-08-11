use super::*;
use crate::model::{AttributeSequence, Viewport};

#[test]
fn marks_sequence_target() {
    let mut state: PageState = serde_json::from_value(serde_json::json!({
        "url":"","title":"","viewport":{"width":800,"height":600},
        "nodes":[],"animations":[],"state_styles":[],"css_rules":[],
        "asset_urls":[],"asset_data":{}
    }))
    .unwrap();
    state.viewport = Viewport::default();
    state.attribute_sequences.push(AttributeSequence {
        target: "html>body>textarea".into(),
        attribute: "placeholder".into(),
        values: vec!["First".into(), "Second".into()],
        interval_ms: 4200,
        steps: Vec::new(),
        repeats: None,
    });
    let mut handlers = BTreeMap::new();
    append_handlers(&state, &mut handlers);
    assert_eq!(
        handlers["html>body>textarea"],
        "data-recreate-sequence=\"0\""
    );
}

#[test]
fn upgrades_runtime_for_multiple_sequences() {
    let source = "useEffect(()=>{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{const element=document.querySelector(`[data-recreate-sequence=\"${index}\"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])},sequence.interval_ms)});return()=>timers.forEach(timer=>timer&&clearInterval(timer))},[viewport,state]);".into();
    assert!(runtime(source).contains("startSequences(document"));
}

#[test]
fn upgrades_current_runtime_dependency() {
    assert!(runtime(CURRENT_RUNTIME.into()).contains("startSequences(document"));
}

#[test]
fn serializes_irregular_steps_without_averaging() {
    let mut specification = crate::generate::project_test_support::specification();
    specification.states[0]
        .attribute_sequences
        .push(AttributeSequence {
            target: "html>body>div".into(),
            attribute: "textContent".into(),
            values: vec!["A".into(), "B".into()],
            interval_ms: 99,
            steps: vec![
                crate::model::SequenceStep {
                    value: "A".into(),
                    delay_ms: 4000,
                },
                crate::model::SequenceStep {
                    value: "B".into(),
                    delay_ms: 2750,
                },
            ],
            repeats: None,
        });
    let output = javascript(&specification);
    assert!(output.contains(r#""delay_ms":4000"#));
    assert!(output.contains(r#""delay_ms":2750"#));
}

/// Emits the app's sequence data for a progression on `attribute` whose element was captured
/// holding `captured`, so the tests below differ in nothing but the kind of value written
/// and the recorded repetition fact.
fn sequence(attribute: &str, values: &[&str], captured: &str, repeats: Option<bool>) -> String {
    let mut specification = crate::generate::project_test_support::specification();
    let target = specification.states[0].nodes[3].path.clone();
    specification.states[0].nodes[3].text = captured.into();
    specification.states[0]
        .attribute_sequences
        .push(AttributeSequence {
            target,
            attribute: attribute.into(),
            values: values.iter().map(|value| (*value).into()).collect(),
            interval_ms: 100,
            steps: values
                .iter()
                .map(|value| crate::model::SequenceStep {
                    value: (*value).into(),
                    delay_ms: 900,
                })
                .collect(),
            repeats,
        });
    javascript(&specification)
}

fn order(output: &str, values: &[&str]) -> Vec<usize> {
    values
        .iter()
        .map(|value| output.find(value).unwrap())
        .collect()
}

/// The emitted data is a record of what the page did, so it says the same thing about two
/// channels the page drove identically. Rotating one of them to the captured value and not
/// the other made the record disagree with itself: `aria-label` began at the first value
/// ever seen while `textContent` began at the captured one, though one tick wrote both with
/// one string. Phase belongs to the replay runtime, which recovers it for every kind at
/// once from the element itself.
#[test]
fn a_progression_is_emitted_in_the_order_it_was_observed_whatever_kind_it_writes() {
    let values = ["Alpha", "Bravo", "Charlie"];
    let steps = |attribute| {
        let output = sequence(attribute, &values, "Bravo", Some(true));
        let emitted: serde_json::Value = serde_json::from_str(&output).unwrap();
        emitted[0][0]["steps"].clone()
    };
    assert_eq!(
        steps("textContent"),
        steps("aria-label"),
        "one kind was rotated and the other was not"
    );
    let places = order(
        &sequence("textContent", &values, "Bravo", Some(true)),
        &values,
    );
    assert!(places[0] < places[1] && places[1] < places[2]);
}

/// Rotation relocated the values observed BEFORE the capture to after it, which is what
/// turned a finished progression into one that rewinds rather than merely repeating. The
/// recorded termination fact still reaches the runtime, which is what stops the rewind.
#[test]
fn a_progression_that_never_repeated_keeps_the_order_it_was_observed_in() {
    let output = sequence(
        "textContent",
        &["Draft", "Reviewing", "Final"],
        "Final",
        Some(false),
    );
    let places = order(&output, &["Draft", "Reviewing", "Final"]);
    assert!(places[0] < places[1] && places[1] < places[2]);
    assert!(output.contains(r#""repeats":false"#));
}

/// A cycle carries no termination fact, and the absence must stay an absence: the runtime
/// treats a recorded `false` as the only reason to stop.
#[test]
fn a_repeating_progression_carries_no_termination_fact() {
    let output = sequence(
        "textContent",
        &["Alpha", "Bravo", "Charlie"],
        "Bravo",
        Some(true),
    );
    assert!(!output.contains("\"repeats\""));
}
