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

/// Emits the app's sequence data for a text progression whose element was captured holding
/// `captured`, so the tests below differ in nothing but the recorded repetition fact.
fn text_sequence(values: &[&str], captured: &str, repeats: Option<bool>) -> String {
    let mut specification = crate::generate::project_test_support::specification();
    let target = specification.states[0].nodes[3].path.clone();
    specification.states[0].nodes[3].text = captured.into();
    specification.states[0]
        .attribute_sequences
        .push(AttributeSequence {
            target,
            attribute: "textContent".into(),
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

#[test]
fn text_sequences_start_from_the_captured_phrase() {
    let output = text_sequence(&["Partial", "Complete phrase"], "Complete phrase", None);
    assert!(output.find("Complete phrase").unwrap() < output.find("Partial").unwrap());
}

/// A cycle has no beginning, so starting it where the capture caught it is correct and must
/// survive the termination fix — this is its positive control.
#[test]
fn a_repeating_progression_still_starts_where_capture_caught_it() {
    let output = text_sequence(&["Alpha", "Bravo", "Charlie"], "Bravo", Some(true));
    let places = order(&output, &["Bravo", "Charlie", "Alpha"]);
    assert!(places[0] < places[1] && places[1] < places[2]);
    assert!(!output.contains("\"repeats\""));
}

/// Rotation relocates the values observed BEFORE the capture to after it, which is what turns
/// a finished progression into one that rewinds rather than merely repeating.
#[test]
fn a_progression_that_never_repeated_keeps_the_order_it_was_observed_in() {
    let output = text_sequence(&["Draft", "Reviewing", "Final"], "Final", Some(false));
    let places = order(&output, &["Draft", "Reviewing", "Final"]);
    assert!(places[0] < places[1] && places[1] < places[2]);
    assert!(output.contains(r#""repeats":false"#));
}
