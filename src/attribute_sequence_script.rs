pub const SOURCE: &str = r#"
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
    const stableGaps = gaps.filter(value => value >= 250);
    if (!stableGaps.length) continue;
    const fallback = Math.round(
      stableGaps.reduce((sum, value) => sum + value, 0) / stableGaps.length
    );
    const cycle = recurringPrefix(group.values);
    sequenceCandidates.push({
      target: group.target,
      attribute: group.attribute,
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
  for (const sequence of attributeSequences) {
    if (sequence.attribute !== 'textContent') continue;
    const captured = nodes.find(node => node.path === sequence.target)?.text
      ?.replace(/\s+/g, ' ').trim();
    const index = sequence.values.indexOf(captured);
    if (index <= 0) continue;
    sequence.values = sequence.values.slice(index).concat(sequence.values.slice(0, index));
    sequence.steps = sequence.steps.slice(index).concat(sequence.steps.slice(0, index));
  }
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn compresses_repeated_sequence_cycles() {
        assert!(super::SOURCE.contains("const recurringPrefix = values"));
        assert!(super::SOURCE.contains("value === values[index % size]"));
        assert!(super::SOURCE.contains("group.values.slice(0, cycle)"));
        assert!(super::SOURCE.contains("Math.max(16, gaps[index])"));
        assert!(super::SOURCE.contains("sequence.target.startsWith(`${other.target}>`)"));
        assert!(super::SOURCE.contains("nodes.find(node => node.path === sequence.target)?.text"));
        assert!(super::SOURCE.contains("sequence.values.slice(index).concat"));
    }
}
