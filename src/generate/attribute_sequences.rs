//! Emitting the recorded value progressions and marking the elements they write to.
//!
//! Phase — which point of a cycle the page was at when it was captured — is not decided
//! here. It used to be, for `textContent` and for nothing else, by rotating the emitted
//! steps until the captured text came first. That was a second answer to a question the
//! replay runtime already answers for every kind of value at once, by reading back what the
//! element actually holds. Two owners meant the emitted data drifted from the replay: two
//! channels the page drove from one tick with one string serialised as different arrays,
//! which is a false statement about what was observed, and the list of kinds the rotation
//! knew about could only ever fall behind the list of kinds the recorder watches.
//!
//! So this module emits the order the page was observed in and nothing else, and
//! `runtime/sequence.mjs` resumes from whatever the captured DOM holds.

use crate::model::{PageState, Specification};
use std::collections::BTreeMap;

const CURRENT_RUNTIME: &str = r#"useEffect(()=>{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{const element=document.querySelector(`[data-recreate-sequence="${index}"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])},sequence.interval_ms)});return()=>timers.forEach(timer=>timer&&clearInterval(timer))},[viewport]);"#;

pub fn append_handlers(state: &PageState, handlers: &mut BTreeMap<String, String>) {
    let mut targets = BTreeMap::<&str, Vec<usize>>::new();
    for (index, sequence) in state.attribute_sequences.iter().enumerate() {
        targets.entry(&sequence.target).or_default().push(index);
    }
    for (target, indexes) in targets {
        let indexes = indexes
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let marker = format!("data-recreate-sequence=\"{indexes}\"");
        handlers
            .entry(target.to_string())
            .and_modify(|binding| binding.push_str(&format!(" {marker}")))
            .or_insert(marker);
    }
}

pub fn javascript(specification: &Specification) -> String {
    let sequences = specification
        .states
        .iter()
        .map(|state| {
            state
                .attribute_sequences
                .iter()
                .map(|sequence| {
                    let steps: Vec<serde_json::Value> = if sequence.steps.is_empty() {
                        sequence
                            .values
                            .iter()
                            .map(|value| {
                                serde_json::json!({
                                    "value": value,
                                    "delay_ms": sequence.interval_ms
                                })
                            })
                            .collect()
                    } else {
                        sequence
                            .steps
                            .iter()
                            .map(|step| {
                                serde_json::json!({
                                    "value": step.value,
                                    "delay_ms": step.delay_ms
                                })
                            })
                            .collect()
                    };
                    let mut emitted = serde_json::json!({
                        "target": sequence.target,
                        "attribute": sequence.attribute,
                        "steps": steps
                    });
                    if sequence.repeats == Some(false) {
                        emitted["repeats"] = serde_json::Value::Bool(false);
                    }
                    emitted
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&sequences).unwrap()
}

pub fn runtime(source: String) -> String {
    const LEGACY: &str = r#"useEffect(()=>{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{const element=document.querySelector(`[data-recreate-sequence="${index}"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])},sequence.interval_ms)});return()=>timers.forEach(timer=>timer&&clearInterval(timer))},[viewport,state]);"#;
    const UPGRADED: &str = "useEffect(()=>startSequences(document,attributeSequences[viewport]||[]),[viewport,state]);";
    const PREVIOUS_UPGRADED: &str = r#"useEffect(()=>{const apply=(element,sequence,value)=>{if(sequence.attribute==='textContent')element.textContent=value;else element.setAttribute(sequence.attribute,value)};const timers=[];for(const element of document.querySelectorAll('[data-recreate-sequence]')){for(const value of element.dataset.recreateSequence.split(',')){const sequence=(attributeSequences[viewport]||[])[Number(value)];if(!sequence||sequence.values.length<2)continue;let current=0;apply(element,sequence,sequence.values[current]);timers.push(setInterval(()=>{current=(current+1)%sequence.values.length;apply(element,sequence,sequence.values[current])},sequence.interval_ms))}}return()=>timers.forEach(clearInterval)},[viewport,state]);"#;
    source
        .replace(CURRENT_RUNTIME, UPGRADED)
        .replace(LEGACY, UPGRADED)
        .replace(PREVIOUS_UPGRADED, UPGRADED)
}

#[cfg(test)]
#[path = "attribute_sequence_tests.rs"]
mod tests;
