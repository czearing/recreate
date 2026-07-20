pub const SOURCE: &str = r#"
  const attributeSequences = [];
  const mutationGroups = new Map();
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
    attributeSequences.push({
      target: group.target,
      attribute: group.attribute,
      values: group.values,
      interval_ms: fallback,
      steps: group.values.map((value, index) => ({
        value,
        delay_ms: Math.round(gaps[index] >= 250 ? gaps[index] : fallback)
      }))
    });
  }
"#;
