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
        });
    let output = javascript(&specification);
    assert!(output.contains(r#""delay_ms":4000"#));
    assert!(output.contains(r#""delay_ms":2750"#));
}

#[test]
fn text_sequences_start_from_the_captured_phrase() {
    let mut specification = crate::generate::project_test_support::specification();
    let target = specification.states[0].nodes[3].path.clone();
    specification.states[0].nodes[3].text = "Complete phrase".into();
    specification.states[0]
        .attribute_sequences
        .push(AttributeSequence {
            target,
            attribute: "textContent".into(),
            values: vec!["Partial".into(), "Complete phrase".into()],
            interval_ms: 100,
            steps: vec![
                crate::model::SequenceStep {
                    value: "Partial".into(),
                    delay_ms: 20,
                },
                crate::model::SequenceStep {
                    value: "Complete phrase".into(),
                    delay_ms: 900,
                },
            ],
        });
    let output = javascript(&specification);
    assert!(output.find("Complete phrase").unwrap() < output.find("Partial").unwrap());
}
