(async () => {
__LIFECYCLE_SETTLE__
  const reading = () => {
    const events = window.__recreateAttributeMutations || [];
    const groups = new Map();
    for (const event of events) {
      const key = `${event.target}|${event.attribute}`;
      const values = groups.get(key) || [];
      if (values.at(-1) !== event.value) values.push(event.value);
      groups.set(key, values);
    }
    let pending = 0;
    for (const values of groups.values()) {
      let cycle = values.length;
      for (let size = 1; size <= Math.floor(values.length / 2); size++) {
        if (values.every((value, index) => value === values[index % size])) {
          cycle = size;
          break;
        }
      }
      if (values.length < 3 || cycle === values.length) pending++;
    }
    return `${events.length}:${pending}`;
  };
  const frame = () => new Promise(resolve => requestAnimationFrame(() => resolve()));
  const started = Date.now();
  let previous = reading();
  let lastChange = started;
  let longestGap = 0;
  for (;;) {
    await frame();
    const now = Date.now();
    const current = reading();
    if (current !== previous) {
      previous = current;
      longestGap = Math.max(longestGap, now - lastChange);
      lastChange = now;
    }
    const pending = Number(current.split(':')[1]) > 0;
    if (lifecycleSettled(now - started, now - lastChange, pending, longestGap)) {
      return now - started;
    }
  }
})()
