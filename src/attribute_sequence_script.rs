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
    if (group.values.length < 2) continue;
    const gaps = group.times.slice(1)
      .map((time, index) => time - group.times[index])
      .filter(value => value >= 250);
    attributeSequences.push({
      target: group.target,
      attribute: group.attribute,
      values: group.values,
      interval_ms: Math.round(
        gaps.reduce((sum, value) => sum + value, 0) / gaps.length
      ) || 1000
    });
  }
"#;
