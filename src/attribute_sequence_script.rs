/// The shortest gap between two values that can still be a value sequence.
///
/// Anything tighter is a burst rather than a cadence, and the gate below discards it. The
/// observation loop needs the same number for the mirror-image question — how long a page
/// must be quiet before an unfinished progression counts as finished — because waiting less
/// than this could only ever end a progression the gate was going to discard anyway.
pub const STABLE_GAP_MS: u32 = 250;

/// The capture rule, with the gate threshold spliced in so no copy of it can drift.
pub fn source() -> String {
    TEMPLATE.replace("__STABLE_GAP_MS__", &STABLE_GAP_MS.to_string())
}

const TEMPLATE: &str = r#"

  const attributeSequences = [];
  const sequenceCandidates = [];
  const mutationGroups = new Map();
  const recurringPrefix = values => {
    for (let size = 1; size <= Math.floor(values.length / 2); size++) {
      if (values.every((value, index) => value === values[index % size])) return size;
    }
    return values.length;
  };
  for (const event of window.__recreateAttributeMutations || []) {
    const key = `${event.target}|${event.attribute}`;
    const group = mutationGroups.get(key) || {
      target: event.target, attribute: event.attribute, values: [], times: []
    };
    if (group.values.at(-1) !== event.value) {
      group.values.push(event.value);
      group.times.push(Number(event.time || 0));
    }
    mutationGroups.set(key, group);
  }
  for (const group of mutationGroups.values()) {
    if (group.values.length < 3) continue;
    const gaps = group.times.slice(1).map((time, index) =>
      Math.max(0, time - group.times[index])
    );
    const stableGaps = gaps.filter(value => value >= __STABLE_GAP_MS__);
    if (!stableGaps.length) continue;
    const fallback = Math.round(
      stableGaps.reduce((sum, value) => sum + value, 0) / stableGaps.length
    );
    const cycle = recurringPrefix(group.values);
    sequenceCandidates.push({
      target: group.target,
      attribute: group.attribute,
      // A period shorter than the run is the only evidence that the values came back round.
      // Equal to the run means the loop found no period at all, so the progression was
      // observed to end. This is the one fact that separates a spinner from a reveal, it is
      // knowable only here, and truncation below destroys it.
      repeats: cycle < group.values.length,
      values: group.values.slice(0, cycle),
      interval_ms: fallback,
      steps: group.values.slice(0, cycle).map((value, index) => ({
        value,
        delay_ms: Math.round(gaps[index] > 0 ? Math.max(16, gaps[index]) : fallback)
      }))
    });
  }
  attributeSequences.push(...sequenceCandidates.filter((sequence, index) =>
    sequence.attribute !== 'textContent' ||
    !sequenceCandidates.some((other, otherIndex) =>
      otherIndex !== index &&
      other.attribute === 'textContent' &&
      sequence.target.startsWith(`${other.target}>`)
    )
  ));
"#;

#[cfg(test)]
mod tests {
    use crate::node_eval;
    use serde_json::Value;

    /// Drives the shipped capture rule against recorded mutations, so the emitted candidate is
    /// produced by the same code the browser runs.
    fn emit(mutations: &str, captured_text: &str) -> Value {
        node_eval::evaluate(
            &format!(
                "const window = {{ __recreateAttributeMutations: {mutations} }};\n\
                 const nodes = [{{ path: 'p', text: {captured_text} }}];\n{source}",
                source = super::source()
            ),
            "attributeSequences",
        )
    }

    /// Three values, each seen once. `recurringPrefix` finds no period, so the progression was
    /// observed to end.
    fn one_shot() -> Value {
        emit(
            "[{target:'p',attribute:'textContent',value:'Draft',time:0},\
             {target:'p',attribute:'textContent',value:'Reviewing',time:300},\
             {target:'p',attribute:'textContent',value:'Final',time:600}]",
            "'Final'",
        )
    }

    /// The same three values seen twice round. Identical on every axis the gates measure.
    fn cyclic() -> Value {
        emit(
            "[{target:'p',attribute:'textContent',value:'Alpha',time:0},\
             {target:'p',attribute:'textContent',value:'Bravo',time:300},\
             {target:'p',attribute:'textContent',value:'Charlie',time:600},\
             {target:'p',attribute:'textContent',value:'Alpha',time:900},\
             {target:'p',attribute:'textContent',value:'Bravo',time:1200},\
             {target:'p',attribute:'textContent',value:'Charlie',time:1500}]",
            "'Bravo'",
        )
    }

    #[test]
    fn compresses_repeated_sequence_cycles() {
        assert!(super::source().contains("const recurringPrefix = values"));
        assert!(super::source().contains("value === values[index % size]"));
        assert!(super::source().contains("group.values.slice(0, cycle)"));
        assert!(super::source().contains("Math.max(16, gaps[index])"));
        assert!(super::source().contains("sequence.target.startsWith(`${other.target}>`)"));
    }

    /// The discriminator is computed at the only point it is knowable and was previously used
    /// to truncate and then discarded, so a one-shot of length 3 and a 3-cycle seen once
    /// emitted byte-identical data.
    #[test]
    fn the_two_kinds_of_progression_are_distinguishable_in_the_emitted_data() {
        assert_eq!(one_shot()[0]["repeats"], Value::Bool(false));
        assert_eq!(cyclic()[0]["repeats"], Value::Bool(true));
    }

    /// Capture records the order it observed and nothing more. Rotation is the generator's,
    /// which recomputes it from the captured text either way, so a second copy here could
    /// only drift from it.
    #[test]
    fn capture_records_the_order_it_observed_for_both_kinds() {
        assert_eq!(
            one_shot()[0]["values"],
            serde_json::json!(["Draft", "Reviewing", "Final"])
        );
        assert_eq!(
            cyclic()[0]["values"],
            serde_json::json!(["Alpha", "Bravo", "Charlie"])
        );
    }
}
