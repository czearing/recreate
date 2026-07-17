use crate::model::{PageState, Specification};
use std::collections::BTreeMap;

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
        .map(|state| &state.attribute_sequences)
        .collect::<Vec<_>>();
    serde_json::to_string(&sequences).unwrap()
}

pub fn runtime(source: String) -> String {
    source.replace(
        r#"useEffect(()=>{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{const element=document.querySelector(`[data-recreate-sequence="${index}"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])},sequence.interval_ms)});return()=>timers.forEach(timer=>timer&&clearInterval(timer))},[viewport,state]);"#,
        r#"useEffect(()=>{const timers=[];for(const element of document.querySelectorAll('[data-recreate-sequence]')){for(const value of element.dataset.recreateSequence.split(',')){const sequence=(attributeSequences[viewport]||[])[Number(value)];if(!sequence||sequence.values.length<2)continue;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);timers.push(setInterval(()=>{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])},sequence.interval_ms))}}return()=>timers.forEach(clearInterval)},[viewport,state]);"#,
    )
}

#[cfg(test)]
mod tests {
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
        let output = runtime(source);
        assert!(output.contains("dataset.recreateSequence.split(',')"));
    }
}
